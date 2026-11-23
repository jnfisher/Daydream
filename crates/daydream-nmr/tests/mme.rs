//! The MME export: NMR's own export of the stock world is the reference, value for value;
//! and the rules it follows on records set up for each case.

use std::collections::HashMap;

use daydream_nmr::mme::{self, Excluded, MmeOptions};
use daydream_nmr::{NmrData, NmrRows, Value};
use daydream_store::Realm;
use daydream_types::records::LayoutSet::Dos;

use crate::support::{empty_realm, l, put, t};

/// Columns NMR fills from its monster attack simulation. NMR draws from VB's `Rnd` and runs
/// each monster until its average settles, so a second run of NMR itself gives slightly
/// different numbers: these are compared within [`close`], not exactly.
const SIMULATED: &[&str] = &[
    "AvgDmg",
    "AvgDmgPhys",
    "AvgDmgSpell",
    "ScriptValue",
    "AttTrue%-0",
    "AttTrue%-1",
    "AttTrue%-2",
    "AttTrue%-3",
    "AttTrue%-4",
];

/// The export's date and time.
const NOT_COMPARED: &[&str] = &["Date", "Time"];

fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Long(x) => Some(f64::from(*x)),
        Value::Double(x) => Some(*x),
        _ => None,
    }
}

/// How far a simulated value is from NMR's, relative to NMR's (or to 1, for values under 1).
fn relative(a: &Value, b: &Value) -> f64 {
    match (number(a), number(b)) {
        (Some(x), Some(y)) => (x - y).abs() / y.abs().max(1.0),
        _ => f64::INFINITY,
    }
}

/// A simulated value close enough to NMR's: within 3 (points of damage or of a percentage,
/// where the values are small) or within 25%. Measured against NMR's export of the stock
/// world, the widest gaps are 3.1 points at 23% and 1.8 points at 35%.
fn close(a: &Value, b: &Value) -> bool {
    match (number(a), number(b)) {
        (Some(x), Some(y)) => (x - y).abs() <= 3.0 || relative(a, b) <= 0.25,
        _ => a == b,
    }
}

/// At most this share of a simulated column may be more than 10% off NMR's (measured: under
/// 4.5%), so that a sim that drifts as a whole fails even while each value is [`close`].
const DRIFT: f64 = 0.06;

fn key(t: &NmrRows, r: &[Value]) -> String {
    let cols: &[&str] = match t.name.as_str() {
        "Rooms" => &["Map Number", "Room Number"],
        "Lairs" => &["GroupIndex"],
        "Info" => &[],
        _ => &["Number"],
    };
    cols.iter()
        .map(|c| format!("{:?}", r[t.position(c).unwrap()]))
        .collect::<Vec<_>>()
        .join("/")
}

