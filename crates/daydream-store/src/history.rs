//! A realm's history of saves, for undo and redo.
//!
//! A save (one request to the server, one command, one batch of an agent's edits) is a step:
//! [`begin`] before its writes and [`end`] after. Every tracked write between them is in the
//! [`journal::STEP`] journal (each record's bytes before the step first wrote it), and `end`
//! keeps those records, with the bytes the step left, as one step of the history. [`undo`]
//! puts a step's records back as they were, if none of them changed since; [`redo`] writes
//! them again, if none changed since the undo. Either is an ordinary tracked write, so the
//! update file and the realm's changes follow it.
//!
//! ```text
//! step_t(step INTEGER PRIMARY KEY, at, label, records, state, undone_seq)
//! step_rec_t(step, stem, id, before, after)    PRIMARY KEY(step, stem, id)
//! ```
//!
//! `state` is `done`, `undone`, or `too_big`: a step of more than [`MAX_RECORDS`] records is
//! listed, but its bytes are not kept and it cannot be undone. The last [`KEEP`] steps are
//! kept. The history lives in the realm's journal file, beside the realm.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{OptionalExtension, params};

use crate::db::{StoreError, WbtrvDb};
use crate::journal::{self, STEP};
use crate::realm::Realm;

/// Steps kept.
pub const KEEP: i64 = 100;
/// The most records a step can have and still be undone.
pub const MAX_RECORDS: usize = if cfg!(test) { 3 } else { 10_000 };

/// Where a step stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Written, and can be undone.
    Done,
    /// Undone, and can be redone.
    Undone,
    /// Written, with too many records to keep for undo.
    TooBig,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Done => "done",
            State::Undone => "undone",
            State::TooBig => "too_big",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "undone" => State::Undone,
            "too_big" => State::TooBig,
            _ => State::Done,
        }
    }
}

/// One step of the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub step: i64,
    /// When it was written, in seconds since 1970.
    pub at: u64,
    /// What it was, for a person (`Save Items 12`).
    pub label: String,
    /// The records it changed.
    pub records: usize,
    pub state: State,
}

/// One record a step changed: its bytes before and after (`None`: no record).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    pub stem: String,
    pub id: i64,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}

/// Why an undo or redo was not done.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("{0}")]
    Refused(String),
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Starts a step: the realm's writes from here on are its own.
pub fn begin(realm_dir: &Path) -> Result<(), StoreError> {
    journal::clear(realm_dir, STEP, None)
}

/// The `.db` of `stem`, opened once.
fn db<'a>(
    realm: &Realm,
    dbs: &'a mut HashMap<String, Option<WbtrvDb>>,
    stem: &str,
    write: bool,
) -> Result<Option<&'a mut WbtrvDb>, StoreError> {
    if !dbs.contains_key(stem) {
        let db = match realm.file(stem) {
            Some(f) if write => Some(WbtrvDb::open(&f.path)?),
            Some(f) => Some(WbtrvDb::open_read_only(&f.path)?),
            None => None,
        };
        dbs.insert(stem.to_string(), db);
    }
    Ok(dbs.get_mut(stem).and_then(Option::as_mut))
}

/// Ends the step [`begin`] started: the records whose bytes it changed are kept as one step of
/// the history, `label`led. `None` when it changed nothing.
pub fn end(realm: &Realm, label: &str) -> Result<Option<Step>, StoreError> {
    let dir = realm.dir();
    let mut records = Vec::new();
    // a step too big to keep is counted, not read
    let (written, mark) = journal::count(dir, STEP)?;
    let too_big = written > MAX_RECORDS;
    let (count, mark) = if too_big {
        (written, mark)
    } else {
        let pending = journal::pending(dir, STEP)?;
        let mut dbs = HashMap::new();
        for e in pending.entries {
            let Some(db) = db(realm, &mut dbs, &e.stem, false)? else {
                continue;
            };
            let after = db.get(e.id)?;
            if after != e.before {
                records.push(StepRecord {
                    stem: e.stem,
                    id: e.id,
                    before: e.before,
                    after,
                });
            }
        }
        (records.len(), pending.mark)
    };
    journal::clear(dir, STEP, Some(mark))?;
    if count == 0 {
        return Ok(None);
    }
    let p = journal::path(dir);
    let conn = journal::connect(&p)?;
    let sql = journal::sql_err(&p);
    let tx = conn.unchecked_transaction().map_err(&sql)?;
    let state = if too_big { State::TooBig } else { State::Done };
    let at = now();
    tx.execute(
        "INSERT INTO step_t(at, label, records, state) VALUES(?1, ?2, ?3, ?4)",
        params![at as i64, label, count as i64, state.name()],
    )
    .map_err(&sql)?;
    let step = tx.last_insert_rowid();
    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO step_rec_t(step, stem, id, before, after) VALUES(?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(&sql)?;
        for r in &records {
            stmt.execute(params![step, r.stem, r.id, r.before, r.after])
                .map_err(&sql)?;
        }
    }
    tx.execute("DELETE FROM step_rec_t WHERE step <= ?1", [step - KEEP])
        .map_err(&sql)?;
    tx.execute("DELETE FROM step_t WHERE step <= ?1", [step - KEEP])
        .map_err(&sql)?;
    tx.commit().map_err(&sql)?;
    Ok(Some(Step {
        step,
        at,
        label: label.to_string(),
        records: count,
        state,
    }))
}

