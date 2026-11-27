//! The realm in a silvermere database (`daydream silvermere-export`, `silvermere-import`):
//! the Workshop's silvermere page.
//!
//! | Route | Does |
//! |---|---|
//! | `GET /api/silvermere` | the open silvermere game's database, else the one `serve --silvermere` was given (password masked), if any; and whether another may be typed |
//! | `POST /api/silvermere/info` | `{database?}`: what the database holds, and whether silvermere runs on it |
//! | `POST /api/silvermere/export` | `{database?, dry_run, reset_state, keep_extra, no_raw}`: writes the realm's world into it |
//! | `POST /api/silvermere/import` | `{database?, dry_run, keep_extra}`: rewrites the realm's world from it, as one step of its history (`step`) |
//!
//! `database` is a SQLite file's path or a `sqlite:`/`postgres:` URL; left out, the open
//! game's when it is a silvermere one, else the one the server was started with. With an
//! access token only a database Daydream knows may be named ([`crate::games`]). silvermere must be stopped: its writer lock is taken first, and a
//! running silvermere is a 409.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use daydream_silvermere::{Error, ExportOptions, ImportOptions, State as LiveState};
use daydream_store::Realm;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking};

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        let status = match e {
            Error::Running { .. } | Error::Schema { .. } | Error::NoDatabase(_) => {
                StatusCode::CONFLICT
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiError(status, e.to_string())
    }
}

fn database(s: &AppState, given: Option<String>) -> Result<String, ApiError> {
    s.silvermere_url(given)
}

pub(crate) async fn configured(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "database": s.silvermere_url(None).ok().as_deref().map(daydream_silvermere::redact_url),
        "custom": s.custom_allowed(),
    }))
}

#[derive(Debug, Deserialize)]
pub(crate) struct InfoJson {
    #[serde(default)]
    database: Option<String>,
}

pub(crate) async fn info(
    State(s): State<Arc<AppState>>,
    Json(body): Json<InfoJson>,
) -> Result<Json<Value>, ApiError> {
    let url = database(&s, body.database)?;
    blocking(move || {
        let info = daydream_silvermere::block_on(daydream_silvermere::info(&url))?;
        Ok(Json(
            serde_json::to_value(info).map_err(crate::tools::internal)?,
        ))
    })
    .await
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExportJson {
    #[serde(default)]
    database: Option<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    reset_state: bool,
    #[serde(default)]
    keep_extra: bool,
    #[serde(default)]
    no_raw: bool,
}

pub(crate) async fn export(
    State(s): State<Arc<AppState>>,
    Json(body): Json<ExportJson>,
) -> Result<Json<Value>, ApiError> {
    let url = database(&s, body.database)?;
    blocking(move || {
        let opts = ExportOptions {
            state: if body.reset_state {
                LiveState::Reset
            } else {
                LiveState::Keep
            },
            delete: !body.keep_extra,
            raw: !body.no_raw,
            dry_run: body.dry_run,
            ..ExportOptions::default()
        };
        // the realm is only read, but not while a save is half done
        let (_writing, dir) = s.lock_write(true, "silvermere")?;
        let realm = Realm::open(&dir)?;
        let tables =
            daydream_silvermere::block_on(daydream_silvermere::export(&realm, &url, &opts))?;
        Ok(Json(json!({ "dry_run": body.dry_run, "tables": tables })))
    })
    .await
}

#[derive(Debug, Deserialize)]
pub(crate) struct ImportJson {
    #[serde(default)]
    database: Option<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    keep_extra: bool,
}

pub(crate) async fn import(
    State(s): State<Arc<AppState>>,
    Json(body): Json<ImportJson>,
) -> Result<Json<Value>, ApiError> {
    let url = database(&s, body.database)?;
    blocking(move || {
        let opts = ImportOptions {
            delete: !body.keep_extra,
            dry_run: body.dry_run,
            ..ImportOptions::default()
        };
        let (writing, dir) = s.lock_write(body.dry_run, "Read from silvermere")?;
        let realm = Realm::open(&dir)?;
        let tables =
            daydream_silvermere::block_on(daydream_silvermere::import(&realm, &url, &opts))?;
        drop(realm);
        let step = writing.finish();
        Ok(Json(
            json!({ "dry_run": body.dry_run, "tables": tables, "step": step }),
        ))
    })
    .await
}
