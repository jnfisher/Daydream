//! Daydream as a Model Context Protocol (MCP) server, so an AI agent can read a realm and
//! make content in it through the same library calls the browser's editors use.
//!
//! [`Server::handle`] answers one JSON-RPC message; [`Server::run_stdio`] serves a local
//! agent over stdin and stdout (`daydream mcp REALM`), and the web server offers the same
//! handler over HTTP at `/mcp`.
//!
//! Tools, in NMR's terms (its tables, columns and keys):
//!
//! | Tool | Does |
//! |---|---|
//! | `list_tables` | every table: key columns, title column |
//! | `describe_table` | a table's columns: types, sizes, choice lists, what they refer to, and notes on what the game does with them |
//! | `find_references` | every row that names a record: refers to it, an exit's parameter, or spawns it |
//! | `list_rows` | a table's rows: key and title, more columns on request, filtered and paged |
//! | `get_row` | one row, every column |
//! | `free_numbers` | record numbers not in use (rooms: within a map) |
//! | `get_map` | one map's rooms and exits, and the realm's maps |
//! | `preview_edits` | what a list of edits (insert, update, delete) would change; a token |
//! | `apply_edits` | makes previewed edits, if the realm still gives the same preview |
//! | `undo` | puts back the records an applied token changed, byte for byte |
//! | `history` | the applied edits that can be undone, by changeset |
//! | `game_status` | what is open, where edits go, and whether the game is running |
//! | `silvermere_status` | what the silvermere database holds, and whether silvermere runs on it |
//! | `silvermere_export` | writes the realm's world into the silvermere database (dry run by default) |
//! | `silvermere_import` | rewrites the realm's world from the silvermere database (dry run by default) |
//!
//! The server also gives prompts (`new_area`, `new_monster`, `new_item`) that walk an agent
//! through making content in small, tested steps ([`prompts`]).
//!
//! The `silvermere_*` tools are offered only when the server was given a silvermere database
//! ([`Options::silvermere`]); the agent cannot name another.
//!
//! Writes go through the strict import, like the editors' saves, under one write lock.
//! Every applied change is journaled in `REALM/.daydream/undo/`, so it can be undone even
//! from a later session. The player tables (users, bank books, gangs) are left out unless
//! [`Options::players`] is set. Nothing here reveals more than the editors show.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use daydream_nmr::rows::{self, Edit, Preview, RecordChange, RowError};
use daydream_nmr::schema::NmrTable;
use daydream_store::Realm;

mod batch;
mod describe;
mod filter;
mod prompts;
use prompts::INSTRUCTIONS;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// The protocol versions this server speaks, newest first.
pub const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// The most rows one `list_rows` call returns.
pub const MAX_ROWS: usize = 500;

/// How the server is set up.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Offer the player tables (users, bank books, gangs).
    pub players: bool,
    /// A silvermere database (a SQLite file's path or a URL): offers the `silvermere_*`
    /// tools, on this database only, so an agent never handles its credentials.
    pub silvermere: Option<String>,
    /// Says what game the realm is and whether it is running; without one the realm is
    /// taken to be a copy no game runs on.
    pub status: Option<StatusCheck>,
    /// Folders `preview_edits` may read edits files from, besides the realm's own
    /// `.daydream/batches`.
    pub batch_dirs: Vec<PathBuf>,
}

/// What the server knows of the game its realm belongs to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct GameStatus {
    /// The game's name, when known.
    pub game: Option<String>,
    /// The realm is the game's own files (not a working copy): a running game would
    /// overwrite edits, so they are refused while it runs.
    pub live: bool,
    /// What is running on the realm's files, when something is.
    pub running_by: Option<String>,
}

/// A look at the game ([`GameStatus`]), made by whoever runs the server.
#[derive(Clone)]
pub struct StatusCheck(pub std::sync::Arc<dyn Fn() -> GameStatus + Send + Sync>);

impl std::fmt::Debug for StatusCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StatusCheck")
    }
}

impl StatusCheck {
    pub fn new(f: impl Fn() -> GameStatus + Send + Sync + 'static) -> Self {
        Self(std::sync::Arc::new(f))
    }
}

/// Previewed edits waiting to be applied.
#[derive(Debug, Clone)]
struct Pending {
    edits: Vec<Edit>,
    previews: Vec<Preview>,
    changeset: Option<String>,
    source: Option<String>,
}

/// An applied token, as the undo journal keeps it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Applied {
    pub token: String,
    /// Unix time it was applied.
    pub at: u64,
    pub edits: Vec<Edit>,
    pub records: Vec<RecordChange>,
    #[serde(default)]
    pub undone: bool,
    /// The piece of work it belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changeset: Option<String>,
    /// The edits file they were read from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Applied {
    /// The order tokens were applied in: the time, then the server's count.
    fn order(&self) -> (u64, u64) {
        let n = self
            .token
            .rsplit_once('-')
            .and_then(|(_, n)| n.parse().ok())
            .unwrap_or(0);
        (self.at, n)
    }

    /// Rows changed per table and operation (`{"Rooms": {"insert": 35}}`).
    fn summary(&self) -> Value {
        batch::count(&self.edits)
    }
}

/// One realm's MCP server. Shared by every connection.
#[derive(Debug)]
pub struct Server {
    realm_dir: PathBuf,
    options: Options,
    write: Mutex<()>,
    pending: Mutex<HashMap<String, Pending>>,
    counter: AtomicU64,
}