/// A value as Jet gives it back: a Long and a Double of the same number are the same.
fn same(a: &Value, b: &Value) -> bool {
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// Every difference between `ours` and `theirs`, but in `skip`'s columns; the
/// [`SIMULATED`] ones are compared within [`close`] and [`DRIFT`].
fn differences(ours: &NmrData, theirs: &NmrData, skip: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for t in &ours.tables {
        let Some(th) = theirs.table(&t.name) else {
            out.push(format!("{}: not in the reference", t.name));
            continue;
        };
        if t.columns != th.columns {
            out.push(format!(
                "{}: columns {:?} vs {:?}",
                t.name, t.columns, th.columns
            ));
            continue;
        }
        let theirs: HashMap<String, &Vec<Value>> =
            th.rows.iter().map(|r| (key(th, r), r)).collect();
        let ours: HashMap<String, &Vec<Value>> = t.rows.iter().map(|r| (key(t, r), r)).collect();
        let mut drift: HashMap<&str, usize> = HashMap::new();
        for k in theirs.keys().filter(|k| !ours.contains_key(*k)) {
            out.push(format!("{} {k}: only in the reference", t.name));
        }
        for (k, r) in &ours {
            let Some(tr) = theirs.get(k) else {
                out.push(format!("{} {k}: not in the reference", t.name));
                continue;
            };
            for (i, c) in t.columns.iter().enumerate() {
                if skip.contains(&c.as_str()) {
                    continue;
                }
                let simulated = SIMULATED.contains(&c.as_str());
                if simulated && relative(&r[i], &tr[i]) > 0.1 {
                    *drift.entry(c).or_default() += 1;
                }
                let ok = if simulated {
                    close(&r[i], &tr[i])
                } else {
                    same(&r[i], &tr[i])
                };
                if !ok {
                    out.push(format!(
                        "{} {k} {c}: ours {:?}, NMR's {:?}",
                        t.name, r[i], tr[i]
                    ));
                }
            }
        }
        for (c, n) in drift {
            if n as f64 > DRIFT * t.rows.len() as f64 {
                out.push(format!(
                    "{} {c}: {n} of {} values more than 10% off NMR's",
                    t.name,
                    t.rows.len()
                ));
            }
        }
    }
    out
}

#[test]
fn the_export_matches_nmrs_own_of_the_stock_world() {
    let reference = std::env::var("DAYDREAM_MME_REFERENCE").expect(
        "set DAYDREAM_MME_REFERENCE to NMR's MME export of the stock world (tools/mme/reference.sh)",
    );
    let realm = Realm::open(&crate::support::stock_realm()).unwrap();
    let ours = mme::export(&realm, Dos, &MmeOptions::default()).unwrap();
    let names: Vec<&str> = ours.tables.iter().map(|t| t.name.as_str()).collect();
    let theirs = daydream_nmr::mdb::read_tables(std::path::Path::new(&reference), &names).unwrap();
    let diff = differences(&ours, &theirs, NOT_COMPARED);
    assert!(
        diff.is_empty(),
        "{} differences:\n{}",
        diff.len(),
        diff.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

fn row<'a>(t: &'a NmrRows, key: &str) -> &'a [Value] {
    t.rows
        .iter()
        .find(|r| self::key(t, r) == key)
        .unwrap_or_else(|| panic!("{} {key}", t.name))
}

fn get<'a>(t: &'a NmrRows, key: &str, column: &str) -> &'a Value {
    &row(t, key)[t.position(column).unwrap()]
}

