//! The batch of cases the real-NMR job imports (`windows_nmr`), checked here against
//! Daydream alone: importing them into the stock world and exporting again gives them back.
//! With `DAYDREAM_NMR_REALM` naming a folder of Worldgroup 3 `.dat` files (as the NMR job
//! uses), it checks them on that realm too.

use std::path::Path;

use daydream_nmr::{ExportOptions, ImportOptions, export, import};
use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::support::{add_cases, assert_same_data, copy_realm, stock_realm};

#[test]
fn the_cases_import_and_export_back() {
    check(&stock_realm(), LayoutSet::Dos);
}

#[test]
fn the_cases_import_and_export_back_on_a_worldgroup_realm() {
    let Some(dats) = std::env::var_os("DAYDREAM_NMR_REALM").filter(|v| !v.is_empty()) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    daydream_store::realm::convert_dir(Path::new(&dats), dir.path()).unwrap();
    check(dir.path(), LayoutSet::Worldgroup);
}

fn check(realm_dir: &Path, layout: LayoutSet) {
    let opts = ExportOptions {
        zero_runtime: false,
        layout,
        ..ExportOptions::default()
    };
    let dir = tempfile::tempdir().unwrap();
    copy_realm(realm_dir, dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let mut data = export(&realm, &opts).unwrap();
    let cases = add_cases(&mut data);
    assert_eq!(cases.len(), 10, "{cases:?}");
    for c in &cases {
        assert!(c.edited > 0 && c.added == 2, "{c:?}");
    }
    let import_opts = ImportOptions {
        layout,
        ..ImportOptions::default()
    };
    let reports = import(&realm, &data, &import_opts).unwrap();
    for r in &reports {
        assert_eq!(r.errors, Vec::<String>::new(), "{}", r.table);
        assert_eq!(r.inserted, 2, "{}: {r:?}", r.table);
        assert!(r.updated > 0, "{}: {r:?}", r.table);
    }
    assert_same_data(&data, &export(&realm, &opts).unwrap());
}
