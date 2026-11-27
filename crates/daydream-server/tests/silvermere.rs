//! The Workshop's silvermere page: the world written into a silvermere database, and read back
//! from it as one step of the world's history, which Undo takes back.

use axum::http::StatusCode;
use serde_json::json;

mod support;
use support::{add, empty_realm, get, key, send};

#[tokio::test]
async fn a_read_from_silvermere_is_one_step_of_the_history() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("realm");
    std::fs::create_dir_all(&dir).unwrap();
    empty_realm(&dir);
    add(&dir, "Items", json!({"Number": 1, "Name": "dagger"})).await;
    let database = tmp.path().join("silvermere.db").display().to_string();

    // the world written into silvermere's database is not a step of the world's history
    let (status, out) = send(
        &dir,
        "POST",
        "/api/silvermere/export",
        Some(json!({ "database": database })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");

    // the world changes, then silvermere's copy is read back over it
    let uri = format!("/api/tables/Items/row?key={}", key(json!([1])));
    let (status, out) = send(
        &dir,
        "PUT",
        &uri,
        Some(json!({"values": {"Name": "spoon"}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");
    let name = |row: serde_json::Value| row["values"]["Name"].clone();

    let (status, out) = send(
        &dir,
        "POST",
        "/api/silvermere/import",
        Some(json!({"database": database, "dry_run": true})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["step"], json!(null), "{out}");
    assert_eq!(name(get(&dir, &uri).await), "spoon");

    let (status, out) = send(
        &dir,
        "POST",
        "/api/silvermere/import",
        Some(json!({ "database": database })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");
    let step = out["step"].as_i64().unwrap_or_else(|| panic!("{out}"));
    assert_eq!(name(get(&dir, &uri).await), "dagger");
    let history = get(&dir, "/api/history").await;
    assert_eq!(history["steps"][0]["step"], step, "{history}");
    assert_eq!(
        history["steps"][0]["label"], "Read from silvermere",
        "{history}"
    );

    // Undo takes the whole read back
    let (status, out) = send(&dir, "POST", "/api/undo", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(name(get(&dir, &uri).await), "spoon");
}
