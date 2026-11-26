//! The open realm's changes: what it changed since it was read from its game or last written
//! back into it, which is what publishing writes ([`daydream_store::changes`]).
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/changes` | `{since, changes: [{stem, id, table, how, what, name, members, was, nmr, key}]}` |
//! | `POST /api/changes/revert` | `{changes: [{stem, id}]}` or `{all: true}`: `{reverted, failed}` |
//!
//! `since` is when the realm was read from its game or last written back (seconds since
//! 1970), or `null` for a realm edited before Daydream journaled its edits: its changes are
//! not known, and publishing writes it whole. `how` is `added`, `changed` or `deleted`;
//! `members` the members a changed record changed; `was` what a renumbered record was;
//! `nmr` and `key` the editors' table and key for the record (`Items`, `[868]`), or `null`
//! for a record no editor shows.
//! Taking a change back puts the record as it was, as a save does (the update file follows).

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use daydream_nmr::rows::{cell_json, record_key};
use daydream_store::Realm;
use daydream_store::changes::{self, Change};
use daydream_types::records::LayoutSet;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking};

fn change_json(c: &Change, set: LayoutSet) -> Value {
    let link = c.table.and_then(|t| record_key(t, set, &c.record));
    json!({
        "stem": c.stem,
        "id": c.id,
        "table": c.table.map(|t| t.slug()),
        "how": c.how.name(),
        "what": c.what,
        "name": c.name,
        "members": c.members,
        "was": c.was,
        "nmr": link.as_ref().map(|(t, _)| t.name),
        "key": link.map(|(_, key)| key.iter().map(cell_json).collect::<Vec<_>>()),
    })
}

pub(crate) async fn list(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let realm = Realm::open(&s.realm_dir()?)?;
        let c = changes::list(&realm)?;
        let set = realm.layout_set();
        Ok(Json(json!({
            "since": c.since,
            "changes": c.changes.iter().map(|c| change_json(c, set)).collect::<Vec<_>>(),
        })))
    })
    .await
}

/// One record, by file and id.
#[derive(Debug, Deserialize)]
pub struct RecordRef {
    stem: String,
    id: i64,
}

/// `POST /api/changes/revert`.
#[derive(Debug, Deserialize)]
pub struct RevertJson {
    #[serde(default)]
    changes: Vec<RecordRef>,
    #[serde(default)]
    all: bool,
}

pub(crate) async fn revert(
    State(s): State<Arc<AppState>>,
    Json(body): Json<RevertJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        if !body.all && body.changes.is_empty() {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "name the changes to take back, or all".into(),
            ));
        }
        let (_writing, dir) = s.lock_write(false, "Take back changes")?;
        let realm = Realm::open(&dir)?;
        let which: Vec<(String, i64)> = body.changes.into_iter().map(|r| (r.stem, r.id)).collect();
        let r = changes::revert(&realm, (!body.all).then_some(&which[..]))?;
        let failed: Vec<Value> = r
            .failed
            .iter()
            .map(|f| json!({ "stem": f.stem, "id": f.id, "why": f.why }))
            .collect();
        Ok(Json(json!({ "reverted": r.reverted, "failed": failed })))
    })
    .await
}
