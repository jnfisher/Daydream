//! Where the World map draws rooms moved by hand, and the builder's own notes on it. MajorMUD
//! keeps no places for its rooms, nor anywhere to write on its maps, so these are kept beside
//! the working copy, in `.daydream/map.json`, and never in the game's data: nothing is
//! published, and the realm's history and changes do not count them. The file can be exported
//! and imported, to share a layout (and its notes) with another builder.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/map/pins` | the whole file, to export |
//! | `POST /api/map/pins` | `{map, pins: {room: [x, y] or null}, replace, levels}`: sets (or, `null`, drops) where rooms of one map are drawn; `replace` drops the map's others; with `levels`, the places are `[x, y, level]` and the map is drawn in levels |
//! | `POST /api/map/pins/import` | an exported file: each map it has takes its rooms' places and notes, and loses the rest |
//! | `POST /api/map/notes` | `{map, set: [note], drop: [id]}`: adds notes to one map (or puts them in place of those with their ids), and takes away those dropped |
//!
//! A map drawn in levels (Up and Down lead a level up and down, and each level is laid out on
//! its own) keeps its own places, with their levels, beside those of the map drawn flat: each
//! way of drawing it keeps its layout. A write says which way the map is drawn from then on.
//!
//! A note is the builder's, for the builder: a few words on a room (a thing to do, which can be
//! ticked off), a label drawn on the map by a room, or a region, a name drawn around rooms. Its
//! id is the page's to give.
//!
//! `GET /api/map` answers each map's places and notes with its rooms. The writes answer the
//! newest step of the realm's history, so the page can tell whether its own undo of a move or
//! a save to the realm came last.
//!
//! ```json
//! {"daydream": "map", "version": 1, "maps": {"1": {"pins": {"12": [3, -4]}, "notes": [
//!     {"id": "k3f9a1", "kind": "note", "text": "Give the guard a line", "rooms": [12], "todo": true},
//!     {"id": "p0c2x8", "kind": "region", "text": "Thieves' quarter", "rooms": [12, 13, 14], "colour": "mint"}]},
//!   "6": {"pins": {}, "levels": true, "level_pins": {"40": [0, 2, -1]}}}}
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
/// The furthest level from the start room's, either way.
pub const MAX_LEVEL: i32 = 10_000;
/// The most places one request sets.
pub const MAX_PINS: usize = 500_000;
/// The most notes a map keeps.
pub const MAX_NOTES: usize = 5_000;
/// The longest note, in characters.
pub const MAX_NOTE: usize = 2_000;
/// The longest label, or name of a region, in characters.
pub const MAX_LABEL: usize = 80;
/// The most rooms a region goes around.
pub const MAX_REGION: usize = 20_000;
/// The colours a note takes: the app's pastels.
pub const COLOURS: [&str; 6] = ["butter", "pink", "peach", "mint", "sky", "lilac"];

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
    /// The map is drawn in levels: Up and Down lead a level up and down.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub levels: bool,
    /// Where its rooms moved by hand are drawn in levels: the cell and the level, by room number.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub level_pins: BTreeMap<i64, [i32; 3]>,
    /// The builder's notes on it, in the order they were first written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

impl MapPart {
    /// Nothing to keep: no room moved, drawn flat, and no notes.
    fn is_empty(&self) -> bool {
        self.pins.is_empty() && !self.levels && self.level_pins.is_empty() && self.notes.is_empty()
    }
}

/// What a note is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteKind {
    /// A few words on a room, shown when the pointer is over its mark: perhaps a thing to do.
    Note,
    /// Words drawn on the map by a room.
    Label,
    /// A name drawn around rooms.
    Region,
}

/// A note on a map: the builder's own, never the game's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// The page's: letters, digits, `-` and `_`.
    pub id: String,
    pub kind: NoteKind,
    pub text: String,
    /// The room it is on (a note's or a label's), or those it goes around (a region's).
    pub rooms: Vec<i64>,
    /// One of [`COLOURS`] (none: the kind's own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// A note that is a thing to do, and whether it is done.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub todo: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
}

