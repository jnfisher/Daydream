//! NMR's Record Number Changer, on records set up for each kind.

use daydream_nmr::renumber::{self, Kind, Move, Renumber};
use daydream_nmr::{NmrRows, Value, export_table, schema};
use daydream_store::Realm;
use daydream_types::records::LayoutSet::Dos;

use crate::support::{empty_realm, l, put, t};

type Row<'a> = Vec<(&'a str, Value)>;

fn add(realm: &Realm, table: &str, rows: &[Row]) {
    let refs: Vec<&[(&str, Value)]> = rows.iter().map(|r| r.as_slice()).collect();
    put(realm, table, &refs);
}

fn rows(realm: &Realm, table: &str) -> NmrRows {
    export_table(realm, schema::table(table).unwrap(), Dos).unwrap()
}

fn n(v: &Value) -> i64 {
    schema::number(v).unwrap() as i64
}

/// `column` of the row whose `key` columns are `at`.
fn get(realm: &Realm, table: &str, key: &[(&str, i64)], column: &str) -> Value {
    let r = rows(realm, table);
    let row = r
        .rows
        .iter()
        .find(|row| {
            key.iter()
                .all(|(c, v)| n(&row[r.position(c).unwrap()]) == *v)
        })
        .unwrap_or_else(|| panic!("{table} {key:?} not found"));
    row[r.position(column).unwrap()].clone()
}

fn has(realm: &Realm, table: &str, key: &[(&str, i64)]) -> bool {
    let r = rows(realm, table);
    r.rows.iter().any(|row| {
        key.iter()
            .all(|(c, v)| n(&row[r.position(c).unwrap()]) == *v)
    })
}

fn text(v: Value) -> String {
    match v {
        Value::Text(s) => s.split('\0').next().unwrap().to_string(),
        v => panic!("{v:?}"),
    }
}

fn run(realm: &Realm, r: &Renumber) {
    let plans = renumber::plan(realm, r, Dos).unwrap();
    for report in renumber::apply(realm, &plans, Dos).unwrap() {
        assert!(report.errors.is_empty(), "{report:?}");
    }
    let again = renumber::plan(realm, r, Dos);
    // once moved, the old numbers are gone and the new ones taken: a second run is refused
    assert!(again.is_err() || again.unwrap().iter().all(|p| p.changes.is_empty()));
}

fn realm() -> (tempfile::TempDir, Realm) {
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    (dir, realm)
}

fn moves(pairs: &[(i64, i64)]) -> Vec<Move> {
    pairs.iter().map(|&(from, to)| Move { from, to }).collect()
}

#[test]
fn items_move_with_everything_that_names_them() {
    let (_d, realm) = realm();
    add(
        &realm,
        "Items",
        &[
            vec![("Number", l(5)), ("Name", t("rope"))],
            vec![("Number", l(6)), ("Name", t("torch"))],
        ],
    );
    add(
        &realm,
        "Shops",
        &[vec![("Number", l(1)), ("Item 0", l(5)), ("Item 1", l(6))]],
    );
    add(
        &realm,
        "Rooms",
        &[vec![
            ("Map Number", l(1)),
            ("Room Number", l(1)),
            ("Placed Item 0", l(5)),
            ("Hidden Item 3", l(5)),
            ("Exit 0", l(2)),
            ("Type 0", l(3)),
            ("Para1 0", l(5)),
            ("Exit 1", l(3)),
            ("Type 1", l(10)),
            ("Para1 1", l(5)),
        ]],
    );
    add(
        &realm,
        "Monsters",
        &[vec![
            ("Number", l(1)),
            ("Item Number 0", l(5)),
            ("Weapon Number", l(5)),
            ("Ability 0", l(185)),
            ("Ability Value 0", l(5)),
            ("Ability 1", l(1)),
            ("Ability Value 1", l(5)),
        ]],
    );
    add(
        &realm,
        "Spells",
        &[vec![
            ("Number", l(1)),
            ("Ability 0", l(143)),
            ("Ability Value 0", l(5)),
        ]],
    );
    add(
        &realm,
        "Textblocks",
        &[vec![
            ("Number", l(1)),
            ("Part #", l(0)),
            (
                "Data",
                t("give rope:checkitem 5 2552:takeitem 55 2552:text 5"),
            ),
        ]],
    );
    run(
        &realm,
        &Renumber::Records {
            table: Kind::Items,
            map: None,
            moves: moves(&[(5, 50)]),
            users: false,
        },
    );
    assert!(!has(&realm, "Items", &[("Number", 5)]));
    assert_eq!(
        text(get(&realm, "Items", &[("Number", 50)], "Name")),
        "rope"
    );
    assert_eq!(n(&get(&realm, "Shops", &[("Number", 1)], "Item 0")), 50);
    assert_eq!(n(&get(&realm, "Shops", &[("Number", 1)], "Item 1")), 6);
    let room = [("Map Number", 1), ("Room Number", 1)];
    assert_eq!(n(&get(&realm, "Rooms", &room, "Placed Item 0")), 50);
    assert_eq!(n(&get(&realm, "Rooms", &room, "Hidden Item 3")), 50);
    assert_eq!(n(&get(&realm, "Rooms", &room, "Para1 0")), 50);
    // a text exit's Para1 is a message, not an item
    assert_eq!(n(&get(&realm, "Rooms", &room, "Para1 1")), 5);
    let mon = [("Number", 1)];
    assert_eq!(n(&get(&realm, "Monsters", &mon, "Item Number 0")), 50);
    assert_eq!(n(&get(&realm, "Monsters", &mon, "Weapon Number")), 50);
    assert_eq!(n(&get(&realm, "Monsters", &mon, "Ability Value 0")), 50);
    assert_eq!(n(&get(&realm, "Monsters", &mon, "Ability Value 1")), 5);
    assert_eq!(
        n(&get(&realm, "Spells", &[("Number", 1)], "Ability Value 0")),
        50
    );
    // "item " is found inside checkitem and takeitem, as NMR's search finds it; 55 does not move
    assert_eq!(
        text(get(&realm, "Textblocks", &[("Number", 1)], "Data")),
        "give rope:checkitem 50 2552:takeitem 55 2552:text 5"
    );
}

