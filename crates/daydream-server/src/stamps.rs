//! The stamps saved from the World map: a part of a map, picked with Select, kept as a shape (its
//! rooms' cells and the exits between them) to put down again from the stamp drawer. Like the
//! map's places ([`crate::pins`]), they are kept beside the working copy, in
//! `.daydream/stamps.json`, and never in the game's data: nothing is published, and the realm's
//! history and changes do not count them.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/map/stamps` | the whole file |
//! | `POST /api/map/stamps` | `{name, map, room, cells, links}` saved: a new stamp takes the next id; one with an id replaces that stamp, or brings it back where it was once deleted; answers `{stamp, stamps, step}` |
//! | `DELETE /api/map/stamps/{id}` | deletes one; answers `{stamps, step}` |
//!
//! The writes answer the newest step of the realm's history, as the places' do, so the page can
//! tell whether its own undo of a save or a save to the realm came last. Ids are never used
//! twice, so an undo brings back the stamp it took away and no other.
//!
//! ```json
//! {"daydream": "stamps", "version": 1, "next": 3, "stamps": [
//!   {"id": "s2", "name": "Gatehouse", "map": 1, "room": 40, "cells": [[0, 0], [1, 0]], "links": [{"a": 0, "dir": 2, "b": 1}]}
//! ]}
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use daydream_nmr::map::{MAX_STAMP_LINKS, MAX_STAMP_ROOMS};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pins::{MAX_CELL, newest_step, write_whole};
use crate::{ApiError, AppState, blocking, not_found};

/// The most stamps kept.
pub const MAX_SAVED: usize = 200;
/// The longest name, in characters.
pub const MAX_NAME: usize = 40;

/// What the file says it is.
const KIND: &str = "stamps";
const VERSION: u32 = 1;
/// The way back of each way, as the map's exits are numbered (N S E W NE NW SE SW U D).
const OPPOSITE: [usize; 10] = [1, 0, 3, 2, 7, 6, 5, 4, 9, 8];

/// The file, beside the realm's journal.
pub fn path(realm_dir: &Path) -> PathBuf {
    realm_dir.join(".daydream").join("stamps.json")
}

/// The file's contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampsFile {
    #[serde(default = "kind")]
    pub daydream: String,
    #[serde(default = "version")]
    pub version: u32,
    /// The number the next stamp saved takes (`s3`): past every one a stamp has had.
    #[serde(default = "first")]
    pub next: u64,
    /// In the order they were first saved.
    #[serde(default)]
    pub stamps: Vec<Saved>,
}

impl Default for StampsFile {
    fn default() -> Self {
        Self {
            daydream: kind(),
            version: VERSION,
            next: first(),
            stamps: Vec::new(),
        }
    }
}

fn kind() -> String {
    KIND.into()
}

fn version() -> u32 {
    VERSION
}

fn first() -> u64 {
    1
}

/// A stamp saved: a shape, as the stamp drawer's own are.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saved {
    /// `s` and a number, given when it is first saved.
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// The map it was saved from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<i64>,
    /// The room on that map its new rooms copy (one with the name most of its rooms had): put
    /// down there, they copy it unless it lands on a room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<i64>,
    /// Each cell as [column, row] from the first, its way in, which is [0, 0].
    pub cells: Vec<[i32; 2]>,
    #[serde(default)]
    pub links: Vec<SavedLink>,
}

/// An exit between two cells of a stamp: from cell `a` in `dir` to cell `b`, and back unless
/// `one_way`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLink {
    pub a: usize,
    pub dir: usize,
    pub b: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub one_way: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

/// A stamp's number, from its id (`s12` is 12).
fn number_of(id: &str) -> Option<u64> {
    let digits = id.strip_prefix('s')?;
    if digits.is_empty()
        || digits.len() > 9
        || digits.starts_with('0')
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    digits.parse().ok()
}

