//! A realm's changes: what it changed since it was last read from its game or written back
//! into it ([`crate::journal::BASE`]), which is what writing it back writes ([`crate::dat`]).
//! Listed for a person, record by record, and taken back.

use std::collections::HashMap;

use daydream_types::fixed::cp437_to_char;
use daydream_types::records::layout::{Layout, LayoutSet};
use daydream_wcc::tables::Table;

use crate::db::{StoreError, WbtrvDb};
use crate::journal;
use crate::realm::{Realm, layout_of};
use crate::update::Kind;

/// What happened to a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    Added,
    Changed,
    Deleted,
}

impl How {
    pub fn name(self) -> &'static str {
        match self {
            How::Added => "added",
            How::Changed => "changed",
            How::Deleted => "deleted",
        }
    }
}

/// One record's change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The `.db` it is in (`WCCITEMS`).
    pub stem: String,
    /// Its record id there.
    pub id: i64,
    pub table: Option<Table>,
    pub how: How,
    /// The record, for a person: `item 12`, `room 1/5`, `player Sysop`.
    pub what: String,
    /// Its name, when it has one.
    pub name: Option<String>,
    /// The members changed, by name (a changed record's).
    pub members: Vec<&'static str>,
    /// What a changed record was, when that was another (`item 2`, renumbered to 3).
    pub was: Option<String>,
}

/// The realm's changes, by file and record id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    /// When the realm was last read from its game or written back, in seconds since 1970;
    /// `None` for a realm edited before Daydream journaled its edits, whose changes are not
    /// known (it is written back whole).
    pub since: Option<u64>,
    pub changes: Vec<Change>,
}

/// Text in CP437 up to its NUL, trimmed.
fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes[..end]
        .iter()
        .map(|&b| cp437_to_char(b))
        .collect::<String>()
        .trim()
        .to_string()
}

fn member_text(layout: &Layout, name: &str, data: &[u8]) -> Option<String> {
    let f = layout.field(name)?;
    let t = text(data.get(f.offset..(f.offset + f.size).min(data.len()))?);
    (!t.is_empty()).then_some(t)
}

/// A record of `table`, for a person, and its name.
pub fn label(
    table: Option<Table>,
    set: LayoutSet,
    id: i64,
    data: &[u8],
) -> (String, Option<String>) {
    let Some(table) = table else {
        return (format!("record {id}"), None);
    };
    let layout = layout_of(table, set);
    let name = ["name", "userid"]
        .iter()
        .find_map(|m| member_text(layout, m, data));
    let what = match Kind::of(table) {
        Some(Kind::Action) => crate::update::describe(Kind::Action, set, data),
        Some(kind) => crate::update::describe(kind, set, data),
        None => match (table, &name) {
            (Table::Users, Some(n)) => format!("player {n}"),
            (Table::BankBooks, Some(n)) => format!("bank book of {n}"),
            (Table::Gangs, Some(n)) => format!("gang {n}"),
            _ => format!("{} record {id}", table.slug()),
        },
    };
    // an action's label is its name already
    let name = name.filter(|_| Kind::of(table) != Some(Kind::Action));
    (what, name)
}

/// The changes of the realm: each record the [`journal::BASE`] journal holds whose bytes
/// differ from what they were.
pub fn list(realm: &Realm) -> Result<Changes, StoreError> {
    let since = journal::established(realm.dir())?;
    let set = realm.layout_set();
    let mut dbs: HashMap<String, Option<WbtrvDb>> = HashMap::new();
    let mut changes = Vec::new();
    for e in journal::pending(realm.dir(), journal::BASE)?.entries {
        let db = match dbs.entry(e.stem.clone()) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                let db = match realm.file(&e.stem) {
                    Some(f) => Some(WbtrvDb::open_read_only(&f.path)?),
                    None => None,
                };
                v.insert(db)
            }
        };
        let Some(db) = db else {
            continue;
        };
        let table = realm.file(&e.stem).and_then(|f| f.table);
        let now = db.get(e.id)?;
        let (how, shown, members) = match (&e.before, &now) {
            (Some(b), Some(n)) if b != n => {
                let layout = table.map(|t| layout_of(t, set));
                (
                    How::Changed,
                    n,
                    crate::three_way::changed_members(layout, b, n),
                )
            }
            (None, Some(n)) => (How::Added, n, Vec::new()),
            (Some(b), None) => (How::Deleted, b, Vec::new()),
            _ => continue,
        };
        let (what, name) = label(table, set, e.id, shown);
        let was = match (how, &e.before) {
            (How::Changed, Some(b)) => Some(label(table, set, e.id, b).0).filter(|w| *w != what),
            _ => None,
        };
        changes.push(Change {
            stem: e.stem,
            id: e.id,
            table,
            how,
            what,
            name,
            members,
            was,
        });
    }
    Ok(Changes { since, changes })
}

