//! The Daydream server: serves a realm's records over HTTP to the web UI.
//!
//! It runs on the sysop's own machine and opens in the browser. `/` is the web UI (built
//! from `web/` into `src/app.js` and embedded, so a cargo build needs no node). The UI's
//! editors work on NMR's tables through the routes in [`tables`], NMR's database tools are
//! in [`tools`], its lists in [`reports`], its Export to MMUD Explorer in [`mme`], and the
//! routes below serve each file's records
//! decoded (every member by name, `unk_*` spans included), read only.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/health` | `{"ok": true, "version": ...}` |
//! | `GET /api/realm` | the realm's files: stem, table, record count and length |
//! | `GET /api/files/{stem}/records?after=&limit=` | a page of records, decoded |
//! | `GET /api/files/{stem}/records/{id}` | one record, decoded |
//!
//! A record of a file Daydream does not decode (`WCCUPDAT`, ...) is served as its raw
//! bytes in base64.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::DefaultBodyLimit;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::StatusCode;
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use daydream_store::{Realm, RealmFile, StoreError, WbtrvDb};
use daydream_types::records::LayoutSet;
use daydream_wcc::decode::base64;
use serde::Deserialize;
use serde_json::{Value, json};

pub mod mme;
pub mod reports;
pub mod tables;
pub mod tools;

/// The largest page of records one request returns.
pub const MAX_PAGE: i64 = 1000;

/// What the handlers share.
#[derive(Debug)]
pub struct AppState {
    realm_dir: PathBuf,
    /// Held by every write, so two saves never interleave.
    write: Mutex<()>,
}

/// The router for the realm in `realm_dir`.
pub fn router(realm_dir: &Path) -> Router {
    let state = Arc::new(AppState {
        realm_dir: realm_dir.to_path_buf(),
        write: Mutex::new(()),
    });
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/app.css", get(app_css))
        .route("/api/health", get(health))
        .route("/api/tables", get(tables::tables))
        .route("/api/tables/{table}/rows", get(tables::rows))
        .route(
            "/api/tables/{table}/row",
            get(tables::row)
                .put(tables::update)
                .post(tables::insert)
                .delete(tables::remove),
        )
        .route("/api/realm", get(realm))
        .route("/api/files/{stem}/records", get(records))
        .route("/api/files/{stem}/records/{id}", get(record))
        .route(
            "/api/tools/import",
            post(tools::import).layer(DefaultBodyLimit::max(tools::MAX_UPLOAD)),
        )
        .route("/api/tools/delete", post(tools::delete))
        .route("/api/tools/modify", post(tools::modify))
        .route("/api/tools/preset", post(tools::preset))
        .route("/api/tools/renumber", post(tools::renumber))
        .route("/api/tools/write-dat", post(tools::write_dat))
        .route("/api/reports/limited-items", get(reports::limited_items))
        .route("/api/reports/find-item", get(reports::find_item))
        .route("/api/reports/control-rooms", get(reports::control_rooms))
        .route("/api/reports/monster-groups", get(reports::monster_groups))
        .route("/api/reports/npc-rooms", get(reports::npc_rooms))
        .route("/api/reports/room-indexes", get(reports::room_indexes))
        .route("/api/map", get(reports::map))
        .route("/api/map/dig", post(tools::dig))
        .route("/api/quests", get(reports::quests))
        .route("/api/mme", get(mme::info))
        .route("/api/mme/export", post(mme::export))
        .route("/api/mme/write-mdb.ps1", get(mme::script))
        .route(
            "/api/tools/merge-users",
            post(tools::merge_users).layer(DefaultBodyLimit::max(tools::MAX_UPLOAD)),
        )
        .with_state(state)
}

/// An error answer: a status and a message.
#[derive(Debug)]
pub struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

fn not_found(what: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::NOT_FOUND, format!("{what} not found"))
}

/// Runs blocking store work off the async runtime.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
}

async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn app_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("app.js"),
    )
}

async fn app_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("app.css"),
    )
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn realm(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let realm = Realm::open(&s.realm_dir)?;
        let mut files = Vec::new();
        for f in realm.files() {
            let db = WbtrvDb::open_read_only(&f.path)?;
            let meta = db.metadata();
            files.push(json!({
                "stem": f.stem,
                "table": f.table.map(|t| t.slug()),
                "records": db.count()?,
                "record_length": meta.record_length,
                "variable_length": meta.variable_length_records,
            }));
        }
        Ok(Json(json!({ "dir": s.realm_dir, "files": files })))
    })
    .await
}

/// A realm file, opened to read, and the layouts of the realm's records.
fn open_file(realm_dir: &Path, stem: &str) -> Result<(RealmFile, WbtrvDb, LayoutSet), ApiError> {
    let realm = Realm::open(realm_dir)?;
    let file = realm
        .file(stem)
        .cloned()
        .ok_or_else(|| not_found(format!("file {stem}")))?;
    let db = WbtrvDb::open_read_only(&file.path)?;
    Ok((file, db, daydream_nmr::layout_of_realm(&realm)))
}

fn to_json(file: &RealmFile, layout: LayoutSet, id: i64, data: &[u8]) -> Result<Value, ApiError> {
    match file.table {
        Some(table) => {
            let ordinal = u32::try_from(id).unwrap_or(u32::MAX);
            let fields = table.to_json_as(ordinal, data, layout).map_err(|e| {
                ApiError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("{} record {id}: {e}", file.stem),
                )
            })?;
            Ok(Value::Object(fields))
        }
        None => Ok(json!({ "_id": id, "_raw_len": data.len(), "_raw": base64(data) })),
    }
}

/// `?after=&limit=`.
#[derive(Debug, Deserialize)]
pub struct PageQuery {
    #[serde(default)]
    after: i64,
    limit: Option<i64>,
}

async fn records(
    State(s): State<Arc<AppState>>,
    UrlPath(stem): UrlPath<String>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let (file, db, layout) = open_file(&s.realm_dir, &stem)?;
        let limit = q.limit.unwrap_or(100).clamp(1, MAX_PAGE);
        let page = db.page(q.after, limit)?;
        let next = if page.len() as i64 == limit {
            page.last().map(|(id, _)| *id)
        } else {
            None
        };
        let records = page
            .iter()
            .map(|(id, data)| to_json(&file, layout, *id, data))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Json(json!({
            "stem": file.stem,
            "table": file.table.map(|t| t.slug()),
            "records": records,
            "next_after": next,
        })))
    })
    .await
}

async fn record(
    State(s): State<Arc<AppState>>,
    UrlPath((stem, id)): UrlPath<(String, i64)>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let (file, db, layout) = open_file(&s.realm_dir, &stem)?;
        let data = db
            .get(id)?
            .ok_or_else(|| not_found(format!("{} record {id}", file.stem)))?;
        Ok(Json(to_json(&file, layout, id, &data)?))
    })
    .await
}
