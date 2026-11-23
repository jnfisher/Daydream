//! Writes keep the key columns, constraints and record positions as wbtrv32 would.

use daydream_store::{StoreError, WbtrvDb};
use rusqlite::Connection;

use crate::support::{Cell, fixture, rows};

/// MBBSEMU.DAT: 74-byte records; key 0 a Zstring at 2 (duplicates, non-modifiable), key 1
/// an Integer at 34 (unique, modifiable), key 2 a Zstring at 38 (duplicates, modifiable),
/// key 3 an AutoInc at 70 (unique, non-modifiable).
fn converted() -> (tempfile::TempDir, std::path::PathBuf, WbtrvDb) {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("MBBSEMU.db");
    let (db, report) = WbtrvDb::convert(&fixture("MBBSEMU.DAT"), &dest).unwrap();
    assert_eq!(report.records, 4);
    (tmp, dest, db)
}

fn record(db: &WbtrvDb, id: i64) -> Vec<u8> {
    db.get(id).unwrap().unwrap()
}

#[test]
fn an_update_recomputes_the_key_columns() {
    let (_tmp, path, mut db) = converted();
    let mut r = record(&db, 2);
    r[34..38].copy_from_slice(&123_456i32.to_le_bytes());
    r[38..42].copy_from_slice(b"abc\0");
    db.update(2, &r).unwrap();
    assert_eq!(record(&db, 2), r);
    let conn = Connection::open(&path).unwrap();
    let row = rows(&conn, "SELECT key_1, key_2 FROM data_t WHERE id = 2");
    assert_eq!(
        row,
        vec![vec![Cell::Integer(123_456), Cell::Text(b"abc".to_vec())]]
    );
}

#[test]
fn a_non_modifiable_key_cannot_change() {
    let (_tmp, _path, mut db) = converted();
    let before = record(&db, 1);
    let mut r = before.clone();
    r[2] = b'X';
    assert!(matches!(db.update(1, &r), Err(StoreError::Sqlite { .. })));
    assert_eq!(record(&db, 1), before);
}

#[test]
fn a_unique_key_cannot_clash() {
    let (_tmp, _path, mut db) = converted();
    let other = record(&db, 1);
    let mut r = record(&db, 2);
    r[34..38].copy_from_slice(&other[34..38]);
    assert!(db.update(2, &r).is_err());
}

#[test]
fn an_insert_fills_a_zero_autoinc_key_and_delete_keeps_other_ids() {
    let (_tmp, path, mut db) = converted();
    let mut r = record(&db, 4);
    r[34..38].copy_from_slice(&999i32.to_le_bytes());
    r[70..74].fill(0);
    let id = db.insert(&r).unwrap();
    assert_eq!(id, 5);
    let stored = record(&db, 5);
    let max: i64 = Connection::open(&path)
        .unwrap()
        .query_row("SELECT MAX(key_3) FROM data_t WHERE id < 5", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        i32::from_le_bytes(stored[70..74].try_into().unwrap()),
        max as i32 + 1
    );
    db.delete(3).unwrap();
    assert_eq!(db.get(3).unwrap(), None);
    assert_eq!(record(&db, 4).len(), 74);
    assert!(matches!(db.delete(3), Err(StoreError::NoRecord { .. })));
}

#[test]
fn records_must_be_the_file_length() {
    let (_tmp, _path, mut db) = converted();
    assert!(matches!(
        db.update(1, &[0; 10]),
        Err(StoreError::RecordLength {
            expected: 74,
            found: 10,
            ..
        })
    ));
}

#[test]
fn a_converted_file_opens_again_with_the_same_keys() {
    let (_tmp, path, db) = converted();
    let keys = db.keys().to_vec();
    drop(db);
    let again = WbtrvDb::open(&path).unwrap();
    assert_eq!(again.keys(), keys.as_slice());
    assert_eq!(again.metadata().record_length, 74);
}
