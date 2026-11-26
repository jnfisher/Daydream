//! The MCP server ([`daydream_mcp`]) over HTTP, at `/mcp`: MCP's Streamable HTTP transport,
//! answering each JSON-RPC message with JSON (no event streams).
//!
//! | Route | Answer |
//! |---|---|
//! | `POST /mcp` | a JSON-RPC message; its answer, or `202` for a notification |
//! | `POST /mcp/{token}` | the same, for a client that takes only a URL (with an access token) |
//! | `GET /mcp` | `405`: the server sends nothing unasked |
//!
//! Without an access token, a request whose `Origin` is not this machine is refused, so a
//! web page in the user's browser cannot reach the realm through it (DNS rebinding). With
//! one, [`crate::auth`] has already checked the request.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::{AppState, blocking};

/// Whether an `Origin` header names this machine.
fn local_origin(origin: &str) -> bool {
    let host = origin
        .split_once("://")
        .map_or(origin, |(_, rest)| rest)
        .split('/')
        .next()
        .unwrap_or("");
    let host = match host.rsplit_once(':') {
        Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    };
    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

pub(crate) async fn post(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(msg): Json<Value>,
) -> Response {
    if s.token.is_none()
        && let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok())
        && !local_origin(origin)
    {
        return (
            StatusCode::FORBIDDEN,
            "MCP requests from other sites are refused",
        )
            .into_response();
    }
    let answer = blocking(move || {
        // as the editors' saves: not into a running game's own files, and held until the
        // answer is made, so the write happens under it and the update file follows it
        let _writing = if writes(&msg) {
            Some(s.lock_write(false)?.0)
        } else {
            None
        };
        if let Some(answer) = game_tool(&s, &msg) {
            return Ok(Some(answer));
        }
        let cur = s.current();
        let mut answer = match cur.mcp.as_ref() {
            Some(mcp) => mcp.handle(&msg),
            None => daydream_mcp::handle_closed(
                &msg,
                s.silvermere.is_some(),
                "No game is open: call list_games, then open_game with one of its ids \
                 (or ask the user to choose one in Daydream's web UI).",
            ),
        };
        // the game tools beside the realm's
        if msg.get("method").and_then(Value::as_str) == Some("tools/list")
            && let Some(tools) = answer
                .as_mut()
                .and_then(|a| a.pointer_mut("/result/tools"))
                .and_then(Value::as_array_mut)
        {
            tools.extend(game_tools());
        }
        Ok(answer)
    })
    .await;
    match answer {
        Ok(Some(v)) => Json(v).into_response(),
        Ok(None) => StatusCode::ACCEPTED.into_response(),
        Err(e) => e.into_response(),
    }
}

/// The tools that choose the game, which work with or without one open.
fn game_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_games",
            "description": "The games Daydream can open (WCC .DAT folders, MBBSEmu .db folders, silvermere databases) with their ids, and the game open now.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "refresh": { "type": "boolean", "description": "Search again for games" }
                }
            }
        }),
        json!({
            "name": "open_game",
            "description": "Open a game by its id from list_games; the other tools then work on it. A legacy or silvermere game is edited as a working copy, made the first time it is opened.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The game's id, from list_games" },
                    "refresh": { "type": "boolean", "description": "Read the game again into its working copy, replacing the copy (and any edits not yet written back)" }
                },
                "required": ["id"]
            }
        }),
    ]
}

/// Answers a `tools/call` of [`game_tools`]; `None` for any other message.
fn game_tool(s: &AppState, msg: &Value) -> Option<Value> {
    if msg.get("method").and_then(Value::as_str) != Some("tools/call") {
        return None;
    }
    let id = msg.get("id").cloned()?;
    let name = msg.pointer("/params/name").and_then(Value::as_str)?;
    let args = msg.pointer("/params/arguments");
    let flag = |k: &str| {
        args.and_then(|a| a.get(k))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let result = match name {
        "list_games" => Ok(crate::games::list_games(s, flag("refresh"))),
        "open_game" => match args.and_then(|a| a.get("id")).and_then(Value::as_str) {
            None => Err("give the game's id, from list_games".to_string()),
            Some(game) => crate::games::open_game(
                s,
                crate::games::OpenJson {
                    id: Some(game.to_string()),
                    refresh: flag("refresh"),
                    ..Default::default()
                },
            )
            .map_err(|e| e.1),
        },
        _ => return None,
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": daydream_mcp::tool_result(result) }))
}

/// Whether an MCP message calls a tool that writes the realm.
fn writes(msg: &Value) -> bool {
    msg.get("method").and_then(Value::as_str) == Some("tools/call")
        && matches!(
            msg.pointer("/params/name").and_then(Value::as_str),
            Some("apply_edits" | "undo" | "silvermere_import")
        )
}

/// `/mcp/{token}`: [`crate::auth`] has matched the token; without one, there is no such route.
pub(crate) async fn post_keyed(
    state: State<Arc<AppState>>,
    Path(_token): Path<String>,
    headers: HeaderMap,
    msg: Json<Value>,
) -> Response {
    if state.token.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    post(state, headers, msg).await
}

pub(crate) async fn get() -> StatusCode {
    StatusCode::METHOD_NOT_ALLOWED
}

#[cfg(test)]
mod tests {
    use super::local_origin;

    #[test]
    fn only_this_machine_is_a_local_origin() {
        assert!(local_origin("http://localhost:7340"));
        assert!(local_origin("http://127.0.0.1"));
        assert!(local_origin("http://[::1]:7340"));
        assert!(!local_origin("https://evil.example"));
        assert!(!local_origin("http://localhost.evil.example"));
    }
}
