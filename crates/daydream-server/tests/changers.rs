//! NMR's changers as the Workshop runs them: the Monster Group/Index Changer over the monsters
//! and the rooms that spawn them, and the Record Number Changer, each one step of the history.

use axum::http::StatusCode;
use serde_json::{Value, json};

mod support;
use support::{add, empty_realm, get, key, send};

/// Each table a run answers for, with how many of its records change.
fn tables(out: &Value) -> Vec<(Value, Value)> {
    out["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["table"].clone(), t["records"].clone()))
        .collect()
}

#[tokio::test]
async fn a_group_and_index_moves_with_the_rooms_that_spawn_it_in_one_step() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    empty_realm(dir);
    add(
        dir,
        "Monsters",
        json!({"Number": 1, "Name": "rat", "Group": 2, "Index": 5}),
    )
    .await;
    add(
        dir,
        "Monsters",
        json!({"Number": 2, "Name": "bat", "Group": 2, "Index": 6}),
    )
    .await;
    add(
        dir,
        "Rooms",
        json!({"Map Number": 1, "Room Number": 1, "Name": "Sewer", "Mon Type": 2, "Min Index": 5, "Max Index": 6}),
    )
    .await;
    let body = |dry_run: bool| {
        json!({
            "changes": [
                {"from_group": 2, "from_index": 5, "to_group": 3, "to_index": 10},
                {"from_group": 2, "from_index": 6, "to_group": 3, "to_index": 11},
            ],
            "dry_run": dry_run,
        })
    };

    // a preview: the monsters, then the rooms, and nothing written
    let (status, out) = send(dir, "POST", "/api/tools/monster-indexes", Some(body(true))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["step"], json!(null));
    assert_eq!(
        tables(&out),
        [(json!("Monsters"), json!(2)), (json!("Rooms"), json!(1))]
    );

    // written: both tables in one step of the history, after the three that added the records
    let (status, out) = send(dir, "POST", "/api/tools/monster-indexes", Some(body(false))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["step"], 4, "{out}");
    let history = get(dir, "/api/history").await;
    assert_eq!(
        history["steps"][0]["label"], "Move monster groups and indexes",
        "{history}"
    );
    assert_eq!(history["steps"][0]["records"], 3, "{history}");
    let room = get(
        dir,
        &format!("/api/tables/Rooms/row?key={}", key(json!([1, 1]))),
    )
    .await;
    let v = &room["values"];
    assert_eq!(
        (
            v["Mon Type"].clone(),
            v["Min Index"].clone(),
            v["Max Index"].clone()
        ),
        (json!(3), json!(10), json!(11))
    );

    // an empty list is refused
    let (status, out) = send(
        dir,
        "POST",
        "/api/tools/monster-indexes",
        Some(json!({"changes": [], "dry_run": true})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{out}");
}

#[tokio::test]
async fn a_renumber_takes_the_name_it_is_given_in_the_history() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    empty_realm(dir);
    add(dir, "Items", json!({"Number": 7, "Name": "sword"})).await;
    add(
        dir,
        "Monsters",
        json!({"Number": 1, "Name": "orc", "Weapon Number": 7}),
    )
    .await;
    let (status, out) = send(
        dir,
        "POST",
        "/api/tools/renumber",
        Some(json!({
            "kind": "records",
            "table": "items",
            "moves": [{"from": 7, "to": 40}],
            "label": "Renumber item 7 to 40",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["step"], 3, "{out}");
    assert_eq!(
        tables(&out),
        [(json!("Monsters"), json!(1)), (json!("Items"), json!(1))]
    );
    let history = get(dir, "/api/history").await;
    assert_eq!(
        history["steps"][0]["label"], "Renumber item 7 to 40",
        "{history}"
    );
    let orc = get(
        dir,
        &format!("/api/tables/Monsters/row?key={}", key(json!([1]))),
    )
    .await;
    assert_eq!(orc["values"]["Weapon Number"], 40, "{orc}");
}
