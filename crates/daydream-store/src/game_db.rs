//! Writing a working copy back into a game's own `.db` files: MBBSEmu's, or wbtrv32's beside
//! a Worldgroup.
//!
//! Such a game keeps its world in wbtrv32's `.db` files and writes them as it runs. Daydream
//! edits a copy of them, taken while the game runs ([`WbtrvDb::snapshot`]), and writes only
//! the copy's edits back, with the game stopped, as it does into a Btrieve game's files
//! ([`crate::dat`]):
//!
//! - an edited record's changed members are merged onto the game's record of the same key
//!   ([`crate::three_way`]) and written where it is: the record keeps its id, the position
//!   MajorMUD remembers;
//! - an added record is added at the next id (as wbtrv32 would add it), a deleted one is
//!   deleted, and a renumbered one replaces the record it was;
//! - a record the game no longer has, or a key the game took meanwhile, is skipped, reported
//!   and kept among the copy's changes, to be written once resolved.
//!
//! A record is found among the game's by its value of the file's first unique key, as wbtrv32
//! stores it (a record whose key is null, or a file with no unique key: by all its bytes). The
//! update file, which the game never writes and Daydream keeps in step with the copy
//! ([`crate::update::sync`]), is written whole, every record at its own id. A file without
//! edits is not written. Each file is written in one transaction, so a failure leaves it as it
//! was, and as the game keeps it: its journal mode (wbtrv32's write-ahead log, or MBBSEmu's
//! rollback journal) is left alone. Once written, the copy's changes count afresh.
//!
//! The game must be stopped: it keeps much of its world in memory as it plays and writes it
//! back over anything written meanwhile.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::dat::{self, Action, Change, DatError, Ident, WriteOptions, Written};
use crate::db::{StoreError, WbtrvDb};
use crate::journal;
use crate::key::{Key, KeyValue};
use crate::realm::{Realm, layout_of};

/// What to do to one of the game's files.
enum Ops {
    /// Nothing: its records are already the copy's.
    None,
    /// Every record replaced by these, each at its id (the update file).
    Whole(Vec<(i64, Vec<u8>)>),
    /// The copy's edits, merged onto the game's records.
    Edits {
        /// `(id, as the game has it, as it is written)`.
        put: Vec<(i64, Vec<u8>, Vec<u8>)>,
        delete: Vec<i64>,
        add: Vec<Vec<u8>>,
    },
}

