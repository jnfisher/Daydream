//! Where the World map draws rooms moved by hand. MajorMUD keeps no places for its rooms, so
//! they are kept beside the working copy, in `.daydream/map.json`, and never in the game's
//! data: nothing is published, and the realm's history and changes do not count them. The
//! file can be exported and imported, to share a layout with another builder.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/map/pins` | the whole file, to export |
//! | `POST /api/map/pins` | `{map, pins: {room: [x, y] or null}, replace}`: sets (or, `null`, drops) where rooms of one map are drawn; `replace` drops the map's others |
//! | `POST /api/map/pins/import` | an exported file: each map it has takes its rooms' places, and loses the rest |
//!
//! `GET /api/map` answers each map's places with its rooms. The writes answer the newest
//! step of the realm's history, so the page can tell whether its own undo of a move or a
//! save to the realm came last.
//!
//! ```json
//! {"daydream": "map", "version": 1, "maps": {"1": {"pins": {"12": [3, -4]}}}}
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking};

/// The furthest a room is drawn from the start room, in cells either way.
pub const MAX_CELL: i32 = 100_000;
/// The most places one request sets.
pub const MAX_PINS: usize = 500_000;

/// What the file says it is.
const KIND: &str = "map";
const VERSION: u32 = 1;

/// The file, beside the realm's journal.
pub fn path(realm_dir: &Path) -> PathBuf {
    realm_dir.join(".daydream").join("map.json")
}

/// The file's contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapFile {
    #[serde(default = "kind")]
    pub daydream: String,
    #[serde(default = "version")]
    pub version: u32,
    /// Each map's own, by map number.
    #[serde(default)]
    pub maps: BTreeMap<i64, MapPart>,
}

impl Default for MapFile {
    fn default() -> Self {
        Self {
            daydream: kind(),
            version: VERSION,
            maps: BTreeMap::new(),
        }
    }
}

fn kind() -> String {
    KIND.into()
}

fn version() -> u32 {
    VERSION
}

/// One map's part of the file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapPart {
    /// Where its rooms moved by hand are drawn: the cell, by room number.
    #[serde(default)]
    pub pins: BTreeMap<i64, [i32; 2]>,
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn checked_map(map: i64) -> Result<(), ApiError> {
    if !(1..=i64::from(i32::MAX)).contains(&map) {
        return Err(bad(format!("no map {map}: a map is numbered from 1")));
    }
    Ok(())
}

fn checked_pin(map: i64, room: i64, at: &[i32; 2]) -> Result<(), ApiError> {
    if !(1..=i64::from(i32::MAX)).contains(&room) {
        return Err(bad(format!(
            "no room {map}/{room}: a room is numbered from 1"
        )));
    }
    if at.iter().any(|v| v.abs() > MAX_CELL) {
        return Err(bad(format!(
            "room {map}/{room} is drawn too far away: a cell is at most {MAX_CELL} from the start room"
        )));
    }
    Ok(())
}

/// The file, or an empty one when there is none yet.
pub fn load(realm_dir: &Path) -> Result<MapFile, ApiError> {
    let p = path(realm_dir);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(MapFile::default()),
        Err(e) => {
            return Err(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("reading {}: {e}", p.display()),
            ));
        }
    };
    serde_json::from_str(&text).map_err(|e| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "{} is not a map file Daydream can read ({e}): fix it, or move it away to start afresh",
                p.display()
            ),
        )
    })
}

/// One map's places, for the map's answer (none when the file cannot be read: the map is
/// laid out as if no room had been moved, and a write says why).
pub fn of_map(realm_dir: &Path, map: i64) -> BTreeMap<i64, [i32; 2]> {
    match load(realm_dir) {
        Ok(mut f) => f.maps.remove(&map).map(|m| m.pins).unwrap_or_default(),
        Err(e) => {
            tracing::warn!("{}", e.1);
            BTreeMap::new()
        }
    }
}

/// Writes the file whole, through a file beside it, so a crash leaves the old one or the new.
fn store(realm_dir: &Path, file: &MapFile) -> Result<(), ApiError> {
    write_whole(&path(realm_dir), file)
}

