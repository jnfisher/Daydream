//! The options of NMR's Database Importer: tables, ranges, Skip/Update, a room's items, and
//! the parts of a text block past the file's last.

use daydream_nmr::{
    ExportOptions, ImportOptions, NmrData, NmrRows, Range, RoomParts, Value, export, import,
};
use daydream_store::Realm;

use crate::support::{empty_realm, random_realm, records};

fn full() -> ExportOptions {
    ExportOptions {
        zero_runtime: false,
        ..ExportOptions::default()
    }
}

/// Sets `column` of every row of `table` to `value`.
fn set_all(data: &mut NmrData, table: &str, column: &str, value: Value) {
    let rows = data.table_mut(table).unwrap();
    let i = rows.position(column).unwrap();
    for r in &mut rows.rows {
        r[i] = value.clone();
    }
}

fn numbers(data: &NmrData, table: &str) -> Vec<i32> {
    let rows = data.table(table).unwrap();
    let i = rows.position("Number").unwrap();
    rows.rows
        .iter()
        .map(|r| match r[i] {
            Value::Long(n) => n,
            _ => panic!(),
        })
        .collect()
}

#[test]
fn only_the_tables_and_range_asked_for() {
    for like_nmr in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), 40, 11);
        let realm = Realm::open(dir.path()).unwrap();
        let mut data = export(&realm, &full()).unwrap();
        set_all(&mut data, "Items", "Name", Value::Text("ranged".into()));
        set_all(&mut data, "Spells", "Name", Value::Text("ranged".into()));
        let n = numbers(&data, "Items");
        let opts = ImportOptions {
            like_nmr,
            tables: Some(vec!["items".into()]),
            ranges: vec![Range {
                table: "Items".into(),
                map: None,
                from: i64::from(n[10]),
                to: i64::from(n[19]),
            }],
            ..ImportOptions::default()
        };
        let reports = import(&realm, &data, &opts).unwrap();
        assert_eq!(reports.len(), 1, "{reports:?}");
        let r = &reports[0];
        assert_eq!(
            (r.updated, r.skipped),
            (10, 30),
            "like_nmr {like_nmr}: {r:?}"
        );
        let after = export(&realm, &full()).unwrap();
        let named: Vec<i32> = {
            let rows = after.table("Items").unwrap();
            let (num, name) = (
                rows.position("Number").unwrap(),
                rows.position("Name").unwrap(),
            );
            rows.rows
                .iter()
                .filter(|r| matches!(&r[name], Value::Text(s) if s.starts_with("ranged")))
                .map(|r| match r[num] {
                    Value::Long(x) => x,
                    _ => panic!(),
                })
                .collect()
        };
        assert_eq!(named, n[10..20].to_vec());
        assert_eq!(
            after.table("Spells"),
            export(&realm, &full()).unwrap().table("Spells")
        );
    }
}

#[test]
fn a_range_needs_numbered_records() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 3, 12);
    let realm = Realm::open(dir.path()).unwrap();
    let data = export(&realm, &full()).unwrap();
    let opts = ImportOptions {
        ranges: vec![Range {
            table: "Actions".into(),
            map: None,
            from: 1,
            to: 2,
        }],
        ..ImportOptions::default()
    };
    assert!(import(&realm, &data, &opts).is_err());
}

#[test]
fn a_rooms_items_or_all_but_them() {
    for (parts, name_changes, item_changes) in [
        (RoomParts::OnlyItems, false, true),
        (RoomParts::NotItems, true, false),
        (RoomParts::All, true, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), 6, 13);
        let realm = Realm::open(dir.path()).unwrap();
        let mut data = export(&realm, &full()).unwrap();
        set_all(&mut data, "Rooms", "Name", Value::Text("Somewhere".into()));
        set_all(&mut data, "Rooms", "Room Item 0", Value::Long(4242));
        let opts = ImportOptions {
            tables: Some(vec!["Rooms".into()]),
            room_parts: parts,
            ..ImportOptions::default()
        };
        import(&realm, &data, &opts).unwrap();
        let after = export(&realm, &full()).unwrap();
        let rooms = after.table("Rooms").unwrap();
        let (name, item) = (
            rooms.position("Name").unwrap(),
            rooms.position("Room Item 0").unwrap(),
        );
        for r in &rooms.rows {
            assert_eq!(
                matches!(&r[name], Value::Text(s) if s.starts_with("Somewhere")),
                name_changes,
                "{parts:?}"
            );
            assert_eq!(r[item] == Value::Long(4242), item_changes, "{parts:?}");
        }
    }
}