/// Writes the edits of the working copy `realm` into the `.db` files of the game folder
/// `game` (see the module); with `options.backup`, each file about to be written is copied
/// there first. Every file is read and merged before any is written, so a file the game
/// lacks, or a refusal, leaves them all as they were. `options.out` does not apply: the
/// files are written where they are.
pub fn write_db(
    realm: &Realm,
    game: &Path,
    options: &WriteOptions,
) -> Result<Vec<Written>, DatError> {
    if let Some(out) = &options.out {
        return Err(DatError::NotElsewhere(out.clone()));
    }
    // the update file must carry the edits, or the game's next start undoes them
    crate::update::sync(realm.dir())?;
    if journal::established(realm.dir())?.is_none() {
        return Err(DatError::NoJournal);
    }
    let mut edits: BTreeMap<String, Vec<journal::Entry>> = BTreeMap::new();
    for e in journal::pending(realm.dir(), journal::BASE)?.entries {
        edits.entry(e.stem.clone()).or_default().push(e);
    }
    let theirs = Realm::open(game)?;
    let set = realm.layout_set();
    let mut plans: Vec<(Written, Ops)> = Vec::new();
    // the changes not written, which stay pending
    let mut keep: Vec<(String, i64)> = Vec::new();
    for f in realm.files() {
        let target = theirs
            .file(&f.stem)
            .ok_or_else(|| DatError::NoDb {
                stem: f.stem.clone(),
                dir: game.to_path_buf(),
            })?
            .path
            .clone();
        let ours = WbtrvDb::open_read_only(&f.path)?;
        let db = WbtrvDb::open_read_only(&target)?;
        let mut skipped: Vec<(i64, String)> = Vec::new();
        let ops = if matches!(f.stem.as_str(), "WCCUPDAT" | "WCCUPDA2") {
            let records = ours.records()?;
            if records == db.records()? {
                Ops::None
            } else {
                Ops::Whole(records)
            }
        } else {
            let entries = edits.get(&f.stem).map_or(&[][..], Vec::as_slice);
            let mut changes = dat::changes_of(&ours, entries)?;
            if changes.is_empty() {
                Ops::None
            } else {
                let live = db.records()?;
                let views: Vec<&[u8]> = live.iter().map(|(_, r)| r.as_slice()).collect();
                let keys = db.keys();
                let layout = f.table.map(|t| layout_of(t, set));
                let what = |r: &[u8]| dat::describe(f.table, set, r, &first_key(keys, r));
                skipped = taken(keys, &views, &mut changes, &what);
                let merged = dat::merge_onto(&views, &changes, ident(keys), layout, what);
                skipped.extend(merged.skipped);
                let (mut put, mut delete, mut add) = (Vec::new(), Vec::new(), Vec::new());
                for (i, record) in merged.records.into_iter().enumerate() {
                    match (live.get(i), record) {
                        (Some((id, _)), None) => delete.push(*id),
                        (Some((id, was)), Some(now)) if now != *was => {
                            put.push((*id, was.clone(), now));
                        }
                        (None, Some(now)) => add.push(now),
                        _ => {}
                    }
                }
                if put.is_empty() && delete.is_empty() && add.is_empty() {
                    Ops::None
                } else {
                    Ops::Edits { put, delete, add }
                }
            }
        };
        let action = match &ops {
            Ops::None => Action::Unchanged,
            Ops::Whole(records) => Action::Rebuilt(records.len()),
            Ops::Edits { put, delete, add } => Action::Rows {
                changed: put.len(),
                added: add.len(),
                deleted: delete.len(),
            },
        };
        keep.extend(skipped.iter().map(|(id, _)| (f.stem.clone(), *id)));
        plans.push((
            Written {
                stem: f.stem.clone(),
                source: target.clone(),
                dest: target,
                action,
                merged: !matches!(ops, Ops::Whole(_)),
                skipped: skipped.into_iter().map(|(_, why)| why).collect(),
            },
            ops,
        ));
    }
    if options.dry_run {
        return Ok(plans.into_iter().map(|(w, _)| w).collect());
    }

    // every file about to be written backed up before any is
    if let Some(backup) = &options.backup {
        for (w, _) in plans.iter().filter(|(w, _)| w.action != Action::Unchanged) {
            std::fs::create_dir_all(backup).map_err(|source| DatError::Io {
                path: backup.clone(),
                source,
            })?;
            let name = w.dest.file_name().unwrap_or_default();
            WbtrvDb::snapshot(&w.dest, &backup.join(name))?;
        }
    }
    for (w, ops) in &plans {
        if matches!(ops, Ops::None) {
            continue;
        }
        // as the game keeps it, its journal mode too
        let mut db = WbtrvDb::open_game(&w.dest)?;
        db.begin()?;
        let written = apply(&mut db, ops);
        match written {
            Ok(()) => db.commit()?,
            Err(e) => {
                let _ = db.rollback();
                return Err(e.into());
            }
        }
    }
    // the game's files now hold the copy's edits: its changes count from here, but for
    // those that could not be written
    journal::establish(realm.dir(), &keep)?;
    Ok(plans.into_iter().map(|(w, _)| w).collect())
}

/// Makes `ops` in `db`: deletions first, so the keys they free can be taken.
fn apply(db: &mut WbtrvDb, ops: &Ops) -> Result<(), StoreError> {
    match ops {
        Ops::None => {}
        Ops::Whole(records) => {
            db.clear()?;
            for (id, record) in records {
                db.restore(*id, record)?;
            }
        }
        Ops::Edits { put, delete, add } => {
            for id in delete {
                db.delete(*id)?;
            }
            for (id, was, now) in put {
                // a key the game never changes is still Daydream's to change: the record
                // goes back at its id rather than through the update wbtrv32 refuses
                let fixed = db
                    .keys()
                    .iter()
                    .any(|k| !k.is_modifiable() && k.values(was) != k.values(now));
                if fixed {
                    db.delete(*id)?;
                    db.restore(*id, now)?;
                } else {
                    db.update(*id, now)?;
                }
            }
            for record in add {
                db.insert(record)?;
            }
        }
    }
    Ok(())
}

