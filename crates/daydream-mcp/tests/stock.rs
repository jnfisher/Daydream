//! The M6 gate on the stock 1.11p world: an agent builds a small area through the MCP
//! tools, the realm is written back to WCC files, and those files hold exactly the
//! realm's records with every index sound.

use daydream_mcp::{Options, Server};
use daydream_store::Realm;
use daydream_store::dat::{WriteOptions, write_dat};
use daydream_store::realm::convert_dir;
use daydream_wcc::btrieve::OpenOptions;
use serde_json::json;

use crate::support::{Client, area_edits};

/// Each file's records, without their ids.
fn payloads(dir: &std::path::Path) -> Vec<(String, Vec<Vec<u8>>)> {
    crate::support::records(dir)
        .into_iter()
        .map(|(f, rs)| (f, rs.into_iter().map(|(_, r)| r).collect()))
        .collect()
}

#[test]
fn an_area_built_by_an_agent_round_trips_through_the_wcc_files() {
    // DAYDREAM_MCP_WCC=DIR runs it on another set of WCC files (the Worldgroup ones, say)
    let vir = match std::env::var_os("DAYDREAM_MCP_WCC") {
        Some(dir) => dir.into(),
        None => daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| {
            panic!("the tests need the stock distribution and could not get it: {e}")
        }),
    };
    let tmp = tempfile::tempdir().unwrap();
    let realm = tmp.path().join("realm");
    convert_dir(&vir, &realm).unwrap();

    let server = Server::new(&realm, Options::default());
    let mut c = Client::new(&server);
    let rooms = c.call(
        "free_numbers",
        json!({ "table": "Rooms", "map": 1, "count": 2 }),
    );
    let rooms: Vec<i64> = serde_json::from_value(rooms["free"].clone()).unwrap();
    let item = c.call("free_numbers", json!({ "table": "Items" }))["free"][0]
        .as_i64()
        .unwrap();
    let monster = c.call("free_numbers", json!({ "table": "Monsters" }))["free"][0]
        .as_i64()
        .unwrap();
    let p = c.call(
        "preview_edits",
        json!({ "edits": area_edits(1, [rooms[0], rooms[1]], item, monster) }),
    );
    c.call("apply_edits", json!({ "token": p["token"] }));
    let row = c.call("get_row", json!({ "table": "Rooms", "key": [1, rooms[0]] }));
    assert_eq!(row["values"]["Name"], "Mossy Grotto");
    assert_eq!(row["values"]["Exit 0"], rooms[1]);

    // written back: the files that changed are rebuilt, and read back as the realm has them
    let out = tmp.path().join("wcc");
    let opts = WriteOptions {
        out: Some(out.clone()),
        ..Default::default()
    };
    let written = write_dat(&Realm::open(&realm).unwrap(), &vir, &opts).unwrap();
    let rebuilt: Vec<&str> = written
        .iter()
        .filter(|w| w.action != daydream_store::dat::Action::Unchanged)
        .map(|w| w.stem.as_str())
        .collect();
    assert_eq!(rebuilt.len(), 3, "{written:?}");
    for w in &written {
        let mut f = OpenOptions::new()
            .variable_length(true)
            .open(&w.dest)
            .unwrap_or_else(|e| panic!("{e}"));
        f.verify_indexes().unwrap_or_else(|e| panic!("{e}"));
    }
    let again = tmp.path().join("again");
    convert_dir(&out, &again).unwrap();
    assert!(
        payloads(&again) == payloads(&realm),
        "the WCC files differ from the realm"
    );

    // and taken back
    c.call("undo", json!({ "token": p["token"] }));
    let again = tmp.path().join("before");
    convert_dir(&vir, &again).unwrap();
    assert!(
        payloads(&again) == payloads(&realm),
        "undo left the realm changed"
    );
}