/// A change that could not be taken back, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotReverted {
    pub stem: String,
    pub id: i64,
    pub why: String,
}

/// What [`revert`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reverted {
    /// The records put back as they were.
    pub reverted: usize,
    pub failed: Vec<NotReverted>,
}

/// Takes changes back: each record of `which` (`(stem, id)`; `None`: every change) put back as
/// it was when the realm was last read from its game or written back (an added record
/// deleted, a deleted one restored), with tracked writes, so the update file follows. A
/// record that cannot be put back (a deleted record whose key another took since) is
/// reported and left.
pub fn revert(realm: &Realm, which: Option<&[(String, i64)]>) -> Result<Reverted, StoreError> {
    let entries: Vec<journal::Entry> = match which {
        None => journal::pending(realm.dir(), journal::BASE)?.entries,
        Some(which) => {
            let mut out = Vec::new();
            for (stem, id) in which {
                let stem = stem.to_ascii_uppercase();
                if let Some(e) = journal::entry(realm.dir(), journal::BASE, &stem, *id)? {
                    out.push(e);
                }
            }
            out
        }
    };
    let mut out = Reverted::default();
    let mut dbs: HashMap<String, WbtrvDb> = HashMap::new();
    for e in entries {
        let Some(f) = realm.file(&e.stem) else {
            continue;
        };
        let db = match dbs.entry(e.stem.clone()) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => v.insert(WbtrvDb::open(&f.path)?),
        };
        let now = db.get(e.id)?;
        let done = match (&e.before, &now) {
            (Some(b), Some(n)) if b != n => db.update(e.id, b).map(|_| true),
            (Some(b), None) => db.restore(e.id, b).map(|_| true),
            (None, Some(_)) => db.delete(e.id).map(|_| true),
            _ => Ok(false),
        };
        match done {
            Ok(changed) => {
                out.reverted += usize::from(changed);
                journal::forget(realm.dir(), journal::BASE, &e.stem, e.id)?;
            }
            Err(err) => out.failed.push(NotReverted {
                stem: e.stem.clone(),
                id: e.id,
                why: err.to_string(),
            }),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Metadata;
    use crate::key::{Key, Segment, attr, ty};
    use daydream_types::records::layout::ITEM;

    fn item(number: i32, name: &[u8], weight: u8) -> Vec<u8> {
        let mut r = vec![0; ITEM.len];
        r[..4].copy_from_slice(&number.to_le_bytes());
        let n = ITEM.field("name").unwrap();
        r[n.offset..n.offset + name.len()].copy_from_slice(name);
        r[ITEM.field("weight").unwrap().offset] = weight;
        r
    }

    /// A realm of one items file, made from `items` and established.
    fn realm(items: &[Vec<u8>]) -> (tempfile::TempDir, Realm) {
        let tmp = tempfile::tempdir().unwrap();
        let meta = Metadata {
            record_length: ITEM.len,
            physical_record_length: ITEM.len + 4,
            page_length: 4096,
            variable_length_records: false,
            version: 4,
        };
        let key = Key {
            segments: vec![Segment {
                number: 0,
                index: 0,
                attributes: attr::MODIFIABLE,
                data_type: ty::INTEGER,
                offset: 0,
                length: 4,
                null_value: 0,
                acs: None,
            }],
            acs_name: None,
            acs: None,
        };
        let mut db = WbtrvDb::create(&tmp.path().join("WCCITEMS.db"), meta, vec![key]).unwrap();
        for r in items {
            db.insert(r).unwrap();
        }
        drop(db);
        journal::reset(tmp.path()).unwrap();
        let realm = Realm::open(tmp.path()).unwrap();
        (tmp, realm)
    }

    #[test]
    fn changes_are_listed_and_taken_back() {
        let (_tmp, realm) = realm(&[item(1, b"Dagger", 2), item(2, b"Torch", 1)]);
        assert!(list(&realm).unwrap().changes.is_empty());
        assert!(list(&realm).unwrap().since.is_some());
        let path = realm.file("WCCITEMS").unwrap().path.clone();
        let mut db = WbtrvDb::open(&path).unwrap();
        db.update(2, &item(2, b"Brand", 1)).unwrap();
        db.delete(1).unwrap();
        let added = db.insert(&item(3, b"Lantern", 3)).unwrap();
        db.update(2, &item(2, b"Brand", 1)).unwrap();
        drop(db);

        // id, how, what, name, members
        type Shown = (i64, How, String, Option<String>, Vec<&'static str>);
        let shown = |c: &[Change]| -> Vec<Shown> {
            c.iter()
                .map(|c| {
                    (
                        c.id,
                        c.how,
                        c.what.clone(),
                        c.name.clone(),
                        c.members.clone(),
                    )
                })
                .collect()
        };
        let c = list(&realm).unwrap().changes;
        assert_eq!(
            shown(&c),
            [
                (
                    1,
                    How::Deleted,
                    "item 1".into(),
                    Some("Dagger".into()),
                    vec![]
                ),
                (
                    2,
                    How::Changed,
                    "item 2".into(),
                    Some("Brand".into()),
                    vec!["name"]
                ),
                (
                    added,
                    How::Added,
                    "item 3".into(),
                    Some("Lantern".into()),
                    vec![]
                ),
            ]
        );
        assert!(c.iter().all(|c| c.was.is_none()));

        // one taken back, then the rest
        let r = revert(&realm, Some(&[("wccitems".into(), 1)])).unwrap();
        assert_eq!((r.reverted, r.failed.len()), (1, 0));
        let db = WbtrvDb::open_read_only(&path).unwrap();
        assert_eq!(db.get(1).unwrap(), Some(item(1, b"Dagger", 2)));
        assert_eq!(list(&realm).unwrap().changes.len(), 2);
        let r = revert(&realm, None).unwrap();
        assert_eq!((r.reverted, r.failed.len()), (2, 0));
        assert!(list(&realm).unwrap().changes.is_empty());
        assert_eq!(
            db.records().unwrap(),
            [(1, item(1, b"Dagger", 2)), (2, item(2, b"Torch", 1))]
        );
        // the update file follows a revert as any write: it is in the sync journal
        assert!(
            !journal::pending(realm.dir(), journal::SYNC)
                .unwrap()
                .entries
                .is_empty()
        );

        // the last record deleted and another added takes its id: one change, a renumbering
        let mut db = WbtrvDb::open(&path).unwrap();
        db.delete(2).unwrap();
        assert_eq!(db.insert(&item(7, b"Rope", 1)).unwrap(), 2);
        drop(db);
        let c = list(&realm).unwrap().changes;
        assert_eq!(c.len(), 1);
        assert_eq!(
            (c[0].what.as_str(), c[0].was.as_deref()),
            ("item 7", Some("item 2"))
        );
    }

    #[test]
    fn a_deleted_record_whose_key_was_taken_is_not_put_back() {
        let (_tmp, realm) = realm(&[item(1, b"Dagger", 2), item(2, b"Torch", 1)]);
        let path = realm.file("WCCITEMS").unwrap().path.clone();
        let mut db = WbtrvDb::open(&path).unwrap();
        db.delete(1).unwrap();
        assert_eq!(db.insert(&item(1, b"Spear", 4)).unwrap(), 3);
        drop(db);
        let r = revert(&realm, Some(&[("WCCITEMS".into(), 1)])).unwrap();
        assert_eq!(r.reverted, 0);
        assert_eq!(r.failed.len(), 1, "{r:?}");
        assert_eq!(list(&realm).unwrap().changes.len(), 2);
    }
}
