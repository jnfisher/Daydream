//! NMR's lists, on a realm set up for them.

use daydream_nmr::Value;
use daydream_nmr::reports::{self, PlaceKind, Scan};
use daydream_nmr::schema::layout_of;
use daydream_store::{Metadata, Realm, WbtrvDb};
use daydream_types::records::LayoutSet::{self, Dos};
use daydream_wcc::tables::Table;

use crate::support::{empty_realm, l, put, t};

/// A user file holding one user carrying `items`.
fn user(dir: &std::path::Path, id: &str, first: &str, items: &[i32]) {
    let layout = layout_of(Table::Users, LayoutSet::Dos);
    let meta = Metadata {
        record_length: layout.len,
        physical_record_length: layout.len,
        page_length: 4096,
        variable_length_records: false,
        version: 4,
    };
    let mut db = WbtrvDb::create(&dir.join("WCCUSERS.db"), meta, Vec::new()).unwrap();
    let mut r = vec![0u8; layout.len];
    let f = |n| layout.field(n).unwrap();
    r[f("userid").offset..][..id.len()].copy_from_slice(id.as_bytes());
    r[f("firstname").offset..][..first.len()].copy_from_slice(first.as_bytes());
    for (i, item) in items.iter().enumerate() {
        r[f("items").offset + 4 * i..][..4].copy_from_slice(&item.to_le_bytes());
    }
    db.insert(&r).unwrap();
}

fn realm(dir: &std::path::Path) -> Realm {
    empty_realm(dir);
    let realm = Realm::open(dir).unwrap();
    put(
        &realm,
        "Items",
        &[
            &[("Number", l(1)), ("Name", t("relic")), ("Game Limit", l(2))],
            &[("Number", l(2)), ("Name", t("stick"))],
            &[("Number", l(3)), ("Game Limit", l(1))],
        ],
    );
    put(
        &realm,
        "Rooms",
        &[
            &[
                ("Map Number", l(1)),
                ("Room Number", l(1)),
                ("Name", t("Hall")),
                ("Room Item 0", l(1)),
                ("Room Item 0 QTY", l(0)),
                ("Room Item 3", l(1)),
                ("Room Item 3 QTY", l(1)),
                ("Perm NPC", l(9)),
            ],
            &[
                ("Map Number", l(1)),
                ("Room Number", l(2)),
                ("Name", t("Den")),
                ("Hidden Item 2", l(1)),
                ("Hidden Item 2 QTY", l(0)),
                ("Control Room", l(1)),
            ],
            &[
                ("Map Number", l(1)),
                ("Room Number", l(3)),
                ("Name", t("Attic")),
                ("Control Room", l(1)),
            ],
            &[
                ("Map Number", l(2)),
                ("Room Number", l(1)),
                ("Name", t("Field")),
                ("Control Room", l(7)),
            ],
        ],
    );
    put(
        &realm,
        "Shops",
        &[
            &[
                ("Number", l(4)),
                ("Name", t("Smith")),
                ("Item 0", l(1)),
                ("Normal 0", l(2)),
                ("Item 1", l(2)),
                ("Normal 1", l(5)),
            ],
            &[
                ("Number", l(109)),
                ("Name", t("Not in game")),
                ("Item 0", l(1)),
                ("Normal 0", l(1)),
            ],
        ],
    );
    realm
}

#[test]
fn limited_items_are_counted_where_nmr_counts_them() {
    let dir = tempfile::tempdir().unwrap();
    realm(dir.path());
    user(dir.path(), "sysop", "Sys", &[1, 2, 1]);
    // the realm lists its files when it opens
    let realm = Realm::open(dir.path()).unwrap();
    let list = reports::limited_items(&realm, Dos, Scan::default()).unwrap();
    // item 3 has no name, so NMR leaves it out
    assert_eq!(list.iter().map(|i| i.number).collect::<Vec<_>>(), [1]);
    let relic = &list[0];
    let at = |kind| {
        relic
            .places
            .iter()
            .filter(|p| p.kind == kind)
            .map(|p| (p.key.as_str(), p.count))
            .collect::<Vec<_>>()
    };
    // a room slot holds its quantity less one; shop 109 is left out
    assert_eq!(at(PlaceKind::Room), [("1/1", 3)]);
    assert_eq!(at(PlaceKind::Hidden), [("1/2", 1)]);
    assert_eq!(at(PlaceKind::Shop), [("4", 2)]);
    assert_eq!(at(PlaceKind::User), [("sysop", 2)]);
    assert_eq!(
        relic
            .places
            .iter()
            .find(|p| p.kind == PlaceKind::User)
            .unwrap()
            .name,
        "Sys/sysop"
    );
    assert_eq!((relic.limit, relic.in_use, relic.over), (2, 8, true));

    let rooms_only = Scan {
        rooms: true,
        shops: false,
        users: false,
    };
    let list = reports::limited_items(&realm, Dos, rooms_only).unwrap();
    assert_eq!(list[0].in_use, 4);
}

