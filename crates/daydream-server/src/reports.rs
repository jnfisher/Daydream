//! NMR's lists, read from the realm (nothing is written).
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/reports/limited-items?rooms=&shops=&users=` | every limited item and where its copies are |
//! | `GET /api/reports/find-item?item=&rooms=&shops=&users=` | every place one item is |
//! | `GET /api/reports/control-rooms` | each control room and the rooms that name it |
//! | `GET /api/reports/monster-groups` | each monster group and index, and its monsters |
//! | `GET /api/reports/npc-rooms` | each room's permanent NPC |
//! | `GET /api/quests?file=default\|custom` | the realm folder's `NMR-Quests.txt` or `NMR-QuestsCustom.txt`, as text |
//! | `GET /api/reports/room-indexes` | the monster index runs rooms' ranges cover, by group |
//! | `GET /api/map?map=` | one map's rooms and exits, and the realm's maps (map 1 when none is given) |
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

pub(crate) async fn room_indexes(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    read(s, |realm, layout| {
        Ok(json!({ "runs": reports::room_index_runs(realm, layout).map_err(failed)? }))
    })
    .await
}

pub(crate) async fn npc_rooms(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    read(s, |realm, layout| {
        Ok(json!({ "rooms": reports::npc_rooms(realm, layout).map_err(failed)? }))
    })
    .await
}

/// The map asked for.
#[derive(Debug, Deserialize)]
pub struct MapQuery {
    #[serde(default)]
    map: Option<i64>,
}

pub(crate) async fn map(
    State(s): State<Arc<AppState>>,
    Query(q): Query<MapQuery>,
) -> Result<Json<Value>, ApiError> {
    let map = q.map.unwrap_or(1);
    read(s, move |realm, layout| {
        let view = daydream_nmr::map::map_view(realm, layout, map).map_err(failed)?;
        Ok(json!({ "map": map, "maps": view.maps, "rooms": view.rooms }))
    })
    .await
}

/// Which quest file.
#[derive(Debug, Deserialize)]
pub struct QuestQuery {
    #[serde(default)]
    file: Option<String>,
}

/// NMR's quest files, read from the realm's folder (NMR reads them from its own).
pub(crate) async fn quests(
    State(s): State<Arc<AppState>>,
    Query(q): Query<QuestQuery>,
) -> Result<Json<Value>, ApiError> {
    let name = match q.file.as_deref().unwrap_or("default") {
        "default" => "NMR-Quests.txt",
        "custom" => "NMR-QuestsCustom.txt",
        other => {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                format!("no quest file {other:?}: give default or custom"),
            ));
        }
    };
    let path = s.realm_dir.join(name);
    blocking(move || match std::fs::read(&path) {
        // NMR's files are Windows-1252; each byte is its character here, as NMR shows it
        Ok(bytes) => Ok(Json(json!({
            "name": name,
            "text": bytes.iter().map(|&b| b as char).collect::<String>(),
        }))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ApiError(
            StatusCode::NOT_FOUND,
            format!("{name} is not in the realm's folder"),
        )),
        Err(e) => Err(failed(e)),
    })
    .await
}
