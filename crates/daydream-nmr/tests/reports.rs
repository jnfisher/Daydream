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
        vec![
            ("Map Number", l(1)),
            ("Room Number", l(3)),
            ("Name", t("Road")),
        ],
    ];
    let refs: Vec<&[(&str, Value)]> = rows.iter().map(|r| r.as_slice()).collect();
    put(&realm, "Rooms", &refs);
    let view = map_view(&realm, Dos, 1).unwrap();
    assert_eq!(view.maps, [(1, 3), (3, 1)]);
    let names: Vec<_> = view
        .rooms
        .iter()
        .map(|r| (r.room, r.name.as_str()))
        .collect();
    assert_eq!(names, [(1, "Square"), (2, "Road"), (3, "Road")]);
    // what each map mostly is: two of map 1's rooms are Road
    let common: Vec<_> = view
        .names
        .iter()
        .map(|m| (m.map, m.name.as_str(), m.rooms))
        .collect();
    assert_eq!(common, [(1, "Road", 2), (3, "Far", 1)]);
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
    // the map change from 1/1 to 3/9 joins map 1 to map 3
    let links: Vec<_> = view
        .links
        .iter()
        .map(|l| (l.from, l.to, l.exits, l.room, l.to_room))
        .collect();
    assert_eq!(links, [(1, 3, 1, 1, 9)]);
    assert!(map_view(&realm, Dos, 7).unwrap().rooms.is_empty());
}

#[test]
fn a_map_view_has_the_links_scripts_and_spells_make() {
    use daydream_nmr::map::map_view;
    use daydream_nmr::scripted::When;
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let rows = [
        vec![
            ("Map Number", l(1)),
            ("Room Number", l(1)),
            ("Name", t("Square")),
            ("Command Text", l(10)),
            ("Spell", l(5)),
            // a Remote Action east: message 7's words work the south exit of room 2
            ("Exit 2", l(2)),
            ("Type 2", l(12)),
            ("Para1 2", l(7)),
            ("Para2 2", l(31)),
        ],
        vec![
            ("Map Number", l(1)),
            ("Room Number", l(2)),
            ("Name", t("Road")),
            // a Cast exit north, whose spell teleports to a room rolled from 20 to 25
            ("Exit 0", l(1)),
            ("Type 0", l(22)),
            ("Para1 0", l(6)),
        ],
        vec![
            ("Map Number", l(2)),
            ("Room Number", l(5)),
            ("Command Text", l(13)),
        ],
        vec![
            ("Map Number", l(3)),
            ("Room Number", l(9)),
            ("Command Text", l(12)),
        ],
    ];
    let refs: Vec<&[(&str, Value)]> = rows.iter().map(|r| r.as_slice()).collect();
    put(&realm, "Rooms", &refs);
    put(
        &realm,
        "Textblocks",
        &[
            &[
                ("Number", l(10)),
                ("Part #", l(0)),
                (
                    "Data",
                    t(
                        "pull lever:teleport 2 1:message 4\nsay hi|hello:random 11\nlook:text 11:teleport_slums:teleport 0 1\n",
                    ),
                ),
            ],
            &[
                ("Number", l(11)),
                ("Part #", l(0)),
                ("Data", t("50:teleport 9 3\n100:remoteaction 3 0 0 4\n")),
            ],
            &[
                ("Number", l(12)),
                ("Part #", l(0)),
                ("Data", t("go home:teleport 1 1\n")),
            ],
            &[
                ("Number", l(13)),
                ("Part #", l(0)),
                ("Data", t("go:teleport 6 2\n")),
            ],
        ],
    );
    put(
        &realm,
        "Spells",
        &[
            &[
                ("Number", l(5)),
                ("Name", t("pit")),
                ("Ability 0", l(140)),
                ("Ability Value 0", l(3)),
                ("Ability 1", l(141)),
                ("Ability Value 1", l(1)),
            ],
            &[
                ("Number", l(6)),
                ("Name", t("scatter")),
                ("Min", l(20)),
                ("Max", l(25)),
                ("Ability 0", l(140)),
                ("Ability Value 0", l(0)),
                ("Ability 1", l(141)),
                ("Ability Value 1", l(1)),
            ],
        ],
    );
    put(
        &realm,
        "Messages",
        &[&[
            ("Number", l(7)),
            ("Line 1", t("pull lever")),
            ("Line 2", t("yank lever")),
        ]],
    );
    let s = map_view(&realm, Dos, 1).unwrap().scripts;
    let said = |w: &When| match w {
        When::Typed { words } => format!("typed {words}"),
        When::Here => "here".into(),
        When::Walking { dir } => format!("walking {dir}"),
        When::Words { dir, words } => format!("words {dir} {}", words.join("/")),
    };
    let uses: Vec<_> = s
        .uses
        .iter()
        .map(|u| {
            let src = &s.sources[u.source];
            let to: Vec<_> = src
                .targets
                .iter()
                .map(|t| (t.kind, t.to_map, t.to_room, t.to_last, t.exit))
                .collect();
            (u.map, u.room, said(&u.when), src.table, to)
        })
        .collect();
    // map 2's own teleport is not map 1's; map 3's leads onto it
    assert_eq!(
        uses,
        [
            (
                1,
                1,
                "typed pull lever".to_string(),
                "Textblocks",
                vec![("teleport", 1, 2, None, None)]
            ),
            (
                1,
                1,
                "typed say hi|hello".to_string(),
                "Textblocks",
                vec![
                    ("teleport", 3, 9, None, None),
                    ("remote", 0, 3, None, Some(4))
                ]
            ),
            (
                1,
                1,
                "here".to_string(),
                "Spells",
                vec![("spell", 1, 3, None, None)]
            ),
            (
                1,
                1,
                "words 2 pull lever/yank lever".to_string(),
                "Rooms",
                vec![("remote", 0, 2, None, Some(1))]
            ),
            (
                1,
                2,
                "walking 0".to_string(),
                "Spells",
                vec![("spell", 1, 20, Some(25), None)]
            ),
            (
                3,
                9,
                "typed go home".to_string(),
                "Textblocks",
                vec![("teleport", 1, 1, None, None)]
            ),
        ]
    );
    // what each is: the clause, or the spell's name; a command line's place in its block
    let what: Vec<_> = s
        .sources
        .iter()
        .map(|x| (x.line, x.name.as_str(), x.targets[0].what.as_str()))
        .collect();
    assert_eq!(
        what,
        [
            (Some((0, 1)), "", "teleport 2 1"),
            (Some((0, 2)), "", "teleport 9 3"),
            (None, "pit", "pit"),
            (None, "", "Remote Action exit E"),
            (None, "scatter", "scatter"),
            (Some((0, 1)), "", "teleport 1 1"),
        ]
    );
    // those of room 2: its own, and those leading to it (a teleport, a remote action)
    let two = s.within(1, 2, 2);
    let rooms: Vec<_> = two
        .uses
        .iter()
        .map(|u| (u.room, two.sources[u.source].targets[0].what.as_str()))
        .collect();
    assert_eq!(
        rooms,
        [
            (1, "teleport 2 1"),
            (1, "Remote Action exit E"),
            (2, "scatter")
        ]
    );
    assert_eq!(two.sources.len(), 3);
    // map 2 has its own, and map 3's leads away
    assert_eq!(map_view(&realm, Dos, 2).unwrap().scripts.uses.len(), 1);
    assert_eq!(map_view(&realm, Dos, 3).unwrap().scripts.uses.len(), 2);
}

