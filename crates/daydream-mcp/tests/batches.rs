//! Big pieces of work at a small cost: edits from a file, clones, short previews and
//! filtering on the server.

use daydream_mcp::{Options, Server};
use serde_json::json;

use crate::support::{Client, area_edits, empty_realm, records};

#[test]
fn edits_come_from_a_file_in_an_allowed_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let realm = tmp.path().join("realm");
    std::fs::create_dir(&realm).unwrap();
    empty_realm(&realm);
    let extra = tmp.path().join("agent");
    std::fs::create_dir(&extra).unwrap();
    let server = Server::new(
        &realm,
        Options {
            batch_dirs: vec![extra.clone()],
            ..Options::default()
        },
    );
    let mut c = Client::new(&server);
    let folder = c.call("game_status", json!({}))["batch_folder"]
        .as_str()
        .unwrap()
        .to_string();
    std::fs::write(
        std::path::Path::new(&folder).join("area.json"),
        json!({ "edits": area_edits(1, [1, 2], 1, 1), "changeset": "grotto" }).to_string(),
    )
    .unwrap();
    let p = c.call("preview_edits", json!({ "edits_file": "area.json" }));
    assert_eq!(p["changeset"], "grotto");
    assert_eq!(p["rows"]["Rooms"]["insert"], 2);
    assert!(p["edits_file"].as_str().unwrap().ends_with("area.json"));
    c.call("apply_edits", json!({ "token": p["token"] }));
    let h = c.call("history", json!({}));
    assert!(
        h["applied"][0]["edits_file"]
            .as_str()
            .unwrap()
            .ends_with("area.json")
    );

    // another allowed folder, by its full path; a bare list of edits
    let more = extra.join("more.json");
    std::fs::write(
        &more,
        json!([{ "op": "insert", "table": "Items", "values": { "Number": 2 } }]).to_string(),
    )
    .unwrap();
    let p = c.call(
        "preview_edits",
        json!({ "edits_file": more.display().to_string(), "changeset": "more" }),
    );
    assert_eq!(p["changeset"], "more");

    // anywhere else, or not JSON, or with edits too: refused
    let outside = tmp.path().join("outside.json");
    std::fs::write(&outside, "[]").unwrap();
    let e = c.fails(
        "preview_edits",
        json!({ "edits_file": outside.display().to_string() }),
    );
    assert!(e.contains("not in a folder"), "{e}");
    let e = c.fails(
        "preview_edits",
        json!({ "edits_file": "../../../outside.json" }),
    );
    assert!(e.contains("without .."), "{e}");
    assert!(
        c.fails("preview_edits", json!({ "edits_file": "area.txt" }))
            .contains(".json")
    );
    assert!(
        c.fails(
            "preview_edits",
            json!({ "edits_file": "area.json", "edits": area_edits(1, [5, 6], 5, 5) })
        )
        .contains("not both")
    );
}

#[test]
fn previews_are_as_big_as_the_change() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);

    let p = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 3, "Name": "rope" } }] }),
    );
    assert_eq!(p["detail"], "changes");
    let changes = p["edits"][0]["changes"].as_array().unwrap();
    let columns: Vec<&str> = changes
        .iter()
        .map(|ch| ch["column"].as_str().unwrap())
        .collect();
    assert!(
        columns.contains(&"Name") && columns.contains(&"Number"),
        "{columns:?}"
    );
    assert!(changes.len() < 5, "{columns:?}");
    assert!(p["edits"][0]["blank_columns"].as_u64().unwrap() > 20);
    let full = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 3 } }], "detail": "full" }),
    );
    assert!(full["edits"][0]["changes"].as_array().unwrap().len() > 20);

    // a big batch is summarized unless asked
    let many: Vec<_> = (1..=60)
        .map(|n| json!({ "op": "insert", "table": "Rooms", "values": { "Map Number": 1, "Room Number": n, "Name": format!("Hall {n}"), "Desc 0": "Dusty.", "Exit 0": n + 1, "Exit 1": n - 1, "Light": 5, "Type": 0, "Gold": 1 } }))
        .collect();
    let p = c.call("preview_edits", json!({ "edits": many }));
    assert_eq!(p["detail"], "summary");
    assert_eq!(p["rows"]["Rooms"]["insert"], 60);
    assert_eq!(p["first"].as_array().unwrap().len(), 3);
    assert!(p.get("edits").is_none());
    let all = c.call(
        "preview_edits",
        json!({ "edits": many, "detail": "changes" }),
    );
    assert_eq!(all["edits"].as_array().unwrap().len(), 60);
}