fn step_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<Step> {
    Ok(Step {
        step: row.get(0)?,
        at: row.get::<_, i64>(1)? as u64,
        label: row.get(2)?,
        records: row.get::<_, i64>(3)? as usize,
        state: State::parse(&row.get::<_, String>(4)?),
    })
}

/// The newest `limit` steps, newest first.
pub fn list(realm_dir: &Path, limit: usize) -> Result<Vec<Step>, StoreError> {
    let Some((p, conn)) = journal::existing(realm_dir)? else {
        return Ok(Vec::new());
    };
    let sql = journal::sql_err(&p);
    let mut stmt = conn
        .prepare("SELECT step, at, label, records, state FROM step_t ORDER BY step DESC LIMIT ?1")
        .map_err(&sql)?;
    stmt.query_map([limit as i64], step_of)
        .map_err(&sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(&sql)
}

/// One step, if the history has it.
pub fn step(realm_dir: &Path, step: i64) -> Result<Option<Step>, StoreError> {
    let Some((p, conn)) = journal::existing(realm_dir)? else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT step, at, label, records, state FROM step_t WHERE step = ?1",
        [step],
        step_of,
    )
    .optional()
    .map_err(journal::sql_err(&p))
}

/// The step [`undo`] takes when given none: the newest that is written, and the step
/// [`redo`] takes: the one undone last.
pub fn next(realm_dir: &Path) -> Result<(Option<Step>, Option<Step>), StoreError> {
    let Some((p, conn)) = journal::existing(realm_dir)? else {
        return Ok((None, None));
    };
    let sql = journal::sql_err(&p);
    let undo = conn
        .query_row(
            "SELECT step, at, label, records, state FROM step_t \
             WHERE state IN ('done', 'too_big') ORDER BY step DESC LIMIT 1",
            [],
            step_of,
        )
        .optional()
        .map_err(&sql)?;
    let redo = conn
        .query_row(
            "SELECT step, at, label, records, state FROM step_t \
             WHERE state = 'undone' ORDER BY undone_seq DESC LIMIT 1",
            [],
            step_of,
        )
        .optional()
        .map_err(&sql)?;
    Ok((undo, redo))
}

/// The records a step changed, by file and id.
pub fn records(realm_dir: &Path, step: i64) -> Result<Vec<StepRecord>, StoreError> {
    let Some((p, conn)) = journal::existing(realm_dir)? else {
        return Ok(Vec::new());
    };
    let sql = journal::sql_err(&p);
    let mut stmt = conn
        .prepare("SELECT stem, id, before, after FROM step_rec_t WHERE step = ?1 ORDER BY stem, id")
        .map_err(&sql)?;
    stmt.query_map([step], |r| {
        Ok(StepRecord {
            stem: r.get(0)?,
            id: r.get(1)?,
            before: r.get(2)?,
            after: r.get(3)?,
        })
    })
    .map_err(&sql)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(&sql)
}

