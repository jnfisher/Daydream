//! The health checks: on records set up for each case (each check finds its records, and its
//! fix leaves nothing for it to find), and on the stock 1.11p world, where what they find is
//! known.

use daydream_nmr::health::{self, Check, Found, Level};
use daydream_nmr::{ExportOptions, Value, export, layout_of_realm, modify};
use daydream_store::Realm;

use crate::support::{copy_realm, empty_realm, l, put, stock_realm, t};

fn found(realm: &Realm, c: Check) -> Found {
    health::report(realm, layout_of_realm(realm))
        .unwrap()
        .into_iter()
        .find(|f| f.check == c)
        .unwrap()
}

/// Puts `c` right as one batch of plans, then checks again.
fn fix(realm: &Realm, c: Check) -> Found {
    let layout = layout_of_realm(realm);
    for p in health::fix(realm, c, layout).unwrap() {
        modify::apply(realm, &p, layout).unwrap();
    }
    found(realm, c)
}

/// The value of `column` in the row of `table` whose key is `key`.
fn get(realm: &Realm, table: &str, key: &[i32], column: &str) -> Value {
    let opts = ExportOptions {
        zero_runtime: false,
        layout: layout_of_realm(realm),
        ..ExportOptions::default()
    };
    let data = export(realm, &opts).unwrap();
    let rows = data.table(table).unwrap();
    let index = daydream_nmr::schema::table(table)
        .unwrap()
        .index
        .as_ref()
        .unwrap();
    let at: Vec<usize> = index
        .columns
        .iter()
        .map(|c| rows.position(c).unwrap())
        .collect();
    let row = rows
        .rows
        .iter()
        .find(|r| at.iter().zip(key).all(|(&i, k)| r[i] == Value::Long(*k)))
        .unwrap_or_else(|| panic!("{table} {key:?}"));
    row[rows.position(column).unwrap()].clone()
}

