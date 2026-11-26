//! Publishing onto a game that ran meanwhile, on the stock 1.11p world: the members of a room
//! the realm edited go in, and what the game changed in the same room meanwhile stays.

use std::path::Path;

use daydream_store::dat::{Action, WriteOptions, write_dat};
use daydream_store::realm::{convert_dir_matching, is_wcc_stem};
use daydream_store::{Realm, WbtrvDb};
use daydream_types::records::layout::ROOM;
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

#[test]
fn an_edit_is_published_onto_the_room_as_the_game_has_it() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("game");
    std::fs::create_dir(&game).unwrap();
    for e in std::fs::read_dir(stock_dir()).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_file() {
            std::fs::copy(e.path(), game.join(e.file_name())).unwrap();
        }
    }
    let realm = tmp.path().join("realm");
    convert_dir_matching(&game, &realm, is_wcc_stem).unwrap();

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
