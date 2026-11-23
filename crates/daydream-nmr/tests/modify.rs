//! The Universal Modifier over a realm of random records.

use daydream_nmr::modify::{self, Action, Cmp, Condition, Modify, Op};
use daydream_nmr::{ExportOptions, NmrRows, Range, Value, export};
use daydream_store::Realm;
use daydream_types::records::LayoutSet::Dos;

use crate::support::random_realm;

fn items(realm: &Realm) -> NmrRows {
    let data = export(
        realm,
        &ExportOptions {
            zero_runtime: false,
            ..ExportOptions::default()
        },
    )
    .unwrap();
    data.table("Items").unwrap().clone()
}

fn n(v: &Value) -> f64 {
    match v {
        Value::Long(x) => f64::from(*x),
        Value::Double(x) => *x,
        _ => panic!("{v:?}"),
    }
}

#[test]
fn math_in_range_and_only_if() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 30, 21);
    let realm = Realm::open(dir.path()).unwrap();
    let before = items(&realm);
    let (num, weight, ty) = (
        before.position("Number").unwrap(),
        before.position("Weight").unwrap(),
        before.position("Type").unwrap(),
    );
    let numbers: Vec<i64> = before.rows.iter().map(|r| n(&r[num]) as i64).collect();
    let pick_type = n(&before.rows[12][ty]);
    let m = Modify {
        table: "Items".into(),
        action: Action::Math {
            columns: vec!["Weight".into()],
            op: Op::Set,
            value: 77.0,
            limit: None,
        },
        range: Some(Range {
            table: "Items".into(),
            map: None,
            from: numbers[10],
            to: numbers[19],
        }),
        only_if: vec![Condition::Column {
            column: "Type".into(),
            cmp: Cmp::Eq,
            value: pick_type,
        }],
    };
    let plan = modify::plan(&realm, &m, Dos).unwrap();
    let expected: Vec<i64> = before.rows[10..20]
        .iter()
        .filter(|r| n(&r[ty]) == pick_type && n(&r[weight]) != 77.0)
        .map(|r| n(&r[num]) as i64)
        .collect();
    assert!(!expected.is_empty());
    let planned: Vec<i64> = plan.changes.iter().map(|c| n(&c.key[0]) as i64).collect();
    assert_eq!(planned, expected);
    assert!(
        plan.changes
            .iter()
            .all(|c| c.column == "Weight" && c.after == Value::Long(77))
    );
    // nothing written yet
    assert_eq!(items(&realm), before);

    let report = modify::apply(&realm, &plan, Dos).unwrap();
    assert_eq!(report.updated, expected.len(), "{report:?}");
    let after = items(&realm);
    for (a, b) in before.rows.iter().zip(&after.rows) {
        let changed = expected.contains(&(n(&a[num]) as i64));
        assert_eq!(
            b[weight] == Value::Long(77),
            changed || a[weight] == Value::Long(77)
        );
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            if i != weight {
                assert_eq!(x, y);
            }
        }
    }
}

#[test]
fn abilities_are_given_taken_and_changed() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 8, 22);
    let realm = Realm::open(dir.path()).unwrap();
    let slots = |rows: &NmrRows, r: usize| -> Vec<(i64, i64)> {
        (0..20)
            .map(|i| {
                let a = rows.position(&format!("Ability {i}")).unwrap();
                let v = rows.position(&format!("Ability Value {i}")).unwrap();
                (n(&rows.rows[r][a]) as i64, n(&rows.rows[r][v]) as i64)
            })
            .collect()
    };
    // clear every slot first, so the test knows where things go
    for i in 0..20 {
        let take_all = Modify {
            table: "Items".into(),
            action: Action::Math {
                columns: vec![format!("Ability {i}"), format!("Ability Value {i}")],
                op: Op::Set,
                value: 0.0,
                limit: None,
            },
            range: None,
            only_if: vec![],
        };
        let p = modify::plan(&realm, &take_all, Dos).unwrap();
        modify::apply(&realm, &p, Dos).unwrap();
    }
    let run = |action: Action| {
        let p = modify::plan(
            &realm,
            &Modify {
                table: "Items".into(),
                action,
                range: None,
                only_if: vec![],
            },
            Dos,
        )
        .unwrap();
        modify::apply(&realm, &p, Dos).unwrap();
        p
    };
    run(Action::GiveAbility {
        ability: 135,
        value: 5,
    });
    run(Action::GiveAbility {
        ability: 135,
        value: 5,
    }); // already there: nothing
    run(Action::GiveAbility {
        ability: 22,
        value: 3,
    });
    let rows = items(&realm);
    assert_eq!(&slots(&rows, 0)[..3], &[(135, 5), (22, 3), (0, 0)]);

    let p = run(Action::ChangeAbility {
        ability: 22,
        op: Op::Mul,
        value: 3.0,
        limit: Some((Cmp::Le, 8.0)),
    });
    assert_eq!(p.changes.len(), 8);
    assert_eq!(&slots(&items(&realm), 0)[..2], &[(135, 5), (22, 8)]);

    // only the items with ability 22 at 8 or more lose ability 135, and its value too
    let m = Modify {
        table: "Items".into(),
        action: Action::TakeAbility {
            ability: 135,
            clear_value: true,
        },
        range: None,
        only_if: vec![Condition::HasAbility {
            ability: 22,
            cmp: Cmp::Ge,
            value: 8.0,
        }],
    };
    modify::apply(&realm, &modify::plan(&realm, &m, Dos).unwrap(), Dos).unwrap();
    assert_eq!(&slots(&items(&realm), 0)[..2], &[(0, 0), (22, 8)]);
}

