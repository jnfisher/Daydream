//! The Export to MMUD Explorer routes over a small realm: the summary and the plan are the
//! library's export with the form's options, and a machine without Jet says so.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use daydream_nmr::Value as V;
use daydream_nmr::columns::TABLES;
use daydream_nmr::schema::layout_of;
use daydream_store::{Metadata, Realm, WbtrvDb};
use daydream_types::records::LayoutSet;
use daydream_wcc::tables::FILES;
use serde_json::{Value, json};
use tower::ServiceExt;

/// An empty `.db` for every NMR table, then two rooms on map 1 (one of them NMR's default
/// exclusion 1/164) and one on map 2.
fn realm() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for t in TABLES.iter().chain(daydream_nmr::players::TABLES) {
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
        WbtrvDb::create(&dir.path().join(format!("{stem}.db")), meta, Vec::new()).unwrap();
    }
    let realm = Realm::open(dir.path()).unwrap();
    let rooms = daydream_nmr::schema::table("Rooms").unwrap();
    let given = [
        (1, 10, "town square"),
        (1, 164, "sysop support chamber"),
        (2, 5, "my test room"),
    ];
    let rows = given
        .iter()
        .map(|&(map, room, name)| {
            rooms
                .columns
                .iter()
                .map(|c| match c.name {
                    "Map Number" => V::Long(map),
                    "Room Number" => V::Long(room),
                    "Name" => V::Text(name.into()),
                    _ => match c.ty {
                        daydream_nmr::schema::ColumnType::Long => V::Long(0),
                        daydream_nmr::schema::ColumnType::Double => V::Double(0.0),
                        _ => V::Text(String::new()),
                    },
                })
                .collect()
        })
        .collect();
    let data = daydream_nmr::NmrData {
        tables: vec![daydream_nmr::NmrRows {
            name: rooms.name.into(),
            columns: rooms.columns.iter().map(|c| c.name.to_string()).collect(),
            rows,
        }],
    };
    let report = daydream_nmr::import(&realm, &data, &Default::default()).unwrap();
    assert!(report.iter().all(|r| r.errors.is_empty()), "{report:?}");
    dir
}

async fn send(dir: &std::path::Path, req: Request<Body>) -> (StatusCode, String, Vec<u8>) {
    let res = daydream_server::router(dir).oneshot(req).await.unwrap();
    let status = res.status();
    let disposition = res
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, disposition, body.to_vec())
}

async fn export(dir: &std::path::Path, body: Value) -> (StatusCode, String, Vec<u8>) {
    let req = Request::post("/api/mme/export")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    send(dir, req).await
}

fn rooms_in(summary: &[u8]) -> u64 {
    let v: Value = serde_json::from_slice(summary).unwrap();
    v["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "Rooms")
        .unwrap()["rows"]
        .as_u64()
        .unwrap()
}

#[tokio::test]
async fn info_lists_nmrs_default_exclusions() {
    let dir = realm();
    let (status, _, body) = send(
        dir.path(),
        Request::get("/api/mme").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["dat_version"], "v1.11p");
    assert_eq!(v["defaults"].as_array().unwrap().len(), 20);
    assert_eq!(
        v["defaults"][4],
        json!({"map": 1, "from": 164, "to": 164, "name": "sysop support chamber"})
    );
    assert_eq!(v["jet"], daydream_nmr::jet::powershell32().is_some());
}

#[tokio::test]
async fn the_summary_follows_the_options() {
    let dir = realm();
    // Legit (the default): NMR's default exclusions leave 1/164 out
    let (status, _, body) = export(dir.path(), json!({ "format": "summary" })).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(rooms_in(&body), 2);

    // Legit ignores the sysop's own list
    let mine = json!({ "exclude_rooms": true, "excluded": [{"map": 2, "from": 1, "to": 9}] });
    let (_, _, body) = export(dir.path(), json!({ "format": "summary", "options": mine })).await;
    assert_eq!(rooms_in(&body), 2);

    // not Legit: the sysop's own list counts, and the defaults can be turned off
    let mine = json!({
        "legit": false, "exclude_default": false,
        "exclude_rooms": true, "excluded": [{"map": 2, "from": 1, "to": 9}],
    });
    let (_, _, body) = export(dir.path(), json!({ "format": "summary", "options": mine })).await;
    assert_eq!(rooms_in(&body), 2);
    let mine = json!({ "legit": false, "exclude_default": false });
    let (_, _, body) = export(dir.path(), json!({ "format": "summary", "options": mine })).await;
    assert_eq!(rooms_in(&body), 3);
}

#[tokio::test]
async fn the_plan_is_the_librarys() {
    let dir = realm();
    let (status, disposition, body) = export(dir.path(), json!({ "format": "plan" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(disposition, "attachment; filename=\"mme.plan\"");
    let plan = String::from_utf8(body).unwrap();
    // the plan the library writes, with the date and time the server took from its clock
    let info = plan
        .lines()
        .find(|l| l.starts_with("R\tSv1."))
        .expect("the Info row");
    let fields: Vec<&str> = info.split('\t').collect();
    let o = daydream_nmr::mme::MmeOptions {
        date: fields[3][1..].to_string(),
        time: fields[4][1..].to_string(),
        ..Default::default()
    };
    assert!(o.date.contains('/') && o.time.ends_with('M'), "{info}");
    let realm = Realm::open(dir.path()).unwrap();
    let ours = daydream_nmr::mme::export(&realm, LayoutSet::Dos, &o).unwrap();
    assert_eq!(plan, daydream_nmr::mme::plan(&ours));
    assert!(plan.contains("town square"));
    assert!(!plan.contains("sysop support chamber"));
}

#[tokio::test]
async fn the_script_downloads_with_a_bom() {
    let dir = realm();
    let (status, disposition, body) = send(
        dir.path(),
        Request::get("/api/mme/write-mdb.ps1")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(disposition, "attachment; filename=\"write-mdb.ps1\"");
    assert_eq!(&body[..3], [0xEF, 0xBB, 0xBF]);
    assert_eq!(&body[3..], daydream_nmr::jet::SCRIPT.as_bytes());
}

#[tokio::test]
async fn without_jet_an_mdb_is_refused() {
    if daydream_nmr::jet::powershell32().is_some() {
        return; // Windows writes it; the Jet job tests that path
    }
    let dir = realm();
    let (status, _, body) = export(dir.path(), json!({ "format": "mdb" })).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert!(String::from_utf8_lossy(&body).contains("Jet"));
}
