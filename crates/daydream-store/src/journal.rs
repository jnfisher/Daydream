//! What Daydream changed in a realm: each record's bytes from before Daydream first wrote it,
//! kept beside the realm in `.daydream/journal.db`.
//!
//! Every write through a tracked [`crate::WbtrvDb`] (one opened with [`crate::WbtrvDb::open`])
//! first records the record as it was (no bytes when there was no record at that id) in each
//! journal, unless that journal already has the record:
//!
//! - [`SYNC`]: since the realm's update file was last brought in step ([`crate::update`]).
//! - [`BASE`]: since the realm was last read from its game or written to it.
//! - [`STEP`]: since the save under way began ([`crate::history`]).
//!
//! Only a record's first write is kept, so an entry and the record as it is now give the net
//! change, however many writes came between. The entry is stored before the record is written,
//! in a transaction of its own, so a write that fails or is rolled back leaves at most an entry
//! whose record did not change, which reads as no change.
//!
//! ```text
//! journal_t(journal, stem, id, before, touched)    PRIMARY KEY(journal, stem, id)
//! ```
//!
//! `touched` counts up with every write of the record, so a journal is cleared only of the
//! entries that were read ([`Pending::mark`]): a record written again meanwhile keeps its entry.
//! The game's own files hold nothing of Daydream's: the journal is a file of its own.
//!
//! The [`BASE`] journal is complete only from the moment the realm was made from its game's
//! files, or last written back into them ([`establish`]; `state_t` records when). A realm
//! edited before Daydream kept journals has none, and is written back whole
//! ([`crate::dat`]).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::StoreError;
use crate::realm::stem_of_db;

/// Changes since the update file was last brought in step.
pub const SYNC: &str = "sync";
/// Changes since the realm was last read from its game or written to it.
pub const BASE: &str = "base";
/// Changes since the save under way began.
pub const STEP: &str = "step";

const JOURNALS: [&str; 3] = [BASE, SYNC, STEP];

/// The journal file of the realm in `realm_dir`.
pub fn path(realm_dir: &Path) -> PathBuf {
    realm_dir.join(".daydream").join("journal.db")
}

pub(crate) fn sql_err(path: &Path) -> impl Fn(rusqlite::Error) -> StoreError + '_ {
    move |source| StoreError::Sqlite {
        path: path.to_path_buf(),
        source,
    }
}

/// Opens (creating) the journal at `path`.
pub(crate) fn connect(path: &Path) -> Result<Connection, StoreError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| StoreError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    let sql = sql_err(path);
    let conn = Connection::open(path).map_err(&sql)?;
    conn.busy_timeout(std::time::Duration::from_secs(10))
        .map_err(&sql)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;\
         PRAGMA synchronous = NORMAL;\
         CREATE TABLE IF NOT EXISTS journal_t(\
             journal TEXT NOT NULL, stem TEXT NOT NULL, id INTEGER NOT NULL, \
             before BLOB, touched INTEGER NOT NULL, \
             PRIMARY KEY(journal, stem, id));\
         CREATE INDEX IF NOT EXISTS journal_touched ON journal_t(touched);\
         CREATE TABLE IF NOT EXISTS state_t(name TEXT PRIMARY KEY, value TEXT NOT NULL);\
         CREATE TABLE IF NOT EXISTS step_t(\
             step INTEGER PRIMARY KEY, at INTEGER NOT NULL, label TEXT NOT NULL, \
             records INTEGER NOT NULL, state TEXT NOT NULL, undone_seq INTEGER);\
         CREATE TABLE IF NOT EXISTS step_rec_t(\
             step INTEGER NOT NULL, stem TEXT NOT NULL, id INTEGER NOT NULL, \
             before BLOB, after BLOB, PRIMARY KEY(step, stem, id));",
    )
    .map_err(&sql)?;
    Ok(conn)
}

/// Opens the journal of `realm_dir` if it has one.
pub(crate) fn existing(realm_dir: &Path) -> Result<Option<(PathBuf, Connection)>, StoreError> {
    let p = path(realm_dir);
    if !p.is_file() {
        return Ok(None);
    }
    let conn = connect(&p)?;
    Ok(Some((p, conn)))
}

/// Records the writes of one `.db` (see [`crate::WbtrvDb::open`]).
pub(crate) struct Recorder {
    path: PathBuf,
    conn: Connection,
    stem: String,
}

impl Recorder {
    /// The recorder for the `.db` at `db_path`: the journal of the folder it is in. `None` for a
    /// file whose name is not a `.db`'s.
    pub(crate) fn open(db_path: &Path) -> Result<Option<Self>, StoreError> {
        let Some(stem) = db_path
            .file_name()
            .and_then(|n| stem_of_db(&n.to_string_lossy()))
        else {
            return Ok(None);
        };
        let dir = db_path.parent().unwrap_or_else(|| Path::new("."));
        let path = path(dir);
        let conn = connect(&path)?;
        Ok(Some(Self { path, conn, stem }))
    }

