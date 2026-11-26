//! Writing a realm back into its Btrieve `WCC*.DAT` files (M5).
//!
//! Each `.db` goes back into the Btrieve file it was converted from, found by its stem in the
//! WCC folder (`WCCITEMS.db` and `WCCITEMS.DAT`, or `WCCITEMS.VIR` where there is no `.DAT`).
//!
//! **Only the edits.** The game changes its own files as it runs (rooms' floors, shop stock,
//! players), so a realm whose changes Daydream journaled ([`crate::journal::BASE`], complete
//! since the realm was made from the files or last written into them) writes only what it
//! changed, onto the files as they are now: each edited record's changed members merged onto
//! the game's record of the same key ([`crate::three_way`]), added records added, deleted
//! ones deleted, renumbered ones in place of the record they were. A record the game no longer has, or a key it took meanwhile, is skipped,
//! reported, and kept among the realm's changes. Files with no edits are not written, however
//! the game changed them.
//!
//! **Whole.** A realm edited before Daydream kept journals, and the update file (which the
//! game never writes; Daydream keeps it in step, [`crate::update`]), are written as they are,
//! and refused when the game's file changed since the conversion, unless `force`.
//!
//! Either way the file is written the least intrusive way:
//!
//! - **unchanged**: the same records in the same order. The file is not written.
//! - **updated**: the same number of records and no key changed. The changed records are
//!   overwritten in their slots ([`update_in_place`]), so the file's bytes change only
//!   where its records did.
//! - **rebuilt**: records added, deleted or re-keyed. The file is written anew, in its
//!   records' order (added ones last), with the source's page size, keys and flags
//!   ([`build`]).
//!
//! A rebuilt record moves to a new position, which only a running game would notice: the
//! game must be stopped while its files are written. The realm's update file is brought in
//! step with its edits first ([`crate::update::sync`]), so the game's next start applies them
//! again rather than undoing them; and once written, the realm's changes count afresh.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use daydream_types::records::layout::{Layout, LayoutSet};
use daydream_wcc::btrieve::build::{BuildError, build};
use daydream_wcc::btrieve::update::{UpdateError, update_in_place};
use daydream_wcc::btrieve::{BtrieveFile, OpenOptions, RecordAddress};
use daydream_wcc::tables::Table;

use crate::db::{Fingerprint, StoreError, WbtrvDb};
use crate::journal;
use crate::realm::{Realm, layout_of};

/// What writing one file did, or would do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The records are those of the Btrieve file: nothing written.
    Unchanged,
    /// This many records overwritten in place.
    Updated(usize),
    /// Written anew with this many records.
    Rebuilt(usize),
}

/// One `.db` written back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub stem: String,
    /// The Btrieve file the `.db` was compared with.
    pub source: PathBuf,
    /// The file written (or that would be). A `.VIR` source is written as a `.DAT`.
    pub dest: PathBuf,
    pub action: Action,
    /// Only the realm's edits went in, merged onto the file as it is (else it was written
    /// whole).
    pub merged: bool,
    /// Edits not written, and why (`item 12: the game no longer has it`). They stay among
    /// the realm's changes, to be written once resolved.
    pub skipped: Vec<String>,
}

/// Why a realm could not be written back.
#[derive(Debug, thiserror::Error)]
pub enum DatError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Btrieve(#[from] daydream_wcc::btrieve::BtrieveError),
    #[error("{}: {source}", file.display())]
    Build {
        file: PathBuf,
        #[source]
        source: BuildError,
    },
    #[error(transparent)]
    Update(UpdateError),
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{stem}: no {stem}.DAT or {stem}.VIR in {}", dir.display())]
    NoSource { stem: String, dir: PathBuf },
    #[error(
        "{}: changed since the realm's {stem}.db was converted from it (the game ran on it?); \
         writing the realm would lose those changes",
        file.display()
    )]
    SourceChanged { stem: String, file: PathBuf },
    #[error("the realm has no update file (WCCUPDAT or WCCUPDA2)")]
    NoUpdateFile,
    #[error(
        "the update file cannot carry {}: write the files with the game stopped instead",
        .0.join("; ")
    )]
    NotCarried(Vec<String>),
}

