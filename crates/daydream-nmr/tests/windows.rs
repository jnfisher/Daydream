//! A real `.mdb` through Windows' Jet: Daydream writes it as NMR would, reads it back with
//! the pure-Rust reader, and imports it into a copy of the realm. Runs where the 32-bit
//! Jet engine is (Windows); elsewhere it checks only that writing says why it cannot.

use daydream_nmr::jet::{self, JetError};
use daydream_nmr::{ExportOptions, ImportOptions, NmrData, Value, export, import, mdb};
use daydream_store::Realm;

use crate::support::{assert_same_data, copy_realm, random_realm, records};

/// What Jet hands back for what Daydream wrote: an empty text goes in as Null.
fn as_written(mut data: NmrData) -> NmrData {
    for t in &mut data.tables {
        for row in &mut t.rows {
            for v in row {
                if matches!(v, Value::Text(s) if s.is_empty()) {
                    *v = Value::Null;
                }
            }
        }
    }
    data
}

fn through_jet(realm_dir: &std::path::Path) {
    let realm = Realm::open(realm_dir).unwrap();
    let opts = ExportOptions {
        zero_runtime: false,
        custom: "Daydream test".into(),
        date: "11/22/2026".into(),
        time: "7:50:00 PM".into(),
        ..ExportOptions::default()
    };
    let data = export(&realm, &opts).unwrap();
    let out = tempfile::tempdir().unwrap();
    let file = out.path().join("realm.mdb");
    jet::write_mdb(&data, &file).unwrap();

    let read = mdb::read(&file).unwrap();
    assert_same_data(&as_written(data), &read);

    let dir = tempfile::tempdir().unwrap();
    copy_realm(realm_dir, dir.path());
    let before = records(dir.path());
    let copy = Realm::open(dir.path()).unwrap();
    let reports = import(&copy, &read, &ImportOptions::default()).unwrap();
    for r in &reports {
        assert_eq!(r.errors, Vec::<String>::new(), "{}", r.table);
        assert_eq!((r.inserted, r.updated), (0, 0), "{}: {r:?}", r.table);
    }
    assert_eq!(records(dir.path()), before);
}

#[test]
fn random_realm_through_jet() {
    if jet::powershell32().is_none() {
        let data = NmrData::default();
        let err = jet::write_mdb(&data, std::path::Path::new("never.mdb")).unwrap_err();
        assert!(matches!(err, JetError::NoJet), "{err}");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 30, 7);
    through_jet(dir.path());
}

#[test]
fn stock_through_jet() {
    if jet::powershell32().is_none() {
        return;
    }
    through_jet(&crate::support::stock_realm());
}

/// The MME export of the stock world, written through Jet as `daydream mme-export` writes
/// it, reads back as it went in.
#[test]
fn stock_mme_export_through_jet() {
    if jet::powershell32().is_none() {
        return;
    }
    let realm = Realm::open(&crate::support::stock_realm()).unwrap();
    let opts = daydream_nmr::mme::MmeOptions {
        date: "11/23/2026".into(),
        time: "2:00:00 AM".into(),
        ..Default::default()
    };
    let data =
        daydream_nmr::mme::export(&realm, daydream_nmr::layout_of_realm(&realm), &opts).unwrap();
    let out = tempfile::tempdir().unwrap();
    let file = out.path().join("data.mdb");
    jet::write_plan_mdb(&daydream_nmr::mme::plan(&data), &file).unwrap();
    let names: Vec<&str> = data.tables.iter().map(|t| t.name.as_str()).collect();
    let read = mdb::read_tables(&file, &names).unwrap();
    // a whole number in a Double column may come back as either
    let numbers = |mut d: NmrData| {
        for t in &mut d.tables {
            for row in &mut t.rows {
                for v in row {
                    if let Value::Long(x) = v {
                        *v = Value::Double(f64::from(*x));
                    }
                }
            }
        }
        d
    };
    // empty texts go in as zero-length texts, as in NMR's own MME export
    assert_same_data(&numbers(data), &numbers(read));
}