    /// Notes that record `id` is about to be written; `before` is its bytes now (`None`: no
    /// record at that id). A journal that has the record already keeps what it has.
    pub(crate) fn record(&self, id: i64, before: Option<&[u8]>) -> Result<(), StoreError> {
        let sql = sql_err(&self.path);
        let tx = self.conn.unchecked_transaction().map_err(&sql)?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO journal_t(journal, stem, id, before, touched) \
                     VALUES(?1, ?2, ?3, ?4, \
                            (SELECT COALESCE(MAX(touched), 0) + 1 FROM journal_t)) \
                     ON CONFLICT(journal, stem, id) DO UPDATE SET touched = excluded.touched",
                )
                .map_err(&sql)?;
            for journal in JOURNALS {
                stmt.execute(params![journal, self.stem, id, before])
                    .map_err(&sql)?;
            }
        }
        tx.commit().map_err(&sql)
    }
}

/// One record a journal holds: the file's stem, the record's id and its bytes before the
/// journal's first write of it (`None`: there was no record at that id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub stem: String,
    pub id: i64,
    pub before: Option<Vec<u8>>,
}

/// A journal's entries as read, and the mark that clears just them ([`clear`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pending {
    pub entries: Vec<Entry>,
    pub mark: i64,
}

/// The entries of `journal` (one of [`SYNC`], [`BASE`]) in the realm at `realm_dir`, by
/// file and id.
pub fn pending(realm_dir: &Path, journal: &str) -> Result<Pending, StoreError> {
    let Some((p, conn)) = existing(realm_dir)? else {
        return Ok(Pending::default());
    };
    let sql = sql_err(&p);
    let mark: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(touched), 0) FROM journal_t WHERE journal = ?1",
            [journal],
            |r| r.get(0),
        )
        .map_err(&sql)?;
    let mut stmt = conn
        .prepare(
            "SELECT stem, id, before FROM journal_t WHERE journal = ?1 AND touched <= ?2 \
             ORDER BY stem, id",
        )
        .map_err(&sql)?;
    let entries = stmt
        .query_map(params![journal, mark], |r| {
            Ok(Entry {
                stem: r.get(0)?,
                id: r.get(1)?,
                before: r.get(2)?,
            })
        })
        .map_err(&sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(&sql)?;
    Ok(Pending { entries, mark })
}

/// How many entries `journal` holds, and the mark that clears just them ([`clear`]).
pub fn count(realm_dir: &Path, journal: &str) -> Result<(usize, i64), StoreError> {
    let Some((p, conn)) = existing(realm_dir)? else {
        return Ok((0, 0));
    };
    conn.query_row(
        "SELECT COUNT(*), COALESCE(MAX(touched), 0) FROM journal_t WHERE journal = ?1",
        [journal],
        |r| Ok((r.get::<_, i64>(0)? as usize, r.get(1)?)),
    )
    .map_err(sql_err(&p))
}

/// The entry of record `id` of file `stem` in `journal`, if it has one.
pub fn entry(
    realm_dir: &Path,
    journal: &str,
    stem: &str,
    id: i64,
) -> Result<Option<Entry>, StoreError> {
    let Some((p, conn)) = existing(realm_dir)? else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT before FROM journal_t WHERE journal = ?1 AND stem = ?2 AND id = ?3",
        params![journal, stem, id],
        |r| r.get::<_, Option<Vec<u8>>>(0),
    )
    .optional()
    .map(|found| {
        found.map(|before| Entry {
            stem: stem.to_string(),
            id,
            before,
        })
    })
    .map_err(sql_err(&p))
}

/// Clears `journal`: of the entries read with `mark` ([`Pending::mark`]), or with `None` of
/// every entry.
pub fn clear(realm_dir: &Path, journal: &str, mark: Option<i64>) -> Result<(), StoreError> {
    let Some((p, conn)) = existing(realm_dir)? else {
        return Ok(());
    };
    conn.execute(
        "DELETE FROM journal_t WHERE journal = ?1 AND touched <= ?2",
        params![journal, mark.unwrap_or(i64::MAX)],
    )
    .map(drop)
    .map_err(sql_err(&p))
}