/// A realm of text blocks 7 (parts 0 to 2) and 9 (parts 0 and 1), and a file of their
/// first parts only.
fn text_blocks(dir: &std::path::Path) -> (Realm, NmrData) {
    empty_realm(dir);
    let src = tempfile::tempdir().unwrap();
    random_realm(src.path(), 1, 14);
    let template = export(&Realm::open(src.path()).unwrap(), &full()).unwrap();
    let t = template.table("Textblocks").unwrap();
    let (num, part) = (t.position("Number").unwrap(), t.position("Part #").unwrap());
    let row = |n: i32, p: i32| {
        let mut r = t.rows[0].clone();
        r[num] = Value::Long(n);
        r[part] = Value::Long(p);
        r
    };
    let table = |keys: &[(i32, i32)]| NmrData {
        tables: vec![NmrRows {
            name: t.name.clone(),
            columns: t.columns.clone(),
            rows: keys.iter().map(|&(n, p)| row(n, p)).collect(),
        }],
    };
    let realm = Realm::open(dir).unwrap();
    import(
        &realm,
        &table(&[(7, 0), (7, 1), (7, 2), (9, 0), (9, 1)]),
        &ImportOptions::default(),
    )
    .unwrap();
    (realm, table(&[(7, 0), (9, 0)]))
}

fn text_block_count(dir: &std::path::Path) -> usize {
    records(dir)
        .into_iter()
        .filter(|(s, _)| s.starts_with("WCCTEXT"))
        .map(|(_, r)| r.len())
        .sum()
}

#[test]
fn the_parts_past_a_text_blocks_last_go() {
    let dir = tempfile::tempdir().unwrap();
    let (realm, file) = text_blocks(dir.path());
    // not asked: every part stays
    import(&realm, &file, &ImportOptions::default()).unwrap();
    assert_eq!(text_block_count(dir.path()), 5);
    let opts = ImportOptions {
        extra_parts: true,
        ..ImportOptions::default()
    };
    let dry = ImportOptions {
        dry_run: true,
        ..opts.clone()
    };
    assert_eq!(import(&realm, &file, &dry).unwrap()[0].deleted, 3);
    assert_eq!(text_block_count(dir.path()), 5);
    assert_eq!(import(&realm, &file, &opts).unwrap()[0].deleted, 3);
    assert_eq!(text_block_count(dir.path()), 2);
}

#[test]
fn nmr_leaves_the_last_text_blocks_parts() {
    let dir = tempfile::tempdir().unwrap();
    let (realm, file) = text_blocks(dir.path());
    let opts = ImportOptions {
        like_nmr: true,
        extra_parts: true,
        ..ImportOptions::default()
    };
    // 7's parts 1 and 2 go when NMR reaches 9; 9 is the last, so its part 1 stays
    assert_eq!(import(&realm, &file, &opts).unwrap()[0].deleted, 2);
    assert_eq!(text_block_count(dir.path()), 3);
}

#[test]
fn skip_leaves_existing_records() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 5, 15);
    let realm = Realm::open(dir.path()).unwrap();
    let before = records(dir.path());
    let mut data = export(&realm, &full()).unwrap();
    set_all(&mut data, "Items", "Name", Value::Text("skipped".into()));
    let opts = ImportOptions {
        skip_existing: true,
        ..ImportOptions::default()
    };
    let r = import(&realm, &data, &opts).unwrap();
    assert_eq!(r.iter().find(|r| r.table == "Items").unwrap().skipped, 5);
    assert_eq!(records(dir.path()), before);
}

#[test]
fn the_deleter_takes_all_or_a_range() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 20, 16);
    let realm = Realm::open(dir.path()).unwrap();
    let data = export(&realm, &full()).unwrap();
    let n = numbers(&data, "Monsters");
    let range = Range {
        table: "Monsters".into(),
        map: None,
        from: i64::from(n[5]),
        to: i64::from(n[9]),
    };
    let layout = daydream_types::records::LayoutSet::Dos;
    let dry = daydream_nmr::delete_range(&realm, "Monsters", Some(&range), layout, true).unwrap();
    assert_eq!(dry.deleted, 5);
    let r = daydream_nmr::delete_range(&realm, "Monsters", Some(&range), layout, false).unwrap();
    assert_eq!(r.deleted, 5);
    let left = numbers(&export(&realm, &full()).unwrap(), "Monsters");
    assert_eq!(left, [&n[..5], &n[10..]].concat());
    let r = daydream_nmr::delete_range(&realm, "Spells", None, layout, false).unwrap();
    assert_eq!(r.deleted, 20);
    assert!(numbers(&export(&realm, &full()).unwrap(), "Spells").is_empty());
}
