//! What the API tests share: an empty realm, and requests to the router for it.
#![allow(dead_code)]

use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use daydream_nmr::schema::layout_of;
use daydream_store::{Metadata, WbtrvDb};
use daydream_types::records::LayoutSet;
use daydream_wcc::tables::FILES;
use serde_json::{Value, json};
use tower::ServiceExt;

/// An empty `.db` for every NMR table (no Btrieve keys: rows are matched by NMR's index).
pub fn empty_realm(dir: &Path) {
    for t in daydream_nmr::columns::TABLES {
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

pub async fn send(dir: &Path, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "127.0.0.1:7340")
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let res = daydream_server::router(dir).oneshot(req).await.unwrap();
    let status = res.status();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

pub async fn get(dir: &Path, uri: &str) -> Value {
    let (status, body) = send(dir, "GET", uri, None).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    body
}

pub async fn add(dir: &Path, table: &str, values: Value) {
    let (status, body) = send(
        dir,
        "POST",
        &format!("/api/tables/{table}/row"),
        Some(json!({ "values": values })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

/// `?key=` for a key.
pub fn key(k: Value) -> String {
    k.to_string()
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