/// A note as kept: checked, its words trimmed, and only a note a thing to do (or done).
fn checked_note(map: i64, mut n: Note) -> Result<Note, ApiError> {
    let id_ok = (1..=40).contains(&n.id.len())
        && n.id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !id_ok {
        return Err(bad(format!(
            "a note's id is 1 to 40 letters, digits, - and _, not {:?}",
            n.id
        )));
    }
    n.text = n.text.trim().to_string();
    let most = if n.kind == NoteKind::Note {
        MAX_NOTE
    } else {
        MAX_LABEL
    };
    let what = match n.kind {
        NoteKind::Note => "a note",
        NoteKind::Label => "a label",
        NoteKind::Region => "a region's name",
    };
    if n.text.is_empty() {
        return Err(bad(format!(
            "{what} needs some words (note {} on map {map})",
            n.id
        )));
    }
    if n.text.chars().count() > most {
        return Err(bad(format!(
            "{what} is at most {most} characters (note {} on map {map})",
            n.id
        )));
    }
    let most_rooms = if n.kind == NoteKind::Region {
        MAX_REGION
    } else {
        1
    };
    if n.rooms.is_empty() || n.rooms.len() > most_rooms {
        return Err(bad(match n.kind {
            NoteKind::Region => format!(
                "a region goes around 1 to {MAX_REGION} rooms (note {} on map {map})",
                n.id
            ),
            _ => format!("{what} is on one room (note {} on map {map})", n.id),
        }));
    }
    if let Some(&room) = n
        .rooms
        .iter()
        .find(|&&r| !(1..=i64::from(i32::MAX)).contains(&r))
    {
        return Err(bad(format!(
            "no room {map}/{room}: a room is numbered from 1"
        )));
    }
    if let Some(c) = &n.colour
        && !COLOURS.contains(&c.as_str())
    {
        return Err(bad(format!(
            "a note's colour is one of {}, not {c:?}",
            COLOURS.join(", ")
        )));
    }
    n.todo &= n.kind == NoteKind::Note;
    n.done &= n.todo;
    Ok(n)
}

/// A map's notes as kept: each checked, and no id twice.
fn checked_notes(map: i64, notes: Vec<Note>) -> Result<Vec<Note>, ApiError> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(notes.len());
    for n in notes {
        if !seen.insert(n.id.clone()) {
            return Err(bad(format!("note {} is given twice for map {map}", n.id)));
        }
        out.push(checked_note(map, n)?);
    }
    Ok(out)
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

