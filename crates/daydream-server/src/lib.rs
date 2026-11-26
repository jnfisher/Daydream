//! The Daydream server: serves a realm's records over HTTP to the web UI.
//!
//! It runs on the sysop's own machine and opens in the browser. `/` is the web UI (built
//! from `web/` into `src/app.js` and embedded, so a cargo build needs no node). The UI's
//! editors work on NMR's tables through the routes in [`tables`], NMR's database tools are
//! in [`tools`], its lists in [`reports`], its Export to MMUD Explorer in [`mme`], the realm
//! in a silvermere database in [`silvermere`], choosing the game to work on in [`games`], the MCP
//! server for AI agents at `/mcp` ([`mcp`]), and the
//! routes below serve each file's records
//! decoded (every member by name, `unk_*` spans included), read only. Who may use it is
//! settled in [`auth`]: this machine only, unless it was given an access token.
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
use std::sync::{Arc, Mutex, RwLock};

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

pub mod auth;
pub mod games;
pub mod mcp;
pub mod mme;
pub mod reports;
pub mod silvermere;
pub mod tables;
pub mod tools;

/// The largest page of records one request returns.
pub const MAX_PAGE: i64 = 1000;

/// What the handlers share.
#[derive(Debug)]
pub struct AppState {
    /// The game open, and its realm: swapped when another is opened ([`games`]).
    current: RwLock<Arc<Current>>,
    /// Held by every write, so two saves never interleave (and no game is opened mid-save).
    write: Mutex<()>,
    /// The access token, when the server takes requests from other machines ([`auth`]).
    token: Option<String>,
    /// The silvermere database the server was given ([`silvermere`]), as a URL.
    silvermere: Option<String>,
    /// How games are found and opened.
    games: games::Config,
    /// The games found by the last search.
    found: Mutex<Option<Vec<games::Found>>>,
}

/// The game open and its realm.
#[derive(Debug)]
pub struct Current {
    /// The realm's folder; `None` until a game is opened.
    pub realm_dir: Option<PathBuf>,
    /// The game the realm is, when known.
    pub game: Option<games::Game>,
    /// The MCP server at `/mcp`, on this realm.
    mcp: Option<daydream_mcp::Server>,
}

impl AppState {
    /// The game open now.
    pub fn current(&self) -> Arc<Current> {
        self.current
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Opens another realm.
    fn set_current(&self, realm_dir: PathBuf, game: Option<games::Game>) {
        let silvermere = match &game {
            Some(games::Game {
                source: games::Source::Silvermere { url },
                ..
            }) => Some(url.clone()),
            _ => self.silvermere.clone(),
        };
        let mcp = daydream_mcp::Server::new(
            &realm_dir,
            daydream_mcp::Options {
                silvermere,
                status: Some(games::status_check(&realm_dir, game.clone())),
                batch_dirs: self.games.batch_dirs.clone(),
                ..daydream_mcp::Options::default()
            },
        );
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(Current {
            realm_dir: Some(realm_dir),
            game,
            mcp: Some(mcp),
        });
    }

    /// The open realm's folder, or a 409 asking for a game to be opened.
    pub(crate) fn realm_dir(&self) -> Result<PathBuf, ApiError> {
        self.current().realm_dir.clone().ok_or_else(|| {
            ApiError(
                StatusCode::CONFLICT,
                "no game is open: choose one with the game picker at the top".into(),
            )
        })
    }

    /// Takes the write lock on the open realm. Unless `dry_run`, refuses when the realm is a
    /// game's own files and the game is running, and when the lock is let go brings the
    /// realm's update file in step with what was written ([`WriteGuard`]).
    pub(crate) fn lock_write(&self, dry_run: bool) -> Result<(WriteGuard<'_>, PathBuf), ApiError> {
        let lock = self
            .write
            .lock()
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let cur = self.current();
        let dir = self.realm_dir()?;
        if !dry_run && let Some(game) = cur.game.as_ref().filter(|g| g.live) {
            let r = games::running::check(&dir);
            if let Some(by) = r.by {
                return Err(ApiError(
                    StatusCode::CONFLICT,
                    format!(
                        "{} is running on these files ({by}): stop it before saving, \
                         or its next write undoes yours",
                        game.label
                    ),
                ));
            }
        }
        let guard = WriteGuard {
            _lock: lock,
            dir: (!dry_run).then(|| dir.clone()),
        };
        Ok((guard, dir))
    }
}

/// The write lock on the open realm ([`AppState::lock_write`]). Let go after a write, it first
/// brings the realm's update file in step with the write ([`daydream_store::update::sync`]), so
/// the game's next start applies the edits instead of undoing them. A sync that fails is
/// logged and tried again after the next write (the journal keeps the edits until then), and
/// writing the realm back into the game's files syncs first and reports the failure.
pub(crate) struct WriteGuard<'a> {
    _lock: std::sync::MutexGuard<'a, ()>,
    /// The realm to sync; `None` for a dry run.
    dir: Option<PathBuf>,
}

impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        if let Some(dir) = &self.dir {
            match daydream_store::update::sync(dir) {
                Ok(r) if r.changed() => tracing::info!(
                    "update file {}: {} images replaced, {} added, {} removed",
                    r.file.as_deref().unwrap_or("?"),
                    r.replaced,
                    r.added,
                    r.removed
                ),
                Ok(_) => {}
                Err(e) => tracing::warn!("bringing the update file in step: {e}"),
            }
        }
    }
}

/// How the server is run.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The access token ([`auth`]). Without one, only this machine is answered.
    pub token: Option<String>,
    /// A silvermere database (a SQLite file's path or a URL) for the silvermere card and the
    /// MCP server's silvermere tools ([`silvermere`]).
    pub silvermere: Option<String>,
    /// How games are found and opened from the browser ([`games`]).
    pub games: games::Config,
}

/// The router for the realm in `realm_dir`, answering this machine only.
pub fn router(realm_dir: &Path) -> Router {
    router_with(Some(realm_dir), Options::default())
}

/// The router for the realm in `realm_dir`, or for none until a game is opened in the
/// browser.
pub fn router_with(realm_dir: Option<&Path>, options: Options) -> Router {
    let state = Arc::new(AppState {
        current: RwLock::new(Arc::new(Current {
            realm_dir: None,
            game: None,
            mcp: None,
        })),
        write: Mutex::new(()),
        token: options.token,
        silvermere: options
            .silvermere
            .as_deref()
            .map(daydream_silvermere::database_url),
        games: options.games,
        found: Mutex::new(None),
    });
    if let Some(dir) = realm_dir {
        state.set_current(dir.to_path_buf(), Some(games::Game::of_realm(dir)));
    }
    Router::new()
        .route("/", get(index))
        .route("/login", get(auth::login_form).post(auth::login))
        .route("/mcp/{token}", post(mcp::post_keyed).get(mcp::get))
        .route("/app.js", get(app_js))
        .route("/app.css", get(app_css))
        .route("/api/health", get(health))
        .route("/mcp", post(mcp::post).get(mcp::get))
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
        .route("/api/games", get(games::list))
        .route("/api/games/status", get(games::status))
        .route("/api/games/open", post(games::open))
        .route("/api/silvermere", get(silvermere::configured))
        .route("/api/silvermere/info", post(silvermere::info))
        .route("/api/silvermere/export", post(silvermere::export))
        .route("/api/silvermere/import", post(silvermere::import))
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
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::guard,
        ))
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

pub(crate) fn not_found(what: impl std::fmt::Display) -> ApiError {
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
        let dir = s.realm_dir()?;
        let realm = Realm::open(&dir)?;
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
        Ok(Json(json!({ "dir": dir, "files": files })))
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
        let (file, db, layout) = open_file(&s.realm_dir()?, &stem)?;
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
        let (file, db, layout) = open_file(&s.realm_dir()?, &stem)?;
        let data = db
            .get(id)?
            .ok_or_else(|| not_found(format!("{} record {id}", file.stem)))?;
        Ok(Json(to_json(&file, layout, id, &data)?))
    })
    .await
}
