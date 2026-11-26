//! The API over a small realm: a WCCRACE.db built record by record, and a converted
//! Btrieve file Daydream does not decode.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use daydream_store::key::{Key, Segment, ty};
use daydream_store::{Metadata, WbtrvDb};
use serde_json::Value;
use tower::ServiceExt;

/// The router for `dir` as this machine's browser reaches it: with a local `Host`.
fn app(dir: &std::path::Path) -> axum::Router {
    daydream_server::router(dir).layer(axum::middleware::map_request(
        |mut req: Request<Body>| async move {
            req.headers_mut()
                .entry("host")
                .or_insert(axum::http::HeaderValue::from_static("127.0.0.1:7340"));
            req
        },
    ))
}

/// A realm with two races (numbers 1 and 2) and wbtrv32's MBBSEMU test file.
fn realm() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut db = WbtrvDb::create(&dir.path().join("WCCRACE.db"), meta(126), number_key()).unwrap();
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

fn meta(record_length: usize) -> Metadata {
    Metadata {
        record_length,
        physical_record_length: record_length + 4,
        page_length: 1024,
        variable_length_records: false,
        version: 4,
    }
}

/// The record number, a 2-byte key at the start, as the WCC files' first key.
fn number_key() -> Vec<Key> {
    vec![Key {
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
    }]
}

async fn get(dir: &std::path::Path, uri: &str) -> (StatusCode, Value) {
    let res = app(dir)
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
async fn a_worldgroup_realms_records_decode_in_its_layouts() {
    // WCCCLAS2: 156 bytes, its 4-byte `titles` aligned to 0x64 (the DOS record is 153)
    let dir = tempfile::tempdir().unwrap();
    let mut db = WbtrvDb::create(&dir.path().join("WCCCLAS2.db"), meta(156), number_key()).unwrap();
    let mut r = vec![0u8; 156];
    r[0..2].copy_from_slice(&3u16.to_le_bytes());
    r[2..9].copy_from_slice(b"Mystic\0");
    r[0x64..0x68].copy_from_slice(&7u32.to_le_bytes());
    db.insert(&r).unwrap();
    drop(db);
    let (status, body) = get(dir.path(), "/api/files/WCCCLAS2/records/1").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["number"], 3);
    assert_eq!(body["name"], "Mystic");
    assert_eq!(body["titles"], 7);
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
    let res = app(dir).oneshot(req).await.unwrap();
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
    let (status, body) = get(dir.path(), "/api/tables/races/rows?columns=Name,Number").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["rows"][1],
        serde_json::json!([[2], "Dwarf", ["Dwarf", 2]])
    );
    let (status, _) = get(dir.path(), "/api/tables/races/rows?columns=Nope").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
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

/// The realm with an update file holding an image of each race: `(seqnum, type, image)`.
fn realm_with_update_file() -> tempfile::TempDir {
    let dir = realm();
    let races = WbtrvDb::open_read_only(&dir.path().join("WCCRACE.db"))
        .unwrap()
        .records()
        .unwrap();
    let mut seq_key = number_key();
    seq_key[0].segments[0].length = 4;
    seq_key[0].segments[0].data_type = ty::AUTO_INC;
    let mut upd = WbtrvDb::create(&dir.path().join("WCCUPDAT.db"), meta(2015), seq_key).unwrap();
    for (i, (_, race)) in races.iter().enumerate() {
        let mut r = vec![0u8; 2015];
        r[..4].copy_from_slice(&(i as i32 + 1).to_le_bytes());
        r[4] = 1;
        r[5..5 + race.len()].copy_from_slice(race);
        upd.insert(&r).unwrap();
    }
    dir
}

fn update_images(dir: &std::path::Path) -> Vec<(i32, u8, Vec<u8>)> {
    WbtrvDb::open_read_only(&dir.join("WCCUPDAT.db"))
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .map(|(_, r)| {
            (
                i32::from_le_bytes(r[..4].try_into().unwrap()),
                r[4],
                r[5..5 + 126].to_vec(),
            )
        })
        .collect()
}

