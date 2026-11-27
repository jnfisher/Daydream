//! A stamp from the World map: its rooms and exits put down as one step of the history,
//! joined to the map's rooms only through free slots.

use axum::http::StatusCode;
use serde_json::json;

mod support;
use support::{add, empty_realm, get, key, send};

#[tokio::test]
async fn a_stamp_is_one_step_and_leaves_the_exits_in_its_way() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    empty_realm(dir);
    add(
        dir,
        "Rooms",
        json!({"Map Number": 1, "Room Number": 1, "Name": "Hall", "Light": 5, "Gold": 9, "Exit 2": 2}),
    )
    .await;
    add(
        dir,
        "Rooms",
        json!({"Map Number": 1, "Room Number": 2, "Name": "Yard", "Exit 3": 1, "Exit 0": 9}),
    )
    .await;
    let body = |dry_run: bool| {
        json!({
            "map": 1,
            "brush": 1,
            "rooms": [10, 11],
            "links": [
                {"from": 10, "dir": 2, "to": 11},
                {"from": 2, "dir": 1, "to": 10},
                {"from": 2, "dir": 0, "to": 11},
            ],
            "what": "a corridor",
            "dry_run": dry_run,
        })
    };
    let room = |n: i64| format!("/api/tables/Rooms/row?key={}", key(json!([1, n])));

    // a dry run says what it would do, and writes nothing
    let (status, out) = send(dir, "POST", "/api/map/stamp", Some(body(true))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(
        out["stamp"],
        json!({"rooms": 2, "exits": 4, "joined": 1, "left": 1})
    );
    assert_eq!(out["step"], json!(null));
    assert_eq!(out["notes"].as_array().unwrap().len(), 1, "{out}");
    let (status, _) = send(dir, "GET", &room(10), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // written: one step of the history (after the two that added the rooms), which it answers
    let (status, out) = send(dir, "POST", "/api/map/stamp", Some(body(false))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["step"], 3, "{out}");
    let history = get(dir, "/api/history").await;
    assert_eq!(history["steps"][0]["step"], 3, "{history}");
    assert_eq!(
        history["steps"][0]["label"], "Stamp a corridor: 2 rooms on map 1",
        "{history}"
    );
    assert_eq!(history["steps"][0]["records"], 3, "{history}");
    let made = get(dir, &room(10)).await;
    assert_eq!(
        (
            made["values"]["Exit 2"].clone(),
            made["values"]["Exit 0"].clone(),
            made["values"]["Light"].clone(),
            made["values"]["Gold"].clone(),
        ),
        (json!(11), json!(2), json!(5), json!(0))
    );
    let yard = get(dir, &room(2)).await;
    assert_eq!(
        (
            yard["values"]["Exit 1"].clone(),
            yard["values"]["Exit 0"].clone()
        ),
        (json!(10), json!(9))
    );

    // undo takes the rooms and the yard's new exit back together
    let (status, out) = send(dir, "POST", "/api/undo", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    let (status, _) = send(dir, "GET", &room(11), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(get(dir, &room(2)).await["values"]["Exit 1"], 0);

    // refused: a new room on a number in use, a brush the map does not have
    for bad in [
        json!({"map": 1, "brush": 1, "rooms": [2]}),
        json!({"map": 1, "brush": 7, "rooms": [20]}),
        json!({"map": 1, "brush": 1}),
    ] {
        let (status, out) = send(dir, "POST", "/api/map/stamp", Some(bad.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {out}");
    }
}