/// Where and how to write.
#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    /// Write the files here instead of over the WCC folder. Files that need no change are
    /// copied, so the folder ends up a complete set.
    pub out: Option<PathBuf>,
    /// Writing over the WCC folder, copy each file to this folder before changing it.
    pub backup: Option<PathBuf>,
    /// Only report what would be done.
    pub dry_run: bool,
    /// Writing whole, write even over a Btrieve file that changed since its `.db` was
    /// converted.
    pub force: bool,
}

/// The Btrieve file of `stem` in `dir`: its `.DAT`, else its `.VIR`.
fn source_of(dir: &Path, stem: &str) -> Result<Option<PathBuf>, DatError> {
    let io = |source| DatError::Io {
        path: dir.to_path_buf(),
        source,
    };
    let (mut dat, mut vir) = (None, None);
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let entry = entry.map_err(io)?;
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if let Some((s, ext)) = name.rsplit_once('.')
            && s == stem
        {
            match ext {
                "DAT" => dat = Some(entry.path()),
                "VIR" => vir = Some(entry.path()),
                _ => {}
            }
        }
    }
    Ok(dat.or(vir))
}

/// The file a source is written as: itself for a `.DAT`, the `.DAT` beside it for a `.VIR`.
fn dat_name(source: &Path) -> PathBuf {
    let is_vir = source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("vir"));
    if is_vir {
        source.with_extension("DAT")
    } else {
        source.to_path_buf()
    }
}

fn read_records(file: &mut BtrieveFile) -> Result<Vec<(RecordAddress, Vec<u8>)>, DatError> {
    let mut out = Vec::new();
    let mut cursor = file.records();
    while let Some(record) = cursor.next_record() {
        let record = record?;
        out.push((RecordAddress::of(&record), record.data.to_vec()));
    }
    Ok(out)
}

/// Whether `records` can go in place of `old`: the same number, fixed-length, every key
/// as it was. The changed records with their positions, or `None` for a rebuild.
fn in_place(
    file: &BtrieveFile,
    old: &[(RecordAddress, Vec<u8>)],
    records: &[Vec<u8>],
) -> Option<Vec<(RecordAddress, Vec<u8>)>> {
    if file.header.is_variable_length() || old.len() != records.len() {
        return None;
    }
    let len = usize::from(file.header.record_length);
    let mut changed = Vec::new();
    for ((at, before), after) in old.iter().zip(records) {
        if before == after {
            continue;
        }
        if after.len() != len
            || file
                .keys
                .iter()
                .any(|k| k.extract(before) != k.extract(after))
        {
            return None;
        }
        changed.push((*at, after.clone()));
    }
    Some(changed)
}

fn copy(from: &Path, to: &Path) -> Result<(), DatError> {
    std::fs::copy(from, to)
        .map(drop)
        .map_err(|source| DatError::Io {
            path: to.to_path_buf(),
            source,
        })
}

/// Writes `bytes` to `path` through a temporary file beside it, so a failure leaves the
/// old file whole.
fn replace(path: &Path, bytes: &[u8]) -> Result<(), DatError> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".daydream-tmp");
    let tmp = PathBuf::from(tmp);
    let io = |source| DatError::Io {
        path: path.to_path_buf(),
        source,
    };
    std::fs::write(&tmp, bytes).map_err(io)?;
    std::fs::rename(&tmp, path).map_err(io)
}

fn now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as u32)
}

/// One record's change since the realm's base, as the journal and the `.db` give it.
enum Change {
    Edit { base: Vec<u8>, now: Vec<u8> },
    Add(Vec<u8>),
    Delete(Vec<u8>),
}

/// The changes of the `.db`'s records that `entries` hold, by record id. An entry whose
/// record is as it was is no change.
fn changes_of(db: &WbtrvDb, entries: &[journal::Entry]) -> Result<Vec<(i64, Change)>, StoreError> {
    let mut out = Vec::new();
    for e in entries {
        let now = db.get(e.id)?;
        let change = match (e.before.clone(), now) {
            (Some(base), Some(now)) if base != now => Change::Edit { base, now },
            (None, Some(now)) => Change::Add(now),
            (Some(base), None) => Change::Delete(base),
            _ => continue,
        };
        out.push((e.id, change));
    }
    Ok(out)
}