#[test]
fn text_blocks_move_every_part_and_their_links() {
    let (_d, realm) = realm();
    add(
        &realm,
        "Textblocks",
        &[
            vec![
                ("Number", l(10)),
                ("Part #", l(0)),
                ("Link To", l(11)),
                ("Data", t("a")),
            ],
            vec![("Number", l(10)), ("Part #", l(1)), ("Data", t("b"))],
            vec![("Number", l(11)), ("Part #", l(0)), ("Data", t("c"))],
            vec![
                ("Number", l(12)),
                ("Part #", l(0)),
                ("Link To", l(10)),
                (
                    "Data",
                    t("hello:10\nask:text 10:testskill picklocks 11:checkspell 4 10"),
                ),
            ],
        ],
    );
    add(
        &realm,
        "Rooms",
        &[vec![
            ("Map Number", l(1)),
            ("Room Number", l(1)),
            ("Command Text", l(10)),
        ]],
    );
    add(
        &realm,
        "Monsters",
        &[vec![("Number", l(1)), ("Greet Txt", l(11))]],
    );
    run(
        &realm,
        &Renumber::Records {
            table: Kind::Textblocks,
            map: None,
            moves: moves(&[(10, 20), (11, 21)]),
            users: false,
        },
    );
    assert!(!has(&realm, "Textblocks", &[("Number", 10)]));
    assert_eq!(
        text(get(
            &realm,
            "Textblocks",
            &[("Number", 20), ("Part #", 1)],
            "Data"
        )),
        "b"
    );
    assert_eq!(
        n(&get(
            &realm,
            "Textblocks",
            &[("Number", 20), ("Part #", 0)],
            "Link To"
        )),
        21
    );
    let tb12 = [("Number", 12), ("Part #", 0)];
    assert_eq!(n(&get(&realm, "Textblocks", &tb12, "Link To")), 20);
    assert_eq!(
        text(get(&realm, "Textblocks", &tb12, "Data")),
        "hello:20\nask:text 20:testskill picklocks 21:checkspell 4 20"
    );
    assert_eq!(
        n(&get(
            &realm,
            "Rooms",
            &[("Map Number", 1), ("Room Number", 1)],
            "Command Text"
        )),
        20
    );
    assert_eq!(
        n(&get(&realm, "Monsters", &[("Number", 1)], "Greet Txt")),
        21
    );
}

