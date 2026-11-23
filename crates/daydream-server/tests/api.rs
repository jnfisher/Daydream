//! The API over a small realm: a WCCRACE.db built record by record, and a converted
//! Btrieve file Daydream does not decode.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use daydream_store::key::{Key, Segment, ty};
use daydream_store::{Metadata, WbtrvDb};
use serde_json::Value;
use tower::ServiceExt;

/// A realm with two races (numbers 1 and 2) and wbtrv32's MBBSEMU test file.
fn realm() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let keys = vec![Key {
        segments: vec![Segment {
            number: 0,
            index: 0,
            attributes: 0,
            data_type: ty::UNSIGNED_BINARY,
            offset: 0,
            length: 2,
            null_value: 0,
            acs: None,
        }],
        acs_name: None,
        acs: None,
    }];
    let meta = Metadata {
        record_length: 126,
        physical_record_length: 130,
        page_length: 1024,
        variable_length_records: false,
        version: 4,
    };
    let mut db = WbtrvDb::create(&dir.path().join("WCCRACE.db"), meta, keys).unwrap();
    for (n, name) in [(1u16, "Human"), (2, "Dwarf")] {
        let mut r = vec![0u8; 126];
        r[0..2].copy_from_slice(&n.to_le_bytes());
        r[2..2 + name.len()].copy_from_slice(name.as_bytes());
        db.insert(&r).unwrap();
    }
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../daydream-wcc/tests/fixtures/btrieve/MBBSEMU.DAT");
    WbtrvDb::convert(&fixture, &dir.path().join("MBBSEMU.db")).unwrap();
    dir
}