/// Writes `to` over records that must now hold `from`, in one transaction per file.
fn put(
    realm: &Realm,
    records: &[StepRecord],
    backwards: bool,
    what: &str,
) -> Result<(), HistoryError> {
    let set = realm.layout_set();
    // what each record holds now, and what it is to hold
    fn ends(r: &StepRecord, backwards: bool) -> (Option<&[u8]>, Option<&[u8]>) {
        if backwards {
            (r.after.as_deref(), r.before.as_deref())
        } else {
            (r.before.as_deref(), r.after.as_deref())
        }
    }
    let ends = |r| ends(r, backwards);
    let mut dbs = HashMap::new();
    for r in records {
        let (from, _) = ends(r);
        let Some(db) = db(realm, &mut dbs, &r.stem, true)? else {
            return Err(HistoryError::Refused(format!(
                "the realm has no {} any more, so the step was not {what}",
                r.stem
            )));
        };
        if db.get(r.id)?.as_deref() != from {
            let table = realm.file(&r.stem).and_then(|f| f.table);
            let shown = from.or(ends(r).1).unwrap_or_default();
            let (label, _) = crate::changes::label(table, set, r.id, shown);
            return Err(HistoryError::Refused(format!(
                "{label} changed since, so the step was not {what}"
            )));
        }
    }
    for db in dbs.values_mut().flatten() {
        db.begin()?;
    }
    let mut result: Result<(), StoreError> = Ok(());
    for r in records {
        let (from, to) = ends(r);
        let Some(db) = dbs.get_mut(&r.stem).and_then(Option::as_mut) else {
            continue;
        };
        result = match (from, to) {
            (Some(_), Some(to)) => db.update(r.id, to),
            (Some(_), None) => db.delete(r.id),
            (None, Some(to)) => db.restore(r.id, to),
            (None, None) => Ok(()),
        };
        if result.is_err() {
            break;
        }
    }
    match result {
        Ok(()) => {
            for db in dbs.values().flatten() {
                db.commit()?;
            }
            Ok(())
        }
        Err(e) => {
            for db in dbs.values().flatten() {
                db.rollback()?;
            }
            Err(HistoryError::Refused(format!(
                "{e}, so the step was not {what}"
            )))
        }
    }
}

fn set_state(realm_dir: &Path, step: i64, state: State) -> Result<(), StoreError> {
    let p = journal::path(realm_dir);
    let conn = journal::connect(&p)?;
    let seq = if state == State::Undone {
        "(SELECT COALESCE(MAX(undone_seq), 0) + 1 FROM step_t)"
    } else {
        "NULL"
    };
    conn.execute(
        &format!("UPDATE step_t SET state = ?1, undone_seq = {seq} WHERE step = ?2"),
        params![state.name(), step],
    )
    .map(drop)
    .map_err(journal::sql_err(&p))
}

/// Undoes a step (the newest written one, or `step`): its records put back as they were before
/// it. Refused when any of them changed since, or the step was too big to keep.
pub fn undo(realm: &Realm, step: Option<i64>) -> Result<Step, HistoryError> {
    let dir = realm.dir();
    let s = match step {
        Some(n) => self::step(dir, n)?
            .ok_or_else(|| HistoryError::Refused(format!("no step {n} in the history")))?,
        None => next(dir)?
            .0
            .ok_or_else(|| HistoryError::Refused("nothing to undo".into()))?,
    };
    match s.state {
        State::Undone => {
            return Err(HistoryError::Refused(format!(
                "\"{}\" is undone already",
                s.label
            )));
        }
        State::TooBig => {
            return Err(HistoryError::Refused(format!(
                "\"{}\" changed {} records, too many to keep for undo",
                s.label, s.records
            )));
        }
        State::Done => {}
    }
    put(realm, &records(dir, s.step)?, true, "undone")?;
    set_state(dir, s.step, State::Undone)?;
    Ok(Step {
        state: State::Undone,
        ..s
    })
}