/// Writes a side file whole as JSON, through a file beside it, so a crash leaves the old one or
/// the new.
pub(crate) fn write_whole(p: &Path, value: &impl Serialize) -> Result<(), ApiError> {
    let io = |e: std::io::Error| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("writing {}: {e}", p.display()),
        )
    };
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    let tmp = p.with_extension("json.tmp");
    let text = serde_json::to_string(value).map_err(|e| io(std::io::Error::other(e)))?;
    std::fs::write(&tmp, text).map_err(io)?;
    std::fs::rename(&tmp, p).map_err(io)
}

/// The newest step of the realm's history (0 when it has none).
pub(crate) fn newest_step(realm_dir: &Path) -> Result<i64, ApiError> {
    Ok(daydream_store::history::list(realm_dir, 1)?
        .first()
        .map_or(0, |s| s.step))
}

/// Changes the file under its own lock: not the realm's write lock, as moving a room is no
/// save to the game (and goes on while the game runs).
async fn change<T: Send + 'static>(
    s: Arc<AppState>,
    f: impl FnOnce(&mut MapFile) -> Result<T, ApiError> + Send + 'static,
) -> Result<(T, i64), ApiError> {
    blocking(move || {
        let dir = s.realm_dir()?;
        let _lock = s
            .side
            .lock()
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let mut file = load(&dir)?;
        let out = f(&mut file)?;
        file.maps.retain(|_, m| !m.pins.is_empty());
        file.daydream = kind();
        file.version = VERSION;
        store(&dir, &file)?;
        Ok((out, newest_step(&dir)?))
    })
    .await
}

/// `GET /api/map/pins`: the whole file, to export.
pub(crate) async fn get(State(s): State<Arc<AppState>>) -> Result<Json<MapFile>, ApiError> {
    blocking(move || load(&s.realm_dir()?).map(Json)).await
}

/// Where rooms of one map are drawn.
#[derive(Debug, Deserialize)]
pub struct PinsBody {
    map: i64,
    /// Each room's cell, or `null` to draw it where the layout puts it.
    #[serde(default)]
    pins: BTreeMap<i64, Option<[i32; 2]>>,
    /// The map's other rooms go back to the layout.
    #[serde(default)]
    replace: bool,
}

/// `POST /api/map/pins`.
pub(crate) async fn set(
    State(s): State<Arc<AppState>>,
    Json(body): Json<PinsBody>,
) -> Result<Json<Value>, ApiError> {
    checked_map(body.map)?;
    if body.pins.len() > MAX_PINS {
        return Err(bad(format!("at most {MAX_PINS} rooms at once")));
    }
    for (room, at) in &body.pins {
        if let Some(at) = at {
            checked_pin(body.map, *room, at)?;
        }
    }
    let (pins, step) = change(s, move |file| {
        let part = file.maps.entry(body.map).or_default();
        if body.replace {
            part.pins.clear();
        }
        for (room, at) in body.pins {
            match at {
                Some(at) => part.pins.insert(room, at),
                None => part.pins.remove(&room),
            };
        }
        Ok(part.pins.len())
    })
    .await?;
    Ok(Json(json!({ "pins": pins, "step": step })))
}

/// `POST /api/map/pins/import`: each map of an exported file takes its places.
pub(crate) async fn import(
    State(s): State<Arc<AppState>>,
    Json(given): Json<MapFile>,
) -> Result<Json<Value>, ApiError> {
    if given.daydream != KIND || given.version != VERSION {
        return Err(bad(format!(
            "not a map file of this Daydream: it says {:?} version {}, and the file is \"{KIND}\" version {VERSION}",
            given.daydream, given.version
        )));
    }
    let mut total = 0;
    for (map, part) in &given.maps {
        checked_map(*map)?;
        for (room, at) in &part.pins {
            checked_pin(*map, *room, at)?;
        }
        total += part.pins.len();
    }
    if total > MAX_PINS {
        return Err(bad(format!("at most {MAX_PINS} rooms at once")));
    }
    let maps = given.maps.len();
    let ((), step) = change(s, move |file| {
        for (map, part) in given.maps {
            file.maps.insert(map, part);
        }
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "maps": maps, "pins": total, "step": step })))
}