async fn get(dir: &std::path::Path, uri: &str) -> (StatusCode, Value) {
    let res = daydream_server::router(dir)
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn health_answers() {
    let dir = realm();
    let (status, body) = get(dir.path(), "/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn the_realm_lists_its_files() {
    let dir = realm();
    let (status, body) = get(dir.path(), "/api/realm").await;
    assert_eq!(status, StatusCode::OK);
    let files = body["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    let race = files.iter().find(|f| f["stem"] == "WCCRACE").unwrap();
    assert_eq!(race["table"], "races");
    assert_eq!(race["records"], 2);
    assert_eq!(race["record_length"], 126);
    let other = files.iter().find(|f| f["stem"] == "MBBSEMU").unwrap();
    assert_eq!(other["table"], Value::Null);
}

#[tokio::test]
async fn records_come_back_decoded_and_paged() {
    let dir = realm();
    let (status, body) = get(dir.path(), "/api/files/wccrace/records?limit=1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["records"][0]["name"], "Human");
    assert_eq!(body["records"][0]["number"], 1);
    assert_eq!(body["next_after"], 1);
    let (_, body) = get(dir.path(), "/api/files/WCCRACE/records?after=1").await;
    assert_eq!(body["records"][0]["name"], "Dwarf");
    assert_eq!(body["next_after"], Value::Null);
    let (status, body) = get(dir.path(), "/api/files/WCCRACE/records/2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["_id"], 2);
    assert_eq!(body["number"], 2);
}

#[tokio::test]
async fn files_daydream_does_not_decode_come_back_raw() {
    let dir = realm();
    let (status, body) = get(dir.path(), "/api/files/MBBSEMU/records/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["_raw_len"], 74);
    assert!(body["_raw"].as_str().unwrap().len() > 90);
}

#[tokio::test]
async fn missing_things_are_404() {
    let dir = realm();
    assert_eq!(
        get(dir.path(), "/api/files/NOPE/records").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(dir.path(), "/api/files/WCCRACE/records/9").await.0,
        StatusCode::NOT_FOUND
    );
}

async fn send(dir: &std::path::Path, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = daydream_server::router(dir).oneshot(req).await.unwrap();
    let status = res.status();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// `?key=` for a key.
fn key(k: Value) -> String {
    let text = k.to_string();
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[tokio::test]
async fn the_tables_are_nmrs() {
    let dir = realm();
    let (status, body) = get(dir.path(), "/api/tables").await;
    assert_eq!(status, StatusCode::OK);
    let tables = body["tables"].as_array().unwrap();
    assert!(tables.iter().all(|t| t["name"] != "Info"));
    let races = tables.iter().find(|t| t["name"] == "Races").unwrap();
    assert_eq!(races["key"], serde_json::json!(["Number"]));
    assert_eq!(races["title"], "Name");
    assert_eq!(races["columns"][1]["name"], "Name");
    assert_eq!(races["columns"][1]["type"], "text");
    assert_eq!(races["columns"][1]["size"], 29);
    let rooms = tables.iter().find(|t| t["name"] == "Rooms").unwrap();
    assert_eq!(
        rooms["key"],
        serde_json::json!(["Map Number", "Room Number"])
    );
}

#[tokio::test]
async fn rows_list_their_keys_and_titles() {
    let dir = realm();
    let (status, body) = get(dir.path(), "/api/tables/races/rows").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["table"], "Races");
    assert_eq!(
        body["rows"],
        serde_json::json!([[[1], "Human"], [[2], "Dwarf"]])
    );
    let (status, body) = get(
        dir.path(),
        &format!("/api/tables/Races/row?key={}", key(serde_json::json!([2]))),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["key"], serde_json::json!([2]));
    assert_eq!(body["values"]["Name"], "Dwarf");
    assert_eq!(body["values"]["Number"], 2);
}

#[tokio::test]
async fn a_save_changes_only_what_it_names() {
    let dir = realm();
    let k = key(serde_json::json!([1]));
    let uri = format!("/api/tables/Races/row?key={k}");
    let before = std::fs::read(dir.path().join("WCCRACE.db")).unwrap();
    let (status, report) = send(
        dir.path(),
        "PUT",
        &uri,
        serde_json::json!({"values": {"Name": "Human"}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["unchanged"], 1);
    assert_eq!(
        std::fs::read(dir.path().join("WCCRACE.db")).unwrap(),
        before
    );

    let (status, report) = send(
        dir.path(),
        "PUT",
        &uri,
        serde_json::json!({"values": {"Name": "Elf", "Min INT": "40"}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["updated"], 1);
    let (_, body) = get(dir.path(), &uri).await;
    assert_eq!(body["values"]["Name"], "Elf");
    assert_eq!(body["values"]["Min INT"], 40);
    // the other record is untouched
    let (_, body) = get(dir.path(), "/api/files/WCCRACE/records/2").await;
    assert_eq!(body["name"], "Dwarf");
}

#[tokio::test]
async fn bad_saves_are_refused() {
    let dir = realm();
    let uri = format!("/api/tables/Races/row?key={}", key(serde_json::json!([1])));
    let put = |body: Value| send(dir.path(), "PUT", &uri, body);
    assert_eq!(
        put(serde_json::json!({"values": {"Number": 5}})).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        put(serde_json::json!({"values": {"Nope": 5}})).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        put(serde_json::json!({"values": {"Min INT": "lots"}}))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let long = "x".repeat(30);
    assert_eq!(
        put(serde_json::json!({"values": {"Name": long}})).await.0,
        StatusCode::BAD_REQUEST
    );
    // too big for NMR's Integer
    let (status, body) = put(serde_json::json!({"values": {"Min INT": 70000}})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let missing = format!("/api/tables/Races/row?key={}", key(serde_json::json!([9])));
    assert_eq!(get(dir.path(), &missing).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        get(dir.path(), "/api/tables/Info/rows").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(dir.path(), "/api/tables/Races/row?key=%5B1,2%5D")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn records_insert_and_delete() {
    let dir = realm();
    let (status, report) = send(
        dir.path(),
        "POST",
        "/api/tables/Races/row",
        serde_json::json!({"values": {"Number": 3, "Name": "Gnome"}}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{report}");
    assert_eq!(report["inserted"], 1);
    let (_, body) = get(dir.path(), "/api/tables/Races/rows").await;
    assert_eq!(body["rows"][2], serde_json::json!([[3], "Gnome"]));

    let again = send(
        dir.path(),
        "POST",
        "/api/tables/Races/row",
        serde_json::json!({"values": {"Number": 3}}),
    )
    .await;
    assert_eq!(again.0, StatusCode::CONFLICT);
    let keyless = send(
        dir.path(),
        "POST",
        "/api/tables/Races/row",
        serde_json::json!({"values": {}}),
    )
    .await;
    assert_eq!(keyless.0, StatusCode::BAD_REQUEST);

    let uri = format!("/api/tables/Races/row?key={}", key(serde_json::json!([3])));
    let (status, report) = send(dir.path(), "DELETE", &uri, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["deleted"], 1);
    assert_eq!(
        send(dir.path(), "DELETE", &uri, Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, body) = get(dir.path(), "/api/tables/Races/rows").await;
    assert_eq!(body["rows"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn the_ui_is_served() {
    let dir = realm();
    for (uri, kind) in [
        ("/", "text/html"),
        ("/app.js", "text/javascript"),
        ("/app.css", "text/css"),
    ] {
        let res = daydream_server::router(dir.path())
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{uri}");
        let ty = res.headers()["content-type"].to_str().unwrap().to_string();
        assert!(ty.starts_with(kind), "{uri}: {ty}");
    }
}