/// A tool's failure, reported to the agent as the tool's result (`isError`).
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error(transparent)]
    Row(#[from] RowError),
    #[error(transparent)]
    Store(#[from] daydream_store::StoreError),
    #[error("{0}")]
    Bad(String),
    #[error("{0}")]
    Io(String),
    #[error(transparent)]
    Silvermere(#[from] daydream_silvermere::Error),
}

fn bad(msg: impl Into<String>) -> ToolError {
    ToolError::Bad(msg.into())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A JSON-RPC error answer.
fn rpc_error(id: &Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn rpc_result(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn string_arg<'a>(args: &'a Map<String, Value>, name: &str) -> Result<&'a str, ToolError> {
    args.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| bad(format!("{name} is required (a string)")))
}

fn number_arg(args: &Map<String, Value>, name: &str) -> Result<Option<i64>, ToolError> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| bad(format!("{name} must be a whole number"))),
    }
}

fn initialize(id: &Value, params: &Value, more: &str) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked
        .filter(|v| PROTOCOL_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    rpc_result(
        id,
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": {}, "prompts": {} },
            "serverInfo": { "name": "daydream", "version": env!("CARGO_PKG_VERSION") },
            "instructions": format!("{INSTRUCTIONS}{more}"),
        }),
    )
}

/// Answers one JSON-RPC message while no realm is open: the handshake and the tool list
/// work, and every tool call is a tool error saying `why`. `None` for a notification.
pub fn handle_closed(msg: &Value, silvermere: bool, why: &str) -> Option<Value> {
    let id = msg.get("id").cloned()?;
    let Some(method) = msg.get("method").and_then(Value::as_str) else {
        return Some(rpc_error(&id, -32600, "not a JSON-RPC request"));
    };
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    Some(match method {
        "initialize" => initialize(&id, &params, &format!(" {why}")),
        "ping" => rpc_result(&id, json!({})),
        "tools/list" => rpc_result(&id, tool_list(silvermere)),
        "prompts/list" => rpc_result(&id, prompts::list()),
        "prompts/get" => prompt(&id, &params),
        "tools/call" => rpc_result(
            &id,
            json!({ "content": [{ "type": "text", "text": why }], "isError": true }),
        ),
        other => rpc_error(&id, -32601, format!("no method {other}")),
    })
}

/// A `prompts/get` answer.
fn prompt(id: &Value, params: &Value) -> Value {
    match prompts::get(params) {
        Ok(v) => rpc_result(id, v),
        Err(e) => rpc_error(id, -32602, e),
    }
}

/// A tool's answer as `tools/call` gives it.
pub fn tool_result(result: Result<Value, String>) -> Value {
    match result {
        Ok(v) => json!({
            "content": [{ "type": "text", "text": v.to_string() }],
            "structuredContent": v,
            "isError": false,
        }),
        Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
    }
}