#[tokio::test]
async fn saves_keep_the_update_file_in_step() {
    let dir = realm_with_update_file();
    let uri = format!("/api/tables/Races/row?key={}", key(serde_json::json!([1])));
    let (status, report) = send(
        dir.path(),
        "PUT",
        &uri,
        serde_json::json!({"values": {"Name": "Elf"}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    let race = WbtrvDb::open_read_only(&dir.path().join("WCCRACE.db"))
        .unwrap()
        .get(1)
        .unwrap()
        .unwrap();
    let images = update_images(dir.path());
    // the game's next start applies the save instead of undoing it
    assert_eq!(images[0], (1, 1, race));
    assert_eq!(&images[1].2[2..7], b"Dwarf");

    let gone = format!("/api/tables/Races/row?key={}", key(serde_json::json!([2])));
    let (status, report) = send(dir.path(), "DELETE", &gone, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    // and a deleted race does not come back
    let images = update_images(dir.path());
    assert_eq!(images.len(), 1);
    assert_eq!(&images[0].2[2..5], b"Elf");
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
        ("/fonts/nunito.woff2", "font/woff2"),
        ("/fonts/fredoka.woff2", "font/woff2"),
        ("/notices.txt", "text/plain"),
    ] {
        let res = app(dir.path())
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{uri}");
        let ty = res.headers()["content-type"].to_str().unwrap().to_string();
        assert!(ty.starts_with(kind), "{uri}: {ty}");
    }
    let (status, _) = get(dir.path(), "/fonts/comic-sans.woff2").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn post_bytes(dir: &std::path::Path, uri: &str, body: Vec<u8>) -> (StatusCode, Value) {
    let req = Request::post(uri).body(Body::from(body)).unwrap();
    let res = app(dir).oneshot(req).await.unwrap();
    let status = res.status();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// The realm's races as NMR's JSON export, with every name changed.
fn races_renamed(dir: &std::path::Path) -> Vec<u8> {
    let realm = daydream_store::Realm::open(dir).unwrap();
    let t = daydream_nmr::schema::table("Races").unwrap();
    let mut rows =
        daydream_nmr::export_table(&realm, t, daydream_types::records::LayoutSet::Dos).unwrap();
    let name = rows.position("Name").unwrap();
    for r in &mut rows.rows {
        r[name] = daydream_nmr::Value::Text("Imported".into());
    }
    serde_json::to_vec(&daydream_nmr::NmrData { tables: vec![rows] }).unwrap()
}

#[tokio::test]
async fn the_importer_takes_an_export_and_its_options() {
    let dir = realm();
    let file = races_renamed(dir.path());
    let opts = |o: Value| format!("/api/tools/import?options={}", key(o));

    let (status, body) = post_bytes(
        dir.path(),
        &opts(serde_json::json!({"dry_run": true})),
        file.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["reports"][0]["updated"], 2);
    let (_, races) = get(dir.path(), "/api/tables/Races/rows").await;
    assert_eq!(races["rows"][0][1], "Human");

    let ranged = serde_json::json!({"ranges": [{"table": "Races", "from": 2, "to": 2}]});
    let (status, body) = post_bytes(dir.path(), &opts(ranged), file.clone()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["reports"][0]["updated"], 1);
    assert_eq!(body["reports"][0]["skipped"], 1);
    let (_, races) = get(dir.path(), "/api/tables/Races/rows").await;
    assert_eq!(
        races["rows"],
        serde_json::json!([[[1], "Human"], [[2], "Imported"]])
    );

    let skip = serde_json::json!({"skip_existing": true});
    let (_, body) = post_bytes(dir.path(), &opts(skip), file.clone()).await;
    assert_eq!(body["reports"][0]["updated"], 0);

    let (status, _) = post_bytes(dir.path(), "/api/tools/import", b"not a file".to_vec()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post_bytes(
        dir.path(),
        &opts(serde_json::json!({"room_parts": "some"})),
        file,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_deleter_needs_a_range_or_all() {
    let dir = realm();
    let del = |body: Value| send(dir.path(), "POST", "/api/tools/delete", body);
    assert_eq!(
        del(serde_json::json!({"table": "Races"})).await.0,
        StatusCode::BAD_REQUEST
    );
    let (status, body) =
        del(serde_json::json!({"table": "Races", "all": true, "dry_run": true})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["report"]["deleted"], 2);
    let (_, body) =
        del(serde_json::json!({"table": "Races", "range": {"table": "", "from": 2, "to": 9}}))
            .await;
    assert_eq!(body["report"]["deleted"], 1);
    let (_, races) = get(dir.path(), "/api/tables/Races/rows").await;
    assert_eq!(races["rows"].as_array().unwrap().len(), 1);
}

/// A user file of 8-byte records keyed by their first two bytes.
fn users(path: &std::path::Path, ids: &[u16]) {
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
        record_length: 8,
        physical_record_length: 12,
        page_length: 1024,
        variable_length_records: false,
        version: 4,
    };
    let mut db = WbtrvDb::create(path, meta, keys).unwrap();
    for &id in ids {
        let mut r = vec![0u8; 8];
        r[..2].copy_from_slice(&id.to_le_bytes());
        db.insert(&r).unwrap();
    }
}

#[tokio::test]
async fn users_merge_in() {
    let dir = realm();
    let other = tempfile::tempdir().unwrap();
    users(&other.path().join("WCCUSERS.db"), &[1, 2, 3]);
    let file = std::fs::read(other.path().join("WCCUSERS.db")).unwrap();
    // no user file in the realm yet
    let (status, _) = post_bytes(dir.path(), "/api/tools/merge-users", file.clone()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    users(&dir.path().join("WCCUSERS.db"), &[2]);
    let (status, body) = post_bytes(
        dir.path(),
        "/api/tools/merge-users?dry_run=true",
        file.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["copied"], 2);
    let (_, body) = post_bytes(dir.path(), "/api/tools/merge-users", file).await;
    assert_eq!(body["copied"], 2);
    assert_eq!(body["skipped"].as_array().unwrap().len(), 1);
    let (_, realm) = get(dir.path(), "/api/realm").await;
    let users = realm["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["stem"] == "WCCUSERS")
        .unwrap()
        .clone();
    assert_eq!(users["records"], 3);
}

#[tokio::test]
async fn the_modifier_previews_then_writes() {
    let dir = realm();
    let run = |dry_run: bool| {
        send(
            dir.path(),
            "POST",
            "/api/tools/modify",
            serde_json::json!({
                "table": "Races",
                "action": {"kind": "math", "columns": ["CP"], "op": "set", "value": 120},
                "only_if": [{"kind": "column", "column": "Number", "cmp": "ge", "value": 2}],
                "dry_run": dry_run,
            }),
        )
    };
    let (status, body) = run(true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["changed"], 1);
    assert_eq!(body["records"], 1);
    assert_eq!(body["skipped"], 1);
    assert_eq!(body["changes"][0]["column"], "CP");
    assert_eq!(body["changes"][0]["after"], 120);
    assert!(body["report"].is_null());
    let (_, row) = get(
        dir.path(),
        &format!("/api/tables/Races/row?key={}", key(serde_json::json!([2]))),
    )
    .await;
    assert_eq!(row["values"]["CP"], 0, "a preview writes nothing");

    let (_, body) = run(false).await;
    assert_eq!(body["report"]["updated"], 1, "{body}");
    let (_, row) = get(
        dir.path(),
        &format!("/api/tables/Races/row?key={}", key(serde_json::json!([2]))),
    )
    .await;
    assert_eq!(row["values"]["CP"], 120);

    let (status, _) = send(
        dir.path(),
        "POST",
        "/api/tools/modify",
        serde_json::json!({"table": "Races", "action": {"kind": "math", "columns": ["Nope"], "op": "add", "value": 1}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn presets_are_checked_before_they_run() {
    let dir = realm();
    let preset = |body: Value| send(dir.path(), "POST", "/api/tools/preset", body);
    let (status, body) = preset(serde_json::json!({"preset": "multiply_exp", "by": 50})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("2 to 20"),
        "{body}"
    );
    let (status, _) = preset(serde_json::json!({"preset": "no_such_fix"})).await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn the_lists_answer() {
    let dir = realm();
    for uri in [
        "/api/reports/limited-items",
        "/api/reports/find-item?item=1&users=false",
        "/api/reports/control-rooms",
        "/api/reports/monster-groups",
        "/api/reports/npc-rooms",
        "/api/map?map=2",
        "/api/reports/room-indexes",
    ] {
        let (status, body) = get(dir.path(), uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    }
    let (status, _) = get(dir.path(), "/api/reports/find-item").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn quest_files_come_from_the_realm_folder() {
    let dir = realm();
    let (status, _) = get(dir.path(), "/api/quests").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    std::fs::write(
        dir.path().join("NMR-QuestsCustom.txt"),
        b"-Caf\xe9 Quest\r\n1|0|2|x\r\n",
    )
    .unwrap();
    let (status, body) = get(dir.path(), "/api/quests?file=custom").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["text"], "-Caf\u{e9} Quest\r\n1|0|2|x\r\n");
    let (status, _) = get(dir.path(), "/api/quests?file=../x").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_record_number_changer_previews_and_refuses() {
    let dir = realm();
    let renumber = |body: Value| send(dir.path(), "POST", "/api/tools/renumber", body);
    let (status, body) = renumber(
        serde_json::json!({"kind": "records", "table": "races", "moves": [], "dry_run": true}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["error"].as_str().unwrap().contains("empty"), "{body}");
    let (status, body) = renumber(
        serde_json::json!({"kind": "records", "table": "races", "moves": [{"from": 1, "to": 900}], "dry_run": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["dry_run"], true);
    assert!(body["tables"].is_array(), "{body}");
}

#[tokio::test]
async fn a_dig_from_a_missing_room_is_refused() {
    let dir = realm();
    let (status, body) = send(
        dir.path(),
        "POST",
        "/api/map/dig",
        serde_json::json!({"map": 1, "room": 1, "dir": 0, "to_map": 1, "to_room": 2, "dry_run": true}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn users_are_edited_and_revived() {
    let dir = realm();
    let meta = Metadata {
        record_length: 1998,
        physical_record_length: 2002,
        page_length: 4096,
        variable_length_records: false,
        version: 4,
    };
    WbtrvDb::create(&dir.path().join("WCCUSERS.db"), meta, Vec::new()).unwrap();
    let (_, tables) = get(dir.path(), "/api/tables").await;
    let users = tables["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "Users")
        .unwrap()
        .clone();
    assert_eq!(users["key"], serde_json::json!(["BBS Name"]));

    let values = serde_json::json!({ "values": {
        "BBS Name": "Ann", "Max HP": 30, "HP": 2, "Lives": 3,
        "Map Number": 1, "Room Number": 9, "Trail 2 Map": 4, "Trail 2 Room": 40,
    }});
    let (status, body) = send(dir.path(), "POST", "/api/tables/Users/row", values).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let revive = serde_json::json!({
        "preset": "revive_character", "user": "Ann", "lives": true, "spells": true,
        "to": { "back": 2 }, "dry_run": false,
    });
    let (status, body) = send(dir.path(), "POST", "/api/tools/preset", revive).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, row) = get(
        dir.path(),
        &format!(
            "/api/tables/Users/row?key={}",
            key(serde_json::json!(["Ann"]))
        ),
    )
    .await;
    let v = &row["values"];
    assert_eq!(
        [&v["HP"], &v["Lives"], &v["Map Number"], &v["Room Number"]],
        [30, 9, 4, 40]
    );
    let nobody =
        serde_json::json!({ "preset": "revive_character", "user": "Bob", "dry_run": true });
    let (status, _) = send(dir.path(), "POST", "/api/tools/preset", nobody).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn changes_are_listed_and_taken_back() {
    let dir = realm_with_update_file();
    daydream_store::journal::reset(dir.path()).unwrap();
    let (status, body) = get(dir.path(), "/api/changes").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["since"].is_u64(), "{body}");
    assert_eq!(body["changes"], serde_json::json!([]));

    let uri = format!("/api/tables/Races/row?key={}", key(serde_json::json!([1])));
    let (status, report) = send(
        dir.path(),
        "PUT",
        &uri,
        serde_json::json!({"values": {"Name": "Elf"}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    let (_, body) = get(dir.path(), "/api/changes").await;
    let c = &body["changes"][0];
    assert_eq!(body["changes"].as_array().unwrap().len(), 1, "{body}");
    assert_eq!(
        (&c["stem"], &c["id"], &c["how"], &c["what"], &c["name"]),
        (
            &serde_json::json!("WCCRACE"),
            &serde_json::json!(1),
            &serde_json::json!("changed"),
            &serde_json::json!("race 1"),
            &serde_json::json!("Elf")
        )
    );
    assert_eq!(c["members"], serde_json::json!(["name"]));
    // what the editors open it by
    assert_eq!(
        (&c["nmr"], &c["key"]),
        (&serde_json::json!("Races"), &serde_json::json!([1]))
    );

    let (status, _) = send(
        dir.path(),
        "POST",
        "/api/changes/revert",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = send(
        dir.path(),
        "POST",
        "/api/changes/revert",
        serde_json::json!({"changes": [{"stem": "WCCRACE", "id": 1}]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["reverted"], 1);
    let (_, body) = get(dir.path(), "/api/changes").await;
    assert_eq!(body["changes"], serde_json::json!([]));
    let (_, body) = get(dir.path(), &uri).await;
    assert_eq!(body["values"]["Name"], "Human");
    // the update file follows the revert as it follows a save
    assert_eq!(&update_images(dir.path())[0].2[2..7], b"Human");
}

#[tokio::test]
async fn saves_are_undone_and_redone() {
    let dir = realm_with_update_file();
    let (_, body) = get(dir.path(), "/api/history").await;
    assert_eq!(body["steps"], serde_json::json!([]));
    assert_eq!(body["undo"], Value::Null);
    let (status, _) = send(dir.path(), "POST", "/api/undo", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let uri = format!("/api/tables/Races/row?key={}", key(serde_json::json!([1])));
    let save = |name: &'static str| {
        let uri = uri.clone();
        let dir = dir.path().to_path_buf();
        async move {
            send(
                &dir,
                "PUT",
                &uri,
                serde_json::json!({"values": {"Name": name}}),
            )
            .await
        }
    };
    assert_eq!(save("Elf").await.0, StatusCode::OK);
    assert_eq!(save("Gnome").await.0, StatusCode::OK);
    // a save that changes nothing is no step
    assert_eq!(save("Gnome").await.0, StatusCode::OK);
    let (_, body) = get(dir.path(), "/api/history").await;
    let steps = body["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2, "{body}");
    assert_eq!(steps[0]["label"], "Save Races 1");
    assert_eq!(steps[0]["records"], 1);
    assert_eq!(body["undo"]["step"], steps[0]["step"]);
    let (status, detail) = get(dir.path(), &format!("/api/history/{}", steps[0]["step"])).await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["changes"][0]["what"], "race 1");
    assert_eq!(detail["changes"][0]["members"], serde_json::json!(["name"]));

    let name = || async {
        get(dir.path(), &uri).await.1["values"]["Name"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let (status, body) = send(dir.path(), "POST", "/api/undo", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["state"], "undone");
    assert_eq!(name().await, "Elf");
    // the update file follows an undo as it follows a save
    assert_eq!(&update_images(dir.path())[0].2[2..5], b"Elf");
    send(dir.path(), "POST", "/api/undo", serde_json::json!({})).await;
    assert_eq!(name().await, "Human");
    let (status, body) = send(dir.path(), "POST", "/api/redo", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(name().await, "Elf");
    // undo and redo are no steps of their own
    let (_, body) = get(dir.path(), "/api/history").await;
    assert_eq!(body["steps"].as_array().unwrap().len(), 2);

    // a step whose record changed since is not undone
    assert_eq!(save("Halfling").await.0, StatusCode::OK);
    let (_, body) = get(dir.path(), "/api/history").await;
    let first = body["steps"].as_array().unwrap().last().unwrap()["step"].clone();
    let (status, body) = send(
        dir.path(),
        "POST",
        "/api/undo",
        serde_json::json!({"step": first}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("changed since"),
        "{body}"
    );
    assert_eq!(name().await, "Halfling");
}

#[tokio::test]
async fn write_dat_previews_then_writes_the_wcc_files() {
    let tmp = tempfile::tempdir().unwrap();
    let wcc = tmp.path().join("wcc");
    let realm = tmp.path().join("realm");
    std::fs::create_dir(&wcc).unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../daydream-wcc/tests/fixtures/btrieve/MBBSEMU.DAT");
    std::fs::copy(&fixture, wcc.join("MBBSEMU.DAT")).unwrap();
    daydream_store::realm::convert_dir(&wcc, &realm).unwrap();
    WbtrvDb::open(&realm.join("MBBSEMU.db"))
        .unwrap()
        .delete(2)
        .unwrap();
    let before = std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap();
    let run = |dry_run: bool| {
        send(
            &realm,
            "POST",
            "/api/tools/write-dat",
            serde_json::json!({"wcc": wcc.display().to_string(), "dry_run": dry_run}),
        )
    };
    let (status, body) = run(true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["files"][0]["action"], "rebuilt");
    assert_eq!(body["files"][0]["records"], 3);
    // only the realm's edits go in, onto the file as it is
    assert_eq!(body["files"][0]["merged"], true);
    assert_eq!(body["files"][0]["skipped"], serde_json::json!([]));
    assert_eq!(std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap(), before);
    let (status, body) = run(false).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_ne!(std::fs::read(wcc.join("MBBSEMU.DAT")).unwrap(), before);
    let backup = std::path::PathBuf::from(body["backup"].as_str().unwrap());
    assert_eq!(std::fs::read(backup.join("MBBSEMU.DAT")).unwrap(), before);
    let (_, body) = run(false).await;
    assert_eq!(body["files"][0]["action"], "unchanged");
    let (status, _) = send(
        &realm,
        "POST",
        "/api/tools/write-dat",
        serde_json::json!({"wcc": tmp.path().join("nowhere").display().to_string()}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn mcp_answers_over_http_from_this_machine_only() {
    let dir = realm();
    let mcp = |origin: Option<&str>, body: Value| {
        let mut req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json");
        if let Some(o) = origin {
            req = req.header("origin", o);
        }
        let req = req.body(Body::from(body.to_string())).unwrap();
        let path = dir.path().to_path_buf();
        async move {
            let res = app(&path).oneshot(req).await.unwrap();
            let status = res.status();
            let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
            (
                status,
                serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null),
            )
        }
    };
    let init = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}});
    let (status, body) = mcp(Some("http://localhost:7340"), init.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["serverInfo"]["name"], "daydream");
    let (status, _) = mcp(
        None,
        serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, _) = mcp(Some("https://evil.example"), init).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let tools = serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "list_tables", "arguments": {}}});
    let (_, body) = mcp(None, tools).await;
    assert_eq!(body["result"]["isError"], false);
}

/// A request through a router, its status, headers and body.
async fn through(
    router: axum::Router,
    req: Request<Body>,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let res = router.oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, headers, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn without_a_token_only_this_machine_is_answered() {
    let dir = realm();
    let at = |host: &str, extra: Option<(&str, &str)>| {
        let mut req = Request::get("/api/realm").header("host", host);
        if let Some((k, v)) = extra {
            req = req.header(k, v);
        }
        through(app(dir.path()), req.body(Body::empty()).unwrap())
    };
    assert_eq!(at("127.0.0.1:7340", None).await.0, StatusCode::OK);
    assert_eq!(at("localhost:7340", None).await.0, StatusCode::OK);
    // DNS rebinding: a page's own host name pointed at this machine
    assert_eq!(at("evil.example", None).await.0, StatusCode::FORBIDDEN);
    // a tunnel or proxy in front of a token-less server
    let (status, _, body) = at("127.0.0.1:7340", Some(("x-forwarded-for", "203.0.113.9"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.contains("--token"), "{body}");
    let cf = at("localhost", Some(("cf-connecting-ip", "203.0.113.9"))).await;
    assert_eq!(cf.0, StatusCode::FORBIDDEN);
    // no secret URL without a token
    let req = Request::post("/mcp/0123456789abcdef")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#))
        .unwrap();
    let keyed = through(app(dir.path()), req).await;
    assert_eq!(keyed.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_token_lets_in_whoever_has_it() {
    let dir = realm();
    let token = "0123456789abcdef-token";
    let router = || {
        daydream_server::router_with(
            Some(dir.path()),
            daydream_server::Options {
                token: Some(token.into()),
                ..daydream_server::Options::default()
            },
        )
    };
    let remote = |uri: &str| {
        Request::get(uri)
            .header("host", "daydream.example.com")
            .header("x-forwarded-for", "203.0.113.9")
    };
    let realm_with = |k: &str, v: &str| remote("/api/realm").header(k, v).body(Body::empty());

    // nothing without it
    let (status, headers, _) =
        through(router(), remote("/api/realm").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(headers["www-authenticate"], "Bearer");
    let wrong = through(
        router(),
        realm_with("authorization", "Bearer nope").unwrap(),
    )
    .await;
    assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);
    // the page sends a stranger to sign in; its files and the health check stay open
    let (status, headers, _) = through(router(), remote("/").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers["location"], "/login");
    for open in ["/login", "/app.js", "/api/health"] {
        let (status, _, _) = through(router(), remote(open).body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK, "{open}");
    }

    // a header
    let bearer = format!("Bearer {token}");
    let ok = through(router(), realm_with("authorization", &bearer).unwrap()).await;
    assert_eq!(ok.0, StatusCode::OK);

    // signing in sets the cookie, which then lets the web UI in
    let login = |t: &str| {
        Request::post("/login")
            .header("host", "daydream.example.com")
            .header("x-forwarded-proto", "https")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("token={t}")))
            .unwrap()
    };
    let (status, _, body) = through(router(), login("nope-nope-nope-nope")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("not this Daydream"), "{body}");
    let (status, headers, _) = through(router(), login(token)).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let cookie = headers["set-cookie"].to_str().unwrap();
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"),
        "{cookie}"
    );
    assert!(cookie.contains("Secure"), "{cookie}");
    let pair = cookie.split(';').next().unwrap();
    let ok = through(
        router(),
        realm_with("cookie", &format!("theme=dark; {pair}")).unwrap(),
    )
    .await;
    assert_eq!(ok.0, StatusCode::OK);
    let (status, _, _) = through(
        router(),
        remote("/")
            .header("cookie", pair)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // MCP: a header from any origin (the token stands in for the origin check), or the URL
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#;
    let mcp = |uri: String, auth: Option<&str>| {
        let mut req = Request::post(uri)
            .header("host", "daydream.example.com")
            .header("origin", "https://chatgpt.com")
            .header("content-type", "application/json");
        if let Some(a) = auth {
            req = req.header("authorization", a);
        }
        req.body(Body::from(init)).unwrap()
    };
    let (status, _, body) = through(router(), mcp("/mcp".into(), Some(&bearer))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("daydream"), "{body}");
    let (status, _, body) = through(router(), mcp(format!("/mcp/{token}"), None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("serverInfo"), "{body}");
    let (status, _, _) = through(router(), mcp("/mcp/0123456789abcdef-nope".into(), None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = through(router(), mcp("/mcp".into(), None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// Every route the router has, read from its source so a new route is swept too, with its
/// `{parameters}` filled in.
fn every_route() -> Vec<String> {
    let src = include_str!("../src/lib.rs");
    let mut routes = Vec::new();
    for (i, _) in src.match_indices(".route(") {
        let rest = src[i + ".route(".len()..].trim_start();
        let path = rest.strip_prefix('"').unwrap().split('"').next().unwrap();
        let filled: Vec<String> = path
            .split('/')
            .map(|seg| {
                if seg.starts_with('{') {
                    "x".to_string()
                } else {
                    seg.to_string()
                }
            })
            .collect();
        routes.push(filled.join("/"));
    }
    assert!(routes.len() > 30, "found only {routes:?}");
    assert!(routes.iter().any(|r| r == "/api/tools/write-dat"));
    routes
}

const METHODS: [&str; 4] = ["GET", "POST", "PUT", "DELETE"];

/// Without a token, no route answers anything but this machine, by any method: not a page
/// whose host name was pointed here, not a proxy or tunnel, not a request naming no host.
#[tokio::test]
async fn without_a_token_every_route_refuses_other_machines() {
    let dir = realm();
    let strangers: [&[(&str, &str)]; 5] = [
        &[("host", "evil.example")],
        &[("host", "localhost.evil.example:7340")],
        &[
            ("host", "127.0.0.1:7340"),
            ("x-forwarded-for", "203.0.113.9"),
        ],
        &[("host", "localhost"), ("forwarded", "for=203.0.113.9")],
        &[],
    ];
    for route in every_route() {
        for method in METHODS {
            for headers in strangers {
                let mut req = Request::builder()
                    .method(method)
                    .uri(&route)
                    .header("content-type", "application/json");
                for (k, v) in headers {
                    req = req.header(*k, *v);
                }
                let req = req.body(Body::from("{}")).unwrap();
                let (status, _, _) = through(daydream_server::router(dir.path()), req).await;
                assert_eq!(
                    status,
                    StatusCode::FORBIDDEN,
                    "{method} {route} with {headers:?}"
                );
            }
        }
    }
}

/// With a token, no route but the page's files, the sign-in page and the health check
/// answers a request without it, or with a wrong one.
#[tokio::test]
async fn with_a_token_every_route_asks_for_it() {
    let dir = realm();
    let token = "0123456789abcdef-token";
    // the page's own files, which hold nothing of the realm, load before signing in
    let open = [
        "/app.js",
        "/app.css",
        "/fonts/x",
        "/notices.txt",
        "/login",
        "/api/health",
    ];
    let wrong: [&[(&str, &str)]; 4] = [
        &[],
        &[("authorization", "Bearer 0123456789abcdef-wrong")],
        &[("authorization", "Basic MDEyMzQ1Njc4OWFiY2RlZi10b2tlbg==")],
        &[("cookie", "daydream_token=0123456789abcdef-wrong")],
    ];
    for route in every_route() {
        if open.contains(&route.as_str()) {
            continue;
        }
        for method in METHODS {
            for headers in wrong {
                let mut req = Request::builder()
                    .method(method)
                    .uri(&route)
                    .header("host", "daydream.example.com")
                    .header("content-type", "application/json");
                for (k, v) in headers {
                    req = req.header(*k, *v);
                }
                let req = req.body(Body::from("{}")).unwrap();
                let router = daydream_server::router_with(
                    Some(dir.path()),
                    daydream_server::Options {
                        token: Some(token.into()),
                        ..daydream_server::Options::default()
                    },
                );
                let (status, _, _) = through(router, req).await;
                let expected = if route == "/" {
                    StatusCode::SEE_OTHER
                } else {
                    StatusCode::UNAUTHORIZED
                };
                assert_eq!(status, expected, "{method} {route} with {headers:?}");
            }
        }
    }
}
