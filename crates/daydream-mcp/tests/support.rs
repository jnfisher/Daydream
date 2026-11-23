//! Realms and an MCP client for the tests.

use std::path::Path;

use daydream_mcp::Server;
use daydream_nmr::schema::layout_of;
use daydream_store::{Metadata, WbtrvDb};
use daydream_types::records::LayoutSet;
use daydream_wcc::tables::FILES;
use serde_json::{Value, json};

/// An empty `.db` for every NMR table (no Btrieve keys: rows are matched by NMR's index).
pub fn empty_realm(dir: &Path) {
    let tables = daydream_nmr::columns::TABLES
        .iter()
        .chain(daydream_nmr::players::TABLES);
    for t in tables {
        let Some(table) = t.table else { continue };
        let layout = layout_of(table, LayoutSet::Dos);
        let variable = table.is_variable_length();
        let fixed = if variable {
            layout.field("text").unwrap().offset
        } else {
            layout.len
        };
        let meta = Metadata {
            record_length: fixed,
            physical_record_length: fixed,
            page_length: 4096,
            variable_length_records: variable,
            version: 4,
        };
        let stem = FILES.iter().find(|f| f.table == table).unwrap().stem;
        WbtrvDb::create(&dir.join(format!("{stem}.db")), meta, Vec::new()).unwrap();
    }
}

/// An agent's side of the conversation: JSON-RPC requests, numbered.
pub struct Client<'a> {
    pub server: &'a Server,
    next: i64,
}

impl<'a> Client<'a> {
    pub fn new(server: &'a Server) -> Self {
        let mut c = Self { server, next: 0 };
        let init = c.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }),
        );
        assert_eq!(init["protocolVersion"], "2025-06-18");
        assert!(
            server
                .handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
                .is_none()
        );
        c
    }

    /// A request's `result`; panics on a JSON-RPC error.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let answer = self
            .server
            .handle(
                &json!({ "jsonrpc": "2.0", "id": self.next, "method": method, "params": params }),
            )
            .expect("a request gets an answer");
        assert_eq!(answer["id"], self.next);
        assert!(answer.get("error").is_none(), "{answer}");
        answer["result"].clone()
    }

    /// A tool's structured result; panics if the tool failed.
    pub fn call(&mut self, tool: &str, args: Value) -> Value {
        let r = self.request("tools/call", json!({ "name": tool, "arguments": args }));
        assert_eq!(r["isError"], false, "{tool}: {r}");
        let text: Value = serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, r["structuredContent"]);
        r["structuredContent"].clone()
    }

    /// A tool's error text; panics if the tool succeeded.
    pub fn fails(&mut self, tool: &str, args: Value) -> String {
        let r = self.request("tools/call", json!({ "name": tool, "arguments": args }));
        assert_eq!(r["isError"], true, "{tool} should fail: {r}");
        r["content"][0]["text"].as_str().unwrap().to_string()
    }
}

/// One file's records: id and bytes.
pub type Records = Vec<(i64, Vec<u8>)>;

/// Every record of every `.db` in a realm, by file name.
pub fn records(dir: &Path) -> Vec<(String, Records)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "db") {
            let db = WbtrvDb::open_read_only(&p).unwrap();
            out.push((
                p.file_name().unwrap().to_string_lossy().into_owned(),
                db.records().unwrap(),
            ));
        }
    }
    out.sort();
    out
}

/// A small area as an agent would build it: two rooms on map `map` joined north and
/// south, an item lying in the first, and a monster.
pub fn area_edits(map: i64, rooms: [i64; 2], item: i64, monster: i64) -> Value {
    json!([
        { "op": "insert", "table": "Rooms", "values": {
            "Map Number": map, "Room Number": rooms[0], "Name": "Mossy Grotto",
            "Desc 0": "Water drips from a low ceiling.",
            "Exit 0": rooms[1],
            "Room Item 0": item, "Room Item 0 QTY": 1
        }},
        { "op": "insert", "table": "Rooms", "values": {
            "Map Number": map, "Room Number": rooms[1], "Name": "Narrow Ledge",
            "Exit 1": rooms[0]
        }},
        { "op": "insert", "table": "Items", "values": {
            "Number": item, "Name": "glowing moss", "Weight": 1
        }},
        { "op": "insert", "table": "Monsters", "values": {
            "Number": monster, "Name": "cave newt"
        }}
    ])
}