#[test]
fn rooms_exits_and_lairs_read_as_nmr_writes_them() {
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    put(
        &realm,
        "Messages",
        &[&[
            ("Number", l(5)),
            ("Line 1", t("you pull a lever")),
            ("Line 2", t("it clicks")),
        ]],
    );
    put(
        &realm,
        "Monsters",
        &[
            &[
                ("Number", l(7)),
                ("Name", t("orc")),
                ("Group", l(6)),
                ("Index", l(2)),
            ],
            &[
                ("Number", l(8)),
                ("Name", t("sdf orc")),
                ("Group", l(6)),
                ("Index", l(2)),
            ],
            &[
                ("Number", l(9)),
                ("Name", t("orc chief")),
                ("Group", l(6)),
                ("Index", l(3)),
            ],
        ],
    );
    let exit = |x: usize, to: i32, ty: i32, p: [i32; 4]| {
        let mut v = vec![(format!("Exit {x}"), l(to)), (format!("Type {x}"), l(ty))];
        for (k, n) in p.iter().enumerate() {
            v.push((format!("Para{} {x}", k + 1), l(*n)));
        }
        v
    };
    let mut first: Vec<(String, Value)> = vec![
        ("Map Number".into(), l(1)),
        ("Room Number".into(), l(10)),
        ("Name".into(), t("Hall")),
        ("Type".into(), l(3)),
        ("Mon Type".into(), l(6)),
        ("Min Index".into(), l(2)),
        ("Max Index".into(), l(3)),
        ("Max Regen".into(), l(4)),
        ("Placed Item 0".into(), l(11)),
        ("Placed Item 3".into(), l(12)),
    ];
    first.extend(exit(0, 11, 0, [0; 4]));
    first.extend(exit(1, 12, 2, [33, 0, -5, 0]));
    first.extend(exit(2, 13, 7, [0, 0, 0, 0]));
    first.extend(exit(3, 14, 7, [0, -2, 0, 40]));
    first.extend(exit(4, 15, 8, [3, 0, 0, 0]));
    first.extend(exit(5, 16, 6, [5, -3, 0, 0]));
    first.extend(exit(6, 17, 10, [5, 0, 0, 0]));
    first.extend(exit(7, 18, 20, [-300, 45, 0, 0]));
    first.extend(exit(8, 19, 99, [0; 4]));
    let first: Vec<(&str, Value)> = first.iter().map(|(c, v)| (c.as_str(), v.clone())).collect();
    put(
        &realm,
        "Rooms",
        &[
            &first,
            &[
                ("Map Number", l(1)),
                ("Room Number", l(11)),
                ("Name", t("Buffer Room")),
                ("Delay", l(3)),
            ],
            &[
                ("Map Number", l(1)),
                ("Room Number", l(164)),
                ("Name", t("sysop support")),
            ],
            &[
                ("Map Number", l(2)),
                ("Room Number", l(5)),
                ("Name", t("my test room")),
                ("Light", l(4)),
            ],
        ],
    );
    put(
        &realm,
        "Shops",
        &[&[
            ("Number", l(1)),
            ("Name", t("gang shop")),
            ("Type", l(11)),
            ("Item 0", l(5)),
            ("Max 0", l(9)),
        ]],
    );

    let o = MmeOptions {
        legit: false,
        exclude_rooms: true,
        excluded: vec![Excluded {
            map: 2,
            from: 1,
            to: 9,
        }],
        hide_excluded: true,
        ..MmeOptions::default()
    };
    let data = mme::export(&realm, Dos, &o).unwrap();
    let rooms = data.table("Rooms").unwrap();
    // buffer rooms and NMR's default exclusions are left out; the sysop's own are blanked
    let keys: Vec<String> = rooms.rows.iter().map(|r| key(rooms, r)).collect();
    assert_eq!(keys, ["Long(1)/Long(10)", "Long(2)/Long(5)"]);
    assert_eq!(get(rooms, "Long(2)/Long(5)", "Name"), &t("my test room"));
    assert_eq!(get(rooms, "Long(2)/Long(5)", "Light"), &l(0));
    assert_eq!(get(rooms, "Long(2)/Long(5)", "N"), &t("\0"));

    let hall = "Long(1)/Long(10)";
    let exits: Vec<&Value> = mme::DIRECTIONS
        .iter()
        .map(|d| get(rooms, hall, d))
        .collect();
    assert_eq!(
        exits,
        [
            &t("1/11"),
            &t("1/12 (Key: 33 [or 6 picklocks])"),
            &t("1/13 (Door)"),
            &t("1/14 (Key: 40 [or 3 picklocks/strength])"),
            &t("3/15"),
            &t("1/16 (Hidden/Needs 3 Actions, any order)"),
            &t("1/17 (Text: you pull a lever, it clicks)"),
            &t("1/18 (Alignment: Saint to Outlaw)"),
            // a type NMR does not know is not written
            &Value::Null,
            &t("0"),
        ]
    );
    assert_eq!(get(rooms, hall, "Delay"), &l(5));
    assert_eq!(get(rooms, hall, "Placed"), &t("11,12,\0"));
    assert_eq!(get(rooms, hall, "Lair"), &t("(Max 4): 7,9,[6-2-3-4]"));

    let shops = data.table("Shops").unwrap();
    assert_eq!(get(shops, "Long(1)", "Item-0"), &l(0));
    assert_eq!(get(shops, "Long(1)", "Max-0"), &l(0));

    let info = &data.table("Info").unwrap().rows[0];
    assert_eq!(info[4], t(""));
    assert_eq!(info[5], l(0));

    // the plan Jet turns into the file has every table
    let plan = mme::plan(&data);
    for name in [
        "Rooms", "Lairs", "Classes", "Races", "Spells", "Monsters", "Items", "Shops", "Info",
        "TBInfo",
    ] {
        assert!(plan.contains(&format!("T\t{name}\n")), "{name}");
    }

    // legit ignores the sysop's own list
    let legit = mme::export(&realm, Dos, &MmeOptions::default()).unwrap();
    assert_eq!(legit.table("Rooms").unwrap().rows.len(), 2);
    assert_eq!(
        get(legit.table("Rooms").unwrap(), "Long(2)/Long(5)", "Light"),
        &l(4)
    );
}

#[test]
fn monster_attacks_are_named_as_nmr_names_them() {
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    put(
        &realm,
        "Messages",
        &[
            &[
                ("Number", l(1)),
                ("Line 1", t("The orc %s bites you for %d damage!")),
            ],
            &[
                ("Number", l(2)),
                ("Line 1", t("The orc %s all-out smashes you!")),
            ],
        ],
    );
    put(
        &realm,
        "Spells",
        &[&[("Number", l(4)), ("Name", t("fireball"))]],
    );
    put(
        &realm,
        "Monsters",
        &[&[
            ("Number", l(1)),
            ("Name", t("orc")),
            ("Attack Type 0", l(1)),
            ("Attack Hit Msg 0", l(1)),
            ("Attack Type 1", l(1)),
            ("Attack Hit Msg 1", l(2)),
            ("Attack Type 2", l(2)),
            ("Attack Accu/Spell 2", l(4)),
            ("Attack Type 3", l(3)),
            ("Attack Type 4", l(2)),
            ("Attack Accu/Spell 4", l(99)),
        ]],
    );
    let data = mme::export(&realm, Dos, &MmeOptions::default()).unwrap();
    let mons = data.table("Monsters").unwrap();
    let names: Vec<&Value> = (0..5)
        .map(|x| get(mons, "Long(1)", &format!("AttName-{x}")))
        .collect();
    assert_eq!(
        names,
        [
            &t("bites you"),
            &t("all-out smashes"),
            &t("fireball"),
            &t("Unknown"),
            &t("Spell 99")
        ]
    );
}