/// Clears one record's entry from `journal`.
pub fn forget(realm_dir: &Path, journal: &str, stem: &str, id: i64) -> Result<(), StoreError> {
    let Some((p, conn)) = existing(realm_dir)? else {
        return Ok(());
    };
    conn.execute(
        "DELETE FROM journal_t WHERE journal = ?1 AND stem = ?2 AND id = ?3",
        params![journal, stem, id],
    )
    .map(drop)
    .map_err(sql_err(&p))
}

/// Clears every journal: the realm as it is now is where its changes start (it was just read
/// from its game, or made). The base is established ([`establish`]).
pub fn reset(realm_dir: &Path) -> Result<(), StoreError> {
    clear(realm_dir, SYNC, None)?;
    establish(realm_dir, &[])
}

/// Starts the [`BASE`] journal afresh: the realm now holds what its game's files hold (it was
/// just made from them, or written back into them), so its changes count from here. The
/// entries of `keep` (`(stem, id)`: changes that could not be written) stay as they are.
pub fn establish(realm_dir: &Path, keep: &[(String, i64)]) -> Result<(), StoreError> {
    let p = path(realm_dir);
    let conn = connect(&p)?;
    let sql = sql_err(&p);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let tx = conn.unchecked_transaction().map_err(&sql)?;
    tx.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS keep_t(stem TEXT, id INTEGER); DELETE FROM keep_t;",
    )
    .map_err(&sql)?;
    for (stem, id) in keep {
        tx.execute(
            "INSERT INTO keep_t(stem, id) VALUES(?1, ?2)",
            params![stem, id],
        )
        .map_err(&sql)?;
    }
    tx.execute(
        "DELETE FROM journal_t WHERE journal = ?1 \
         AND (stem, id) NOT IN (SELECT stem, id FROM keep_t)",
        [BASE],
    )
    .map_err(&sql)?;
    tx.execute(
        "INSERT INTO state_t(name, value) VALUES('base', ?1) \
         ON CONFLICT(name) DO UPDATE SET value = excluded.value",
        [now.to_string()],
    )
    .map_err(&sql)?;
    tx.commit().map_err(&sql)
}

/// When the [`BASE`] journal was last established, in seconds since 1970: `None` when it never
/// was, so it does not hold every change the realm has.
pub fn established(realm_dir: &Path) -> Result<Option<u64>, StoreError> {
    let Some((p, conn)) = existing(realm_dir)? else {
        return Ok(None);
    };
    conn.query_row("SELECT value FROM state_t WHERE name = 'base'", [], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .map(|v| v.and_then(|v| v.parse().ok()))
    .map_err(sql_err(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_write_is_kept_and_clearing_spares_later_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("WCCITEMS.db");
        let r = Recorder::open(&db).unwrap().unwrap();
        r.record(1, Some(b"one")).unwrap();
        r.record(1, Some(b"later")).unwrap();
        r.record(2, None).unwrap();
        let p = pending(tmp.path(), SYNC).unwrap();
        assert_eq!(
            p.entries,
            vec![
                Entry {
                    stem: "WCCITEMS".into(),
                    id: 1,
                    before: Some(b"one".to_vec())
                },
                Entry {
                    stem: "WCCITEMS".into(),
                    id: 2,
                    before: None
                },
            ]
        );
        // written again after the read: its entry stays (with its first bytes)
        r.record(2, Some(b"two")).unwrap();
        clear(tmp.path(), SYNC, Some(p.mark)).unwrap();
        let left = pending(tmp.path(), SYNC).unwrap().entries;
        assert_eq!(left.len(), 1);
        assert_eq!((left[0].id, left[0].before.as_deref()), (2, None));
        // the other journal still has both
        assert_eq!(pending(tmp.path(), BASE).unwrap().entries.len(), 2);
        assert_eq!(
            entry(tmp.path(), BASE, "WCCITEMS", 1)
                .unwrap()
                .unwrap()
                .before
                .as_deref(),
            Some(&b"one"[..])
        );
        assert_eq!(established(tmp.path()).unwrap(), None);
        // established keeping one change: the other goes
        establish(tmp.path(), &[("WCCITEMS".into(), 2)]).unwrap();
        let kept = pending(tmp.path(), BASE).unwrap().entries;
        assert_eq!(kept.len(), 1);
        assert_eq!((kept[0].id, kept[0].before.as_deref()), (2, None));
        assert!(established(tmp.path()).unwrap().is_some());
        reset(tmp.path()).unwrap();
        assert!(pending(tmp.path(), BASE).unwrap().entries.is_empty());
        assert!(established(tmp.path()).unwrap().is_some());
    }

    #[test]
    fn a_realm_without_a_journal_has_no_entries_and_gets_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(pending(tmp.path(), BASE).unwrap().entries.is_empty());
        clear(tmp.path(), BASE, None).unwrap();
        assert!(!path(tmp.path()).exists());
    }
}
