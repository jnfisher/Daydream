//! Export and import over realms of random records: every column of every NMR table.

use daydream_nmr::schema::{Source, layout_of};
use daydream_nmr::{ExportOptions, ImportOptions, NmrData, Value, export, import, jet};
use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::support::{assert_same_data, empty_realm, random_realm, records, show, stem};

fn full() -> ExportOptions {
    ExportOptions {
        zero_runtime: false,
        ..ExportOptions::default()
    }
}

#[test]
fn importing_an_export_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 40, 1);
    let before = records(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let data = export(&realm, &full()).unwrap();
    let reports = import(&realm, &data, &ImportOptions::default()).unwrap();
    for r in &reports {
        assert_eq!(r.errors, Vec::<String>::new(), "{}", r.table);
        assert_eq!((r.inserted, r.updated), (0, 0), "{}: {r:?}", r.table);
        assert_eq!(r.unchanged, 40, "{}", r.table);
    }
    assert_eq!(records(dir.path()), before);
}

#[test]
fn an_empty_realm_takes_every_value_back() {
    let src = tempfile::tempdir().unwrap();
    random_realm(src.path(), 25, 2);
    let data = export(&Realm::open(src.path()).unwrap(), &full()).unwrap();

    let dest = tempfile::tempdir().unwrap();
    empty_realm(dest.path());
    let realm = Realm::open(dest.path()).unwrap();
    let reports = import(&realm, &data, &ImportOptions::default()).unwrap();
    for r in &reports {
        assert_eq!(r.errors, Vec::<String>::new(), "{}", r.table);
        assert_eq!(r.inserted, 25, "{}", r.table);
    }
    let again = export(&realm, &full()).unwrap();
    assert_same_data(&data, &again);
}

#[test]
fn json_and_plans_carry_the_data() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 10, 3);
    let data = export(&Realm::open(dir.path()).unwrap(), &full()).unwrap();
    let json = serde_json::to_string(&data).unwrap();
    let back: NmrData = serde_json::from_str(&json).unwrap();
    assert_eq!(back, data);
    let plan = jet::plan(&data).unwrap();
    // one row line per row, and only text values carry a NUL, escaped
    let rows: usize = data.tables.iter().map(|t| t.rows.len()).sum();
    assert_eq!(plan.lines().filter(|l| l.starts_with("R\t")).count(), rows);
    assert!(!plan.contains('\0'));
}

#[test]
fn an_edit_changes_only_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 5, 4);
    let realm = Realm::open(dir.path()).unwrap();
    let before = records(dir.path());
    let mut data = export(&realm, &full()).unwrap();
    let items = data.table_mut("Items").unwrap();
    let (name, cost) = (
        items.position("Name").unwrap(),
        items.position("Cost").unwrap(),
    );
    items.rows[2][name] = Value::Text("Daydream sword".into());
    items.rows[2][cost] = Value::Long(-2);
    let number = items.rows[2][items.position("Number").unwrap()].clone();
    let reports = import(&realm, &data, &ImportOptions::default()).unwrap();
    let r = reports.iter().find(|r| r.table == "Items").unwrap();
    assert_eq!((r.updated, r.unchanged), (1, 4), "{r:?}");
    assert!(reports.iter().all(|r| r.table == "Items" || r.updated == 0));

    let after = records(dir.path());
    let layout = layout_of(daydream_wcc::tables::Table::Items, LayoutSet::Dos);
    let name_f = layout.field("name").unwrap();
    let cost_f = layout.field("cost").unwrap();
    let stem = stem(daydream_wcc::tables::Table::Items);
    let (_, old) = before.iter().find(|(s, _)| s == stem).unwrap();
    let (_, new) = after.iter().find(|(s, _)| s == stem).unwrap();
    let mut changed = 0;
    for ((_, a), (_, b)) in old.iter().zip(new) {
        if a == b {
            continue;
        }
        changed += 1;
        assert_eq!(&b[..4], &(number_of(&number)).to_le_bytes());
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            let in_name = (name_f.offset..name_f.offset + 29).contains(&i);
            let in_cost = (cost_f.offset..cost_f.offset + 2).contains(&i);
            assert!(x == y || in_name || in_cost, "byte {i:#x} changed");
        }
        assert_eq!(&b[name_f.offset..name_f.offset + 15], b"Daydream sword\0");
        assert_eq!(&b[cost_f.offset..cost_f.offset + 2], &[0xFE, 0xFF]);
    }
    assert_eq!(changed, 1);
}

fn number_of(v: &Value) -> i32 {
    match v {
        Value::Long(n) => *n,
        other => panic!("{}", show(other)),
    }
}

