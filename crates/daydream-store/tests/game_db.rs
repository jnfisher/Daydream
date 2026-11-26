//! Writing a working copy back into a game's own `.db` files: only the copy's edits, onto the
//! game's records as play left them, each at its own id.

use std::path::{Path, PathBuf};

use daydream_store::dat::{Action, DatError, WriteOptions, Written};
use daydream_store::game_db::write_db;
use daydream_store::{Realm, WbtrvDb, journal};

use crate::support::fixture;

/// A game folder holding MBBSEMU.DB, as MBBSEmu keeps a converted file, and a working copy
/// taken from it. MBBSEMU.DAT's records are told apart by the number at 34 (its first unique
/// key); the auto-increment number at 70 is unique too, and neither it nor the name at 2 may
/// change through an update.
fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("game");
    let copy = tmp.path().join("copy");
    std::fs::create_dir_all(&game).unwrap();
    std::fs::create_dir_all(&copy).unwrap();
    WbtrvDb::convert(&fixture("MBBSEMU.DAT"), &game.join("MBBSEMU.DB")).unwrap();
    WbtrvDb::snapshot(&game.join("MBBSEMU.DB"), &copy.join("MBBSEMU.db")).unwrap();
    journal::reset(&copy).unwrap();
    (tmp, game, copy)
}

/// The working copy's file, its writes journaled as the editor's are.
fn ours(copy: &Path) -> WbtrvDb {
    WbtrvDb::open(&copy.join("MBBSEMU.db")).unwrap()
}

/// The game's file, written as the game writes it.
fn theirs(game: &Path) -> WbtrvDb {
    let mut db = WbtrvDb::open(&game.join("MBBSEMU.DB")).unwrap();
    db.track(false);
    db
}

fn records(db: &WbtrvDb) -> Vec<(i64, Vec<u8>)> {
    db.records().unwrap()
}

fn numbered(mut r: Vec<u8>, number: i32, auto: i32) -> Vec<u8> {
    r[34..38].copy_from_slice(&number.to_le_bytes());
    r[70..74].copy_from_slice(&auto.to_le_bytes());
    r
}

fn write(copy: &Path, game: &Path, options: &WriteOptions) -> Result<Vec<Written>, DatError> {
    write_db(&Realm::open(copy).unwrap(), game, options)
}

#[test]
fn a_snapshot_is_whole_while_the_game_writes() {
    let (tmp, game, _) = setup();
    let mut db = theirs(&game);
    let first = db.get(1).unwrap().unwrap();
    // written and kept open: the record is in the game's write-ahead log, not its file
    let id = db.insert(&numbered(first.clone(), 4321, 0)).unwrap();
    // and a write under way, not stored yet
    db.begin().unwrap();
    db.insert(&numbered(first, 4322, 0)).unwrap();
    let snap = tmp.path().join("snap.db");
    WbtrvDb::snapshot(&game.join("MBBSEMU.DB"), &snap).unwrap();
    db.rollback().unwrap();
    let copied = WbtrvDb::open_read_only(&snap).unwrap();
    assert_eq!(records(&copied), records(&db));
    assert_eq!(
        copied.get(id).unwrap().map(|r| r[34..38].to_vec()),
        Some(4321i32.to_le_bytes().to_vec())
    );
    assert_eq!(copied.keys(), db.keys());
}

#[test]
fn an_untouched_copy_writes_nothing() {
    let (tmp, game, copy) = setup();
    theirs(&game).delete(4).unwrap();
    let before = records(&theirs(&game));
    let backup = tmp.path().join("backup");
    let options = WriteOptions {
        backup: Some(backup.clone()),
        ..Default::default()
    };
    let written = write(&copy, &game, &options).unwrap();
    assert_eq!(written[0].action, Action::Unchanged);
    assert_eq!(records(&theirs(&game)), before);
    assert!(!backup.exists());
}