#[test]
fn an_item_is_found_in_every_shop() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    let places = reports::find_item(&realm, Dos, 1, Scan::default()).unwrap();
    let shops: Vec<_> = places
        .iter()
        .filter(|p| p.kind == PlaceKind::Shop)
        .map(|p| p.key.as_str())
        .collect();
    assert_eq!(shops, ["4", "109"]);
    let stick = reports::find_item(&realm, Dos, 2, Scan::default()).unwrap();
    assert_eq!(stick.len(), 1);
    assert_eq!(stick[0].count, 5);
}

#[test]
fn control_rooms_and_npcs_are_listed() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    let control = reports::control_rooms(&realm, Dos).unwrap();
    let got: Vec<_> = control
        .iter()
        .map(|c| (c.map, c.room, c.name.as_str(), c.rooms.clone()))
        .collect();
    assert_eq!(got, [(1, 1, "Hall", vec![2, 3]), (2, 7, "", vec![1])]);
    let npcs = reports::npc_rooms(&realm, Dos).unwrap();
    assert_eq!(npcs.len(), 1);
    assert_eq!((npcs[0].map, npcs[0].room, npcs[0].monster), (1, 1, 9));
}

#[test]
fn monster_groups_keep_nmrs_fifteen() {
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let mut rows: Vec<Vec<(&str, Value)>> = (1..=17)
        .map(|n| {
            vec![
                ("Number", l(n)),
                ("Name", t(&format!("rat {n}"))),
                ("Group", l(3)),
                ("Index", l(5)),
            ]
        })
        .collect();
    rows.push(vec![
        ("Number", l(18)),
        ("Name", t("sdf")),
        ("Group", l(1)),
        ("Index", l(1)),
    ]);
    rows.push(vec![
        ("Number", l(19)),
        ("Name", t("lost")),
        ("Group", l(1)),
        ("Index", l(-1)),
    ]);
    rows.push(vec![
        ("Number", l(20)),
        ("Name", t("bat")),
        ("Group", l(1)),
        ("Index", l(2)),
    ]);
    let refs: Vec<&[(&str, Value)]> = rows.iter().map(|r| r.as_slice()).collect();
    put(&realm, "Monsters", &refs);
    let groups = reports::monster_groups(&realm, Dos).unwrap();
    let got: Vec<_> = groups
        .iter()
        .map(|g| (g.group, g.index, g.monsters.len()))
        .collect();
    assert_eq!(got, [(1, 2, 1), (3, 5, 15)]);
}

#[test]
fn a_map_view_has_its_rooms_exits_and_the_maps() {
    use daydream_nmr::map::map_view;
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let rows = [
        vec![
            ("Map Number", l(1)),
            ("Room Number", l(1)),
            ("Name", t("Square")),
            ("Exit 0", l(2)),
            ("Exit 6", l(9)),
            ("Type 6", l(8)),
            ("Para1 6", l(3)),
            ("Shop Number", l(4)),
        ],
        vec![
            ("Map Number", l(1)),
            ("Room Number", l(2)),
            ("Name", t("Road")),
            ("Exit 1", l(1)),
        ],
        vec![
            ("Map Number", l(3)),
            ("Room Number", l(9)),
            ("Name", t("Far")),
        ],
    ];
    let refs: Vec<&[(&str, Value)]> = rows.iter().map(|r| r.as_slice()).collect();
    put(&realm, "Rooms", &refs);
    let view = map_view(&realm, Dos, 1).unwrap();
    assert_eq!(view.maps, [(1, 2), (3, 1)]);
    let names: Vec<_> = view
        .rooms
        .iter()
        .map(|r| (r.room, r.name.as_str()))
        .collect();
    assert_eq!(names, [(1, "Square"), (2, "Road")]);
    let square = &view.rooms[0];
    assert_eq!(square.shop, 4);
    let exits: Vec<_> = square
        .exits
        .iter()
        .map(|e| (e.dir, e.to, e.kind, e.para1))
        .collect();
    assert_eq!(exits, [(0, 2, 0, 0), (6, 9, 8, 3)]);
    assert!(map_view(&realm, Dos, 7).unwrap().rooms.is_empty());
}