#[test]
fn rooms_move_within_their_map() {
    let (_d, realm) = realm();
    let room = |map, n, exits: &[(i32, i32, i32, i32)]| {
        let mut r: Row = vec![
            ("Map Number", l(map)),
            ("Room Number", l(n)),
            ("Name", t("r")),
        ];
        for &(d, to, ty, p1) in exits {
            let d = d as usize;
            r.push((["Exit 0", "Exit 1", "Exit 2"][d], l(to)));
            r.push((["Type 0", "Type 1", "Type 2"][d], l(ty)));
            r.push((["Para1 0", "Para1 1", "Para1 2"][d], l(p1)));
        }
        r
    };
    let mut first = room(1, 1, &[(0, 2, 0, 0)]);
    first.push(("Death Room", l(2)));
    add(
        &realm,
        "Rooms",
        &[
            first,
            room(1, 2, &[(1, 1, 0, 0)]),
            room(1, 3, &[(0, 1, 0, 0), (1, 2, 8, 1)]),
            room(2, 2, &[(0, 2, 0, 0), (1, 2, 8, 1)]),
        ],
    );
    add(
        &realm,
        "Spells",
        &[
            vec![
                ("Number", l(1)),
                ("Ability 0", l(141)),
                ("Ability Value 0", l(1)),
                ("Ability 1", l(140)),
                ("Ability Value 1", l(2)),
            ],
            vec![
                ("Number", l(2)),
                ("Ability 0", l(140)),
                ("Ability Value 0", l(2)),
            ],
        ],
    );
    add(
        &realm,
        "Textblocks",
        &[vec![
            ("Number", l(1)),
            ("Part #", l(0)),
            ("Data", t("go:teleport 2 1:teleport 2 3:teleport 22 1")),
        ]],
    );
    run(
        &realm,
        &Renumber::Records {
            table: Kind::Rooms,
            map: Some(1),
            moves: moves(&[(1, 101), (2, 102)]),
            users: false,
        },
    );
    let r = |map: i64, n: i64, c: &str| {
        get(
            &realm,
            "Rooms",
            &[("Map Number", map), ("Room Number", n)],
            c,
        )
    };
    assert!(!has(
        &realm,
        "Rooms",
        &[("Map Number", 1), ("Room Number", 1)]
    ));
    assert_eq!(n(&r(1, 101, "Exit 0")), 102);
    assert_eq!(n(&r(1, 101, "Death Room")), 102);
    assert_eq!(n(&r(1, 102, "Exit 1")), 101);
    assert_eq!(n(&r(1, 3, "Exit 0")), 101);
    // a map change onto its own map moves too, and stays a map change
    assert_eq!(n(&r(1, 3, "Exit 1")), 102);
    assert_eq!(n(&r(1, 3, "Type 1")), 8);
    // map 2: its own room 2 stays; its map change into map 1 follows
    assert!(has(
        &realm,
        "Rooms",
        &[("Map Number", 2), ("Room Number", 2)]
    ));
    assert_eq!(n(&r(2, 2, "Exit 0")), 2);
    assert_eq!(n(&r(2, 2, "Exit 1")), 102);
    // a teleport spell moves only when it teleports to this map
    assert_eq!(
        n(&get(&realm, "Spells", &[("Number", 1)], "Ability Value 1")),
        102
    );
    assert_eq!(
        n(&get(&realm, "Spells", &[("Number", 2)], "Ability Value 0")),
        2
    );
    assert_eq!(
        text(get(&realm, "Textblocks", &[("Number", 1)], "Data")),
        "go:teleport 102 1:teleport 2 3:teleport 22 1"
    );
}

#[test]
fn bad_runs_are_refused() {
    let (_d, realm) = realm();
    add(
        &realm,
        "Items",
        &[
            vec![("Number", l(5))],
            vec![("Number", l(6))],
            vec![("Number", l(7))],
        ],
    );
    let items = |pairs: &[(i64, i64)]| Renumber::Records {
        table: Kind::Items,
        map: None,
        moves: moves(pairs),
        users: false,
    };
    for bad in [
        items(&[]),
        items(&[(5, 6)]),          // taken
        items(&[(5, 9), (5, 10)]), // moved twice
        items(&[(5, 9), (6, 9)]),  // same new number
        items(&[(5, 6), (6, 9)]),  // moved onto one that moves
        items(&[(0, 9)]),
        Renumber::Records {
            table: Kind::Rooms,
            map: None,
            moves: moves(&[(1, 2)]),
            users: false,
        },
    ] {
        assert!(renumber::plan(&realm, &bad, Dos).is_err(), "{bad:?}");
    }
    // a moved record counts once
    let plans = renumber::plan(&realm, &items(&[(5, 9)]), Dos).unwrap();
    assert_eq!(plans.last().unwrap().records(), 1);
    // a number not in the realm still moves its references, with a note
    let plans = renumber::plan(&realm, &items(&[(8, 9)]), Dos).unwrap();
    assert!(
        plans.last().unwrap().notes[0].contains("not in the realm"),
        "{plans:?}"
    );
}