#[test]
fn edits_go_onto_the_games_records_where_they_are() {
    let (tmp, game, copy) = setup();
    // play changes a record the copy never had so
    let mut g = theirs(&game);
    let mut other = g.get(3).unwrap().unwrap();
    other[0] ^= 0xFF;
    g.update(3, &other).unwrap();
    drop(g);
    let mut edited = ours(&copy).get(2).unwrap().unwrap();
    edited[1] ^= 0xFF;
    ours(&copy).update(2, &edited).unwrap();
    let before = records(&theirs(&game));

    let dry = WriteOptions {
        dry_run: true,
        ..Default::default()
    };
    let planned = write(&copy, &game, &dry).unwrap();
    assert_eq!(
        planned[0].action,
        Action::Rows {
            changed: 1,
            added: 0,
            deleted: 0
        }
    );
    assert_eq!(records(&theirs(&game)), before);

    let backup = tmp.path().join("backup");
    let options = WriteOptions {
        backup: Some(backup.clone()),
        ..Default::default()
    };
    let written = write(&copy, &game, &options).unwrap();
    assert!(written[0].merged && written[0].skipped.is_empty());
    let g = theirs(&game);
    // the edit at the record's own id, and play's change kept though the copy never had it
    assert_eq!(g.get(2).unwrap().unwrap(), edited);
    assert_eq!(g.get(3).unwrap().unwrap(), other);
    assert_ne!(ours(&copy).get(3).unwrap().unwrap(), other);
    assert_eq!(g.count().unwrap(), 4);
    // what was replaced is in the backup
    let saved = WbtrvDb::open_read_only(&backup.join("MBBSEMU.DB")).unwrap();
    assert_eq!(records(&saved), before);
    // the game's folder keeps no journal of Daydream's
    assert!(!game.join(".daydream").exists());
    // written: the copy's changes count afresh
    assert!(
        journal::pending(&copy, journal::BASE)
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(
        write(&copy, &game, &WriteOptions::default()).unwrap()[0].action,
        Action::Unchanged
    );
}

#[test]
fn adds_deletes_and_renumbering_keep_the_other_ids() {
    let (_tmp, game, copy) = setup();
    let mut d = ours(&copy);
    let first = d.get(1).unwrap().unwrap();
    let added = numbered(first.clone(), 987_654, 0);
    d.insert(&added).unwrap();
    d.delete(3).unwrap();
    let mut renumbered = d.get(4).unwrap().unwrap();
    renumbered[34..38].copy_from_slice(&888i32.to_le_bytes());
    d.update(4, &renumbered).unwrap();
    drop(d);
    // play adds a record of its own meanwhile
    let mine = theirs(&game).insert(&numbered(first, 555, 98)).unwrap();

    let written = write(&copy, &game, &WriteOptions::default()).unwrap();
    assert_eq!(
        written[0].action,
        Action::Rows {
            changed: 0,
            added: 2,
            deleted: 2
        },
        "{written:?}"
    );
    let g = theirs(&game);
    let now = records(&g);
    let ids: Vec<i64> = now.iter().map(|(id, _)| *id).collect();
    // 1 and 2 where they were, play's own record kept, 3 gone, 4 as another record
    assert!(ids.starts_with(&[1, 2]), "{ids:?}");
    assert!(ids.contains(&mine));
    assert!(!ids.contains(&3) && !ids.contains(&4), "{ids:?}");
    let numbers: Vec<i32> = now
        .iter()
        .map(|(_, r)| i32::from_le_bytes(r[34..38].try_into().unwrap()))
        .collect();
    assert!(numbers.contains(&987_654) && numbers.contains(&888) && numbers.contains(&555));
    assert_eq!(now.len(), 5);
    // the added record's auto-increment number was given as wbtrv32 gives one
    let (_, r) = now
        .iter()
        .find(|(_, r)| r[34..38] == 987_654i32.to_le_bytes())
        .unwrap();
    assert_ne!(r[70..74], [0, 0, 0, 0]);
}

#[test]
fn changes_the_game_made_impossible_are_skipped_and_kept() {
    let (_tmp, game, copy) = setup();
    let mut d = ours(&copy);
    let first = d.get(1).unwrap().unwrap();
    // an edit of a record play deletes meanwhile
    let mut r = d.get(2).unwrap().unwrap();
    r[0] ^= 0xFF;
    d.update(2, &r).unwrap();
    // an add of a number play takes meanwhile, one of an auto-increment number play takes,
    // and one it does not clash with
    let clash = d.insert(&numbered(first.clone(), 555, 99)).unwrap();
    let auto = d.insert(&numbered(first.clone(), 557, 97)).unwrap();
    let fine = d.insert(&numbered(first.clone(), 556, 100)).unwrap();
    drop(d);
    let mut g = theirs(&game);
    g.delete(2).unwrap();
    g.insert(&numbered(first.clone(), 555, 98)).unwrap();
    g.insert(&numbered(first, 600, 97)).unwrap();
    drop(g);

    let written = write(&copy, &game, &WriteOptions::default()).unwrap();
    let w = &written[0];
    assert_eq!(
        w.action,
        Action::Rows {
            changed: 0,
            added: 1,
            deleted: 0
        },
        "{w:?}"
    );
    assert_eq!(w.skipped.len(), 3, "{w:?}");
    assert!(w.skipped[0].contains("has the same key"), "{w:?}");
    assert!(w.skipped[1].contains("no longer has it"), "{w:?}");
    assert!(w.skipped[2].contains("added one of its own"), "{w:?}");
    // the three stay among the copy's changes, and are reported again
    let mut ids: Vec<i64> = journal::pending(&copy, journal::BASE)
        .unwrap()
        .entries
        .iter()
        .map(|e| e.id)
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, [2, clash, auto]);
    assert!(!ids.contains(&fine));
    let again = write(&copy, &game, &WriteOptions::default()).unwrap();
    assert_eq!(again[0].action, Action::Unchanged);
    assert_eq!(again[0].skipped, w.skipped);
}

