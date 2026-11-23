//! NMR's lists, read from the realm (nothing is written).
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/reports/limited-items?rooms=&shops=&users=` | every limited item and where its copies are |
//! | `GET /api/reports/find-item?item=&rooms=&shops=&users=` | every place one item is |
//! | `GET /api/reports/control-rooms` | each control room and the rooms that name it |
//! | `GET /api/reports/monster-groups` | each monster group and index, and its monsters |
//! | `GET /api/reports/npc-rooms` | each room's permanent NPC |
//!
//! `rooms`, `shops` and `users` default to true.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use daydream_nmr::reports::{self, Scan};
use daydream_store::Realm;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking};

fn yes() -> bool {
    true
}

/// Where to look, as the query gives it.
#[derive(Debug, Deserialize)]
pub struct ScanQuery {
    #[serde(default = "yes")]
    rooms: bool,
    #[serde(default = "yes")]
    shops: bool,
    #[serde(default = "yes")]
    users: bool,
    #[serde(default)]
    item: Option<i64>,
}

impl ScanQuery {
    fn scan(&self) -> Scan {
        Scan {
            rooms: self.rooms,
            shops: self.shops,
            users: self.users,
        }
    }
}

fn failed(e: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// Runs `f` on the realm off the async threads.
async fn read(
    s: Arc<AppState>,
    f: impl FnOnce(&Realm, daydream_types::records::LayoutSet) -> Result<Value, ApiError>
    + Send
    + 'static,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let realm = Realm::open(&s.realm_dir)?;
        let layout = daydream_nmr::layout_of_realm(&realm);
        f(&realm, layout).map(Json)
    })
    .await
}

pub(crate) async fn limited_items(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ScanQuery>,
) -> Result<Json<Value>, ApiError> {
    read(s, move |realm, layout| {
        let items = reports::limited_items(realm, layout, q.scan()).map_err(failed)?;
        Ok(json!({ "items": items }))
    })
    .await
}

pub(crate) async fn find_item(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ScanQuery>,
) -> Result<Json<Value>, ApiError> {
    let Some(item) = q.item else {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "give the item: ?item=".into(),
        ));
    };
    read(s, move |realm, layout| {
        let places = reports::find_item(realm, layout, item, q.scan()).map_err(failed)?;
        Ok(json!({ "item": item, "places": places }))
    })
    .await
}

pub(crate) async fn control_rooms(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    read(s, |realm, layout| {
        Ok(json!({ "rooms": reports::control_rooms(realm, layout).map_err(failed)? }))
    })
    .await
}

pub(crate) async fn monster_groups(
    State(s): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    read(s, |realm, layout| {
        Ok(json!({ "groups": reports::monster_groups(realm, layout).map_err(failed)? }))
    })
    .await
}

pub(crate) async fn npc_rooms(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    read(s, |realm, layout| {
        Ok(json!({ "rooms": reports::npc_rooms(realm, layout).map_err(failed)? }))
    })
    .await
}
