//! Writing a realm back into its Btrieve files: untouched files stay as they are, edits
//! that keep the keys go in place, and adds or deletes rebuild the file.

use std::path::{Path, PathBuf};

use daydream_store::dat::{Action, DatError, WriteOptions, write_dat};
use daydream_store::{Realm, WbtrvDb, realm::convert_dir};
use daydream_wcc::btrieve::{BtrieveFile, OpenOptions};

use crate::support::{copy_as, fixture};

/// A WCC folder holding MBBSEMU.DAT and its realm.
fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let wcc = tmp.path().join("wcc");
    let realm = tmp.path().join("realm");
    std::fs::create_dir(&wcc).unwrap();
    copy_as(&fixture("MBBSEMU.DAT"), &wcc, "MBBSEMU.DAT");
    convert_dir(&wcc, &realm).unwrap();
    (tmp, wcc, realm)
}

fn db(realm: &Path) -> WbtrvDb {
    WbtrvDb::open(&realm.join("MBBSEMU.db")).unwrap()
}

fn write(realm: &Path, wcc: &Path, options: &WriteOptions) -> Result<Vec<Action>, DatError> {
    let realm = Realm::open(realm).unwrap();
    Ok(write_dat(&realm, wcc, options)?
        .into_iter()
        .map(|w| w.action)
        .collect())
}

/// The file's records in physical order, after its indexes check out.
fn records(path: &Path) -> Vec<Vec<u8>> {
    let mut file: BtrieveFile = OpenOptions::new().open(path).unwrap();
    file.verify_indexes().unwrap();
    let mut out = Vec::new();
    let mut cursor = file.records();
    while let Some(r) = cursor.next_record() {
        out.push(r.unwrap().data.to_vec());
    }
    out
}

fn db_records(realm: &Path) -> Vec<Vec<u8>> {
    db(realm)
        .records()
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect()
}

#[test]
fn an_untouched_realm_writes_nothing() {
    let (tmp, wcc, realm) = setup();
    let before = std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap();
    let backup = tmp.path().join("backup");
    let options = WriteOptions {
        backup: Some(backup.clone()),
        ..Default::default()
    };
    assert_eq!(write(&realm, &wcc, &options).unwrap(), [Action::Unchanged]);
    assert_eq!(std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap(), before);
    assert!(!backup.exists());
}

#[test]
fn an_edit_outside_the_keys_goes_in_place() {
    let (tmp, wcc, realm) = setup();
    let dat = wcc.join("MBBSEMU.DAT");
    let before = std::fs::read(&dat).unwrap();
    let mut r = db(&realm).get(2).unwrap().unwrap();
    r[0] ^= 0xFF;
    db(&realm).update(2, &r).unwrap();
    let backup = tmp.path().join("backup");
    let options = WriteOptions {
        backup: Some(backup.clone()),
        ..Default::default()
    };
    assert_eq!(write(&realm, &wcc, &options).unwrap(), [Action::Updated(1)]);
    let after = std::fs::read(&dat).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(after.iter().zip(&before).filter(|(a, b)| a != b).count(), 1);
    assert_eq!(records(&dat), db_records(&realm));
    assert_eq!(std::fs::read(backup.join("MBBSEMU.DAT")).unwrap(), before);
    // the .db now matches the file written, so a second write has nothing to do
    assert_eq!(
        write(&realm, &wcc, &WriteOptions::default()).unwrap(),
        [Action::Unchanged]
    );
}

#[test]
fn adds_and_deletes_rebuild_the_file() {
    let (_tmp, wcc, realm) = setup();
    let mut d = db(&realm);
    let mut r = d.get(1).unwrap().unwrap();
    r[34..38].copy_from_slice(&987_654i32.to_le_bytes());
    r[70..74].copy_from_slice(&0i32.to_le_bytes());
    d.insert(&r).unwrap();
    d.delete(3).unwrap();
    drop(d);
    let dry = WriteOptions {
        dry_run: true,
        ..Default::default()
    };
    let before = std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap();
    assert_eq!(write(&realm, &wcc, &dry).unwrap(), [Action::Rebuilt(4)]);
    assert_eq!(std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap(), before);
    assert_eq!(
        write(&realm, &wcc, &WriteOptions::default()).unwrap(),
        [Action::Rebuilt(4)]
    );
    assert_eq!(records(&wcc.join("MBBSEMU.DAT")), db_records(&realm));
}

#[test]
fn writing_to_another_folder_leaves_the_wcc_folder_alone() {
    let (tmp, wcc, realm) = setup();
    let before = std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap();
    let mut d = db(&realm);
    let r = d.get(4).unwrap().unwrap();
    d.delete(4).unwrap();
    drop(d);
    let out = tmp.path().join("out");
    let options = WriteOptions {
        out: Some(out.clone()),
        ..Default::default()
    };
    assert_eq!(write(&realm, &wcc, &options).unwrap(), [Action::Rebuilt(3)]);
    assert_eq!(std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap(), before);
    let written = records(&out.join("MBBSEMU.DAT"));
    assert_eq!(written, db_records(&realm));
    assert!(!written.contains(&r));
}

#[test]
fn a_wcc_file_that_changed_since_the_conversion_is_refused() {
    let (_tmp, wcc, realm) = setup();
    let dat = wcc.join("MBBSEMU.DAT");
    let mut bytes = std::fs::read(&dat).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    std::fs::write(&dat, &bytes).unwrap();
    assert!(matches!(
        write(&realm, &wcc, &WriteOptions::default()),
        Err(DatError::SourceChanged { .. })
    ));
    assert_eq!(std::fs::read(&dat).unwrap(), bytes);
}
