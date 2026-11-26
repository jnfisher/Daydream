//! Writing a realm back into its Btrieve files: untouched files stay as they are, edits
//! that keep the keys go in place, and adds or deletes rebuild the file. Only the realm's
//! edits go in, onto the files as the game has them now.

use std::path::{Path, PathBuf};

use daydream_store::dat::{Action, DatError, WriteOptions, Written, write_dat};
use daydream_store::{Realm, WbtrvDb, journal, realm::convert_dir};
use daydream_wcc::btrieve::build::build;
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
    Ok(write_all(realm, wcc, options)?
        .into_iter()
        .map(|w| w.action)
        .collect())
}

fn write_all(realm: &Path, wcc: &Path, options: &WriteOptions) -> Result<Vec<Written>, DatError> {
    write_dat(&Realm::open(realm).unwrap(), wcc, options)
}

/// The game writing its file meanwhile: `change` gets its records, in order, and the file is
/// rebuilt with what it leaves. The records written.
fn game_writes(dat: &Path, change: impl FnOnce(&mut Vec<Vec<u8>>)) -> Vec<Vec<u8>> {
    let file = OpenOptions::new().open(dat).unwrap();
    let mut rs = records(dat);
    change(&mut rs);
    std::fs::write(dat, build(&file, &rs, 0).unwrap()).unwrap();
    rs
}