/// A record, for a person: `item 12`, `room 1/5`, else its first key's value.
fn describe(file: &BtrieveFile, table: Option<Table>, set: LayoutSet, record: &[u8]) -> String {
    if let Some(kind) = table.and_then(crate::update::Kind::of) {
        return crate::update::describe(kind, set, record);
    }
    let key = file
        .keys
        .first()
        .and_then(|k| k.extract(record))
        .unwrap_or_default();
    let text: String = key
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    if !text.is_empty() && text.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return format!("record {text}");
    }
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    format!("record {hex}")
}

/// How a record is found in the game's file: by its value of the file's first unique key,
/// else (a file with no unique key, a record whose key is null) by all its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Ident {
    Key(Vec<u8>),
    Bytes(Vec<u8>),
}

/// The file's records once the changes are merged onto the game's (`live`, in its order;
/// added records after), and the changes skipped (by record id, with why).
fn merge_onto(
    file: &BtrieveFile,
    live: &[(RecordAddress, Vec<u8>)],
    changes: &[(i64, Change)],
    layout: Option<&Layout>,
    what: impl Fn(&[u8]) -> String,
) -> (Vec<Vec<u8>>, Vec<(i64, String)>) {
    let unique = file.keys.iter().find(|k| !k.allows_duplicates());
    let ident = |r: &[u8]| match unique.filter(|k| !k.is_null(r)).and_then(|k| k.identity(r)) {
        Some(v) => Ident::Key(v),
        None => Ident::Bytes(r.to_vec()),
    };
    let mut target: Vec<Option<Vec<u8>>> = live.iter().map(|(_, r)| Some(r.clone())).collect();
    let mut by_ident: HashMap<Ident, Vec<usize>> = HashMap::new();
    for (i, (_, r)) in live.iter().enumerate() {
        by_ident.entry(ident(r)).or_default().push(i);
    }
    // the record found by `id` still in the file; of several, the one like `like`
    let find = |by_ident: &HashMap<Ident, Vec<usize>>,
                target: &[Option<Vec<u8>>],
                id: &Ident,
                like: Option<&[u8]>| {
        let found: Vec<usize> = by_ident
            .get(id)
            .map(|v| v.iter().copied().filter(|&i| target[i].is_some()).collect())
            .unwrap_or_default();
        found
            .iter()
            .copied()
            .find(|&i| like.is_some_and(|l| target[i].as_deref() == Some(l)))
            .or_else(|| found.first().copied())
    };
    let mut skipped = Vec::new();
    for (rid, c) in changes {
        match c {
            Change::Edit { base, now } => {
                let (ib, inow) = (ident(base), ident(now));
                let Some(i) = find(&by_ident, &target, &ib, Some(base)) else {
                    let why = match ib {
                        Ident::Key(_) => "the game no longer has it",
                        Ident::Bytes(_) => "the game changed it meanwhile",
                    };
                    skipped.push((
                        *rid,
                        format!("{}: {why}, so the edit was not written", what(base)),
                    ));
                    continue;
                };
                if let (Ident::Key(_), Ident::Key(_)) = (&ib, &inow)
                    && ib != inow
                {
                    // renumbered: to the game another record, which replaces the one it was
                    // (nothing of the game's record carries over)
                    if find(&by_ident, &target, &inow, None).is_some() {
                        skipped.push((
                            *rid,
                            format!(
                                "{}: renumbered to {}, which the game has meanwhile",
                                what(base),
                                what(now)
                            ),
                        ));
                        continue;
                    }
                    target[i] = None;
                    by_ident.entry(inow).or_default().push(target.len());
                    target.push(Some(now.clone()));
                    continue;
                }
                if ib != inow {
                    if let Some(v) = by_ident.get_mut(&ib) {
                        v.retain(|&j| j != i);
                    }
                    by_ident.entry(inow).or_default().push(i);
                }
                let theirs = target[i].clone().unwrap_or_default();
                target[i] = Some(crate::three_way::merge(layout, base, now, &theirs));
            }
            Change::Add(now) => {
                let inow = ident(now);
                if matches!(inow, Ident::Key(_)) && find(&by_ident, &target, &inow, None).is_some()
                {
                    skipped.push((
                        *rid,
                        format!(
                            "{}: the game has added one of its own meanwhile, so it was not added",
                            what(now)
                        ),
                    ));
                    continue;
                }
                by_ident.entry(inow).or_default().push(target.len());
                target.push(Some(now.clone()));
            }
            Change::Delete(base) => {
                let ib = ident(base);
                match find(&by_ident, &target, &ib, Some(base)) {
                    Some(i) => target[i] = None,
                    // the game deleted it too
                    None if matches!(ib, Ident::Key(_)) => {}
                    None => skipped.push((
                        *rid,
                        format!(
                            "{}: the game changed it meanwhile, so it was not deleted",
                            what(base)
                        ),
                    )),
                }
            }
        }
    }
    (target.into_iter().flatten().collect(), skipped)
}