#[test]
fn each_check_finds_its_records_and_its_fix_leaves_none() {
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    put(
        &realm,
        "Items",
        &[
            &[("Number", l(1)), ("Name", t("wand")), ("Uses", l(0))],
            &[
                ("Number", l(2)),
                ("Name", t("robe")),
                ("Uses", l(-1)),
                ("Ability 3", l(0)),
                ("Ability Value 3", l(8224)),
            ],
            &[
                ("Number", l(3)),
                ("Name", t("crown")),
                ("Uses", l(-1)),
                ("Game Limit", l(1)),
            ],
        ],
    );
    put(
        &realm,
        "Messages",
        &[
            &[("Number", l(1))],
            &[("Number", l(2)), ("Line 1", t("The rat bites you!"))],
        ],
    );
    // a rat whose blows are told by the blank message 1, and that drops a used-up wand
    put(
        &realm,
        "Monsters",
        &[&[
            ("Number", l(1)),
            ("Name", t("rat")),
            ("Attack Type 0", l(1)),
            ("Attack % 0", l(100)),
            ("Attack Hit Msg 0", l(1)),
            ("Attack Dodge Msg 0", l(2)),
            ("Item Number 0", l(1)),
            ("Item Uses 0", l(5)),
        ]],
    );
    // a text that goes on from part to part, with stray spaces only at its end
    put(
        &realm,
        "Textblocks",
        &[
            &[("Number", l(7)), ("Part #", l(0)), ("Data", t("giveitem "))],
            &[
                ("Number", l(7)),
                ("Part #", l(1)),
                ("Data", t("8:say hi  \n")),
            ],
            &[("Number", l(8)), ("Part #", l(0)), ("Data", t("fine\n"))],
        ],
    );
    // room 1 leads to room 2, and to room 3, which the world does not have
    put(
        &realm,
        "Rooms",
        &[
            &[
                ("Map Number", l(1)),
                ("Room Number", l(1)),
                ("Exit 0", l(2)),
                ("Exit 1", l(3)),
            ],
            &[
                ("Map Number", l(1)),
                ("Room Number", l(2)),
                ("Exit 1", l(1)),
            ],
        ],
    );

    let all = health::report(&realm, layout_of_realm(&realm)).unwrap();
    let got: Vec<(&str, Level, usize, usize)> = all
        .iter()
        .map(|f| (f.check.id(), f.level, f.records, f.values))
        .collect();
    assert_eq!(
        got,
        [
            ("uses", Level::Warn, 1, 1),
            ("blank_blows", Level::Warn, 1, 1),
            ("exits", Level::Warn, 1, 1),
            ("drop_uses", Level::Note, 1, 1),
            ("leftovers", Level::Note, 1, 1),
            ("spaces", Level::Note, 1, 1),
            ("limited", Level::Info, 1, 1),
        ]
    );
    let exits = found(&realm, Check::Exits);
    assert_eq!(exits.list[0].key, [1, 1]);
    assert!(
        health::fix(&realm, Check::Exits, layout_of_realm(&realm))
            .unwrap()
            .is_empty()
    );

    assert_eq!(fix(&realm, Check::Uses).level, Level::Ok);
    assert_eq!(get(&realm, "Items", &[1], "Uses"), Value::Long(-1));
    // the drop now takes the wand's −1
    assert_eq!(fix(&realm, Check::DropUses).level, Level::Ok);
    assert_eq!(
        get(&realm, "Monsters", &[1], "Item Uses 0"),
        Value::Long(-1)
    );
    assert_eq!(fix(&realm, Check::BlankBlows).level, Level::Ok);
    assert_eq!(
        get(&realm, "Monsters", &[1], "Attack Hit Msg 0"),
        Value::Long(0)
    );
    assert_eq!(
        get(&realm, "Monsters", &[1], "Attack Dodge Msg 0"),
        Value::Long(2)
    );
    assert_eq!(fix(&realm, Check::Leftovers).level, Level::Ok);
    assert_eq!(
        get(&realm, "Items", &[2], "Ability Value 3"),
        Value::Long(0)
    );
    assert_eq!(fix(&realm, Check::Spaces).level, Level::Ok);
    // the part that goes on keeps its space; the end of the text loses its own
    assert_eq!(get(&realm, "Textblocks", &[7, 0], "Data"), t("giveitem "));
    assert_eq!(get(&realm, "Textblocks", &[7, 1], "Data"), t("8:say hi\n"));
    assert_eq!(get(&realm, "Textblocks", &[8, 0], "Data"), t("fine\n"));
}

#[test]
fn the_stock_world_s_health() {
    let stock = Realm::open(&stock_realm()).unwrap();
    let r = health::report(&stock, layout_of_realm(&stock)).unwrap();
    let got: Vec<(&str, Level, usize, usize)> = r
        .iter()
        .map(|f| (f.check.id(), f.level, f.records, f.values))
        .collect();
    assert_eq!(
        got,
        [
            ("uses", Level::Warn, 34, 34),
            ("blank_blows", Level::Warn, 9, 27),
            ("exits", Level::Ok, 0, 0),
            ("drop_uses", Level::Note, 42, 47),
            ("leftovers", Level::Note, 1050, 4307),
            ("spaces", Level::Note, 19, 19),
            ("limited", Level::Info, 205, 205),
        ]
    );
    // NMR's notes: items kept after their uses with no recharge to set
    let uses = r.iter().find(|f| f.check == Check::Uses).unwrap();
    assert_eq!(uses.notes.len(), 3, "{:?}", uses.notes);

    // trimmed on a copy, the texts that go on from part to part keep their spaces
    let dir = tempfile::tempdir().unwrap();
    copy_realm(&stock_realm(), dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let before = get(&realm, "Textblocks", &[4684, 14], "Data");
    assert_eq!(fix(&realm, Check::Spaces).level, Level::Ok);
    assert_eq!(get(&realm, "Textblocks", &[4684, 14], "Data"), before);
    let Value::Text(six) = get(&realm, "Textblocks", &[6, 0], "Data") else {
        panic!("text block 6 holds text")
    };
    assert!(six.ends_with("spits to the side.\n"), "{six:?}");
}
