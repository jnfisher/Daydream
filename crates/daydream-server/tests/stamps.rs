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

/// A join dragged across the map answers its step, so the page can take the places it keeps
/// back with it; a dry run makes none.
#[tokio::test]
async fn a_join_answers_its_step() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    empty_realm(dir);
    for n in 1..=2 {
        add(
            dir,
            "Rooms",
            json!({"Map Number": 1, "Room Number": n, "Name": format!("Wood {n}")}),
        )
        .await;
    }
    let body = |dry_run: bool| json!({"map": 1, "room": 1, "dir": 2, "to_map": 1, "to_room": 2, "existing": true, "dry_run": dry_run});
    let (status, out) = send(dir, "POST", "/api/map/dig", Some(body(true))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["step"], json!(null), "{out}");
    let (status, out) = send(dir, "POST", "/api/map/dig", Some(body(false))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["step"], 3, "{out}");
    let history = get(dir, "/api/history").await;
    assert_eq!(history["steps"][0]["step"], 3, "{history}");
    assert_eq!(
        history["steps"][0]["label"], "Join 1/1 east to 1/2",
        "{history}"
    );
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

#[tokio::test]
async fn stamps_saved_from_the_map_are_kept_beside_the_working_copy() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    empty_realm(dir);
    let body = get(dir, "/api/map/stamps").await;
    assert_eq!(
        body,
        json!({"daydream": "stamps", "version": 1, "next": 1, "stamps": []})
    );

    // a new stamp takes the next id, its name trimmed
    let hall = json!({
        "name": "  Great hall ",
        "map": 1,
        "room": 40,
        "cells": [[0, 0], [1, 0], [1, -1]],
        "links": [{"a": 0, "dir": 2, "b": 1}, {"a": 1, "dir": 0, "b": 2, "one_way": true}],
    });
    let (status, out) = send(dir, "POST", "/api/map/stamps", Some(hall.clone())).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["stamp"]["id"], "s1");
    assert_eq!(out["stamp"]["name"], "Great hall");
    assert_eq!(
        (out["stamps"].clone(), out["step"].clone()),
        (json!(1), json!(0))
    );
    let yard = json!({"name": "Yard", "cells": [[0, 0]]});
    let (_, out) = send(dir, "POST", "/api/map/stamps", Some(yard)).await;
    assert_eq!(out["stamp"]["id"], "s2", "{out}");

    // kept as given: a two-way link has no one_way, and a stamp from no map no map
    let body = get(dir, "/api/map/stamps").await;
    assert_eq!(
        body["stamps"],
        json!([
            {"id": "s1", "name": "Great hall", "map": 1, "room": 40, "cells": [[0, 0], [1, 0], [1, -1]],
             "links": [{"a": 0, "dir": 2, "b": 1}, {"a": 1, "dir": 0, "b": 2, "one_way": true}]},
            {"id": "s2", "name": "Yard", "cells": [[0, 0]], "links": []},
        ])
    );

    // one with an id replaces that one, where it is
    let mut renamed = body["stamps"][0].clone();
    renamed["name"] = json!("Hall");
    let (status, out) = send(dir, "POST", "/api/map/stamps", Some(renamed.clone())).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["stamps"], 2);
    let body = get(dir, "/api/map/stamps").await;
    assert_eq!(body["stamps"][0]["name"], "Hall");
    assert_eq!(body["stamps"][1]["id"], "s2");

    // deleted, and brought back (an undo) where it was; a new one never takes its id
    let (status, out) = send(dir, "DELETE", "/api/map/stamps/s1", None).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out, json!({"stamps": 1, "step": 0}));
    let (status, _) = send(dir, "DELETE", "/api/map/stamps/s1", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, out) = send(dir, "POST", "/api/map/stamps", Some(renamed)).await;
    assert_eq!(out["stamp"]["id"], "s1", "{out}");
    let (_, out) = send(dir, "POST", "/api/map/stamps", Some(hall)).await;
    assert_eq!(out["stamp"]["id"], "s3", "{out}");
    let body = get(dir, "/api/map/stamps").await;
    let ids: Vec<_> = body["stamps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["s1", "s2", "s3"]);
    assert_eq!(body["next"], 4);

    // beside the realm, and no save to it: no step of the history, no change to publish
    assert!(dir.join(".daydream").join("stamps.json").is_file());
    let history = get(dir, "/api/history").await;
    assert_eq!(history["steps"], json!([]), "{history}");
    let changes = get(dir, "/api/changes").await;
    assert_eq!(changes["changes"], json!([]), "{changes}");
    // and the realm's newest step comes back, so the page knows whether this came last
    add(
        dir,
        "Rooms",
        json!({"Map Number": 1, "Room Number": 1, "Name": "Hall"}),
    )
    .await;
    let (_, out) = send(dir, "DELETE", "/api/map/stamps/s3", None).await;
    assert_eq!(out["step"], 1, "{out}");

    let shape = |cells: serde_json::Value, links: serde_json::Value| json!({"name": "Bad", "cells": cells, "links": links});
    let too_many: Vec<[i32; 2]> = (0..2001).map(|x| [x, 0]).collect();
    for (bad, why) in [
        (json!({"name": " ", "cells": [[0, 0]]}), "needs a name"),
        (
            json!({"name": "x".repeat(41), "cells": [[0, 0]]}),
            "at most 40",
        ),
        (json!({"name": "a\nb", "cells": [[0, 0]]}), "one line"),
        (
            json!({"name": "A", "id": "x1", "cells": [[0, 0]]}),
            "gives a stamp its id",
        ),
        (
            json!({"name": "A", "map": 0, "cells": [[0, 0]]}),
            "no map 0",
        ),
        (
            json!({"name": "A", "map": 1, "room": -2, "cells": [[0, 0]]}),
            "no room -2",
        ),
        (shape(json!([]), json!([])), "at least one room"),
        (shape(json!([[1, 0]]), json!([])), "first cell"),
        (shape(json!(too_many), json!([])), "at most 2000 rooms"),
        (shape(json!([[0, 0], [300000, 0]]), json!([])), "too far"),
        (
            shape(json!([[0, 0], [0, 0]]), json!([])),
            "two rooms in one cell",
        ),
        (
            shape(json!([[0, 0]]), json!([{"a": 0, "dir": 2, "b": 1}])),
            "leads to no cell",
        ),
        (
            shape(json!([[0, 0], [1, 0]]), json!([{"a": 1, "dir": 2, "b": 1}])),
            "its own cell",
        ),
        (
            shape(
                json!([[0, 0], [1, 0]]),
                json!([{"a": 0, "dir": 10, "b": 1}]),
            ),
            "0 to 9",
        ),
        (
            // the way back of the first is the way there of the second
            shape(
                json!([[0, 0], [1, 0], [2, 0]]),
                json!([{"a": 0, "dir": 2, "b": 1}, {"a": 1, "dir": 3, "b": 2, "one_way": true}]),
            ),
            "two exits the same way",
        ),
    ] {
        let (status, out) = send(dir, "POST", "/api/map/stamps", Some(bad.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {out}");
        assert!(out["error"].as_str().unwrap().contains(why), "{bad}: {out}");
    }

    // so many saved, no more: one has to go first
    let mut file = get(dir, "/api/map/stamps").await;
    let one = file["stamps"][0].clone();
    file["stamps"] = json!(
        (1..=200)
            .map(|n| {
                let mut s = one.clone();
                s["id"] = json!(format!("s{n}"));
                s
            })
            .collect::<Vec<_>>()
    );
    std::fs::write(dir.join(".daydream").join("stamps.json"), file.to_string()).unwrap();
    let (status, out) = send(
        dir,
        "POST",
        "/api/map/stamps",
        Some(json!({"name": "One more", "cells": [[0, 0]]})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{out}");
    assert!(
        out["error"].as_str().unwrap().contains("delete one"),
        "{out}"
    );
    // the file's next is past every id in it, even one written by hand
    let body = get(dir, "/api/map/stamps").await;
    assert_eq!(body["next"], 201);
}