/// A stamp as it is kept: its name trimmed, and its shape one the drawer can put down.
fn checked(mut s: Saved) -> Result<Saved, ApiError> {
    s.name = s.name.trim().to_string();
    let chars = s.name.chars().count();
    if chars == 0 {
        return Err(bad("a stamp needs a name"));
    }
    if chars > MAX_NAME {
        return Err(bad(format!(
            "a stamp's name is at most {MAX_NAME} characters"
        )));
    }
    if s.name.chars().any(char::is_control) {
        return Err(bad("a stamp's name is one line of text"));
    }
    if !s.id.is_empty() && number_of(&s.id).is_none() {
        return Err(bad(format!(
            "no stamp {:?}: Daydream gives a stamp its id when it is first saved",
            s.id
        )));
    }
    if let Some(map) = s.map
        && !(1..=i64::from(i32::MAX)).contains(&map)
    {
        return Err(bad(format!("no map {map}: a map is numbered from 1")));
    }
    if let Some(room) = s.room
        && !(1..=i64::from(i32::MAX)).contains(&room)
    {
        return Err(bad(format!("no room {room}: a room is numbered from 1")));
    }
    match s.cells.first() {
        None => return Err(bad("a stamp has at least one room")),
        Some(c) if *c != [0, 0] => {
            return Err(bad(
                "a stamp's first cell, its way in, is [0, 0]: the others are counted from it",
            ));
        }
        Some(_) => {}
    }
    if s.cells.len() > MAX_STAMP_ROOMS {
        return Err(bad(format!(
            "a stamp makes at most {MAX_STAMP_ROOMS} rooms"
        )));
    }
    if s.links.len() > MAX_STAMP_LINKS {
        return Err(bad(format!(
            "a stamp makes at most {MAX_STAMP_LINKS} exits"
        )));
    }
    let far = 2 * MAX_CELL;
    let mut seen = HashSet::with_capacity(s.cells.len());
    for (i, c) in s.cells.iter().enumerate() {
        if c.iter().any(|v| v.abs() > far) {
            return Err(bad(format!(
                "cell {i} is too far from the first: at most {far} cells"
            )));
        }
        if !seen.insert(*c) {
            return Err(bad(format!("two rooms in one cell: [{}, {}]", c[0], c[1])));
        }
    }
    // each room has one exit each way
    let mut slots = HashSet::with_capacity(s.links.len() * 2);
    for (i, l) in s.links.iter().enumerate() {
        if l.a >= s.cells.len() || l.b >= s.cells.len() {
            return Err(bad(format!(
                "exit {i} leads to no cell: the stamp has {}",
                s.cells.len()
            )));
        }
        if l.a == l.b {
            return Err(bad(format!("exit {i} leads back to its own cell")));
        }
        let Some(&back) = OPPOSITE.get(l.dir) else {
            return Err(bad(format!(
                "exit {i} goes way {}: the ways are 0 to 9",
                l.dir
            )));
        };
        let mut uses = vec![(l.a, l.dir)];
        if !l.one_way {
            uses.push((l.b, back));
        }
        for (cell, dir) in uses {
            if !slots.insert((cell, dir)) {
                return Err(bad(format!(
                    "cell {cell} has two exits the same way ({dir})"
                )));
            }
        }
    }
    Ok(s)
}

/// The file, or an empty one when there is none yet.
pub fn load(realm_dir: &Path) -> Result<StampsFile, ApiError> {
    let p = path(realm_dir);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(StampsFile::default()),
        Err(e) => {
            return Err(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("reading {}: {e}", p.display()),
            ));
        }
    };
    let mut file: StampsFile = serde_json::from_str(&text).map_err(|e| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "{} is not a stamps file Daydream can read ({e}): fix it, or move it away to start afresh",
                p.display()
            ),
        )
    })?;
    // past every id there is, should the file have been put together by hand
    let top = file
        .stamps
        .iter()
        .filter_map(|s| number_of(&s.id))
        .max()
        .unwrap_or(0);
    file.next = file.next.max(top + 1);
    Ok(file)
}

/// Changes the file under the map's side-file lock (not the realm's write lock: saving a stamp
/// is no save to the game, and goes on while the game runs).
async fn change<T: Send + 'static>(
    s: Arc<AppState>,
    f: impl FnOnce(&mut StampsFile) -> Result<T, ApiError> + Send + 'static,
) -> Result<(T, i64), ApiError> {
    blocking(move || {
        let dir = s.realm_dir()?;
        let _lock = s
            .side
            .lock()
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let mut file = load(&dir)?;
        let out = f(&mut file)?;
        file.stamps
            .sort_by_key(|s| number_of(&s.id).unwrap_or(u64::MAX));
        file.daydream = kind();
        file.version = VERSION;
        write_whole(&path(&dir), &file)?;
        Ok((out, newest_step(&dir)?))
    })
    .await
}

/// `GET /api/map/stamps`.
pub(crate) async fn get(State(s): State<Arc<AppState>>) -> Result<Json<StampsFile>, ApiError> {
    blocking(move || load(&s.realm_dir()?).map(Json)).await
}

/// `POST /api/map/stamps`.
pub(crate) async fn save(
    State(s): State<Arc<AppState>>,
    Json(body): Json<Saved>,
) -> Result<Json<Value>, ApiError> {
    let mut stamp = checked(body)?;
    let ((stamp, count), step) = change(s, move |file| {
        if !stamp.id.is_empty()
            && let Some(at) = file.stamps.iter().position(|o| o.id == stamp.id)
        {
            file.stamps[at] = stamp.clone();
            return Ok((stamp, file.stamps.len()));
        }
        if file.stamps.len() >= MAX_SAVED {
            return Err(bad(format!(
                "{MAX_SAVED} stamps are saved already: delete one to make room"
            )));
        }
        match number_of(&stamp.id) {
            // one deleted, coming back
            Some(n) => file.next = file.next.max(n + 1),
            None => {
                stamp.id = format!("s{}", file.next);
                file.next += 1;
            }
        }
        file.stamps.push(stamp.clone());
        Ok((stamp, file.stamps.len()))
    })
    .await?;
    Ok(Json(
        json!({ "stamp": stamp, "stamps": count, "step": step }),
    ))
}

/// `DELETE /api/map/stamps/{id}`.
pub(crate) async fn delete(
    State(s): State<Arc<AppState>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<Value>, ApiError> {
    let (count, step) = change(s, move |file| {
        let at = file
            .stamps
            .iter()
            .position(|o| o.id == id)
            .ok_or_else(|| not_found(format!("stamp {id:?}")))?;
        file.stamps.remove(at);
        Ok(file.stamps.len())
    })
    .await?;
    Ok(Json(json!({ "stamps": count, "step": step })))
}