/// Redoes a step (the one undone last, or `step`): its records written again as it left them.
/// Refused when any of them changed since it was undone.
pub fn redo(realm: &Realm, step: Option<i64>) -> Result<Step, HistoryError> {
    let dir = realm.dir();
    let s = match step {
        Some(n) => self::step(dir, n)?
            .ok_or_else(|| HistoryError::Refused(format!("no step {n} in the history")))?,
        None => next(dir)?
            .1
            .ok_or_else(|| HistoryError::Refused("nothing to redo".into()))?,
    };
    if s.state != State::Undone {
        return Err(HistoryError::Refused(format!(
            "\"{}\" is not undone",
            s.label
        )));
    }
    put(realm, &records(dir, s.step)?, false, "redone")?;
    set_state(dir, s.step, State::Done)?;
    Ok(Step {
        state: State::Done,
        ..s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{item, items_realm};

    fn items(realm: &Realm) -> Vec<(i64, Vec<u8>)> {
        WbtrvDb::open_read_only(&realm.file("WCCITEMS").unwrap().path)
            .unwrap()
            .records()
            .unwrap()
    }

    /// One step of writes to the items file.
    fn step(realm: &Realm, label: &str, writes: impl FnOnce(&mut WbtrvDb)) -> Option<Step> {
        begin(realm.dir()).unwrap();
        let mut db = WbtrvDb::open(&realm.file("WCCITEMS").unwrap().path).unwrap();
        writes(&mut db);
        drop(db);
        end(realm, label).unwrap()
    }

    #[test]
    fn saves_are_steps_that_undo_and_redo() {
        let (_tmp, realm) = items_realm(&[item(1, b"Dagger", 2), item(2, b"Torch", 1)]);
        let start = items(&realm);
        let one = step(&realm, "Save Items 1", |db| {
            db.update(1, &item(1, b"Dirk", 2)).unwrap();
        })
        .unwrap();
        assert_eq!((one.records, one.state), (1, State::Done));
        let two = step(&realm, "Tools", |db| {
            db.delete(2).unwrap();
            db.insert(&item(3, b"Lantern", 3)).unwrap();
            db.insert(&item(4, b"Rope", 1)).unwrap();
            // written and put back within the step: not one of its records
            db.update(1, &item(1, b"Blade", 2)).unwrap();
            db.update(1, &item(1, b"Dirk", 2)).unwrap();
        })
        .unwrap();
        // the record deleted and the first added share an id: one record, then the second
        assert_eq!(two.records, 2);
        let after = items(&realm);
        assert_eq!(step(&realm, "nothing", |_| {}), None);
        let labels: Vec<String> = list(realm.dir(), 10)
            .unwrap()
            .into_iter()
            .map(|s| s.label)
            .collect();
        assert_eq!(labels, ["Tools", "Save Items 1"]);

        // undone newest first, then redone in the order undone, last first
        assert_eq!(undo(&realm, None).unwrap().step, two.step);
        assert_eq!(undo(&realm, None).unwrap().step, one.step);
        assert_eq!(items(&realm), start);
        assert!(matches!(undo(&realm, None), Err(HistoryError::Refused(_))));
        let (u, r) = next(realm.dir()).unwrap();
        assert_eq!((u, r.map(|s| s.step)), (None, Some(one.step)));
        assert_eq!(redo(&realm, None).unwrap().step, one.step);
        assert_eq!(redo(&realm, None).unwrap().step, two.step);
        assert_eq!(items(&realm), after);
        // and the realm's changes and update file follow, as for any write
        assert_eq!(crate::changes::list(&realm).unwrap().changes.len(), 3);
        assert!(
            !journal::pending(realm.dir(), journal::SYNC)
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn a_step_whose_records_changed_since_is_not_undone() {
        let (_tmp, realm) = items_realm(&[item(1, b"Dagger", 2), item(2, b"Torch", 1)]);
        let one = step(&realm, "rename", |db| {
            db.update(1, &item(1, b"Dirk", 2)).unwrap();
        })
        .unwrap();
        let two = step(&realm, "rename again", |db| {
            db.update(1, &item(1, b"Kris", 2)).unwrap();
        })
        .unwrap();
        let e = undo(&realm, Some(one.step)).unwrap_err();
        assert!(e.to_string().contains("item 1 changed since"), "{e}");
        assert_eq!(items(&realm)[0].1, item(1, b"Kris", 2));
        // in order, both go
        undo(&realm, Some(two.step)).unwrap();
        undo(&realm, Some(one.step)).unwrap();
        assert_eq!(items(&realm)[0].1, item(1, b"Dagger", 2));
        // a step written again over an undone one: the undone one cannot be redone
        step(&realm, "rename once more", |db| {
            db.update(1, &item(1, b"Sax", 2)).unwrap();
        });
        assert!(redo(&realm, Some(one.step)).is_err());
    }

    #[test]
    fn a_step_too_big_to_keep_is_listed_but_not_undone() {
        let (_tmp, realm) = items_realm(&[]);
        let big = step(&realm, "Import", |db| {
            for n in 1..=4 {
                db.insert(&item(n, b"Rock", 1)).unwrap();
            }
        })
        .unwrap();
        assert_eq!((big.records, big.state), (4, State::TooBig));
        assert!(records(realm.dir(), big.step).unwrap().is_empty());
        let e = undo(&realm, None).unwrap_err();
        assert!(e.to_string().contains("too many"), "{e}");
        assert_eq!(items(&realm).len(), 4);
    }
}
