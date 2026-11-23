//! An opt-in check of a whole real realm: `DAYDREAM_REALM_FIXTURE` names a folder of a
//! sysop's `WCC*.DAT` files (Worldgroup 3 `WCCITEM2.DAT`, ... or DOS `WCCITEMS.DAT`, ...).
//! Real realms are someone's creative work, so none is kept in the repository; without
//! the variable the test passes without checking anything.
//!
//! Only the world files are used (items, monsters, spells, classes, races, shops, rooms,
//! messages, text blocks, actions). Players, banks, gangs, active monsters and item
//! ownership are left out: they are private, and they are state, not content.
//!
//! For each file: Daydream's conversion is wbtrv32's own (when the oracle is built, see
//! `oracle`), every index checks out and holds every record; then a third of the records
//! are deleted and added back, `write_dat` rebuilds the file, and the rebuilt file holds
//! the same records, passes its index check and converts as wbtrv32 converts it.

use std::path::{Path, PathBuf};

use daydream_store::dat::{Action, WriteOptions, write_dat};
use daydream_store::{Realm, WbtrvDb, realm::convert_dir};
use daydream_wcc::btrieve::{BtrieveFile, OpenOptions};
use daydream_wcc::tables::{Table, table_for_stem};

use crate::oracle::{compare, oracle};
use crate::support::copy_as;

fn fixture_dir() -> Option<PathBuf> {
    std::env::var_os("DAYDREAM_REALM_FIXTURE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Whether the file with this stem is world content (see the module comment).
fn is_world(stem: &str) -> bool {
    !matches!(
        table_for_stem(stem),
        None | Some(
            Table::Users
                | Table::BankBooks
                | Table::Gangs
                | Table::ActiveMonsters
                | Table::ItemOwnership
        )
    )
}

/// Copies the world `.DAT` files of `src` into `dest`, returning their stems.
fn copy_world(src: &Path, dest: &Path) -> Vec<String> {
    let mut stems = Vec::new();
    for entry in std::fs::read_dir(src).unwrap() {
        let path = entry.unwrap().path();
        let Some(stem) = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_ascii_uppercase())
        else {
            continue;
        };
        let dat = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("dat"));
        if dat && is_world(&stem) {
            copy_as(&path, dest, &format!("{stem}.DAT"));
            stems.push(stem);
        }
    }
    stems.sort();
    stems
}

/// The file's records in physical order, after its indexes check out.
fn records(path: &Path) -> Vec<Vec<u8>> {
    let mut file: BtrieveFile = OpenOptions::new()
        .variable_length(true)
        .open(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    file.verify_indexes()
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let expected = file.header.record_count;
    let mut out = Vec::new();
    let mut cursor = file.records();
    while let Some(r) = cursor.next_record() {
        out.push(r.unwrap().data.to_vec());
    }
    assert_eq!(out.len() as u64, u64::from(expected), "{}", path.display());
    out
}

fn sorted(mut v: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    v.sort();
    v
}

#[test]
fn a_real_realm_converts_rebuilds_and_writes_back() {
    let Some(src) = fixture_dir() else {
        return;
    };
    let oracle = oracle();
    let tmp = tempfile::tempdir().unwrap();
    let (wcc, realm_dir, out) = (
        tmp.path().join("wcc"),
        tmp.path().join("realm"),
        tmp.path().join("out"),
    );
    std::fs::create_dir(&wcc).unwrap();
    let stems = copy_world(&src, &wcc);
    assert!(
        !stems.is_empty(),
        "no world .DAT files in {}",
        src.display()
    );

    let mut before = Vec::new();
    for stem in &stems {
        let dat = wcc.join(format!("{stem}.DAT"));
        if let Some(o) = &oracle {
            compare(o, &dat);
        }
        before.push(records(&dat));
    }

    convert_dir(&wcc, &realm_dir).unwrap();
    for stem in &stems {
        let mut db = WbtrvDb::open(&realm_dir.join(format!("{stem}.db"))).unwrap();
        let rows = db.records().unwrap();
        for (id, data) in rows.iter().step_by(3) {
            db.delete(*id).unwrap();
            db.insert(data).unwrap();
        }
    }

    let realm = Realm::open(&realm_dir).unwrap();
    let options = WriteOptions {
        out: Some(out.clone()),
        ..WriteOptions::default()
    };
    let written = write_dat(&realm, &wcc, &options).unwrap();
    for (stem, before) in stems.iter().zip(before) {
        let w = written.iter().find(|w| &w.stem == stem).unwrap();
        if !before.is_empty() {
            assert!(
                matches!(w.action, Action::Rebuilt(_)),
                "{stem}: {:?}",
                w.action
            );
        }
        let dat = out.join(format!("{stem}.DAT"));
        assert_eq!(sorted(records(&dat)), sorted(before), "{stem}");
        if let Some(o) = &oracle {
            compare(o, &dat);
        }
    }
}
