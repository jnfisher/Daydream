//! What columns mean, and what points at a record.

use daydream_mcp::{Options, Server};
use serde_json::{Value, json};

use crate::support::{Client, empty_realm};

#[test]
fn describe_table_explains_the_columns() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let rooms = c.call("describe_table", json!({ "table": "Rooms" }));
    let column = |d: &Value, name: &str| {
        d["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(column(&rooms, "Type")["options"], "ROOM_TYPE");
    assert_eq!(column(&rooms, "Room Item 3")["refers_to"], "Items");
    assert_eq!(column(&rooms, "Exit 4")["refers_to"], "Rooms");
    assert_eq!(rooms["lists"]["ROOM_TYPE"]["2"], "Arena");
    assert!(
        rooms["notes"]["Max Index"]["note"]
            .as_str()
            .unwrap()
            .contains("highest player level")
    );
    // a family's note is given once
    assert!(rooms["notes"]["Exit #"].is_object());
    assert!(rooms["notes"].get("Exit 4").is_none());
    assert_eq!(
        rooms["exit_params"]["types"]["2 Key"]["Para1"]["links"],
        "Items"
    );
    assert!(rooms["note"].as_str().unwrap().contains("Mon Type"));

    let monsters = c.call("describe_table", json!({ "table": "Monsters" }));
    assert_eq!(column(&monsters, "Death Msg")["refers_to"], "Messages");
    assert_eq!(column(&monsters, "Ability 0")["ability"], true);
    assert_eq!(monsters["abilities"]["1"], "Damage");
    assert!(monsters.get("exit_params").is_none());
    let with = c.call(
        "describe_table",
        json!({ "table": "Monsters", "ability_notes": true }),
    );
    assert!(with["abilities"]["1"]["note"].is_string());
}

#[test]
fn references_find_what_names_a_record() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let p = c.call("preview_edits", json!({ "edits": [
        { "op": "insert", "table": "Messages", "values": { "Number": 9, "Line 3": "The newt dies." } },
        { "op": "insert", "table": "Items", "values": { "Number": 1, "Name": "iron key" } },
        { "op": "insert", "table": "Monsters", "values": { "Number": 1, "Name": "cave newt", "Group": 6, "Index": 4, "Death Msg": 9 } },
        { "op": "insert", "table": "Monsters", "values": { "Number": 2, "Name": "bat", "Group": 6, "Index": 40, "Attack Hit Msg 0": 9 } },
        { "op": "insert", "table": "Rooms", "values": {
            "Map Number": 1, "Room Number": 1, "Name": "Grotto",
            "Mon Type": 6, "Min Index": 1, "Max Index": 10, "Max Regen": 2,
            "Exit 0": 2, "Type 0": 2, "Para1 0": 1,
            "Room Item 0": 1, "Room Item 0 QTY": 1 } },
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 1, "Room Number": 2, "Name": "Ledge", "Exit 1": 1 } },
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 2, "Room Number": 5, "Name": "Elsewhere", "Exit 1": 2 } }
    ] }));
    c.call("apply_edits", json!({ "token": p["token"] }));
    let by = |v: &Value| -> Vec<String> {
        v["references"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| format!("{} {} {}", r["table"], r["key"], r["by"].as_str().unwrap()))
            .collect()
    };

    let item = c.call("find_references", json!({ "table": "Items", "key": [1] }));
    let item = by(&item);
    assert!(
        item.contains(&"\"Rooms\" [1,1] Room Item 0".to_string()),
        "{item:?}"
    );
    assert!(
        item.iter()
            .any(|r| r.contains("Para1 0 (Key: the key item)")),
        "{item:?}"
    );

    // the exit into room 2 is from map 1, not the one on map 2
    let room = by(&c.call(
        "find_references",
        json!({ "table": "Rooms", "key": [1, 2] }),
    ));
    assert_eq!(room, ["\"Rooms\" [1,1] Exit 0"]);

    let monster = by(&c.call(
        "find_references",
        json!({ "table": "Monsters", "key": [1] }),
    ));
    assert_eq!(monster.len(), 1, "{monster:?}");
    assert!(
        monster[0].starts_with("\"Rooms\" [1,1] spawns it"),
        "{monster:?}"
    );
    // out of the room's index range: nothing spawns it
    let bat = c.call(
        "find_references",
        json!({ "table": "Monsters", "key": [2] }),
    );
    assert!(by(&bat).is_empty());

    let msg = by(&c.call(
        "find_references",
        json!({ "table": "Messages", "key": [9] }),
    ));
    assert_eq!(msg.len(), 2, "{msg:?}");
    assert!(
        msg.contains(&"\"Monsters\" [1] Death Msg".to_string()),
        "{msg:?}"
    );

    assert!(
        c.fails("find_references", json!({ "table": "Items", "key": [77] }))
            .contains("not found")
    );

    let groups = c.call(
        "free_numbers",
        json!({ "table": "Monsters", "column": "Group", "from": 6, "to": 35, "count": 3 }),
    );
    assert_eq!(groups["free"], json!([7, 8, 9]));
}