/// The tools' descriptions and input schemas, as `tools/list` gives them.
fn tool_list(silvermere: bool) -> Value {
    let table = json!({ "type": "string", "description": "An NMR table name, as list_tables gives it (Items, Monsters, Rooms, ...)" });
    let key = json!({ "type": "array", "description": "The row's key: its key columns' values in order, as list_tables gives them (Rooms: [map, room])" });
    let edit = json!({
        "type": "object",
        "description": "One edit. op insert: table and values (must include the key columns; other columns start blank). op update: table, key and the values to change (not the key). op delete: table and key. op clone: table, from (the key of a row to copy) and values (the new row's key columns, and any columns to change); get_row the row first, since stock rows can hold junk.",
        "properties": {
            "op": { "type": "string", "enum": ["insert", "update", "delete", "clone"] },
            "table": table,
            "key": key,
            "from": { "type": "array", "description": "clone: the key of the row to copy" },
            "values": { "type": "object", "description": "{column: value}, column names as describe_table gives them" }
        },
        "required": ["op", "table"]
    });
    let changeset = json!({ "type": "string", "maxLength": 64, "description": "A label for one piece of work (\"cave area\"), the same on each of its batches, so it can be undone as a whole" });
    let mut list = json!({ "tools": [
        {
            "name": "list_tables",
            "description": "Every table of the realm in NMR's terms, with the columns each row is keyed by and the column that names a row.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "describe_table",
            "description": "A table's columns, as NMR's forms label them, with each column's type (long, double, text with its size, memo), the choice list its values are labelled from (lists), the table its values name (refers_to), and notes on what the game does with them (notes, by column or family: Room Item # is Room Item 0, 1, ...). Rooms adds exit_params: what Para1 # to Para4 # mean for each exit type. Read it before writing a table you have not written yet.",
            "inputSchema": { "type": "object", "properties": {
                "table": table,
                "ability_notes": { "type": "boolean", "description": "Give each ability number's note, not just its name" }
            }, "required": ["table"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "list_rows",
            "description": "A table's rows: each row's key and title, and the values of `columns` when asked. `contains` keeps rows whose title contains it (any case); `where` keeps rows meeting conditions on any columns. Paged by `offset` and `limit` (at most 500).",
            "inputSchema": { "type": "object", "properties": {
                "table": table,
                "columns": { "type": "array", "items": { "type": "string" } },
                "contains": { "type": "string" },
                "where": {
                    "type": "array",
                    "description": "Conditions that must all hold. A column with * stands for every column it fits, one word per * (\"Room Item *\" is Room Item 0, 1, ...; \"Exit *\" is every exit), and the condition holds when any of them does (for != : when none equals). Text compares ignoring case.",
                    "items": { "type": "object", "properties": {
                        "column": { "type": "string" },
                        "op": { "type": "string", "enum": filter::OPS, "description": "default =" },
                        "value": { "description": "a number or text; between: [low, high]; in: a list" }
                    }, "required": ["column", "value"] }
                },
                "offset": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_ROWS }
            }, "required": ["table"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "get_row",
            "description": "One row with every column's value.",
            "inputSchema": { "type": "object", "properties": { "table": table, "key": key }, "required": ["table", "key"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "free_numbers",
            "description": "Record numbers not in use, from `from` up (default 1). For Rooms give `map`: room numbers free on that map. With `column`, the values of that column no row uses, from `from` to `to` (Monsters, column Group, 6 to 35: the monster groups free).",
            "inputSchema": { "type": "object", "properties": {
                "table": table,
                "count": { "type": "integer", "minimum": 1, "maximum": 1000 },
                "from": { "type": "integer" },
                "to": { "type": "integer" },
                "map": { "type": "integer" },
                "column": { "type": "string" }
            }, "required": ["table"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "find_references",
            "description": "Every row that names a record: columns that refer to its table (a monster's messages, a shop's items, a room's exits), exit parameters by exit type (a key exit's item), and for a monster the rooms that spawn it. Use it before changing or reusing a record that others may share, such as a message.",
            "inputSchema": { "type": "object", "properties": { "table": table, "key": key }, "required": ["table", "key"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "get_map",
            "description": "One map's rooms with their exits (direction, target room, exit type) and what NMR's map marks on them, plus every map number and its room count. `from` and `to` keep a range of room numbers; `columns` adds Rooms columns to each room (Type, Min Index, Max Index, Mon Type, ...).",
            "inputSchema": { "type": "object", "properties": {
                "map": { "type": "integer" },
                "from": { "type": "integer" },
                "to": { "type": "integer" },
                "columns": { "type": "array", "items": { "type": "string" } }
            }, "required": ["map"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "preview_edits",
            "description": "Checks a list of edits against the realm without writing: every value each would change, after the import has accepted the row. Edits must touch different rows. Returns a token for apply_edits. For many edits, write them to a JSON file in the realm's batch folder (game_status gives it) and pass edits_file instead of retyping them. An insert's preview leaves out the columns it leaves blank; a large preview comes back as a summary unless detail asks for more.",
            "inputSchema": { "type": "object", "properties": {
                "edits": { "type": "array", "items": edit, "minItems": 1 },
                "edits_file": { "type": "string", "description": "A .json file of edits (a list, or {\"edits\": [...], \"changeset\": \"...\"}) in the realm's batch folder or another folder the server allows; a relative path is taken from the batch folder" },
                "changeset": changeset.clone(),
                "detail": { "type": "string", "enum": ["summary", "changes", "full"], "description": "summary: rows per table and the first few; changes: every changed value, but not an insert's blank columns; full: everything. Default: changes, or summary for a large batch" }
            } },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "apply_edits",
            "description": "Writes the edits a preview_edits token stands for, in order, if the realm still gives the same preview. Show the preview to the user first. The token can be undone with undo.",
            "inputSchema": { "type": "object", "properties": { "token": { "type": "string" } }, "required": ["token"] },
            "annotations": { "destructiveHint": false, "idempotentHint": false }
        },
        {
            "name": "undo",
            "description": "Puts back every record an applied token changed, exactly as it was, if none of them has changed since. Give a token, or a changeset to undo all of its tokens, newest first.",
            "inputSchema": { "type": "object", "properties": {
                "token": { "type": "string" },
                "changeset": changeset.clone()
            } },
            "annotations": { "destructiveHint": true }
        },
        {
            "name": "history",
            "description": "The edits applied through this server, newest first: each token's changeset, what it changed (rows per table and operation), and whether it has been undone. `edits` adds the edits themselves.",
            "inputSchema": { "type": "object", "properties": {
                "changeset": changeset,
                "edits": { "type": "boolean" }
            } },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "game_status",
            "description": "The realm open: which game it is, whether edits go to the game's own files or to a working copy, whether the game is running (edits to its own files are refused while it runs), and the silvermere database's state when there is one.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true }
        }
    ]});
    if silvermere {
        let dry = json!({ "type": "boolean", "description": "Report what would change without writing (the default). Show the user the report, then call again with false." });
        let keep = json!({ "type": "boolean", "description": "Keep the rows the source does not have, instead of deleting them" });
        let tools = list["tools"].as_array_mut().expect("a list");
        tools.push(json!({
            "name": "silvermere_status",
            "description": "The silvermere database this server was set up with: its rows per world table, whether silvermere is running on it, and whether it keeps the bytes of a Daydream export.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true }
        }));
        tools.push(json!({
            "name": "silvermere_export",
            "description": "Writes the realm's world (rooms, items, monsters, spells, classes, races, shops, messages, text blocks, actions) into the silvermere database, as silvermere's import does. silvermere must be stopped. Its live state (room floors, shop stock, monster kills) is kept unless reset_state.",
            "inputSchema": { "type": "object", "properties": {
                "dry_run": dry,
                "reset_state": { "type": "boolean", "description": "Rewrite the live state of rows silvermere has from the realm" },
                "keep_extra": keep
            } },
            "annotations": { "destructiveHint": true }
        }));
        tools.push(json!({
            "name": "silvermere_import",
            "description": "Rewrites the realm's world from the silvermere database: changed records are rewritten, records silvermere lacks are deleted. silvermere must be stopped. This cannot be taken back with undo.",
            "inputSchema": { "type": "object", "properties": {
                "dry_run": dry,
                "keep_extra": keep
            } },
            "annotations": { "destructiveHint": true }
        }));
    }
    list
}

impl Server {
    pub fn new(realm_dir: &Path, options: Options) -> Self {
        Self {
            realm_dir: realm_dir.to_path_buf(),
            options,
            write: Mutex::new(()),
            pending: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(0),
        }
    }

    /// The `silvermere_*` tools, on the server's own database.
    fn silvermere(
        &self,
        name: &str,
        database: &str,
        args: &Map<String, Value>,
    ) -> Result<Value, ToolError> {
        use daydream_silvermere as sm;
        let url = sm::database_url(database);
        let flag = |n: &str, default: bool| args.get(n).and_then(Value::as_bool).unwrap_or(default);
        if name == "silvermere_status" {
            let info = sm::block_on(sm::info(&url))?;
            let mut v = serde_json::to_value(info).map_err(|e| ToolError::Io(e.to_string()))?;
            running_flag(&mut v);
            return Ok(v);
        }
        let dry_run = flag("dry_run", true);
        let _writing = self
            .write
            .lock()
            .map_err(|e| ToolError::Io(e.to_string()))?;
        let realm = self.realm()?;
        let tables = if name == "silvermere_export" {
            let opts = sm::ExportOptions {
                state: if flag("reset_state", false) {
                    sm::State::Reset
                } else {
                    sm::State::Keep
                },
                delete: !flag("keep_extra", false),
                dry_run,
                ..sm::ExportOptions::default()
            };
            sm::block_on(sm::export(&realm, &url, &opts))?
        } else {
            let opts = sm::ImportOptions {
                delete: !flag("keep_extra", false),
                dry_run,
                ..sm::ImportOptions::default()
            };
            sm::block_on(sm::import(&realm, &url, &opts))?
        };
        Ok(json!({ "dry_run": dry_run, "tables": tables }))
    }

    fn realm(&self) -> Result<Realm, ToolError> {
        Ok(Realm::open(&self.realm_dir)?)
    }

    fn journal_dir(&self) -> PathBuf {
        self.realm_dir.join(".daydream").join("undo")
    }

    /// A table the agent may use.
    fn table(&self, name: &str) -> Result<&'static NmrTable, ToolError> {
        let t = rows::record_table(name)?;
        if !self.options.players
            && daydream_nmr::players::TABLES
                .iter()
                .any(|p| p.name == t.name)
        {
            return Err(bad(format!(
                "{} is a player table, which this server does not offer",
                t.name
            )));
        }
        Ok(t)
    }

    /// Answers one JSON-RPC message; `None` for a notification.
    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str);
        let Some(id) = id else {
            // notifications (initialized, cancelled, ...) need no answer
            return None;
        };
        let Some(method) = method else {
            return Some(rpc_error(&id, -32600, "not a JSON-RPC request"));
        };
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        Some(match method {
            "initialize" => initialize(&id, &params, ""),
            "ping" => rpc_result(&id, json!({})),
            "tools/list" => rpc_result(&id, tool_list(self.options.silvermere.is_some())),
            "prompts/list" => rpc_result(&id, prompts::list()),
            "prompts/get" => prompt(&id, &params),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params
                    .get("arguments")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                match self.call(name, &args) {
                    None => rpc_error(&id, -32602, format!("no tool {name}")),
                    Some(Ok(v)) => rpc_result(
                        &id,
                        json!({
                            "content": [{ "type": "text", "text": v.to_string() }],
                            "structuredContent": v,
                            "isError": false,
                        }),
                    ),
                    Some(Err(e)) => rpc_result(
                        &id,
                        json!({
                            "content": [{ "type": "text", "text": e.to_string() }],
                            "isError": true,
                        }),
                    ),
                }
            }
            other => rpc_error(&id, -32601, format!("no method {other}")),
        })
    }

    /// Runs a tool; `None` when there is no such tool.
    pub fn call(&self, name: &str, args: &Map<String, Value>) -> Option<Result<Value, ToolError>> {
        let mut out = self.run(name, args)?;
        if matches!(name, "apply_edits" | "undo" | "silvermere_import") {
            // the game's update file follows every write, or its next start undoes them
            let synced = daydream_store::update::sync(&self.realm_dir);
            if let Ok(Value::Object(answer)) = &mut out {
                match synced {
                    Ok(r) if r.changed() => {
                        answer.insert(
                            "update_file".into(),
                            json!({
                                "file": r.file,
                                "images_replaced": r.replaced,
                                "images_added": r.added,
                                "images_removed": r.removed,
                                "too_long": r.too_long,
                            }),
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        answer.insert("update_file_error".into(), json!(e.to_string()));
                    }
                }
            }
        }
        Some(out)
    }

    fn run(&self, name: &str, args: &Map<String, Value>) -> Option<Result<Value, ToolError>> {
        Some(match name {
            "list_tables" => Ok(self.list_tables()),
            "describe_table" => self.describe_table(args),
            "find_references" => self.find_references(args),
            "list_rows" => self.list_rows(args),
            "get_row" => self.get_row(args),
            "free_numbers" => self.free_numbers(args),
            "get_map" => self.get_map(args),
            "preview_edits" => self.preview_edits(args),
            "apply_edits" => self.apply_edits(args),
            "undo" => self.undo(args),
            "history" => self.history(args),
            "game_status" => self.game_status(),
            "silvermere_status" | "silvermere_export" | "silvermere_import" => {
                let Some(db) = &self.options.silvermere else {
                    return None;
                };
                self.silvermere(name, db, args)
            }
            _ => return None,
        })
    }

    fn list_tables(&self) -> Value {
        let tables: Vec<Value> = rows::record_tables()
            .filter(|t| self.table(t.name).is_ok())
            .map(|t| {
                json!({
                    "name": t.name,
                    "key": rows::index_columns(t),
                    "title": rows::title_column(t),
                })
            })
            .collect();
        json!({ "tables": tables })
    }

    fn describe_table(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let t = self.table(string_arg(args, "table")?)?;
        let notes = args
            .get("ability_notes")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok(describe::describe(t, notes))
    }

    fn find_references(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let t = self.table(string_arg(args, "table")?)?;
        let key = rows::parse_key(t, args.get("key").unwrap_or(&Value::Null))?;
        let realm = self.realm()?;
        let (_, all) = rows::load(&realm, t)?;
        if rows::find_row(&all, &rows::index_columns(t), &key).is_none() {
            return Err(
                RowError::NotFound(format!("{} {}", t.name, rows::key_display(&key))).into(),
            );
        }
        let tables: Vec<&'static NmrTable> = rows::record_tables()
            .filter(|s| self.table(s.name).is_ok())
            .collect();
        describe::find_references(&realm, &tables, t, &key)
    }

    fn list_rows(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let t = self.table(string_arg(args, "table")?)?;
        let realm = self.realm()?;
        let (_, all) = rows::load(&realm, t)?;
        let index = rows::index_columns(t);
        let title = rows::TITLE_COLUMNS.iter().find_map(|c| all.position(c));
        let more: Vec<(String, usize)> = match args.get("columns") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(cs)) => cs
                .iter()
                .map(|c| {
                    let c = c.as_str().ok_or_else(|| bad("columns are names"))?;
                    all.position(c)
                        .map(|i| (all.columns[i].clone(), i))
                        .ok_or_else(|| bad(format!("{} has no column {c}", t.name)))
                })
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(bad("columns must be a list of names")),
        };
        let contains = args
            .get("contains")
            .and_then(Value::as_str)
            .map(str::to_lowercase);
        let conds = filter::parse(args.get("where"), &all, t.name)?;
        let offset = number_arg(args, "offset")?.unwrap_or(0).max(0) as usize;
        let limit = (number_arg(args, "limit")?.unwrap_or(100).max(1) as usize).min(MAX_ROWS);
        let matching: Vec<&Vec<daydream_nmr::Value>> = all
            .rows
            .iter()
            .filter(|r| match (&contains, title) {
                (Some(needle), Some(i)) => rows::title_json(&r[i])
                    .as_str()
                    .is_some_and(|s| s.to_lowercase().contains(needle)),
                (Some(_), None) => false,
                (None, _) => true,
            })
            .filter(|r| filter::matches(&conds, r))
            .collect();
        let list: Vec<Value> = matching
            .iter()
            .skip(offset)
            .take(limit)
            .map(|r| {
                let mut o = Map::new();
                o.insert(
                    "key".into(),
                    Value::Array(
                        rows::key_of_row(&all, r, &index)
                            .iter()
                            .map(rows::cell_json)
                            .collect(),
                    ),
                );
                if let Some(i) = title {
                    o.insert("title".into(), rows::title_json(&r[i]));
                }
                for (name, i) in &more {
                    o.insert(name.clone(), rows::cell_json(&r[*i]));
                }
                Value::Object(o)
            })
            .collect();
        Ok(json!({
            "table": t.name,
            "key": index,
            "total": matching.len(),
            "offset": offset,
            "rows": list,
        }))
    }

    fn get_row(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let t = self.table(string_arg(args, "table")?)?;
        let key = rows::parse_key(t, args.get("key").unwrap_or(&Value::Null))?;
        let realm = self.realm()?;
        let (_, all) = rows::load(&realm, t)?;
        let row = rows::find_row(&all, &rows::index_columns(t), &key)
            .ok_or_else(|| RowError::NotFound(format!("{} {}", t.name, rows::key_display(&key))))?;
        Ok(json!({
            "table": t.name,
            "key": key.iter().map(rows::cell_json).collect::<Vec<_>>(),
            "values": rows::row_json(&all, row),
        }))
    }

    fn free_numbers(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let t = self.table(string_arg(args, "table")?)?;
        let count = number_arg(args, "count")?.unwrap_or(1).clamp(1, 1000) as usize;
        let from = number_arg(args, "from")?.unwrap_or(1).max(1);
        let map = number_arg(args, "map")?;
        let index = rows::index_columns(t);
        let rooms = index.contains(&"Room Number");
        if rooms && map.is_none() {
            return Err(bad("rooms are numbered within a map: give map"));
        }
        if let Some(column) = args.get("column").and_then(Value::as_str) {
            return self.free_values(t, column, from, number_arg(args, "to")?, count);
        }
        let number_column = if rooms { "Room Number" } else { index[0] };
        let realm = self.realm()?;
        let (_, all) = rows::load(&realm, t)?;
        let at = all
            .position(number_column)
            .ok_or_else(|| bad(format!("{} has no {number_column}", t.name)))?;
        let map_at = all.position("Map Number");
        let as_long = |v: &daydream_nmr::Value| match v {
            daydream_nmr::Value::Long(n) => Some(i64::from(*n)),
            daydream_nmr::Value::Double(d) => Some(*d as i64),
            _ => None,
        };
        let used: std::collections::HashSet<i64> = all
            .rows
            .iter()
            .filter(|r| !rooms || map_at.and_then(|i| as_long(&r[i])) == map)
            .filter_map(|r| as_long(&r[at]))
            .collect();
        let free: Vec<i64> = (from..).filter(|n| !used.contains(n)).take(count).collect();
        Ok(json!({ "table": t.name, "map": map, "free": free }))
    }

    /// The values of `column` from `from` to `to` that no row of `t` holds.
    fn free_values(
        &self,
        t: &'static NmrTable,
        column: &str,
        from: i64,
        to: Option<i64>,
        count: usize,
    ) -> Result<Value, ToolError> {
        let to = to.ok_or_else(|| bad("with column, give to: the highest value to look at"))?;
        let realm = self.realm()?;
        let (_, all) = rows::load(&realm, t)?;
        let at = all
            .position(column)
            .ok_or_else(|| bad(format!("{} has no column {column}", t.name)))?;
        let used: std::collections::HashSet<i64> = all
            .rows
            .iter()
            .filter_map(|r| match &r[at] {
                daydream_nmr::Value::Long(n) => Some(i64::from(*n)),
                daydream_nmr::Value::Double(d) => Some(*d as i64),
                _ => None,
            })
            .collect();
        let free: Vec<i64> = (from..=to)
            .filter(|n| !used.contains(n))
            .take(count)
            .collect();
        Ok(json!({ "table": t.name, "column": all.columns[at], "free": free }))
    }

    fn get_map(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let map = number_arg(args, "map")?.ok_or_else(|| bad("map is required"))?;
        let realm = self.realm()?;
        let layout = daydream_nmr::layout_of_realm(&realm);
        let view = daydream_nmr::map::map_view(&realm, layout, map)
            .map_err(|e| ToolError::Io(e.to_string()))?;
        let mut v = serde_json::to_value(view).map_err(|e| ToolError::Io(e.to_string()))?;
        let from = number_arg(args, "from")?.unwrap_or(i64::MIN);
        let to = number_arg(args, "to")?.unwrap_or(i64::MAX);
        if let Some(rooms) = v.get_mut("rooms").and_then(Value::as_array_mut) {
            rooms.retain(|r| r["room"].as_i64().is_some_and(|n| n >= from && n <= to));
        }
        self.room_columns(&realm, map, args.get("columns"), &mut v)?;
        v["map"] = json!(map);
        v["directions"] = json!(daydream_nmr::map::DIRECTIONS);
        Ok(v)
    }

    /// Adds `columns` of the Rooms table to each room of a map view.
    fn room_columns(
        &self,
        realm: &Realm,
        map: i64,
        columns: Option<&Value>,
        view: &mut Value,
    ) -> Result<(), ToolError> {
        let names: Vec<&str> = match columns {
            None | Some(Value::Null) => return Ok(()),
            Some(Value::Array(l)) => l
                .iter()
                .map(|c| c.as_str().ok_or_else(|| bad("columns are names")))
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(bad("columns must be a list of names")),
        };
        let t = self.table("Rooms")?;
        let (_, all) = rows::load(realm, t)?;
        let at: Vec<(String, usize)> = names
            .iter()
            .map(|c| {
                all.position(c)
                    .map(|i| (all.columns[i].clone(), i))
                    .ok_or_else(|| bad(format!("Rooms has no column {c}")))
            })
            .collect::<Result<_, _>>()?;
        let (Some(map_at), Some(room_at)) =
            (all.position("Map Number"), all.position("Room Number"))
        else {
            return Ok(());
        };
        let num = |v: &daydream_nmr::Value| match v {
            daydream_nmr::Value::Long(n) => Some(i64::from(*n)),
            daydream_nmr::Value::Double(d) => Some(*d as i64),
            _ => None,
        };
        let by_room: HashMap<i64, &Vec<daydream_nmr::Value>> = all
            .rows
            .iter()
            .filter(|r| num(&r[map_at]) == Some(map))
            .filter_map(|r| num(&r[room_at]).map(|n| (n, r)))
            .collect();
        if let Some(rooms) = view.get_mut("rooms").and_then(Value::as_array_mut) {
            for room in rooms {
                if let Some(row) = room["room"].as_i64().and_then(|n| by_room.get(&n)) {
                    for (name, i) in &at {
                        room[name.as_str()] = rows::cell_json(&row[*i]);
                    }
                }
            }
        }
        Ok(())
    }

    /// The edits asked for, inline or from a file, with the file's changeset and name.
    fn edits_arg(&self, realm: &Realm, args: &Map<String, Value>) -> Result<GivenEdits, ToolError> {
        let (raw, changeset, source) = match (args.get("edits"), args.get("edits_file")) {
            (Some(e), None) => (e.clone(), None, None),
            (None, Some(Value::String(f))) => {
                let allowed = batch::allowed_dirs(&self.realm_dir, &self.options.batch_dirs);
                let (path, v) = batch::read_file(f, &allowed)?;
                let (edits, changeset) = batch::file_edits(v)?;
                (edits, changeset, Some(path.display().to_string()))
            }
            (None, Some(_)) => return Err(bad("edits_file is a path")),
            (None, None) => return Err(bad("give edits, or edits_file")),
            (Some(_), Some(_)) => return Err(bad("give edits or edits_file, not both")),
        };
        let edits = batch::expand(realm, &raw)?;
        for e in &edits {
            self.table(e.table())?;
        }
        Ok((edits, changeset, source))
    }

    fn preview_all(&self, realm: &Realm, edits: &[Edit]) -> Result<Vec<Preview>, ToolError> {
        let previews = edits
            .iter()
            .map(|e| rows::preview(realm, e))
            .collect::<Result<Vec<_>, _>>()?;
        // two edits of one row would each be previewed against the realm as it is
        let mut seen = std::collections::HashSet::new();
        for p in &previews {
            if !seen.insert((p.table.clone(), rows::key_display(&p.key))) {
                return Err(bad(format!(
                    "two edits of {} {}: make them one",
                    p.table,
                    rows::key_display(&p.key)
                )));
            }
        }
        Ok(previews)
    }

    fn preview_edits(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let detail = batch::Detail::parse(args.get("detail"))?;
        let realm = self.realm()?;
        let (edits, in_file, source) = self.edits_arg(&realm, args)?;
        let mut changeset = changeset_arg(args)?;
        if changeset.is_none()
            && let Some(c) = in_file
        {
            changeset = changeset_arg(&Map::from_iter([("changeset".to_string(), c)]))?;
        }
        let previews = self.preview_all(&realm, &edits)?;
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let token = format!("e{}-{n}", now());
        let mut shown = batch::shown(&edits, &previews, detail);
        self.pending
            .lock()
            .map_err(|e| ToolError::Io(e.to_string()))?
            .insert(
                token.clone(),
                Pending {
                    edits,
                    previews,
                    changeset: changeset.clone(),
                    source: source.clone(),
                },
            );
        shown["token"] = json!(token);
        shown["changeset"] = json!(changeset);
        if let Some(s) = source {
            shown["edits_file"] = json!(s);
        }
        Ok(shown)
    }

    fn apply_edits(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let token = string_arg(args, "token")?.to_string();
        let pending = self
            .pending
            .lock()
            .map_err(|e| ToolError::Io(e.to_string()))?
            .get(&token)
            .cloned()
            .ok_or_else(|| {
                bad(format!(
                    "no previewed edits {token}: call preview_edits first"
                ))
            })?;
        let _writing = self
            .write
            .lock()
            .map_err(|e| ToolError::Io(e.to_string()))?;
        self.refuse_if_running()?;
        let realm = self.realm()?;
        if self.preview_all(&realm, &pending.edits)? != pending.previews {
            return Err(RowError::Conflict(
                "the realm changed since the preview: preview again".into(),
            )
            .into());
        }
        let mut reports = Vec::new();
        let mut records = Vec::new();
        for e in &pending.edits {
            match rows::apply(&realm, e) {
                Ok((report, changed)) => {
                    reports.push(rows::report_json(&report));
                    records.extend(changed);
                }
                Err(err) => {
                    // put back what the earlier edits of this token wrote
                    let undone = rows::revert(&realm, &records).is_ok();
                    return Err(ToolError::Bad(format!(
                        "{err}{}",
                        if undone {
                            " (the token's earlier edits were undone)"
                        } else {
                            " (the token's earlier edits could NOT be undone)"
                        }
                    )));
                }
            }
        }
        let applied = Applied {
            token: token.clone(),
            at: now(),
            edits: pending.edits,
            records,
            undone: false,
            changeset: pending.changeset,
            source: pending.source,
        };
        self.save(&applied)?;
        self.pending
            .lock()
            .map_err(|e| ToolError::Io(e.to_string()))?
            .remove(&token);
        Ok(json!({
            "token": token,
            "changeset": applied.changeset,
            "reports": reports,
            "records_changed": applied.records.len(),
        }))
    }

    fn journal_path(&self, token: &str) -> Result<PathBuf, ToolError> {
        if token.is_empty() || !token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(bad(format!("{token} is not a token")));
        }
        Ok(self.journal_dir().join(format!("{token}.json")))
    }

    fn save(&self, a: &Applied) -> Result<(), ToolError> {
        let io = |e: std::io::Error| ToolError::Io(e.to_string());
        std::fs::create_dir_all(self.journal_dir()).map_err(io)?;
        let text = serde_json::to_string(a).map_err(|e| ToolError::Io(e.to_string()))?;
        std::fs::write(self.journal_path(&a.token)?, text).map_err(io)
    }

    fn load_applied(&self, token: &str) -> Result<Applied, ToolError> {
        let path = self.journal_path(token)?;
        let text = std::fs::read_to_string(&path).map_err(|_| {
            bad(format!(
                "{token} was not applied through this realm's server"
            ))
        })?;
        serde_json::from_str(&text).map_err(|e| ToolError::Io(format!("{}: {e}", path.display())))
    }

    fn undo(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let token = args.get("token").and_then(Value::as_str);
        let changeset = changeset_arg(args)?;
        let _writing = self
            .write
            .lock()
            .map_err(|e| ToolError::Io(e.to_string()))?;
        self.refuse_if_running()?;
        let realm = self.realm()?;
        match (token, changeset) {
            (Some(token), None) => {
                let mut a = self.load_applied(token)?;
                if a.undone {
                    return Err(bad(format!("{token} is already undone")));
                }
                rows::revert(&realm, &a.records)?;
                a.undone = true;
                self.save(&a)?;
                Ok(json!({ "token": token, "records_restored": a.records.len() }))
            }
            (None, Some(changeset)) => {
                let tokens: Vec<Applied> = self
                    .applied()
                    .into_iter()
                    .filter(|a| !a.undone && a.changeset.as_deref() == Some(changeset.as_str()))
                    .collect();
                if tokens.is_empty() {
                    return Err(bad(format!(
                        "nothing applied under the changeset {changeset} is left to undo"
                    )));
                }
                let mut undone = Vec::new();
                let mut restored = 0;
                for mut a in tokens {
                    if let Err(e) = rows::revert(&realm, &a.records) {
                        return Err(bad(format!(
                            "{}: {e}. Undone before it, newest first: {}",
                            a.token,
                            if undone.is_empty() {
                                "none".to_string()
                            } else {
                                undone.join(", ")
                            }
                        )));
                    }
                    a.undone = true;
                    self.save(&a)?;
                    restored += a.records.len();
                    undone.push(a.token);
                }
                Ok(
                    json!({ "changeset": changeset, "undone": undone, "records_restored": restored }),
                )
            }
            _ => Err(bad("give a token or a changeset (not both)")),
        }
    }

    /// Every applied token in the journal, newest first.
    fn applied(&self) -> Vec<Applied> {
        let mut all = Vec::new();
        if let Ok(dir) = std::fs::read_dir(self.journal_dir()) {
            for entry in dir.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(token) = name.strip_suffix(".json")
                    && let Ok(a) = self.load_applied(token)
                {
                    all.push(a);
                }
            }
        }
        all.sort_by_key(|a| std::cmp::Reverse(a.order()));
        all
    }

    fn history(&self, args: &Map<String, Value>) -> Result<Value, ToolError> {
        let changeset = changeset_arg(args)?;
        let with_edits = args.get("edits").and_then(Value::as_bool).unwrap_or(false);
        let list: Vec<Value> = self
            .applied()
            .iter()
            .filter(|a| changeset.is_none() || a.changeset == changeset)
            .map(|a| {
                let mut v = json!({
                    "token": a.token,
                    "at": a.at,
                    "changeset": a.changeset,
                    "rows": a.summary(),
                    "undone": a.undone,
                });
                if let Some(s) = &a.source {
                    v["edits_file"] = json!(s);
                }
                if with_edits {
                    v["edits"] = json!(a.edits);
                }
                v
            })
            .collect();
        Ok(json!({ "applied": list }))
    }

    /// The game's state as the server was told how to look at it.
    fn status(&self) -> GameStatus {
        self.options
            .status
            .as_ref()
            .map(|c| (c.0)())
            .unwrap_or_default()
    }

    /// Refuses a write when the realm is a running game's own files.
    fn refuse_if_running(&self) -> Result<(), ToolError> {
        let s = self.status();
        match (s.live, s.running_by) {
            (true, Some(by)) => Err(bad(format!(
                "{} is running on these files ({by}): ask the user to stop it first, or its next \
                 write undoes this one",
                s.game.as_deref().unwrap_or("the game")
            ))),
            _ => Ok(()),
        }
    }

    fn game_status(&self) -> Result<Value, ToolError> {
        let s = self.status();
        let running = s.running_by.is_some();
        let mut v = json!({
            "realm": self.realm_dir.display().to_string(),
            "game": s.game,
            "edits_go_to": if s.live { "the game's own files" } else { "a working copy" },
            "running": running,
            "running_by": s.running_by,
            "can_apply": !(s.live && running),
            "batch_folder": batch::allowed_dirs(&self.realm_dir, &[])[0].display().to_string(),
        });
        let mut notes = Vec::new();
        if s.live && running {
            notes.push("The game is running on these files: apply_edits and undo are refused until it stops.");
        }
        if let Some(db) = &self.options.silvermere {
            use daydream_silvermere as sm;
            let url = sm::database_url(db);
            match sm::block_on(sm::info(&url)) {
                Ok(info) => {
                    let mut i =
                        serde_json::to_value(info).map_err(|e| ToolError::Io(e.to_string()))?;
                    running_flag(&mut i);
                    v["silvermere"] =
                        json!({ "running": i["running"], "running_by": i["running_by"] });
                }
                Err(e) => v["silvermere"] = json!({ "error": e.to_string() }),
            }
            notes.push(
                "Edits here change the working copy only. silvermere_export writes them into silvermere's \
                 database, which needs silvermere stopped; restart it afterwards to play them.",
            );
        }
        v["notes"] = json!(notes);
        Ok(v)
    }

    /// Serves JSON-RPC messages, one per line, until `input` ends (the stdio transport).
    pub fn run_stdio(&self, input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let answer = match serde_json::from_str::<Value>(&line) {
                Ok(msg) => self.handle(&msg),
                Err(e) => Some(rpc_error(&Value::Null, -32700, format!("not JSON: {e}"))),
            };
            if let Some(a) = answer {
                writeln!(output, "{a}")?;
                output.flush()?;
            }
        }
        Ok(())
    }
}

/// Edits as asked for: the edits, a file's changeset, and the file's path.
type GivenEdits = (Vec<Edit>, Option<Value>, Option<String>);

/// A changeset label, when given.
fn changeset_arg(args: &Map<String, Value>) -> Result<Option<String>, ToolError> {
    match args.get("changeset") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() || s.chars().count() > 64 || s.chars().any(char::is_control) {
                Err(bad("changeset is a label of 1 to 64 characters"))
            } else {
                Ok(Some(s.to_string()))
            }
        }
        Some(_) => Err(bad("changeset is a label (a string)")),
    }
}

/// silvermere's status with `running` a yes or no and its holder in `running_by` (the
/// library gives the holder or null, which reads as "unknown").
fn running_flag(v: &mut Value) {
    let by = v.get("running").cloned().unwrap_or(Value::Null);
    v["running"] = json!(!by.is_null());
    v["running_by"] = by;
}
