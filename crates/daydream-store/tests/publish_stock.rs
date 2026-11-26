//! Publishing onto a game that ran meanwhile, on the stock 1.11p world: the members of a room
//! the realm edited go in, and what the game changed in the same room meanwhile stays.

use std::path::Path;

use daydream_store::dat::{
    Action, DatError, WriteOptions, not_carried, write_dat, write_update_file,
};
use daydream_store::realm::{convert_dir_matching, is_wcc_stem};
use daydream_store::update::Kind;
use daydream_store::{Realm, WbtrvDb};
use daydream_types::records::layout::{ITEM, ROOM};
use daydream_wcc::btrieve::OpenOptions;
use daydream_wcc::btrieve::build::build;

use crate::support::stock_dir;

fn records(path: &Path) -> Vec<Vec<u8>> {
    let mut file = OpenOptions::new().open(path).unwrap();
    file.verify_indexes().unwrap();
    let mut out = Vec::new();
    let mut cursor = file.records();
    while let Some(r) = cursor.next_record() {
        out.push(r.unwrap().data.to_vec());
    }
    out
}

fn slot(room: &mut [u8], i: usize, item: i32) {
    let at = ROOM.field("items").unwrap().offset + 4 * i;
    room[at..at + 4].copy_from_slice(&item.to_le_bytes());
}

fn slot_of(room: &[u8], i: usize) -> i32 {
    let at = ROOM.field("items").unwrap().offset + 4 * i;
    i32::from_le_bytes(room[at..at + 4].try_into().unwrap())
}

/// A copy of the stock game in `tmp/game`, and a realm converted from it in `tmp/realm`.
fn game_and_realm(tmp: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let game = tmp.join("game");
    std::fs::create_dir(&game).unwrap();
    for e in std::fs::read_dir(stock_dir()).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_file() {
            std::fs::copy(e.path(), game.join(e.file_name())).unwrap();
        }
    }
    let realm = tmp.join("realm");
    convert_dir_matching(&game, &realm, is_wcc_stem).unwrap();
    (game, realm)
}

#[test]
fn an_edit_is_published_onto_the_room_as_the_game_has_it() {
    let tmp = tempfile::tempdir().unwrap();
    let (game, realm) = game_and_realm(tmp.path());

    // the game runs meanwhile: an item lands in the first room's slot 3
    let vir = game.join("WCCMP001.VIR");
    let mut rooms = records(&vir);
    slot(&mut rooms[0], 3, 40);
    let template = OpenOptions::new().open(&vir).unwrap();
    std::fs::write(
        game.join("WCCMP001.DAT"),
        build(&template, &rooms, 0).unwrap(),
    )
    .unwrap();

    // the realm renames the room and puts an item in its slot 0
    let mut db = WbtrvDb::open(&realm.join("WCCMP001.db")).unwrap();
    let (id, mut room) = db
        .records()
        .unwrap()
        .into_iter()
        .find(|(_, r)| r[..8] == rooms[0][..8])
        .unwrap();
    let name = ROOM.field("name").unwrap();
    room[name.offset..name.offset + name.size].fill(0);
    room[name.offset..name.offset + 7].copy_from_slice(b"Reverie");
    slot(&mut room, 0, 12);
    db.update(id, &room).unwrap();
    drop(db);

    let written = write_dat(
        &Realm::open(&realm).unwrap(),
        &game,
        &WriteOptions::default(),
    )
    .unwrap();
    for w in &written {
        match w.stem.as_str() {
            "WCCMP001" => {
                assert!(w.merged && w.skipped.is_empty(), "{w:?}");
                assert_eq!(w.action, Action::Updated(1));
            }
            "WCCUPDAT" => assert!(!w.merged),
            _ => assert_eq!(w.action, Action::Unchanged, "{w:?}"),
        }
    }
    let now = records(&game.join("WCCMP001.DAT"));
    let r = now.iter().find(|r| r[..8] == rooms[0][..8]).unwrap();
    assert_eq!(&r[name.offset..name.offset + 8], b"Reverie\0");
    assert_eq!((slot_of(r, 0), slot_of(r, 3)), (12, 40));
    // the rest of the room is the game's
    let mut expected = rooms[0].clone();
    expected[name.offset..name.offset + name.size]
        .copy_from_slice(&room[name.offset..name.offset + name.size]);
    slot(&mut expected, 0, 12);
    assert_eq!(r, &expected);
}

#[test]
fn a_publish_through_the_update_file_alone_leaves_the_world_files_to_the_game() {
    let tmp = tempfile::tempdir().unwrap();
    let (game, realm_dir) = game_and_realm(tmp.path());
    let world: Vec<(std::ffi::OsString, Vec<u8>)> = std::fs::read_dir(&game)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| !e.file_name().to_string_lossy().starts_with("WCCUPDAT"))
        .map(|e| (e.file_name(), std::fs::read(e.path()).unwrap()))
        .collect();

    // the realm makes the first item hit harder (stock items start at 9, not 1)
    let maxdmg = ITEM.field("maxdmg").unwrap();
    let mut db = WbtrvDb::open(&realm_dir.join("WCCITEMS.db")).unwrap();
    let (id, mut item) = db.records().unwrap().remove(0);
    let number = item[..4].to_vec();
    item[maxdmg.offset..maxdmg.offset + 2].copy_from_slice(&99i16.to_le_bytes());
    db.update(id, &item).unwrap();
    drop(db);

    let realm = Realm::open(&realm_dir).unwrap();
    assert_eq!(not_carried(&realm).unwrap(), Vec::<String>::new());
    let w = write_update_file(&realm, &game, &WriteOptions::default()).unwrap();
    assert_eq!(w.stem, "WCCUPDAT");
    // the world's files are as they were: the game merges the edit at its next start
    for (name, bytes) in &world {
        assert!(
            std::fs::read(game.join(name)).unwrap() == *bytes,
            "{name:?} was written"
        );
    }
    // its update file carries the item as edited (a DOS update record: seqnum, type, image)
    let images: Vec<Vec<u8>> = records(&game.join("WCCUPDAT.DAT"))
        .into_iter()
        .filter(|r| r[4] == Kind::Item.code() && r[5..9] == number[..])
        .collect();
    assert_eq!(images.len(), 1);
    let at = 5 + maxdmg.offset;
    assert_eq!(images[0][at..at + 2], 99i16.to_le_bytes());
    // and the realm's changes count afresh: the game has them to apply
    let changes = daydream_store::changes::list(&realm).unwrap();
    assert!(changes.changes.is_empty(), "{:?}", changes.changes);

    // what lies on a room's floor is the game's: the update file cannot carry it
    let mut db = WbtrvDb::open(&realm_dir.join("WCCMP001.db")).unwrap();
    let (id, mut room) = db.records().unwrap().into_iter().next().unwrap();
    slot(&mut room, 0, 12);
    db.update(id, &room).unwrap();
    drop(db);
    let not = not_carried(&realm).unwrap();
    assert_eq!(not.len(), 1, "{not:?}");
    assert!(not[0].contains("items"), "{not:?}");
    let refused = write_update_file(&realm, &game, &WriteOptions::default());
    assert!(
        matches!(refused, Err(DatError::NotCarried(_))),
        "{refused:?}"
    );
}
