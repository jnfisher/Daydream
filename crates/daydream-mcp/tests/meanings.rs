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
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 2, "Room Number": 5, "Name": "Elsewhere", "Exit 1": 2 } },
        // group 6 rooms that spawn nothing: no Max Index (a pick from the group needs one),
        // and a jail, which the spawn passes leave alone
        { "op": "insert", "table": "Rooms", "values": {
            "Map Number": 2, "Room Number": 6, "Name": "Pit", "Mon Type": 6, "Max Regen": 3 } },
        { "op": "insert", "table": "Rooms", "values": {
            "Map Number": 2, "Room Number": 7, "Name": "Cell", "Type": 6,
            "Mon Type": 6, "Min Index": 1, "Max Index": 10, "Max Regen": 3 } },
        { "op": "insert", "table": "Rooms", "values": {
            "Map Number": 2, "Room Number": 8, "Name": "Grotto",
            "Mon Type": 6, "Min Index": 1, "Max Index": 10, "Max Regen": 1 } }
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
    assert_eq!(monster.len(), 2, "{monster:?}");
    assert!(
        monster[0].starts_with("\"Rooms\" [1,1] spawns it"),
        "{monster:?}"
    );
    assert!(
        monster[1].starts_with("\"Rooms\" [2,8] spawns it"),
        "{monster:?}"
    );
    // out of the room's index range: nothing spawns it
    let bat = c.call(
        "find_references",
        json!({ "table": "Monsters", "key": [2] }),
    );
    assert!(by(&bat).is_empty());

    // the group's ladder: its monsters by index, its spawning rooms by range
    let realm = daydream_store::Realm::open(tmp.path()).unwrap();
    let mut tables = daydream_nmr::refs::Tables::new(&realm);
    let ladder = daydream_nmr::refs::spawn_ladder(&mut tables, 6).unwrap();
    assert_eq!(
        ladder.monsters,
        [(1, "cave newt".to_string(), 4), (2, "bat".to_string(), 40)]
    );
    assert_eq!(ladder.ranges.len(), 1, "{:?}", ladder.ranges);
    let r = &ladder.ranges[0];
    assert_eq!((r.min, r.max, r.rooms), (1, 10, 2));
    assert_eq!(r.maps, [(1, 1), (2, 1)]);
    assert_eq!(r.names, [("Grotto".to_string(), 2)]);
    assert_eq!(ladder.idle, 2);

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

#[test]
fn previews_warn_of_what_would_point_at_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let p = c.call("preview_edits", json!({ "edits": [
        { "op": "insert", "table": "Messages", "values": { "Number": 9, "Line 3": "The newt dies." } },
        { "op": "insert", "table": "Items", "values": { "Number": 1, "Name": "iron key" } },
        { "op": "insert", "table": "Monsters", "values": { "Number": 1, "Name": "cave newt", "Death Msg": 9, "Weapon Number": 1 } },
        { "op": "insert", "table": "Monsters", "values": { "Number": 2, "Name": "bat", "Weapon Number": 1 } },
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 1, "Room Number": 1, "Name": "Grotto", "Exit 0": 2 } },
        { "op": "insert", "table": "Rooms", "values": { "Map Number": 1, "Room Number": 2, "Name": "Ledge", "Exit 1": 1 } }
    ] }));
    assert!(p.get("warnings").is_none(), "{p}");
    c.call("apply_edits", json!({ "token": p["token"] }));
    let warnings = |c: &mut Client, edits: Value| -> Vec<String> {
        let p = c.call("preview_edits", json!({ "edits": edits }));
        p["warnings"]
            .as_array()
            .map(|w| w.iter().map(|s| s.as_str().unwrap().to_string()).collect())
            .unwrap_or_default()
    };

    // values that name nothing, counting what the batch itself adds
    let w = warnings(
        &mut c,
        json!([
            { "op": "update", "table": "Monsters", "key": [1], "values": { "Death Msg": 99 } },
            { "op": "insert", "table": "Items", "values": { "Number": 2, "Name": "net", "Hit Msg": 10, "Miss Msg": 11 } },
            { "op": "insert", "table": "Messages", "values": { "Number": 10, "Line 1": "Caught!" } },
            { "op": "update", "table": "Rooms", "key": [1, 1], "values": {
                "Exit 1": 7,
                "Exit 2": 1, "Type 2": 8, "Para1 2": 2,
                "Exit 3": 2, "Type 3": 2, "Para1 3": 50 } }
        ]),
    );
    assert_eq!(
        w,
        [
            "Monsters [1]: Death Msg names Messages 99, which the realm does not have",
            "Items [2]: Miss Msg names Messages 11, which the realm does not have",
            "Rooms [1,1]: Exit 1 names room 1/7, which the realm does not have",
            "Rooms [1,1]: Exit 2 names room 2/1, which the realm does not have",
            "Rooms [1,1]: Para1 3 (Key: the key item) names Items 50, which the realm does not have",
        ],
        "{w:?}"
    );

    // a value the edit leaves alone is not the edit's to warn of
    let p = c.call(
        "preview_edits",
        json!({ "edits": [
        { "op": "update", "table": "Monsters", "key": [1], "values": { "Death Msg": 99 } }
    ] }),
    );
    c.call("apply_edits", json!({ "token": p["token"] }));
    assert!(
        warnings(
            &mut c,
            json!([
                { "op": "update", "table": "Monsters", "key": [1], "values": { "Name": "newt" } }
            ])
        )
        .is_empty()
    );

    // deleting what rows still name; a row the batch points elsewhere, or deletes, does not
    let w = warnings(
        &mut c,
        json!([{ "op": "delete", "table": "Items", "key": [1] }]),
    );
    assert_eq!(
        w,
        [
            "deleting Items [1] leaves 2 rows naming it: Monsters [1] Weapon Number, Monsters [2] Weapon Number"
        ],
        "{w:?}"
    );
    let w = warnings(
        &mut c,
        json!([
            { "op": "delete", "table": "Items", "key": [1] },
            { "op": "update", "table": "Monsters", "key": [1], "values": { "Weapon Number": 0 } },
            { "op": "delete", "table": "Monsters", "key": [2] }
        ]),
    );
    assert!(w.is_empty(), "{w:?}");
    let w = warnings(
        &mut c,
        json!([{ "op": "delete", "table": "Rooms", "key": [1, 2] }]),
    );
    assert_eq!(
        w,
        ["deleting Rooms [1,2] leaves 1 row naming it: Rooms [1,1] Exit 0"],
        "{w:?}"
    );
}