#[test]
fn bad_values_are_reported_per_row() {
    let src = tempfile::tempdir().unwrap();
    random_realm(src.path(), 3, 5);
    let realm = Realm::open(src.path()).unwrap();
    let mut data = export(&realm, &full()).unwrap();
    let races = data.table_mut("Races").unwrap();
    let hp = races.position("HP Bonus").unwrap();
    races.rows[1][hp] = Value::Long(1 << 20);
    let reports = import(&realm, &data, &ImportOptions::default()).unwrap();
    let r = reports.iter().find(|r| r.table == "Races").unwrap();
    assert_eq!(r.errors.len(), 1, "{r:?}");
    assert!(r.errors[0].contains("HP Bonus"), "{}", r.errors[0]);
    assert_eq!(r.unchanged, 2);
}

#[test]
fn zero_runtime_clears_game_state_only() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 8, 6);
    let realm = Realm::open(dir.path()).unwrap();
    let all = export(&realm, &full()).unwrap();
    let zeroed = export(&realm, &ExportOptions::default()).unwrap();
    let zero = Value::Long(0);
    for (a, z) in all.tables.iter().zip(&zeroed.tables) {
        for (ra, rz) in a.rows.iter().zip(&z.rows) {
            for (i, c) in a.columns.iter().enumerate() {
                let runtime = match a.name.as_str() {
                    "Rooms" => {
                        c.contains("Runic")
                            || c.contains("Platinum")
                            || c.contains("Gold")
                            || c.contains("Silver")
                            || c.contains("Copper")
                            || c.starts_with("Room Item")
                            || c.starts_with("Hidden Item")
                    }
                    "Shops" => c.starts_with("Normal "),
                    "Monsters" => ["Active", "Date Killed", "Time Killed"].contains(&c.as_str()),
                    _ => false,
                };
                if !runtime {
                    assert_eq!(ra[i], rz[i], "{}.{c}", a.name);
                } else if a.name != "Shops" && !c.starts_with("Hidden Item") {
                    assert_eq!(rz[i], zero, "{}.{c}", a.name);
                }
            }
        }
    }
    // every schema column is written (CurrentRoomMon is always 0)
    let rooms = zeroed.table("Rooms").unwrap();
    let t = rooms.schema().unwrap();
    assert_eq!(rooms.columns.len(), t.columns.len());
    assert!(t.columns.iter().any(|c| matches!(c.source, Source::Zero)));
}

#[test]
fn like_nmr_trims_text_and_builds_new_records_from_the_one_before() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 3, 6);
    let realm = Realm::open(dir.path()).unwrap();
    let mut data = export(&realm, &full()).unwrap();
    let races = data.table_mut("Races").unwrap();
    let (name, number) = (
        races.position("Name").unwrap(),
        races.position("Number").unwrap(),
    );
    races.rows[0][name] = Value::Text("  Elf \0".into());
    let mut new = races.rows[1].clone();
    new[number] = Value::Long(100);
    new[name] = Value::Text("Gnome".into());
    races.rows.push(new);
    let opts = ImportOptions {
        like_nmr: true,
        ..ImportOptions::default()
    };
    let reports = import(&realm, &data, &opts).unwrap();
    let r = reports.iter().find(|r| r.table == "Races").unwrap();
    assert_eq!(r.inserted, 1, "{r:?}");

    let layout = layout_of(daydream_wcc::tables::Table::Races, LayoutSet::Dos);
    let f = layout.field("name").unwrap();
    let recs = &records(dir.path())
        .into_iter()
        .find(|(s, _)| s == stem(daydream_wcc::tables::Table::Races))
        .unwrap()
        .1;
    let by_number = |n: i32| {
        &recs
            .iter()
            .find(|(_, b)| b[..2] == (n as i16).to_le_bytes())
            .unwrap()
            .1
    };
    // trimmed, then padded with spaces (the NUL was inside the text)
    let first = by_number(number_of(&data.table("Races").unwrap().rows[0][number]));
    let mut want = b"Elf \0".to_vec();
    want.resize(29, b' '); // NMR's Name is 29 of the member's 30 bytes
    assert_eq!(&first[f.offset..f.offset + 29], &want[..]);
    // the new record carries the last row's bytes outside the columns
    let last = by_number(number_of(&data.table("Races").unwrap().rows[2][number]));
    let added = by_number(100);
    let unknown = layout.field("unk_0x05C").unwrap();
    let span = unknown.offset..unknown.offset + unknown.size;
    assert_eq!(&added[span.clone()], &last[span]);
}