/// A room's place: its cell, and in levels its level after it.
fn checked_pin(map: i64, room: i64, at: &[i32]) -> Result<(), ApiError> {
    if !(1..=i64::from(i32::MAX)).contains(&room) {
        return Err(bad(format!(
            "no room {map}/{room}: a room is numbered from 1"
        )));
    }
    if at.iter().take(2).any(|v| v.abs() > MAX_CELL) {
        return Err(bad(format!(
            "room {map}/{room} is drawn too far away: a cell is at most {MAX_CELL} from the start room"
        )));
    }
    if at.get(2).is_some_and(|z| z.abs() > MAX_LEVEL) {
        return Err(bad(format!(
            "room {map}/{room} is drawn too far up or down: a level is at most {MAX_LEVEL} from the start room's"
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

/// One map's places, and whether it is drawn in levels, for the map's answer (none when the
/// file cannot be read: the map is laid out as if no room had been moved, and a write says why).
pub fn of_map(realm_dir: &Path, map: i64) -> MapPart {
    match load(realm_dir) {
        Ok(mut f) => f.maps.remove(&map).unwrap_or_default(),
        Err(e) => {
            tracing::warn!("{}", e.1);
            MapPart::default()
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
        file.maps.retain(|_, m| !m.is_empty());
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
    /// Each room's cell (and in levels its level), or `null` to draw it where the layout puts it.
    #[serde(default)]
    pins: BTreeMap<i64, Option<Vec<i32>>>,
    /// The map's other rooms go back to the layout.
    #[serde(default)]
    replace: bool,
    /// The places are the map's drawn in levels, and it is drawn so from now on (else flat).
    #[serde(default)]
    levels: bool,
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
    let size = if body.levels { 3 } else { 2 };
    for (room, at) in &body.pins {
        if let Some(at) = at {
            if at.len() != size {
                return Err(bad(format!(
                    "room {}/{room}'s place is {at:?}: {}",
                    body.map,
                    if body.levels {
                        "in levels, a place is [x, y, level]"
                    } else {
                        "a place is [x, y]"
                    }
                )));
            }
            checked_pin(body.map, *room, at)?;
        }
    }
    let (pins, step) = change(s, move |file| {
        let part = file.maps.entry(body.map).or_default();
        part.levels = body.levels;
        if body.levels {
            if body.replace {
                part.level_pins.clear();
            }
            for (room, at) in body.pins {
                match at {
                    Some(at) => part.level_pins.insert(room, [at[0], at[1], at[2]]),
                    None => part.level_pins.remove(&room),
                };
            }
            return Ok(part.level_pins.len());
        }
        if body.replace {
            part.pins.clear();
        }
        for (room, at) in body.pins {
            match at {
                Some(at) => part.pins.insert(room, [at[0], at[1]]),
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
    let mut notes = 0;
    let mut maps = BTreeMap::new();
    for (map, mut part) in given.maps {
        checked_map(map)?;
        for (room, at) in &part.pins {
            checked_pin(map, *room, at)?;
        }
        for (room, at) in &part.level_pins {
            checked_pin(map, *room, at)?;
        }
        total += part.pins.len() + part.level_pins.len();
        if part.notes.len() > MAX_NOTES {
            return Err(bad(format!(
                "map {map} has {} notes: a map keeps at most {MAX_NOTES}",
                part.notes.len()
            )));
        }
        part.notes = checked_notes(map, part.notes)?;
        notes += part.notes.len();
        maps.insert(map, part);
    }
    if total > MAX_PINS {
        return Err(bad(format!("at most {MAX_PINS} rooms at once")));
    }
    let count = maps.len();
    let ((), step) = change(s, move |file| {
        file.maps.extend(maps);
        Ok(())
    })
    .await?;
    Ok(Json(
        json!({ "maps": count, "pins": total, "notes": notes, "step": step }),
    ))
}

/// Notes written on one map, and those taken away.
#[derive(Debug, Deserialize)]
pub struct NotesBody {
    map: i64,
    /// Notes to add, or to put in place of those with their ids.
    #[serde(default)]
    set: Vec<Note>,
    /// The ids of notes to take away (those it has not are let be).
    #[serde(default)]
    drop: Vec<String>,
}

/// `POST /api/map/notes`: answers how many notes the map has now.
pub(crate) async fn notes(
    State(s): State<Arc<AppState>>,
    Json(body): Json<NotesBody>,
) -> Result<Json<Value>, ApiError> {
    checked_map(body.map)?;
    if body.set.len() > MAX_NOTES || body.drop.len() > MAX_NOTES {
        return Err(bad(format!("a map keeps at most {MAX_NOTES} notes")));
    }
    let map = body.map;
    let set = checked_notes(map, body.set)?;
    let drop = body.drop;
    let (count, step) = change(s, move |file| {
        let part = file.maps.entry(map).or_default();
        part.notes.retain(|n| !drop.contains(&n.id));
        for n in set {
            match part.notes.iter_mut().find(|m| m.id == n.id) {
                Some(m) => *m = n,
                None => part.notes.push(n),
            }
        }
        if part.notes.len() > MAX_NOTES {
            return Err(bad(format!(
                "map {map} would have {} notes: a map keeps at most {MAX_NOTES}",
                part.notes.len()
            )));
        }
        Ok(part.notes.len())
    })
    .await?;
    Ok(Json(json!({ "notes": count, "step": step })))
}
