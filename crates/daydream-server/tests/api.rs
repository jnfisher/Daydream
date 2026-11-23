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