/// Writes every `.db` of `realm` back into the Btrieve files of the WCC folder `wcc` (or
/// into `options.out`): only its edits when it journaled them, else whole (see the module).
/// Every file is checked before any is written, so a refusal leaves everything as it was.
pub fn write_dat(
    realm: &Realm,
    wcc: &Path,
    options: &WriteOptions,
) -> Result<Vec<Written>, DatError> {
    write(realm, wcc, options, None)
}

/// The changes of `realm` that its update file cannot carry into the game, each with why
/// (`item 12: the update file cannot delete a record`); empty when it carries them all
/// ([`crate::update::carries`]).
pub fn not_carried(realm: &Realm) -> Result<Vec<String>, DatError> {
    let changes = crate::changes::list(realm)?;
    if changes.since.is_none() {
        return Ok(vec![
            "the edits made before Daydream kept a list of them, which are not known".into(),
        ]);
    }
    let set = realm.layout_set();
    let room = crate::update::UpdateFile::find(realm)?.map(|f| f.room());
    let mut out = Vec::new();
    for c in &changes.changes {
        let why = match crate::update::carries(c, set) {
            Err(why) => Some(why),
            Ok(()) if room.is_some_and(|room| c.record.len() > room) => {
                Some("too long for one record of the update file".into())
            }
            Ok(()) => None,
        };
        if let Some(why) = why {
            out.push(format!("{}: {why}", c.what));
        }
    }
    Ok(out)
}

/// Writes only the realm's update file (`WCCUPDAT`, `WCCUPDA2` on Worldgroup) into the WCC
/// folder, for the game to merge into its world at its next start or nightly cleanup: a
/// publish that needs no stop, the game's own merge keeping what play changed. Every change
/// must be one the update file carries ([`not_carried`]), else nothing is written; once
/// written, the realm's changes count afresh, as the game now has them to apply.
pub fn write_update_file(
    realm: &Realm,
    wcc: &Path,
    options: &WriteOptions,
) -> Result<Written, DatError> {
    let not = not_carried(realm)?;
    if !not.is_empty() {
        return Err(DatError::NotCarried(not));
    }
    let stem = crate::update::UpdateFile::find(realm)?
        .ok_or(DatError::NoUpdateFile)?
        .stem;
    let mut written = write(realm, wcc, options, Some(&stem))?;
    written.pop().ok_or(DatError::NoUpdateFile)
}