#[test]
fn the_walk_follows_what_players_can_reach() {
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    put(
        &realm,
        "Rooms",
        &[&[
            ("Map Number", l(1)),
            ("Room Number", l(1)),
            ("Name", t("Square")),
            ("Type", l(1)),
            ("Shop Number", l(2)),
            ("Mon Type", l(3)),
            ("Min Index", l(1)),
            ("Max Index", l(1)),
            ("Command Text", l(10)),
            ("Placed Item 0", l(5)),
        ]],
    );
    put(
        &realm,
        "Shops",
        &[&[("Number", l(2)), ("Name", t("pawn")), ("Item 0", l(6))]],
    );
    put(
        &realm,
        "Monsters",
        &[&[
            ("Number", l(7)),
            ("Name", t("rat")),
            ("Group", l(3)),
            ("Index", l(1)),
            ("Item Number 0", l(8)),
            ("Item Drop % 0", l(25)),
        ]],
    );
    let items: Vec<Vec<(&str, Value)>> = [5, 6, 8, 9, 11, 12]
        .iter()
        .map(|&n| vec![("Number", l(n)), ("Name", t("thing"))])
        .collect();
    let items: Vec<&[(&str, Value)]> = items.iter().map(|r| r.as_slice()).collect();
    put(&realm, "Items", &items);
    put(
        &realm,
        "Spells",
        &[&[("Number", l(4)), ("Name", t("light"))]],
    );
    put(
        &realm,
        "Textblocks",
        &[&[
            ("Number", l(10)),
            ("Part #", l(0)),
            (
                "Data",
                t("giveitem 9\nclass 2 learnspell 4\ncheckitem 12 giveitem 11\n"),
            ),
        ]],
    );
    let data = mme::export(&realm, Dos, &MmeOptions::default()).unwrap();
    let items = data.table("Items").unwrap();
    let item = |n: i32, c: &str| get(items, &format!("Long({n})"), c).clone();
    assert_eq!(item(5, "In Game"), l(1));
    assert_eq!(item(5, "Obtained From"), t("Room 1/1"));
    // a shop that only buys it: noted, not in the game
    assert_eq!(item(6, "In Game"), l(0));
    assert_eq!(item(6, "Obtained From"), t("Shop(sell) #2"));
    assert_eq!(item(8, "Obtained From"), t("Monster #7(25%)"));
    assert_eq!(item(9, "Obtained From"), t("Textblock #10"));
    // held back on the first pass, as item 12 is not in the game; but NMR counts an item
    // a block checks for as found, so the next pass gives it
    assert_eq!(item(11, "In Game"), l(1));
    assert_eq!(item(11, "Obtained From"), t("Textblock #10"));
    assert_eq!(item(12, "In Game"), l(0));
    assert_eq!(item(12, "References"), t("Textblock #10"));
    let spells = data.table("Spells").unwrap();
    assert_eq!(get(spells, "Long(4)", "Learnable"), &l(1));
    assert_eq!(get(spells, "Long(4)", "Learned From"), &t("Textblock #10"));
    assert_eq!(get(spells, "Long(4)", "Classes"), &t("(2)"));
    let mons = data.table("Monsters").unwrap();
    assert_eq!(get(mons, "Long(7)", "In Game"), &l(1));
    assert_eq!(get(mons, "Long(7)", "Summoned By"), &t("Group: 1/1"));
    let shops = data.table("Shops").unwrap();
    assert_eq!(get(shops, "Long(2)", "Assigned To"), &t("Room 1/1"));
    let tb = data.table("TBInfo").unwrap();
    assert_eq!(get(tb, "Long(10)", "Called From"), &t("Room 1/1"));
    assert_eq!(
        get(tb, "Long(10)", "Action"),
        &t("giveitem 9\nclass 2 learnspell 4\ncheckitem 12 giveitem 11\n")
    );
}
