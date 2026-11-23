//! NMR's Export to MMUD Explorer in the browser, over the same library call
//! (`daydream_nmr::mme::export`) the command line's `daydream mme-export` makes, so the
//! browser writes exactly what NMR's export writes and nothing more.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/mme` | `{jet, dat_version, defaults: [{map, from, to, name}]}` |
//! | `POST /api/mme/export` | `{options, format}`: the export, as `format` asks |
//! | `GET /api/mme/write-mdb.ps1` | the script that turns a plan into the `.mdb` on Windows |
//!
//! `options` is a [`MmeOptions`] (NMR's boxes; `date` and `time` are set here, as NMR sets
//! them from the clock). `format` is `summary` (each table's row count, as JSON), `plan`
//! (the plan `write-mdb.ps1` turns into the `.mdb`, for a machine without Jet) or `mdb`
//! (the `.mdb` itself, written through Windows' Jet; `jet` says whether this machine has
//! it). The export only reads the realm.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use daydream_nmr::mme::{DEFAULT_EXCLUDED, MmeOptions};
use daydream_store::Realm;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking};

/// What the export form needs before it is filled in.
pub(crate) async fn info() -> Json<Value> {
    let defaults: Vec<Value> = DEFAULT_EXCLUDED
        .iter()
        .map(|&(map, from, to, name)| json!({ "map": map, "from": from, "to": to, "name": name }))
        .collect();
    Json(json!({
        "jet": daydream_nmr::jet::powershell32().is_some(),
        "dat_version": daydream_nmr::export::DAT_VERSION,
        "defaults": defaults,
    }))
}

/// What an export answers with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Summary,
    Plan,
    Mdb,
}

/// The body of an export.
#[derive(Debug, Deserialize)]
pub struct ExportJson {
    #[serde(default)]
    options: MmeOptions,
    #[serde(default)]
    format: Format,
}

fn attachment(bytes: Vec<u8>, content_type: &'static str, name: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        bytes,
    )
        .into_response()
}

pub(crate) async fn export(
    State(s): State<Arc<AppState>>,
    Json(body): Json<ExportJson>,
) -> Result<Response, ApiError> {
    blocking(move || {
        let realm = Realm::open(&s.realm_dir)?;
        let mut opts = body.options;
        // VB's Date and Time in the US format NMR's users mostly have, as the command line
        let now = jiff::Zoned::now();
        opts.date = now.strftime("%-m/%-d/%Y").to_string();
        opts.time = now.strftime("%-I:%M:%S %p").to_string();
        let data = daydream_nmr::mme::export(&realm, daydream_nmr::layout_of_realm(&realm), &opts)
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        match body.format {
            Format::Summary => {
                let tables: Vec<Value> = data
                    .tables
                    .iter()
                    .map(|t| json!({ "name": t.name, "rows": t.rows.len() }))
                    .collect();
                Ok(Json(json!({ "tables": tables })).into_response())
            }
            Format::Plan => Ok(attachment(
                daydream_nmr::mme::plan(&data).into_bytes(),
                "text/plain; charset=utf-8",
                "mme.plan",
            )),
            Format::Mdb => {
                let dir = tempfile::tempdir().map_err(internal)?;
                let out = dir.path().join("mme.mdb");
                daydream_nmr::jet::write_plan_mdb(&daydream_nmr::mme::plan(&data), &out).map_err(
                    |e| match e {
                        daydream_nmr::jet::JetError::NoJet => {
                            ApiError(StatusCode::NOT_IMPLEMENTED, e.to_string())
                        }
                        e => internal(e),
                    },
                )?;
                let bytes = std::fs::read(&out).map_err(internal)?;
                Ok(attachment(bytes, "application/x-msaccess", "mme.mdb"))
            }
        }
    })
    .await
}

/// `write-mdb.ps1`, with the BOM Windows PowerShell 5.1 needs to read it as UTF-8.
pub(crate) async fn script() -> Response {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(daydream_nmr::jet::SCRIPT.as_bytes());
    attachment(bytes, "text/plain; charset=utf-8", "write-mdb.ps1")
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