/// Takes out of `changes` the records added whose value of a unique key other than the first
/// (an auto-increment number, say) one of the game's records has: the game added it
/// meanwhile, and it cannot be had twice. Those skipped, with why. (The first unique key is
/// how records are found, and [`dat::merge_onto`] checks it.)
fn taken(
    keys: &[Key],
    live: &[&[u8]],
    changes: &mut Vec<(i64, Change)>,
    what: &impl Fn(&[u8]) -> String,
) -> Vec<(i64, String)> {
    let others: Vec<(usize, &Key)> = keys
        .iter()
        .enumerate()
        .filter(|(_, k)| k.is_unique())
        .skip(1)
        .collect();
    let value = |n: usize, k: &Key, r: &[u8]| {
        k.values(r)
            .ok()
            .filter(|v| !v.iter().any(|v| matches!(v, KeyValue::Null)))
            .map(|v| (n, encode(&v)))
    };
    let held: HashSet<(usize, Vec<u8>)> = live
        .iter()
        .flat_map(|r| others.iter().filter_map(|&(n, k)| value(n, k, r)))
        .collect();
    let mut skipped = Vec::new();
    changes.retain(|(id, c)| match c {
        Change::Add(now)
            if others
                .iter()
                .any(|&(n, k)| value(n, k, now).is_some_and(|v| held.contains(&v))) =>
        {
            skipped.push((
                *id,
                format!(
                    "{}: one the game added meanwhile has the same key, so it was not added",
                    what(now)
                ),
            ));
            false
        }
        _ => true,
    });
    skipped
}

/// How a record is found among the game's: by its first unique key's values as wbtrv32 stores
/// them (collated, so as its index tells keys apart), else by all its bytes.
fn ident(keys: &[Key]) -> impl Fn(&[u8]) -> Ident + '_ {
    let unique = keys.iter().find(|k| k.is_unique());
    move |r: &[u8]| {
        let values = unique
            .and_then(|k| k.values(r).ok())
            .filter(|v| !v.iter().all(|v| matches!(v, KeyValue::Null)));
        match values {
            Some(v) => Ident::Key(encode(&v)),
            None => Ident::Bytes(r.to_vec()),
        }
    }
}

/// Key column values as bytes that compare as the values do.
fn encode(values: &[KeyValue]) -> Vec<u8> {
    let mut out = Vec::new();
    for v in values {
        match v {
            KeyValue::Null => out.push(0),
            KeyValue::Integer(i) => {
                out.push(1);
                out.extend(i.to_le_bytes());
            }
            KeyValue::Real(f) => {
                out.push(2);
                out.extend(f.to_bits().to_le_bytes());
            }
            KeyValue::Text(t) => {
                out.push(3);
                out.extend((t.len() as u64).to_le_bytes());
                out.extend(t);
            }
            KeyValue::Blob(b) => {
                out.push(4);
                out.extend((b.len() as u64).to_le_bytes());
                out.extend(b);
            }
        }
    }
    out
}

/// The bytes of a record's first key, to name the record by.
fn first_key(keys: &[Key], record: &[u8]) -> Vec<u8> {
    let Some(k) = keys.first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for s in &k.segments {
        let at = usize::from(s.offset).min(record.len());
        let end = (at + usize::from(s.length)).min(record.len());
        out.extend_from_slice(&record[at..end]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_values_encode_apart() {
        let a = encode(&[KeyValue::Integer(1)]);
        let b = encode(&[KeyValue::Text(b"\x01\0\0\0\0\0\0\0".to_vec())]);
        assert_ne!(a, b);
        assert_ne!(
            encode(&[
                KeyValue::Text(b"ab".to_vec()),
                KeyValue::Text(b"c".to_vec())
            ]),
            encode(&[
                KeyValue::Text(b"a".to_vec()),
                KeyValue::Text(b"bc".to_vec())
            ])
        );
    }
}
