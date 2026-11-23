//! The MCP server as an agent sees it: the handshake, the read tools, and edits previewed,
//! applied and undone.

use daydream_mcp::{Options, Server};
use serde_json::json;

use crate::support::{Client, area_edits, empty_realm, records};

#[test]
fn the_handshake_lists_the_tools() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let tools = c.request("tools/list", json!({}));
    let names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for n in [
        "list_tables",
        "get_row",
        "preview_edits",
        "apply_edits",
        "undo",
    ] {
        assert!(names.contains(&n), "{names:?}");
    }
    // unknown methods and tools are JSON-RPC errors; an old protocol version is kept
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 9, "method": "nope" }))
        .unwrap();
    assert_eq!(answer["error"]["code"], -32601);
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": { "name": "nope" } }))
        .unwrap();
    assert_eq!(answer["error"]["code"], -32602);
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 11, "method": "initialize", "params": { "protocolVersion": "2024-11-05" } }))
        .unwrap();
    assert_eq!(answer["result"]["protocolVersion"], "2024-11-05");
}

#[test]
fn player_tables_are_offered_only_when_asked() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let tables = c.call("list_tables", json!({}));
    let names: Vec<&str> = tables["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Items") && names.contains(&"Rooms"));
    assert!(!names.contains(&"Users"), "{names:?}");
    assert!(
        c.fails("list_rows", json!({ "table": "Users" }))
            .contains("player table")
    );

    let server = Server::new(tmp.path(), Options { players: true });
    let mut c = Client::new(&server);
    c.call("list_rows", json!({ "table": "Users" }));
}

#[test]
fn an_agent_builds_an_area_and_takes_it_back() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let before = records(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);

    let rooms = c.call(
        "free_numbers",
        json!({ "table": "Rooms", "map": 1, "count": 2 }),
    );
    assert_eq!(rooms["free"], json!([1, 2]));
    let items = c.call("free_numbers", json!({ "table": "Items" }));
    assert_eq!(items["free"], json!([1]));
    assert!(
        c.fails("free_numbers", json!({ "table": "Rooms" }))
            .contains("map")
    );

    let preview = c.call(
        "preview_edits",
        json!({ "edits": area_edits(1, [1, 2], 1, 1) }),
    );
    let token = preview["token"].as_str().unwrap().to_string();
    let first = &preview["edits"][0];
    assert_eq!(first["key"], json!([1, 1]));
    assert!(
        first["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|ch| ch["column"] == "Name" && ch["after"] == "Mossy Grotto"),
        "{first}"
    );
    // a preview writes nothing
    assert_eq!(records(tmp.path()), before);

    let applied = c.call("apply_edits", json!({ "token": token }));
    assert_eq!(applied["reports"].as_array().unwrap().len(), 4);
    assert!(applied["records_changed"].as_u64().unwrap() >= 4);
    // a token is used once
    assert!(
        c.fails("apply_edits", json!({ "token": token }))
            .contains("preview")
    );

    let map = c.call("get_map", json!({ "map": 1 }));
    let built = map["rooms"].as_array().unwrap();
    assert_eq!(built.len(), 2);
    assert_eq!(built[0]["name"], "Mossy Grotto");
    assert_eq!(built[0]["exits"][0]["to"], 2);
    let row = c.call("get_row", json!({ "table": "Items", "key": [1] }));
    assert_eq!(row["values"]["Name"], "glowing moss");
    let list = c.call(
        "list_rows",
        json!({ "table": "Rooms", "contains": "ledge" }),
    );
    assert_eq!(list["total"], 1);
    assert_eq!(list["rows"][0]["key"], json!([1, 2]));

    // an edit of an existing row, then both undone, newest first
    let p = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "update", "table": "Items", "key": [1], "values": { "Weight": 3 } }] }),
    );
    assert_eq!(
        p["edits"][0]["changes"],
        json!([{ "column": "Weight", "before": 1, "after": 3 }])
    );
    let second = p["token"].as_str().unwrap().to_string();
    c.call("apply_edits", json!({ "token": second }));
    let history = c.call("history", json!({}));
    assert_eq!(history["applied"].as_array().unwrap().len(), 2);
    assert_eq!(history["applied"][0]["token"], second);
    // the area's undo would put back the item as it was before the second edit: refused
    assert!(
        c.fails("undo", json!({ "token": token }))
            .contains("changed since")
    );
    // a delete, undone: the record comes back where it was
    let monsters = records(tmp.path());
    let d = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "delete", "table": "Monsters", "key": [1] }] }),
    );
    assert!(
        d["edits"][0]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|ch| ch["column"] == "Name"
                && ch["before"] == "cave newt"
                && ch["after"].is_null())
    );
    c.call("apply_edits", json!({ "token": d["token"] }));
    c.fails("get_row", json!({ "table": "Monsters", "key": [1] }));
    c.call("undo", json!({ "token": d["token"] }));
    assert_eq!(records(tmp.path()), monsters);

    c.call("undo", json!({ "token": second }));
    c.call("undo", json!({ "token": token }));
    assert_eq!(records(tmp.path()), before);
    assert!(
        c.fails("undo", json!({ "token": token }))
            .contains("already")
    );
}

#[test]
fn bad_edits_are_refused_and_write_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let before = records(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let no_column = json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 5, "Colour": "red" } }] });
    assert!(
        c.fails("preview_edits", no_column)
            .contains("no column Colour")
    );
    let too_long = json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 5, "Name": "x".repeat(40) } }] });
    assert!(c.fails("preview_edits", too_long).contains("longer than"));
    let missing = json!({ "edits": [{ "op": "update", "table": "Items", "key": [5], "values": { "Weight": 1 } }] });
    assert!(c.fails("preview_edits", missing).contains("not found"));
    let twice = json!({ "edits": [
        { "op": "insert", "table": "Items", "values": { "Number": 5 } },
        { "op": "insert", "table": "Items", "values": { "Number": 5, "Name": "again" } }
    ]});
    assert!(c.fails("preview_edits", twice).contains("two edits"));

    // the realm changing between preview and apply
    let p = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 5 } }] }),
    );
    let other = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 5, "Name": "first" } }] }),
    );
    c.call("apply_edits", json!({ "token": other["token"] }));
    assert!(
        c.fails("apply_edits", json!({ "token": p["token"] }))
            .contains("already has")
    );
    c.call("undo", json!({ "token": other["token"] }));
    assert_eq!(records(tmp.path()), before);
}

#[test]
fn the_undo_journal_outlives_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let before = records(tmp.path());
    let token = {
        let server = Server::new(tmp.path(), Options::default());
        let mut c = Client::new(&server);
        let p = c.call(
            "preview_edits",
            json!({ "edits": area_edits(2, [10, 11], 7, 7) }),
        );
        c.call("apply_edits", json!({ "token": p["token"] }));
        p["token"].as_str().unwrap().to_string()
    };
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    c.call("undo", json!({ "token": token }));
    assert_eq!(records(tmp.path()), before);
    assert!(
        c.fails("undo", json!({ "token": "../../etc/passwd" }))
            .contains("not a token")
    );
}

#[test]
fn stdio_answers_one_line_per_request() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n\n",
        "not json\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#,
        "\n"
    );
    let mut out = Vec::new();
    server.run_stdio(input.as_bytes(), &mut out).unwrap();
    let lines: Vec<serde_json::Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["id"], 1);
    assert_eq!(lines[1]["error"]["code"], -32700);
    assert_eq!(lines[2]["id"], 2);
}
