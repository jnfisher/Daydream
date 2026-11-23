//! The MCP server ([`daydream_mcp`]) over HTTP, at `/mcp`: MCP's Streamable HTTP transport,
//! answering each JSON-RPC message with JSON (no event streams).
//!
//! | Route | Answer |
//! |---|---|
//! | `POST /mcp` | a JSON-RPC message; its answer, or `202` for a notification |
//! | `GET /mcp` | `405`: the server sends nothing unasked |
//!
//! A request whose `Origin` is not this machine is refused, so a web page in the user's
//! browser cannot reach the realm through it (DNS rebinding).

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

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
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok())
        && !local_origin(origin)
    {
        return (
            StatusCode::FORBIDDEN,
            "MCP requests from other sites are refused",
        )
            .into_response();
    }
    let answer = blocking(move || Ok(s.mcp.handle(&msg))).await;
    match answer {
        Ok(Some(v)) => Json(v).into_response(),
        Ok(None) => StatusCode::ACCEPTED.into_response(),
        Err(e) => e.into_response(),
    }
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
