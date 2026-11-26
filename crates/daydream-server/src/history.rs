//! The open realm's history of saves: each save a step, undone and redone as a whole
//! ([`daydream_store::history`]).
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/history?limit=` | `{steps: [{step, at, label, records, state}], undo, redo}` |
//! | `GET /api/history/{step}` | the step, and its records: `{stem, id, table, how, what, name, members}` |
//! | `POST /api/undo` | `{step?}`: undoes the newest step (or that one), and answers it |
//! | `POST /api/redo` | `{step?}`: redoes the step undone last (or that one), and answers it |
//!
//! Steps come newest first; `state` is `done`, `undone`, or `too_big` (too many records to
//! keep for undo); `undo` and `redo` are the steps the next undo and redo take, or `null`.
//! An undo is refused (409) when a record of the step changed since, a redo when one changed
//! since the undo.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::StatusCode;
use daydream_store::Realm;
use daydream_store::history::{self, HistoryError, Step};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking, not_found};

fn step_json(s: &Step) -> Value {
    json!({
        "step": s.step,
        "at": s.at,
        "label": s.label,
        "records": s.records,
        "state": s.state.name(),
    })
}

/// `?limit=`.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    limit: Option<usize>,
}

pub(crate) async fn list(
    State(s): State<Arc<AppState>>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let dir = s.realm_dir()?;
        let limit = q.limit.unwrap_or(50).clamp(1, history::KEEP as usize);
        let steps = history::list(&dir, limit)?;
        let (undo, redo) = history::next(&dir)?;
        Ok(Json(json!({
            "steps": steps.iter().map(step_json).collect::<Vec<_>>(),
            "undo": undo.as_ref().map(step_json),
            "redo": redo.as_ref().map(step_json),
        })))
    })
    .await
}

pub(crate) async fn step(
    State(s): State<Arc<AppState>>,
    UrlPath(n): UrlPath<i64>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let dir = s.realm_dir()?;
        let step = history::step(&dir, n)?.ok_or_else(|| not_found(format!("step {n}")))?;
        let realm = Realm::open(&dir)?;
        let set = realm.layout_set();
        let records: Vec<Value> = history::records(&dir, n)?
            .iter()
            .map(|r| {
                let table = realm.file(&r.stem).and_then(|f| f.table);
                let shown = r
                    .after
                    .as_deref()
                    .or(r.before.as_deref())
                    .unwrap_or_default();
                let (what, name) = daydream_store::changes::label(table, set, r.id, shown);
                let how = match (&r.before, &r.after) {
                    (None, _) => "added",
                    (_, None) => "deleted",
                    _ => "changed",
                };
                let members = match (&r.before, &r.after) {
                    (Some(b), Some(a)) => daydream_store::three_way::changed_members(
                        table.map(|t| daydream_store::realm::layout_of(t, set)),
                        b,
                        a,
                    ),
                    _ => Vec::new(),
                };
                json!({
                    "stem": r.stem,
                    "id": r.id,
                    "table": table.map(|t| t.slug()),
                    "how": how,
                    "what": what,
                    "name": name,
                    "members": members,
                })
            })
            .collect();
        let mut out = step_json(&step);
        out["changes"] = json!(records);
        Ok(Json(out))
    })
    .await
}

/// `POST /api/undo` and `/api/redo`.
#[derive(Debug, Default, Deserialize)]
pub struct StepJson {
    step: Option<i64>,
}

fn refused(e: HistoryError) -> ApiError {
    match e {
        HistoryError::Refused(why) => ApiError(StatusCode::CONFLICT, why),
        HistoryError::Store(e) => e.into(),
    }
}

async fn run(
    s: Arc<AppState>,
    body: Option<Json<StepJson>>,
    redo: bool,
) -> Result<Json<Value>, ApiError> {
    let step = body.and_then(|b| b.0.step);
    blocking(move || {
        let (_writing, dir) = s.lock_write_unrecorded()?;
        let realm = Realm::open(&dir)?;
        let done = if redo {
            history::redo(&realm, step)
        } else {
            history::undo(&realm, step)
        }
        .map_err(refused)?;
        Ok(Json(step_json(&done)))
    })
    .await
}

pub(crate) async fn undo(
    State(s): State<Arc<AppState>>,
    body: Option<Json<StepJson>>,
) -> Result<Json<Value>, ApiError> {
    run(s, body, false).await
}

pub(crate) async fn redo(
    State(s): State<Arc<AppState>>,
    body: Option<Json<StepJson>>,
) -> Result<Json<Value>, ApiError> {
    run(s, body, true).await
}
