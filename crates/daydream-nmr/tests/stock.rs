//! The stock 1.11p world through NMR's tables: exporting and importing it back changes no
//! record, and every world record rebuilt from the export alone matches the original in
//! every byte NMR's tables carry.

use daydream_nmr::{ExportOptions, ImportOptions, export, import};
use daydream_store::Realm;

use crate::support::{assert_same_data, copy_realm, empty_realm, records, stock_realm};

fn full() -> ExportOptions {
    ExportOptions {
        zero_runtime: false,
        ..ExportOptions::default()
    }
}

#[test]
fn stock_round_trips_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    copy_realm(&stock_realm(), dir.path());
    let before = records(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let data = export(&realm, &full()).unwrap();
    for t in &data.tables {
        assert!(!t.rows.is_empty(), "{}: no rows", t.name);
    }
    let reports = import(&realm, &data, &ImportOptions::default()).unwrap();
    for r in &reports {
        assert_eq!(r.errors, Vec::<String>::new(), "{}", r.table);
        assert_eq!((r.inserted, r.updated), (0, 0), "{}: {r:?}", r.table);
    }
    assert_eq!(records(dir.path()), before);
}

#[test]
fn stock_rebuilds_from_the_export() {
    let data = export(&Realm::open(&stock_realm()).unwrap(), &full()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let reports = import(&realm, &data, &ImportOptions::default()).unwrap();
    for r in &reports {
        assert_eq!(r.errors, Vec::<String>::new(), "{}", r.table);
    }
    assert_same_data(&data, &export(&realm, &full()).unwrap());
}