#[test]
fn a_key_the_game_never_changes_is_rewritten_at_its_id() {
    let (_tmp, game, copy) = setup();
    // the name at 2 cannot change through an update: the editor puts the record back
    let mut d = ours(&copy);
    let mut r = d.get(2).unwrap().unwrap();
    r[2..6].copy_from_slice(b"new\0");
    d.delete(2).unwrap();
    d.restore(2, &r).unwrap();
    drop(d);
    let written = write(&copy, &game, &WriteOptions::default()).unwrap();
    assert_eq!(
        written[0].action,
        Action::Rows {
            changed: 1,
            added: 0,
            deleted: 0
        }
    );
    assert_eq!(theirs(&game).get(2).unwrap().unwrap(), r);
}

/// MBBSEmu keeps its `.db` files in SQLite's rollback journal, not wbtrv32's write-ahead log:
/// they are written as they are kept.
#[test]
fn a_games_journal_mode_is_kept() {
    let (tmp, game, copy) = setup();
    let file = game.join("MBBSEMU.DB");
    let mode = |set: &str| -> String {
        let conn = rusqlite::Connection::open(&file).unwrap();
        conn.query_row(&format!("PRAGMA journal_mode{set}"), [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(mode(" = DELETE"), "delete");
    let mut d = ours(&copy);
    let mut r = d.get(2).unwrap().unwrap();
    r[0] ^= 0xFF;
    d.update(2, &r).unwrap();
    drop(d);
    let backup = tmp.path().join("backup");
    let options = WriteOptions {
        backup: Some(backup.clone()),
        ..WriteOptions::default()
    };
    let written = write(&copy, &game, &options).unwrap();
    assert!(matches!(written[0].action, Action::Rows { changed: 1, .. }));
    assert_eq!(mode(""), "delete");
    assert!(!game.join("MBBSEMU.DB-wal").exists());
    let got = WbtrvDb::open_read_only(&file).unwrap();
    assert_eq!(got.get(2).unwrap().unwrap(), r);
    assert_ne!(
        WbtrvDb::open_read_only(&backup.join("MBBSEMU.DB"))
            .unwrap()
            .get(2)
            .unwrap()
            .unwrap(),
        r
    );
}

#[test]
fn the_update_file_is_written_whole() {
    let (_tmp, game, copy) = setup();
    WbtrvDb::convert(&fixture("MBBSEMU.DAT"), &game.join("WCCUPDAT.DB")).unwrap();
    WbtrvDb::snapshot(&game.join("WCCUPDAT.DB"), &copy.join("WCCUPDAT.db")).unwrap();
    // as the update file is kept in step: no journal of its own
    let mut u = WbtrvDb::open(&copy.join("WCCUPDAT.db")).unwrap();
    u.track(false);
    u.delete(1).unwrap();
    let mut r = u.get(3).unwrap().unwrap();
    r[0] ^= 0xFF;
    u.update(3, &r).unwrap();
    let want = records(&u);
    drop(u);
    let written = write(&copy, &game, &WriteOptions::default()).unwrap();
    let upd = written.iter().find(|w| w.stem == "WCCUPDAT").unwrap();
    assert_eq!(upd.action, Action::Rebuilt(3));
    assert!(!upd.merged);
    let got = WbtrvDb::open_read_only(&game.join("WCCUPDAT.DB")).unwrap();
    assert_eq!(records(&got), want);
}

#[test]
fn a_copy_that_cannot_tell_its_edits_is_refused() {
    let (_tmp, game, copy) = setup();
    std::fs::remove_dir_all(copy.join(".daydream")).unwrap();
    assert!(matches!(
        write(&copy, &game, &WriteOptions::default()),
        Err(DatError::NoJournal)
    ));
    let (_tmp, game, copy) = setup();
    std::fs::remove_file(game.join("MBBSEMU.DB")).unwrap();
    assert!(matches!(
        write(&copy, &game, &WriteOptions::default()),
        Err(DatError::NoDb { .. })
    ));
}