#[test]
fn a_clone_copies_a_row_with_changes() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let p = c.call(
        "preview_edits",
        json!({ "edits": area_edits(1, [1, 2], 1, 1) }),
    );
    c.call("apply_edits", json!({ "token": p["token"] }));
    let before = records(tmp.path());

    let p = c.call(
        "preview_edits",
        json!({ "edits": [
            { "op": "clone", "table": "Rooms", "from": [1, 1], "values": { "Map Number": 1, "Room Number": 3, "name": "Damp Grotto" } },
            { "op": "clone", "table": "Items", "from": [1], "values": { "Number": 2 } }
        ] }),
    );
    assert_eq!(p["rows"]["Rooms"]["insert"], 1);
    c.call("apply_edits", json!({ "token": p["token"] }));
    let room = c.call("get_row", json!({ "table": "Rooms", "key": [1, 3] }));
    assert_eq!(room["values"]["Name"], "Damp Grotto");
    assert_eq!(room["values"]["Desc 0"], "Water drips from a low ceiling.");
    assert_eq!(room["values"]["Exit 0"], 2);
    let item = c.call("get_row", json!({ "table": "Items", "key": [2] }));
    assert_eq!(item["values"]["Name"], "glowing moss");
    // the journal keeps the inserts the clones stood for
    let h = c.call("history", json!({ "edits": true }));
    assert_eq!(h["applied"][0]["edits"][0]["op"], "insert");
    c.call("undo", json!({ "token": p["token"] }));
    assert_eq!(records(tmp.path()), before);

    let e = c.fails(
        "preview_edits",
        json!({ "edits": [{ "op": "clone", "table": "Items", "from": [1], "values": { "Name": "x" } }] }),
    );
    assert!(e.contains("needs the new row's Number"), "{e}");
    let e = c.fails(
        "preview_edits",
        json!({ "edits": [{ "op": "clone", "table": "Items", "from": [99], "values": { "Number": 5 } }] }),
    );
    assert!(e.contains("not found"), "{e}");
    let e = c.fails(
        "preview_edits",
        json!({ "edits": [{ "op": "clone", "table": "Items", "from": [1], "values": { "Number": 5, "Colour": 1 } }] }),
    );
    assert!(e.contains("no column Colour"), "{e}");
}

#[test]
fn rows_are_found_on_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let p = c.call("preview_edits", json!({ "edits": [
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 1, "Room Number": 1, "Name": "Den", "Type": 3, "Mon Type": 6, "Min Index": 4, "Max Index": 4, "Room Item 2": 7, "Room Item 2 QTY": 1 } },
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 1, "Room Number": 2, "Name": "Hall", "Room Item 0": 5, "Room Item 0 QTY": 7, "Exit 0": 1 } },
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 1, "Room Number": 3, "Name": "Arena", "Type": 2, "Max Index": 10 } }
    ] }));
    c.call("apply_edits", json!({ "token": p["token"] }));

    let keys = |v: &serde_json::Value| -> Vec<serde_json::Value> {
        v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["key"][1].clone())
            .collect()
    };
    let lairs = c.call(
        "list_rows",
        json!({ "table": "Rooms", "where": [{ "column": "Type", "value": 3 }], "columns": ["Min Index"] }),
    );
    assert_eq!(keys(&lairs), [json!(1)]);
    assert_eq!(lairs["rows"][0]["Min Index"], 4);
    // the room holding item 7, not the one holding 7 of item 5
    let holding = c.call(
        "list_rows",
        json!({ "table": "Rooms", "where": [{ "column": "Room Item *", "value": 7 }] }),
    );
    assert_eq!(keys(&holding), [json!(1)]);
    let typed = c.call(
        "list_rows",
        json!({ "table": "Rooms", "where": [
            { "column": "Type", "op": "in", "value": [2, 3] },
            { "column": "Max Index", "op": "between", "value": [5, 20] }
        ] }),
    );
    assert_eq!(keys(&typed), [json!(3)]);
    let named = c.call(
        "list_rows",
        json!({ "table": "Rooms", "where": [{ "column": "Name", "value": "hall" }] }),
    );
    assert_eq!(keys(&named), [json!(2)]);
    let no_exit = c.call(
        "list_rows",
        json!({ "table": "Rooms", "where": [{ "column": "Exit *", "op": "!=", "value": 1 }] }),
    );
    assert_eq!(keys(&no_exit), [json!(1), json!(3)]);
    assert!(
        c.fails(
            "list_rows",
            json!({ "table": "Rooms", "where": [{ "column": "Colour", "value": 1 }] })
        )
        .contains("no column")
    );
    assert!(
        c.fails(
            "list_rows",
            json!({ "table": "Rooms", "where": [{ "column": "Type", "op": "~", "value": 1 }] })
        )
        .contains("no operator")
    );

    let map = c.call(
        "get_map",
        json!({ "map": 1, "from": 2, "to": 3, "columns": ["Type", "Max Index"] }),
    );
    let rooms = map["rooms"].as_array().unwrap();
    assert_eq!(rooms.len(), 2);
    assert_eq!(rooms[1]["Type"], 2);
    assert_eq!(rooms[1]["Max Index"], 10);
}
