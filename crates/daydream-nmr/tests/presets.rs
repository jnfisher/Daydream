//! NMR's one-click fixes, on records set up for each case.

use daydream_nmr::modify::{self, Cmp};
use daydream_nmr::presets::{self, DropPct, Limited, Preset};
use daydream_nmr::{ExportOptions, ImportOptions, NmrData, NmrRows, Value, export, import};
use daydream_store::Realm;
use daydream_types::records::LayoutSet::Dos;

use crate::support::random_realm;

fn table(realm: &Realm, name: &str) -> NmrRows {
    let opts = ExportOptions {
        zero_runtime: false,
        ..ExportOptions::default()
    };
    export(realm, &opts).unwrap().table(name).unwrap().clone()
}

fn n(v: &Value) -> f64 {
    match v {
        Value::Long(x) => f64::from(*x),
        Value::Double(x) => *x,
        _ => panic!("{v:?}"),
    }
}

/// The record numbered `number`'s value of `column`.
fn get(realm: &Realm, name: &str, number: i64, column: &str) -> f64 {
    let t = table(realm, name);
    let (k, c) = (t.position("Number").unwrap(), t.position(column).unwrap());
    let row = t.rows.iter().find(|r| n(&r[k]) as i64 == number).unwrap();
    n(&row[c])
}

/// Sets the `i`th record of `name` to `number`, every ability slot empty, and `values`.
fn set(realm: &Realm, name: &str, i: usize, number: i64, values: &[(&str, f64)]) {
    let mut t = table(realm, name);
    let mut row = t.rows[i].clone();
    let like = |v: &Value, x: f64| match v {
        Value::Double(_) => Value::Double(x),
        _ => Value::Long(x as i32),
    };
    for (c, v) in t.columns.iter().zip(row.iter_mut()) {
        if c.starts_with("Ability") || c.starts_with("Item Number") {
            *v = like(v, 0.0);
        }
    }
    row[t.position("Number").unwrap()] = Value::Long(number as i32);
    for (c, x) in values {
        let p = t.position(c).unwrap_or_else(|| panic!("{c}"));
        row[p] = like(&row[p], *x);
    }
    t.rows = vec![row];
    let report = import(
        realm,
        &NmrData { tables: vec![t] },
        &ImportOptions::default(),
    )
    .unwrap();
    assert!(report.iter().all(|r| r.errors.is_empty()), "{report:?}");
}

fn realm(dir: &std::path::Path) -> Realm {
    random_realm(dir, 12, 31);
    Realm::open(dir).unwrap()
}

#[test]
fn no_limited_items_keeps_the_quest_items_and_gang_items() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    set(&realm, "Items", 0, 839, &[("Game Limit", 5.0)]);
    set(
        &realm,
        "Items",
        1,
        5001,
        &[("Game Limit", 3.0), ("Ability 4", 182.0)],
    );
    set(&realm, "Items", 2, 5002, &[("Game Limit", 3.0)]);
    let plan = presets::plan(&realm, &Preset::NoLimitedItems, Dos).unwrap();
    assert!(plan.skipped >= 2, "{plan:?}");
    modify::apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(&realm, "Items", 839, "Game Limit"), 5.0);
    assert_eq!(get(&realm, "Items", 5001, "Game Limit"), 3.0);
    assert_eq!(get(&realm, "Items", 5002, "Game Limit"), 0.0);
}

#[test]
fn level_restrictions_go_with_their_values() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    set(
        &realm,
        "Items",
        0,
        5001,
        &[
            ("Ability 1", 135.0),
            ("Ability Value 1", 10.0),
            ("Ability 3", 136.0),
            ("Ability Value 3", 20.0),
            ("Ability 5", 2.0),
            ("Ability Value 5", 4.0),
        ],
    );
    let plan = presets::plan(&realm, &Preset::NoLevelRestrictions, Dos).unwrap();
    modify::apply(&realm, &plan, Dos).unwrap();
    for (c, want) in [
        ("Ability 1", 0.0),
        ("Ability Value 1", 0.0),
        ("Ability 3", 0.0),
        ("Ability Value 3", 0.0),
        ("Ability 5", 2.0),
        ("Ability Value 5", 4.0),
    ] {
        assert_eq!(get(&realm, "Items", 5001, c), want, "{c}");
    }
}

#[test]
fn uses_are_fixed_on_items_then_on_drops() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    set(&realm, "Items", 0, 5001, &[("Uses", 0.0)]);
    set(
        &realm,
        "Items",
        1,
        5002,
        &[
            ("Uses", 5.0),
            ("Retain After Uses", 1.0),
            ("Ability 2", 121.0),
            ("Ability Value 2", 2.0),
        ],
    );
    set(
        &realm,
        "Items",
        2,
        5003,
        &[("Uses", 5.0), ("Retain After Uses", 1.0)],
    );
    let plan = presets::plan(&realm, &Preset::FixItemUses, Dos).unwrap();
    assert!(
        plan.notes.iter().any(|n| n.starts_with("Item #5003")),
        "{:?}",
        plan.notes
    );
    modify::apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(&realm, "Items", 5001, "Uses"), -1.0);
    assert_eq!(get(&realm, "Items", 5002, "Ability Value 2"), 5.0);

    set(
        &realm,
        "Monsters",
        0,
        7001,
        &[
            ("Item Number 3", 5002.0),
            ("Item Uses 3", 9.0),
            ("Item Number 4", 9999.0),
        ],
    );
    let plan = presets::plan(&realm, &Preset::FixDropUses, Dos).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("item #9999 not found")),
        "{:?}",
        plan.notes
    );
    modify::apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(&realm, "Monsters", 7001, "Item Uses 3"), 5.0);
}