#[test]
fn a_map_moves_to_another() {
    let (_d, realm) = realm();
    let room = |map, n, to, ty, p1| {
        vec![
            ("Map Number", l(map)),
            ("Room Number", l(n)),
            ("Exit 0", l(to)),
            ("Type 0", l(ty)),
            ("Para1 0", l(p1)),
        ]
    };
    add(
        &realm,
        "Rooms",
        &[
            room(1, 1, 7, 8, 3),
            room(3, 7, 8, 0, 0),
            room(3, 8, 7, 0, 0),
            room(4, 50, 7, 8, 3),
        ],
    );
    add(
        &realm,
        "Spells",
        &[vec![
            ("Number", l(1)),
            ("Ability 0", l(141)),
            ("Ability Value 0", l(3)),
        ]],
    );
    add(
        &realm,
        "Textblocks",
        &[vec![
            ("Number", l(1)),
            ("Part #", l(0)),
            ("Data", t("go:teleport 7 3:teleport 7 33")),
        ]],
    );
    let run_map = |to| Renumber::Map {
        from: 3,
        to,
        exits: true,
        normal: true,
        textblocks: true,
        spells: true,
        users: false,
    };
    // map 1 has no room 7 or 8, but its room 1 would clash with nothing: fine; map 4 has 50
    let plans = renumber::plan(&realm, &run_map(4), Dos).unwrap();
    renumber::apply(&realm, &plans, Dos).unwrap();
    let r = |map: i64, n: i64, c: &str| {
        get(
            &realm,
            "Rooms",
            &[("Map Number", map), ("Room Number", n)],
            c,
        )
    };
    assert!(!has(
        &realm,
        "Rooms",
        &[("Map Number", 3), ("Room Number", 7)]
    ));
    assert_eq!(n(&r(4, 7, "Exit 0")), 8);
    assert_eq!(n(&r(1, 1, "Para1 0")), 4);
    assert_eq!(n(&r(1, 1, "Type 0")), 8);
    // map 4's map change into map 3 now leads to its own map: a normal exit
    assert_eq!(n(&r(4, 50, "Type 0")), 0);
    assert_eq!(
        n(&get(&realm, "Spells", &[("Number", 1)], "Ability Value 0")),
        4
    );
    assert_eq!(
        text(get(&realm, "Textblocks", &[("Number", 1)], "Data")),
        "go:teleport 7 4:teleport 7 33"
    );
    // map 1 has a room 1, so map 4 (now holding 7 and 8) cannot move onto map 1 if it clashes
    add(&realm, "Rooms", &[room(1, 7, 0, 0, 0)]);
    assert!(
        renumber::plan(
            &realm,
            &Renumber::Map {
                from: 4,
                to: 1,
                exits: true,
                normal: true,
                textblocks: true,
                spells: true,
                users: false
            },
            Dos
        )
        .is_err()
    );
}

/// Users follow only when asked (NMR leaves them): items in every slot they hold one, where
/// they stand and their trail.
#[test]
fn users_follow_when_asked() {
    let (_d, realm) = realm();
    add(
        &realm,
        "Items",
        &[
            vec![("Number", l(5)), ("Name", t("sword"))],
            vec![("Number", l(6)), ("Name", t("key"))],
        ],
    );
    add(
        &realm,
        "Rooms",
        &[
            vec![
                ("Map Number", l(1)),
                ("Room Number", l(10)),
                ("Name", t("Square")),
            ],
            vec![
                ("Map Number", l(2)),
                ("Room Number", l(10)),
                ("Name", t("Cave")),
            ],
        ],
    );
    add(
        &realm,
        "Users",
        &[vec![
            ("BBS Name", t("Ann")),
            ("Item 0", l(5)),
            ("Item 1", l(6)),
            ("Key 0", l(5)),
            ("Worn 0", l(5)),
            ("Weapon", l(5)),
            ("Map Number", l(1)),
            ("Room Number", l(10)),
            ("Trail 0 Map", l(1)),
            ("Trail 0 Room", l(10)),
            ("Trail 1 Map", l(2)),
            ("Trail 1 Room", l(10)),
        ]],
    );
    let ann = |c: &str| n(&get(&realm, "Users", &[], c));
    let items = |users| Renumber::Records {
        table: Kind::Items,
        map: None,
        moves: moves(&[(5, 50)]),
        users,
    };
    // as NMR: users are left alone
    let plans = renumber::plan(&realm, &items(false), Dos).unwrap();
    assert!(plans.iter().all(|p| p.table != "Users"));
    run(&realm, &items(true));
    for c in ["Item 0", "Key 0", "Worn 0", "Weapon"] {
        assert_eq!(ann(c), 50, "{c}");
    }
    assert_eq!(ann("Item 1"), 6);

    run(
        &realm,
        &Renumber::Records {
            table: Kind::Rooms,
            map: Some(1),
            moves: moves(&[(10, 20)]),
            users: true,
        },
    );
    assert_eq!(
        (ann("Room Number"), ann("Trail 0 Room"), ann("Trail 1 Room")),
        (20, 20, 10)
    );

    let plans = renumber::plan(
        &realm,
        &Renumber::Map {
            from: 2,
            to: 3,
            exits: true,
            normal: true,
            textblocks: true,
            spells: true,
            users: true,
        },
        Dos,
    )
    .unwrap();
    renumber::apply(&realm, &plans, Dos).unwrap();
    assert_eq!(
        (ann("Map Number"), ann("Trail 0 Map"), ann("Trail 1 Map")),
        (1, 1, 3)
    );
}