/// [`write_dat`], for every file or only the one of stem `only`.
fn write(
    realm: &Realm,
    wcc: &Path,
    options: &WriteOptions,
    only: Option<&str>,
) -> Result<Vec<Written>, DatError> {
    struct Plan {
        written: Written,
        db: PathBuf,
        bytes: Option<Vec<u8>>,
        updates: Vec<(RecordAddress, Vec<u8>)>,
    }
    // the update file must carry the edits, or the game's next start undoes them
    crate::update::sync(realm.dir())?;
    let set = realm.layout_set();
    let edits: Option<BTreeMap<String, Vec<journal::Entry>>> =
        match journal::established(realm.dir())? {
            Some(_) => {
                let mut by_stem: BTreeMap<String, Vec<journal::Entry>> = BTreeMap::new();
                for e in journal::pending(realm.dir(), journal::BASE)?.entries {
                    by_stem.entry(e.stem.clone()).or_default().push(e);
                }
                Some(by_stem)
            }
            None => None,
        };
    let mut plans = Vec::new();
    // the changes not written, which stay pending
    let mut keep: Vec<(String, i64)> = Vec::new();
    for f in realm.files() {
        if only.is_some_and(|only| only != f.stem) {
            continue;
        }
        let source = source_of(wcc, &f.stem)?.ok_or_else(|| DatError::NoSource {
            stem: f.stem.clone(),
            dir: wcc.to_path_buf(),
        })?;
        let db = WbtrvDb::open_read_only(&f.path)?;
        let mut file = OpenOptions::new().variable_length(true).open(&source)?;
        let is_update_file = matches!(f.stem.as_str(), "WCCUPDAT" | "WCCUPDA2");
        let merged = edits.is_some() && !is_update_file && !file.keys.is_empty();
        if !merged
            && !options.force
            && let Some(recorded) = db.source_fingerprint()?
        {
            let now = Fingerprint::of(&source).map_err(|e| DatError::Io {
                path: source.clone(),
                source: e,
            })?;
            if (now.size, now.hash) != (recorded.size, recorded.hash) {
                return Err(DatError::SourceChanged {
                    stem: f.stem.clone(),
                    file: source,
                });
            }
        }
        let old = read_records(&mut file)?;
        let mut skipped: Vec<(i64, String)> = Vec::new();
        let records: Vec<Vec<u8>> = if merged {
            let entries = edits
                .as_ref()
                .and_then(|e| e.get(&f.stem))
                .map_or(&[][..], Vec::as_slice);
            let changes = changes_of(&db, entries)?;
            if changes.is_empty() {
                old.iter().map(|(_, r)| r.clone()).collect()
            } else {
                let layout = f.table.map(|t| layout_of(t, set));
                let (records, s) = merge_onto(&file, &old, &changes, layout, |r| {
                    describe(&file, f.table, set, r)
                });
                skipped = s;
                records
            }
        } else {
            db.records()?.into_iter().map(|(_, r)| r).collect()
        };
        let unchanged =
            old.len() == records.len() && old.iter().zip(&records).all(|(a, b)| a.1 == *b);
        let (action, bytes, updates) = if unchanged {
            (Action::Unchanged, None, Vec::new())
        } else if let Some(updates) = in_place(&file, &old, &records) {
            (Action::Updated(updates.len()), None, updates)
        } else {
            let bytes = build(&file, &records, now()).map_err(|source| DatError::Build {
                file: f.path.clone(),
                source,
            })?;
            (Action::Rebuilt(records.len()), Some(bytes), Vec::new())
        };
        let name = dat_name(&source);
        let dest = match &options.out {
            Some(out) => out.join(name.file_name().unwrap()),
            None => name,
        };
        keep.extend(skipped.iter().map(|(id, _)| (f.stem.clone(), *id)));
        plans.push(Plan {
            written: Written {
                stem: f.stem.clone(),
                source,
                dest,
                action,
                merged,
                skipped: skipped.into_iter().map(|(_, why)| why).collect(),
            },
            db: f.path.clone(),
            bytes,
            updates,
        });
    }
    if options.dry_run {
        return Ok(plans.into_iter().map(|p| p.written).collect());
    }

    let create = |dir: &PathBuf| {
        std::fs::create_dir_all(dir).map_err(|source| DatError::Io {
            path: dir.clone(),
            source,
        })
    };
    if let Some(out) = &options.out {
        create(out)?;
    }
    for p in &plans {
        let w = &p.written;
        let over_source = w.dest == w.source;
        if options.out.is_none() && w.action == Action::Unchanged {
            continue;
        }
        if options.out.is_none()
            && let Some(backup) = &options.backup
            && w.dest.exists()
        {
            create(backup)?;
            copy(&w.dest, &backup.join(w.dest.file_name().unwrap()))?;
        }
        match (&p.bytes, w.action) {
            (Some(bytes), _) => replace(&w.dest, bytes)?,
            (None, _) if !over_source => {
                copy(&w.source, &w.dest)?;
                if !p.updates.is_empty() {
                    update_in_place(&w.dest, &p.updates).map_err(DatError::Update)?;
                }
            }
            (None, _) => {
                update_in_place(&w.dest, &p.updates).map_err(DatError::Update)?;
            }
        }
        // a file written whole: the .db now holds what its WCC file holds. Record that, so
        // the next write compares with the file just written
        if options.out.is_none() && !w.merged {
            let fp = Fingerprint::of(&w.dest).map_err(|source| DatError::Io {
                path: w.dest.clone(),
                source,
            })?;
            WbtrvDb::open(&p.db)?.set_source_fingerprint(fp)?;
        }
    }
    // the game's files now hold the realm's edits: its changes count from here, but for
    // those that could not be written
    if options.out.is_none() {
        journal::establish(realm.dir(), &keep)?;
    }
    Ok(plans.into_iter().map(|p| p.written).collect())
}