#[test]
fn exp_is_multiplied_divided_and_combined() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    set(
        &realm,
        "Monsters",
        0,
        7001,
        &[("Experience", 60_000.0), ("Exp Multiplier", 2.0)],
    );
    set(
        &realm,
        "Monsters",
        1,
        7002,
        &[("Experience", 5.0), ("Exp Multiplier", 1.0)],
    );
    let run = |p: Preset| {
        let plan = presets::plan(&realm, &p, Dos).unwrap();
        modify::apply(&realm, &plan, Dos).unwrap();
    };
    let exp = |m| {
        (
            get(&realm, "Monsters", m, "Experience"),
            get(&realm, "Monsters", m, "Exp Multiplier"),
        )
    };
    run(Preset::MultiplyExp { by: 2, cap: None });
    assert_eq!(exp(7001), (12_000.0, 20.0));
    assert_eq!(exp(7002), (10.0, 1.0));
    run(Preset::DivideExp { by: 4 });
    // 240,000 / 4 = 60,000, under 100,000: no multiplier
    assert_eq!(exp(7001), (60_000.0, 1.0));
    // 10 / 4 = 2.5, which VB rounds to 2
    assert_eq!(exp(7002), (2.0, 1.0));
    set(
        &realm,
        "Monsters",
        0,
        7001,
        &[("Experience", 300.0), ("Exp Multiplier", 7.0)],
    );
    run(Preset::CombineExp);
    assert_eq!(exp(7001), (2100.0, 1.0));
    assert!(presets::plan(&realm, &Preset::MultiplyExp { by: 21, cap: None }, Dos).is_err());
}

#[test]
fn kill_times_are_set_and_reset() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    set(&realm, "Monsters", 0, 7001, &[]);
    let plan = presets::plan(
        &realm,
        &Preset::SetKillTimes {
            year: 1990,
            month: 1,
            day: 1,
        },
        Dos,
    )
    .unwrap();
    modify::apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(&realm, "Monsters", 7001, "Date Killed"), 5153.0);
    assert_eq!(get(&realm, "Monsters", 7001, "Time Killed"), 2081.0);
    let plan = presets::plan(&realm, &Preset::ResetKillTimes, Dos).unwrap();
    modify::apply(&realm, &plan, Dos).unwrap();
    assert_eq!(get(&realm, "Monsters", 7001, "Date Killed"), 0.0);
    assert_eq!(get(&realm, "Monsters", 7001, "Time Killed"), 0.0);
}

#[test]
fn drop_percentages_follow_nmrs_form() {
    let dir = tempfile::tempdir().unwrap();
    let realm = realm(dir.path());
    // a weapon, worn-somewhere armour, limited armour, and a weapon deleted at maintenance
    set(
        &realm,
        "Items",
        0,
        5001,
        &[("Type", 1.0), ("Game Limit", 0.0)],
    );
    set(
        &realm,
        "Items",
        1,
        5002,
        &[("Type", 0.0), ("Worn On", 2.0), ("Game Limit", 0.0)],
    );
    set(
        &realm,
        "Items",
        2,
        5003,
        &[("Type", 0.0), ("Worn On", 2.0), ("Game Limit", 1.0)],
    );
    set(
        &realm,
        "Items",
        3,
        5004,
        &[("Type", 1.0), ("Game Limit", 0.0), ("Ability 0", 119.0)],
    );
    set(
        &realm,
        "Monsters",
        0,
        7001,
        &[
            ("Regen Time", 5.0),
            ("Item Number 0", 5001.0),
            ("Item Drop % 0", 10.0),
            ("Item Number 1", 5002.0),
            ("Item Drop % 1", 10.0),
            ("Item Number 2", 5003.0),
            ("Item Drop % 2", 10.0),
            ("Item Number 3", 5004.0),
            ("Item Drop % 3", 10.0),
            ("Item Number 4", 5001.0),
            ("Item Drop % 4", 40.0),
        ],
    );
    let form = DropPct {
        set: false,
        value: 95,
        only_if: (Cmp::Le, 20),
        limited: Limited::Exclude,
        exclude_del_at_maint: true,
        min_regen: None,
        weapons: true,
        armour: true,
        keys: false,
        containers: false,
        scrolls: false,
    };
    let plan = presets::plan(&realm, &Preset::ItemDropPct(form.clone()), Dos).unwrap();
    let changed: Vec<&str> = plan.changes.iter().map(|c| c.column.as_str()).collect();
    assert_eq!(changed, ["Item Drop % 0", "Item Drop % 1"], "{plan:?}");
    // adding holds the result at 100
    assert_eq!(plan.changes[0].after, Value::Long(100));

    // "only if regen > 5" leaves the monster out
    let regen = DropPct {
        min_regen: Some(5),
        ..form.clone()
    };
    let plan = presets::plan(&realm, &Preset::ItemDropPct(regen), Dos).unwrap();
    assert!(!plan.changes.iter().any(|c| n(&c.key[0]) == 7001.0));

    // only limited items, set to 30
    let only = DropPct {
        set: true,
        value: 30,
        limited: Limited::Only,
        ..form
    };
    let plan = presets::plan(&realm, &Preset::ItemDropPct(only.clone()), Dos).unwrap();
    let mine: Vec<_> = plan
        .changes
        .iter()
        .filter(|c| n(&c.key[0]) == 7001.0)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(
        (mine[0].column.as_str(), &mine[0].after),
        ("Item Drop % 2", &Value::Long(30))
    );
    assert!(
        presets::plan(
            &realm,
            &Preset::ItemDropPct(DropPct { value: 101, ..only }),
            Dos
        )
        .is_err()
    );
}
