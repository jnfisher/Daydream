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
            ("Para4 6", l(5)),
            ("Shop Number", l(4)),
            ("Type", l(1)),
            ("Mon Type", l(6)),
            ("Light", l(-50)),
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
    assert_eq!(
        (square.room_type, square.mon_type, square.light),
        (1, 6, -50)
    );
    let exits: Vec<_> = square
        .exits
        .iter()
        .map(|e| (e.dir, e.to, e.kind, e.para1, e.para4))
        .collect();
    assert_eq!(exits, [(0, 2, 0, 0, 0), (6, 9, 8, 3, 5)]);
    assert!(map_view(&realm, Dos, 7).unwrap().rooms.is_empty());
}

#[test]
fn the_map_editor_digs_new_rooms_and_joins_old_ones() {
    use daydream_nmr::map::{Dig, dig};
    use daydream_nmr::modify::apply;
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let rows = [
        vec![
            ("Map Number", l(1)),
            ("Room Number", l(1)),
            ("Name", t("Hall")),
            ("Light", l(5)),
            ("Gold", l(9)),
            ("Room Item 0", l(7)),
            ("Exit 1", l(2)),
        ],
        vec![
            ("Map Number", l(1)),
            ("Room Number", l(2)),
            ("Name", t("Yard")),
            ("Exit 0", l(1)),
        ],
        vec![
            ("Map Number", l(2)),
            ("Room Number", l(5)),
            ("Name", t("Far")),
            ("Exit 3", l(9)),
        ],
    ];
    let refs: Vec<&[(&str, Value)]> = rows.iter().map(|r| r.as_slice()).collect();
    put(&realm, "Rooms", &refs);
    let get = |map: i64, room: i64, col: &str| -> i64 {
        let view =
            daydream_nmr::export_table(&realm, daydream_nmr::schema::table("Rooms").unwrap(), Dos)
                .unwrap();
        let (m, r, c) = (
            view.position("Map Number").unwrap(),
            view.position("Room Number").unwrap(),
            view.position(col).unwrap(),
        );
        let row = view
            .rows
            .iter()
            .find(|row| {
                daydream_nmr::schema::number(&row[m]).unwrap() as i64 == map
                    && daydream_nmr::schema::number(&row[r]).unwrap() as i64 == room
            })
            .unwrap();
        match &row[c] {
            Value::Text(s) => s.len() as i64,
            v => daydream_nmr::schema::number(v).unwrap() as i64,
        }
    };
    let d = |dir, to_map, to_room, existing, replace_back| Dig {
        map: 1,
        room: 1,
        dir,
        to_map,
        to_room,
        existing,
        replace_back,
        count: 1,
    };

    // east from the hall: a new room 3, a copy of the hall without its gold and items
    let plan = dig(&realm, Dos, &d(2, 1, 3, false, false)).unwrap();
    apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(1, 1, "Exit 2"), 3);
    assert_eq!(get(1, 3, "Exit 3"), 1);
    assert_eq!(get(1, 3, "Light"), 5);
    assert_eq!(get(1, 3, "Gold"), 0);
    assert_eq!(get(1, 3, "Room Item 0"), 0);
    assert_eq!(get(1, 3, "Exit 1"), 0);
    // south already leads to room 2; a new room onto an existing number is refused
    assert!(dig(&realm, Dos, &d(1, 1, 9, false, false)).is_err());
    assert!(dig(&realm, Dos, &d(4, 1, 2, false, false)).is_err());
    // NE to the existing room 2/5, on another map: map change exits both ways
    let plan = dig(&realm, Dos, &d(4, 2, 5, true, false)).unwrap();
    apply(&realm, &plan, Dos).unwrap();
    assert_eq!(
        (
            get(1, 1, "Exit 4"),
            get(1, 1, "Type 4"),
            get(1, 1, "Para1 4")
        ),
        (5, 8, 2)
    );
    assert_eq!(
        (
            get(2, 5, "Exit 7"),
            get(2, 5, "Type 7"),
            get(2, 5, "Para1 7")
        ),
        (1, 8, 1)
    );
    // the existing room's way back in use: left, with a note, unless replaced
    let plan = dig(&realm, Dos, &d(2, 2, 5, true, false));
    assert!(plan.is_err(), "east is taken by room 3");
    let plan = dig(&realm, Dos, &d(5, 2, 5, true, false)).unwrap();
    assert_eq!(plan.notes.len(), 0);
    let mut moved = Dig {
        map: 1,
        room: 2,
        dir: 2,
        to_map: 2,
        to_room: 5,
        existing: true,
        replace_back: false,
        count: 1,
    };
    let plan = dig(&realm, Dos, &moved).unwrap();
    assert_eq!(plan.notes.len(), 1, "{plan:?}");
    apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(2, 5, "Exit 3"), 9);
    moved.dir = 6;
    moved.replace_back = true;
    let plan = dig(&realm, Dos, &moved).unwrap();
    apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(2, 5, "Exit 5"), 2);

    // a line of three new rooms southeast from room 3: 4, 5 and 6 (map 2's room 5 is no
    // matter), each joined to the one before both ways
    let line = Dig {
        map: 1,
        room: 3,
        dir: 6,
        to_map: 1,
        to_room: 4,
        existing: false,
        replace_back: false,
        count: 3,
    };
    assert_eq!(line.label(), "Dig 3 rooms southeast of 1/3");
    let plan = dig(&realm, Dos, &line).unwrap();
    apply(&realm, &plan, Dos).unwrap();
    // southeast is exit 6, and its way back (northwest) exit 5
    assert_eq!(
        (
            get(1, 3, "Exit 6"),
            get(1, 4, "Exit 5"),
            get(1, 4, "Exit 6")
        ),
        (4, 3, 5)
    );
    assert_eq!(
        (
            get(1, 5, "Exit 5"),
            get(1, 5, "Exit 6"),
            get(1, 6, "Exit 5")
        ),
        (4, 6, 5)
    );
    assert_eq!(get(1, 6, "Exit 6"), 0, "the line ends at its last room");
    assert_eq!(get(1, 6, "Light"), 5, "a copy of the room it is dug from");
    // the line's numbers after the first skip those the map uses
    put(
        &realm,
        "Rooms",
        &[&[
            ("Map Number", l(1)),
            ("Room Number", l(8)),
            ("Name", t("Shed")),
        ]],
    );
    let east = Dig {
        room: 6,
        dir: 2,
        to_room: 7,
        ..line.clone()
    };
    assert_eq!(east.label(), "Dig 3 rooms east of 1/6");
    apply(&realm, &dig(&realm, Dos, &east).unwrap(), Dos).unwrap();
    assert_eq!(
        (
            get(1, 6, "Exit 2"),
            get(1, 7, "Exit 2"),
            get(1, 9, "Exit 3")
        ),
        (7, 9, 7)
    );
    assert_eq!((get(1, 9, "Exit 2"), get(1, 10, "Exit 3")), (10, 9));
    // too long a line, or a line of existing rooms, is refused
    let long = Dig {
        room: 10,
        to_room: 11,
        count: 101,
        ..line.clone()
    };
    assert!(dig(&realm, Dos, &long).is_err());
    assert!(
        dig(
            &realm,
            Dos,
            &Dig {
                existing: true,
                ..line
            }
        )
        .is_err()
    );
    let one = Dig {
        room: 10,
        dir: 0,
        to_room: 11,
        count: 1,
        ..east
    };
    assert_eq!(one.label(), "Dig room 1/11 north of 1/10");
}