#[test]
fn bad_runs_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), 2, 23);
    let realm = Realm::open(dir.path()).unwrap();
    let math = |columns: &[&str], op| Modify {
        table: "Items".into(),
        action: Action::Math {
            columns: columns.iter().map(|c| c.to_string()).collect(),
            op,
            value: 0.0,
            limit: None,
        },
        range: None,
        only_if: vec![],
    };
    assert!(modify::plan(&realm, &math(&["Weight"], Op::Div), Dos).is_err());
    assert!(modify::plan(&realm, &math(&["Nope"], Op::Set), Dos).is_err());
    assert!(modify::plan(&realm, &math(&["Name"], Op::Set), Dos).is_err());
    let give = Modify {
        table: "Messages".into(),
        action: Action::GiveAbility {
            ability: 1,
            value: 1,
        },
        range: None,
        only_if: vec![],
    };
    assert!(modify::plan(&realm, &give, Dos).is_err());
}

#[test]
fn set_fills_in_rooms_like_the_mass_room_editor() {
    use crate::support::{empty_realm, l, put, t};
    let dir = tempfile::tempdir().unwrap();
    empty_realm(dir.path());
    let realm = Realm::open(dir.path()).unwrap();
    let room = |map, n, name: &str| {
        vec![
            ("Map Number", l(map)),
            ("Room Number", l(n)),
            ("Name", t(name)),
            ("Light", l(0)),
        ]
    };
    let rows = [
        room(1, 1, "a"),
        room(1, 2, "b"),
        room(1, 3, "Cave"),
        room(1, 4, "d"),
        room(2, 2, "e"),
    ];
    let refs: Vec<&[(&str, Value)]> = rows.iter().map(|r| r.as_slice()).collect();
    put(&realm, "Rooms", &refs);
    let m = Modify {
        table: "Rooms".into(),
        action: Action::Set {
            values: vec![
                ("Name".into(), Value::Text("Cave".into())),
                ("Light".into(), Value::Long(-200)),
            ],
        },
        range: Some(Range {
            table: "Rooms".into(),
            map: Some(1),
            from: 2,
            to: 3,
        }),
        only_if: vec![],
    };
    let plan = modify::plan(&realm, &m, Dos).unwrap();
    // room 3 already has the name: only its light changes
    let got: Vec<_> = plan
        .changes
        .iter()
        .map(|c| (n(&c.key[1]), c.column.as_str()))
        .collect();
    assert_eq!(got, [(2.0, "Name"), (2.0, "Light"), (3.0, "Light")]);
    modify::apply(&realm, &plan, Dos).unwrap();
    let again = modify::plan(&realm, &m, Dos).unwrap();
    assert!(again.changes.is_empty(), "{again:?}");

    let key = Modify {
        action: Action::Set {
            values: vec![("Room Number".into(), Value::Long(9))],
        },
        ..m.clone()
    };
    assert!(modify::plan(&realm, &key, Dos).is_err());
    let text_in_number = Modify {
        action: Action::Set {
            values: vec![("Light".into(), Value::Text("dim".into()))],
        },
        ..m
    };
    assert!(modify::plan(&realm, &text_in_number, Dos).is_err());
}
