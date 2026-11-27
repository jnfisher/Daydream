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

/// Fields set on the rooms picked on the map: those rooms alone, as one step.
#[tokio::test]
async fn a_set_on_picked_rooms_reaches_those_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    empty_realm(dir);
    for n in 1..=4 {
        add(
            dir,
            "Rooms",
            json!({"Map Number": 1, "Room Number": n, "Name": format!("Wood {n}"), "Light": 5}),
        )
        .await;
    }
    add(
        dir,
        "Rooms",
        json!({"Map Number": 2, "Room Number": 2, "Name": "Elsewhere", "Light": 5}),
    )
    .await;
    let body = json!({
        "map": 1,
        "rooms": [1, 3],
        "values": [["Light", -40], ["Mon Type", 6], ["Name", "Dark wood"]],
        "what": "Light, Monsters and Name",
    });
    let room = |m: i64, n: i64| format!("/api/tables/Rooms/row?key={}", key(json!([m, n])));

    let (status, out) = send(dir, "POST", "/api/map/set", Some(body.clone())).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["records"], 2, "{out}");
    let history = get(dir, "/api/history").await;
    assert_eq!(
        history["steps"][0]["label"], "Set Light, Monsters and Name on 2 rooms of map 1",
        "{history}"
    );
    for (m, n, set) in [
        (1, 1, true),
        (1, 2, false),
        (1, 3, true),
        (1, 4, false),
        (2, 2, false),
    ] {
        let r = get(dir, &room(m, n)).await;
        let v = &r["values"];
        assert_eq!(v["Light"], json!(if set { -40 } else { 5 }), "{m}/{n}: {r}");
        assert_eq!(
            v["Mon Type"],
            json!(if set { 6 } else { 0 }),
            "{m}/{n}: {r}"
        );
        assert_eq!(v["Name"] == json!("Dark wood"), set, "{m}/{n}: {r}");
    }

    // one room: its own label; and undo takes the set back
    let one = json!({"map": 1, "rooms": [2], "values": [["Light", 9]], "what": "Light"});
    let (status, _) = send(dir, "POST", "/api/map/set", Some(one)).await;
    assert_eq!(status, StatusCode::OK);
    let history = get(dir, "/api/history").await;
    assert_eq!(
        history["steps"][0]["label"], "Set Light on room 1/2",
        "{history}"
    );
    let (status, out) = send(dir, "POST", "/api/undo", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(get(dir, &room(1, 2)).await["values"]["Light"], json!(5));

    // the map's rooms carry what the bar shows of their monsters
    let view = get(dir, "/api/map?map=1").await;
    assert_eq!(view["rooms"][0]["mon_type"], 6, "{view}");
    for k in ["min_index", "max_index", "max_regen"] {
        assert_eq!(view["rooms"][0][k], 0, "{k}: {view}");
    }

    // refused: no rooms, a room not on the map, no fields, a key column, no map, a number
    // the record cannot hold
    for bad in [
        json!({"map": 1, "rooms": [], "values": [["Light", 1]]}),
        json!({"map": 1, "rooms": [9], "values": [["Light", 1]]}),
        json!({"map": 1, "rooms": [1], "values": []}),
        json!({"map": 1, "rooms": [1], "values": [["Room Number", 7]]}),
        json!({"map": 0, "rooms": [1], "values": [["Light", 1]]}),
        json!({"map": 1, "rooms": [1], "values": [["Light", 40000]]}),
    ] {
        let (status, out) = send(dir, "POST", "/api/map/set", Some(bad.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {out}");
    }
}