/// MBBSEMU.DAT's records are told apart by the number at 34 (its first unique key); the
/// auto-increment number at 70 must be unique too.
fn numbered(mut r: Vec<u8>, number: i32, auto: i32) -> Vec<u8> {
    r[34..38].copy_from_slice(&number.to_le_bytes());
    r[70..74].copy_from_slice(&auto.to_le_bytes());
    r
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
fn edits_merge_onto_what_the_game_changed_meanwhile() {
    let (_tmp, wcc, realm) = setup();
    let dat = wcc.join("MBBSEMU.DAT");
    let game = game_writes(&dat, |rs| rs[2][0] ^= 0xFF);
    let mut r = db(&realm).get(2).unwrap().unwrap();
    r[1] ^= 0xFF;
    db(&realm).update(2, &r).unwrap();

    let written = write_all(&realm, &wcc, &WriteOptions::default()).unwrap();
    assert_eq!(written[0].action, Action::Updated(1));
    assert!(written[0].merged && written[0].skipped.is_empty());
    let now = records(&dat);
    assert_eq!(now[1], r);
    // the game's own change stays, though the realm never had it
    assert_eq!(now[2], game[2]);
    assert_ne!(db_records(&realm)[2], game[2]);
    // written: the realm's changes count afresh
    assert!(
        journal::pending(&realm, journal::BASE)
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(
        write(&realm, &wcc, &WriteOptions::default()).unwrap(),
        [Action::Unchanged]
    );
}

#[test]
fn a_renumbered_record_replaces_the_one_it_was() {
    let (_tmp, wcc, realm) = setup();
    let dat = wcc.join("MBBSEMU.DAT");
    let game = game_writes(&dat, |rs| rs[2][0] ^= 0xFF);
    let mut r = db(&realm).get(3).unwrap().unwrap();
    r[34..38].copy_from_slice(&888i32.to_le_bytes());
    db(&realm).update(3, &r).unwrap();
    let written = write_all(&realm, &wcc, &WriteOptions::default()).unwrap();
    assert_eq!(written[0].action, Action::Rebuilt(4));
    let now = records(&dat);
    // the realm's record whole: nothing of the game's record it replaces carries over
    assert!(now.contains(&r));
    assert!(!now.contains(&game[2]));
    assert_eq!(now.len(), 4);
}

#[test]
fn a_file_without_edits_stays_as_the_game_has_it() {
    let (_tmp, wcc, realm) = setup();
    let dat = wcc.join("MBBSEMU.DAT");
    game_writes(&dat, |rs| {
        rs[0][0] ^= 0xFF;
        rs.remove(3);
    });
    let before = std::fs::read(&dat).unwrap();
    assert_eq!(
        write(&realm, &wcc, &WriteOptions::default()).unwrap(),
        [Action::Unchanged]
    );
    assert_eq!(std::fs::read(&dat).unwrap(), before);
}

#[test]
fn changes_the_game_has_made_impossible_are_skipped_and_kept() {
    let (_tmp, wcc, realm) = setup();
    let dat = wcc.join("MBBSEMU.DAT");
    let mut d = db(&realm);
    let first = d.get(1).unwrap().unwrap();
    // an edit of a record the game deletes meanwhile
    let mut r = d.get(2).unwrap().unwrap();
    r[0] ^= 0xFF;
    d.update(2, &r).unwrap();
    // a renumbering to, and an add of, numbers the game takes meanwhile
    let r3 = d.get(3).unwrap().unwrap();
    let mut renumbered = r3.clone();
    renumbered[34..38].copy_from_slice(&777i32.to_le_bytes());
    d.update(3, &renumbered).unwrap();
    let added = d.insert(&numbered(first.clone(), 555, 99)).unwrap();
    // and an add it does not clash with
    let fine = d.insert(&numbered(first.clone(), 556, 100)).unwrap();
    drop(d);
    game_writes(&dat, |rs| {
        rs.remove(1);
        rs.push(numbered(first.clone(), 555, 98));
        rs.push(numbered(first.clone(), 777, 97));
    });

    let written = write_all(&realm, &wcc, &WriteOptions::default()).unwrap();
    let w = &written[0];
    assert_eq!(w.action, Action::Rebuilt(6), "{w:?}");
    assert_eq!(w.skipped.len(), 3, "{w:?}");
    assert!(w.skipped[0].contains("no longer has it"), "{w:?}");
    assert!(w.skipped[1].contains("renumbered to"), "{w:?}");
    assert!(w.skipped[2].contains("added one of its own"), "{w:?}");
    let now = records(&dat);
    assert!(now.contains(&numbered(first.clone(), 556, 100)));
    assert!(now.contains(&r3));
    // the three stay among the realm's changes, and are reported again
    let ids: Vec<i64> = journal::pending(&realm, journal::BASE)
        .unwrap()
        .entries
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, [2, 3, added]);
    assert!(!ids.contains(&fine));
    let again = write_all(&realm, &wcc, &WriteOptions::default()).unwrap();
    assert_eq!(again[0].action, Action::Unchanged);
    assert_eq!(again[0].skipped, w.skipped);

    // the add given a free number goes in
    db(&realm)
        .update(added, &numbered(first.clone(), 557, 99))
        .unwrap();
    let written = write_all(&realm, &wcc, &WriteOptions::default()).unwrap();
    assert_eq!(written[0].action, Action::Rebuilt(7));
    assert_eq!(written[0].skipped.len(), 2);
    assert!(records(&dat).contains(&numbered(first, 557, 99)));
}

#[test]
fn a_realm_without_a_journal_is_written_whole_unless_its_file_changed() {
    let (_tmp, wcc, realm) = setup();
    // as a realm converted before Daydream journaled edits is
    std::fs::remove_dir_all(realm.join(".daydream")).unwrap();
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

    // forced, the file is the realm's; from then on only edits are written
    game_writes(&dat, |rs| rs[0][0] ^= 0xFF);
    let forced = WriteOptions {
        force: true,
        ..Default::default()
    };
    let written = write_all(&realm, &wcc, &forced).unwrap();
    assert!(!written[0].merged);
    assert_eq!(records(&dat), db_records(&realm));
    game_writes(&dat, |rs| rs[0][0] ^= 0xFF);
    let mut r = db(&realm).get(2).unwrap().unwrap();
    r[0] ^= 0xFF;
    db(&realm).update(2, &r).unwrap();
    let written = write_all(&realm, &wcc, &WriteOptions::default()).unwrap();
    assert!(written[0].merged);
    assert_eq!(written[0].action, Action::Updated(1));
    assert_ne!(records(&dat)[0], db_records(&realm)[0]);
}