/// The stock world's links no exit draws: the small statue's words working the ways out of its
/// tree, and the Island of Bones' portal (its command lines and its room spell) scattering
/// players over the lake.
#[test]
fn the_stock_world_s_scripted_links() {
    use daydream_nmr::map::map_view;
    use daydream_nmr::scripted::When;
    use std::collections::HashSet;
    let realm = Realm::open(&crate::support::stock_realm()).unwrap();
    let nine = map_view(&realm, Dos, 9).unwrap().scripts;
    let statue: HashSet<_> = nine
        .uses
        .iter()
        .filter(|u| (u.map, u.room) == (9, 1412))
        .flat_map(|u| {
            let first = match &u.when {
                When::Words { words, .. } => words.first().cloned().unwrap_or_default(),
                _ => String::new(),
            };
            nine.sources[u.source]
                .targets
                .iter()
                .filter(|t| t.kind == "remote")
                .map(move |t| (first.clone(), t.to_room, t.exit))
        })
        .collect();
    assert_eq!(statue.len(), 9, "{statue:?}");
    assert!(statue.contains(&("turn statue".to_string(), 1343, Some(1))));
    assert!(statue.contains(&("push violet eye".to_string(), 666, Some(0))));
    let lake = map_view(&realm, Dos, 17).unwrap().scripts;
    let isle: Vec<_> = lake
        .uses
        .iter()
        .filter(|u| (u.map, u.room) == (17, 1753))
        .collect();
    assert!(isle.iter().any(|u| u.when == When::Here));
    assert!(isle.iter().any(|u| u.when
        == When::Typed {
            words: "go portal".into()
        }));
    let rooms: HashSet<i64> = isle
        .iter()
        .flat_map(|u| &lake.sources[u.source].targets)
        .filter(|t| t.kind == "teleport" && t.to_map == 17)
        .map(|t| t.to_room)
        .collect();
    assert_eq!(rooms.len(), 90);
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

#[test]
fn a_stamp_puts_down_rooms_and_joins_them_only_through_free_slots() {
    use daydream_nmr::map::{Link, Stamp, stamp};
    use daydream_nmr::modify::apply;
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    put(
        &realm,
        "Rooms",
        &[
            &[
                ("Map Number", l(1)),
                ("Room Number", l(1)),
                ("Name", t("Hall")),
                ("Light", l(5)),
                ("Gold", l(9)),
                ("Room Item 0", l(7)),
                ("Perm NPC", l(4)),
                ("Exit 2", l(2)),
            ],
            &[
                ("Map Number", l(1)),
                ("Room Number", l(2)),
                ("Name", t("Yard")),
                ("Exit 3", l(1)),
                ("Exit 0", l(9)),
                ("Type 0", l(7)),
            ],
            &[
                ("Map Number", l(1)),
                ("Room Number", l(3)),
                ("Name", t("Shed")),
            ],
        ],
    );
    let get = |room: i64, col: &str| -> i64 {
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
                daydream_nmr::schema::number(&row[m]).unwrap() as i64 == 1
                    && daydream_nmr::schema::number(&row[r]).unwrap() as i64 == room
            })
            .unwrap();
        daydream_nmr::schema::number(&row[c]).unwrap() as i64
    };
    let link = |from, dir, to| Link {
        from,
        dir,
        to,
        one_way: false,
    };
    let s = Stamp {
        map: 1,
        brush: 1,
        rooms: vec![10, 11],
        links: vec![
            // new to new, both ways
            link(10, 2, 11),
            // the yard's south is free and so is 10's north: joined both ways
            link(2, 1, 10),
            // the yard's north leads to 9: left whole, not made one way
            link(2, 0, 11),
            // already there both ways: nothing to do
            link(1, 2, 2),
            // a one-way drop from 11 into the shed
            Link {
                one_way: true,
                ..link(11, 2, 3)
            },
        ],
        overwrite: false,
        what: "a corridor".into(),
        dug: false,
    };
    assert_eq!(s.label(), "Stamp a corridor: 2 rooms on map 1");
    // dragged out of a room on the map, it is a dig
    let dug = Stamp {
        dug: true,
        what: "a corridor east of 1/1".into(),
        ..s.clone()
    };
    assert_eq!(dug.label(), "Dig a corridor east of 1/1: 2 rooms on map 1");
    let (plan, done) = stamp(&realm, Dos, &s).unwrap();
    assert_eq!(
        (done.rooms, done.exits, done.joined, done.left),
        (2, 5, 1, 1),
        "{done:?}"
    );
    assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
    assert!(
        plan.notes[0].starts_with("Room 1/2's exit north leads to 9"),
        "{:?}",
        plan.notes
    );
    apply(&realm, &plan, Dos).unwrap();
    assert_eq!((get(10, "Exit 2"), get(11, "Exit 3")), (11, 10));
    assert_eq!((get(2, "Exit 1"), get(10, "Exit 0")), (10, 2));
    assert_eq!(
        (get(2, "Exit 0"), get(2, "Type 0")),
        (9, 7),
        "left as it was"
    );
    assert_eq!((get(11, "Exit 2"), get(3, "Exit 3")), (3, 0), "one way");
    // a copy of the brush without its coins, items and permanent NPC
    assert_eq!(get(10, "Light"), 5);
    assert_eq!(
        (get(11, "Gold"), get(11, "Room Item 0"), get(11, "Perm NPC")),
        (0, 0, 0)
    );

    // told to, a stamp replaces the exit in its way (a normal exit now) and makes the link
    let over = Stamp {
        rooms: vec![],
        links: vec![link(2, 0, 11)],
        overwrite: true,
        ..s.clone()
    };
    assert_eq!(over.label(), "Stamp a corridor on map 1");
    let (plan, done) = stamp(&realm, Dos, &over).unwrap();
    assert_eq!((done.exits, done.joined, done.left), (2, 2, 0));
    apply(&realm, &plan, Dos).unwrap();
    assert_eq!(
        (get(2, "Exit 0"), get(2, "Type 0"), get(11, "Exit 1")),
        (11, 0, 2)
    );

    // refused: a new room on a number the map uses, a room neither on the map nor made, a
    // brush the map does not have, nothing at all, a direction past down
    for bad in [
        Stamp {
            rooms: vec![3],
            links: vec![],
            ..s.clone()
        },
        Stamp {
            rooms: vec![],
            links: vec![link(1, 0, 77)],
            ..s.clone()
        },
        Stamp {
            brush: 99,
            rooms: vec![20],
            links: vec![],
            ..s.clone()
        },
        Stamp {
            rooms: vec![],
            links: vec![],
            ..s.clone()
        },
        Stamp {
            rooms: vec![],
            links: vec![link(1, 10, 2)],
            ..s.clone()
        },
    ] {
        assert!(stamp(&realm, Dos, &bad).is_err(), "{bad:?}");
    }
}
