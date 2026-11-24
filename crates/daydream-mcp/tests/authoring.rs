//! What helps an agent author content: prompts, changesets, and the game's state.

use daydream_mcp::{GameStatus, Options, Server, StatusCheck};
use serde_json::json;

use crate::support::{Client, area_edits, empty_realm, records};

#[test]
fn prompts_walk_the_agent_through_the_work() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let list = c.request("prompts/list", json!({}));
    let names: Vec<&str> = list["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["new_area", "new_monster", "new_item"]);

    let p = c.request(
        "prompts/get",
        json!({ "name": "new_monster", "arguments": { "idea": "a cave bear, level 8" } }),
    );
    let text = p["messages"][0]["content"]["text"].as_str().unwrap();
    assert!(text.contains("a cave bear, level 8"), "{text}");
    assert!(text.contains("Start from: (not given)"), "{text}");
    assert!(text.contains("one of each kind first"), "{text}");

    // a missing required argument is a JSON-RPC error
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 50, "method": "prompts/get", "params": { "name": "new_area" } }))
        .unwrap();
    assert_eq!(answer["error"]["code"], -32602);
}

#[test]
fn a_changeset_is_undone_as_a_whole() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let before = records(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);

    let first = c.call(
        "preview_edits",
        json!({ "edits": area_edits(1, [1, 2], 1, 1), "changeset": "grotto" }),
    );
    assert_eq!(first["changeset"], "grotto");
    c.call("apply_edits", json!({ "token": first["token"] }));
    let second = c.call(
        "preview_edits",
        json!({
            "edits": [{ "op": "update", "table": "Items", "key": [1], "values": { "Weight": 3 } }],
            "changeset": "grotto"
        }),
    );
    c.call("apply_edits", json!({ "token": second["token"] }));
    let other = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 9 } }] }),
    );
    c.call("apply_edits", json!({ "token": other["token"] }));

    let h = c.call("history", json!({ "changeset": "grotto" }));
    let applied = h["applied"].as_array().unwrap();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0]["token"], second["token"]);
    assert_eq!(applied[1]["rows"]["Rooms"]["insert"], 2);
    assert!(applied[1].get("edits").is_none());
    let h = c.call("history", json!({ "edits": true }));
    assert_eq!(h["applied"].as_array().unwrap().len(), 3);
    assert!(h["applied"][0]["edits"].is_array());

    assert!(c.fails("undo", json!({})).contains("token or a changeset"));
    let undone = c.call("undo", json!({ "changeset": "grotto" }));
    assert_eq!(undone["undone"], json!([second["token"], first["token"]]));
    assert!(
        c.fails("undo", json!({ "changeset": "grotto" }))
            .contains("left to undo")
    );
    c.call("undo", json!({ "token": other["token"] }));
    assert_eq!(records(tmp.path()), before);
}

#[test]
fn writes_wait_for_a_running_game_to_stop() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let flag = running.clone();
    let server = Server::new(
        tmp.path(),
        Options {
            status: Some(StatusCheck::new(move || GameStatus {
                game: Some("Test BBS".into()),
                live: true,
                running_by: flag
                    .load(std::sync::atomic::Ordering::SeqCst)
                    .then(|| "MBBSEmu (pid 7)".to_string()),
            })),
            ..Options::default()
        },
    );
    let mut c = Client::new(&server);
    let s = c.call("game_status", json!({}));
    assert_eq!(s["running"], true);
    assert_eq!(s["can_apply"], false);
    assert_eq!(s["edits_go_to"], "the game's own files");

    let p = c.call(
        "preview_edits",
        json!({ "edits": [{ "op": "insert", "table": "Items", "values": { "Number": 4 } }] }),
    );
    let refused = c.fails("apply_edits", json!({ "token": p["token"] }));
    assert!(refused.contains("Test BBS is running"), "{refused}");

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(c.call("game_status", json!({}))["can_apply"], true);
    c.call("apply_edits", json!({ "token": p["token"] }));
}

#[test]
fn a_copy_with_no_game_can_always_be_written() {
    let tmp = tempfile::tempdir().unwrap();
    empty_realm(tmp.path());
    let server = Server::new(tmp.path(), Options::default());
    let mut c = Client::new(&server);
    let s = c.call("game_status", json!({}));
    assert_eq!(s["running"], false);
    assert_eq!(s["can_apply"], true);
    assert_eq!(s["edits_go_to"], "a working copy");
}
