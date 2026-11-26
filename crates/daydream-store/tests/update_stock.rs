//! The stock update file follows edits: an edited record's image changes, a deleted record's
//! image goes, and the file written back into a WCC folder holds the edits.

use std::path::Path;

use daydream_store::dat::{WriteOptions, write_dat};
use daydream_store::realm::{convert_dir_matching, is_wcc_stem};
use daydream_store::update::{self, Compile, Kind};
use daydream_store::{Realm, WbtrvDb};
use daydream_types::records::layout::{ITEM, ROOM};
use daydream_wcc::btrieve::OpenOptions;

use crate::support::stock_dir;

/// The records of the update file `path` (`.DAT` or `.db`): `(seqnum, type, image)`.
fn images_of_dat(path: &Path) -> Vec<(i32, u8, Vec<u8>)> {
    let mut file = OpenOptions::new().open(path).unwrap();
    file.verify_indexes().unwrap();
    let mut out = Vec::new();
    let mut cursor = file.records();
    while let Some(r) = cursor.next_record() {
        let d = r.unwrap().data;
        out.push((
            i32::from_le_bytes(d[..4].try_into().unwrap()),
            d[4],
            d[5..].to_vec(),
        ));
    }
    out
}

fn images_of_db(path: &Path) -> Vec<(i32, u8, Vec<u8>)> {
    WbtrvDb::open_read_only(path)
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .map(|(_, d)| {
            (
                i32::from_le_bytes(d[..4].try_into().unwrap()),
                d[4],
                d[5..].to_vec(),
            )
        })
        .collect()
}

fn image<'a>(
    images: &'a [(i32, u8, Vec<u8>)],
    kind: Kind,
    prefix: &[u8],
) -> Vec<&'a (i32, u8, Vec<u8>)> {
    images
        .iter()
        .filter(|(_, t, d)| *t == kind.code() && d.starts_with(prefix))
        .collect()
}

#[test]
fn the_stock_update_file_follows_edits_into_the_wcc_files() {
    let vir = stock_dir();
    let tmp = tempfile::tempdir().unwrap();
    let realm = tmp.path().join("realm");
    convert_dir_matching(&vir, &realm, is_wcc_stem).unwrap();
    let stock = images_of_db(&realm.join("WCCUPDAT.db"));
    assert_eq!(stock.len(), 38_754);

    // the first item and room renamed, the second monster deleted
    let name = ITEM.field("name").unwrap();
    let mut items = WbtrvDb::open(&realm.join("WCCITEMS.db")).unwrap();
    let (id, mut item) = items.records().unwrap().remove(0);
    let item_key = item[..4].to_vec();
    item[name.offset..name.offset + name.size].fill(0);
    item[name.offset..name.offset + 13].copy_from_slice(b"daydream test");
    items.update(id, &item).unwrap();
    let room_name = ROOM.field("name").unwrap();
    let mut rooms = WbtrvDb::open(&realm.join("WCCMP001.db")).unwrap();
    let (rid, mut room) = rooms.records().unwrap().remove(0);
    let key = room[..8].to_vec();
    room[room_name.offset..room_name.offset + 8].copy_from_slice(b"Reverie\0");
    rooms.update(rid, &room).unwrap();
    let mut monsters = WbtrvDb::open(&realm.join("WCCKNMSR.db")).unwrap();
    let (mid, monster) = monsters.records().unwrap().remove(1);
    let monster_key = monster[..4].to_vec();
    monsters.delete(mid).unwrap();

    let r = update::sync(&realm).unwrap();
    assert_eq!(r.file.as_deref(), Some("WCCUPDAT"));
    assert_eq!((r.replaced, r.added, r.removed), (2, 0, 1), "{r:?}");
    assert_eq!(r.records, 38_753);
    let synced = images_of_db(&realm.join("WCCUPDAT.db"));
    let item_images = image(&synced, Kind::Item, &item_key);
    assert_eq!(item_images.len(), 1);
    assert_eq!(&item_images[0].2[..item.len()], &item[..]);
    // it kept its place in the order the game applies the file in
    let was = image(&stock, Kind::Item, &item_key)[0].0;
    assert_eq!(item_images[0].0, was);
    assert!(image(&synced, Kind::Monster, &monster_key).is_empty());
    assert_eq!(
        &image(&synced, Kind::Room, &key)[0].2[..room.len()],
        &room[..]
    );

    // written back: the WCCUPDAT.DAT the game reads holds the edits
    let out = tmp.path().join("out");
    let realm_files = Realm::open(&realm).unwrap();
    let options = WriteOptions {
        out: Some(out.clone()),
        ..WriteOptions::default()
    };
    write_dat(&realm_files, &vir, &options).unwrap();
    let written = images_of_dat(&out.join("WCCUPDAT.DAT"));
    assert_eq!(written.len(), 38_753);
    let item_images = image(&written, Kind::Item, &item_key);
    // the file keeps its 2015-byte records: 2010 bytes of each image
    assert_eq!(&item_images[0].2[..item.len()], &item[..]);
    assert!(image(&written, Kind::Monster, &monster_key).is_empty());

    // compiled in full: every record the 2015-byte records can hold
    let r = update::compile(&realm, Compile::Full).unwrap();
    let mut total = 0;
    let mut too_long = 0;
    for kind in update::COMPILE_ORDER {
        for f in realm_files.files_of(kind.table()) {
            for (_, d) in WbtrvDb::open_read_only(&f.path).unwrap().records().unwrap() {
                total += 1;
                too_long += usize::from(d.len() > 2010);
            }
        }
    }
    assert_eq!(r.too_long.len(), too_long);
    assert_eq!(r.records as usize, total - too_long);
    assert!(
        r.too_long.iter().all(|t| t.starts_with("text block")),
        "{:?}",
        r.too_long
    );
    println!(
        "full compile: {} records, {} too long ({})",
        r.records,
        too_long,
        r.too_long.first().map_or("", String::as_str)
    );
}
