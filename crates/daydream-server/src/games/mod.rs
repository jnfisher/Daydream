//! Choosing the game Daydream works on, from the browser: the header's game picker.
//!
//! Daydream assumes it runs on the machine that hosts the game, and finds the games there
//! ([`discover`]). Each kind is worked on its own way:
//!
//! | Kind | The game keeps its world in | Daydream edits | Back into the game |
//! |---|---|---|---|
//! | [`Source::Wcc`] | `WCC*.DAT` Btrieve files (Worldgroup, The Major BBS) | a working copy, converted from them | Tools, **Write to WCC files** (`write-dat`) |
//! | [`Source::Db`] | `WCC*.db` files (MBBSEmu, wbtrv32), or a realm folder | the files themselves | nothing to do: every save is in the game's files |
//! | [`Source::Silvermere`] | silvermere's SQLite file or PostgreSQL | a working copy, read from it | Tools, **silvermere**: Export |
//!
//! Working copies live in Daydream's workspace ([`workspace`]), one per game, and remember
//! where they came from, so the write-back cards know their target. A copy is made the
//! first time a game is opened and kept after; **refresh** reads the game again (the copy it
//! replaces is kept as `<copy>.previous`).
//!
//! A running game must not have its files written ([`running`]): saves into a live `.db`
//! folder and `write-dat` into a folder a game runs on are refused, and silvermere's writer
//! lock refuses while `silvermere serve` runs.
//!
//! **Who may name a game.** Without an access token Daydream answers only this machine, and
//! any folder or database URL can be typed. With a token (reached through a tunnel such as
//! Funnel) only the games Daydream found itself, the ones opened before, and those it was
//! started with can be opened, and the write-back cards write only to the open game's
//! source; `serve --allow-any-source` lifts that.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/games?refresh=` | `{current, games: [...], custom}`: the open game and those found (searched again with `refresh`) |
//! | `GET /api/games/status?id=` | `{running, by, elsewhere}` for one game (`id=current`: the open one) |
//! | `POST /api/games/open` | `{id}` or `{kind, location}`, and `refresh`: opens it; `{current}` |

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use daydream_store::Realm;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{ApiError, AppState, blocking};

pub mod discover;
pub mod running;
pub mod workspace;

pub use discover::Found;
use workspace::{Saved, Workspace};

/// Where a game keeps its world.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Source {
    /// A folder of `WCC*.DAT`/`.VIR` Btrieve files.
    Wcc { dir: PathBuf },
    /// A folder of `WCC*.db` files, edited where they are.
    Db { dir: PathBuf },
    /// silvermere's database, as a `sqlite:` or `postgres:` URL.
    Silvermere { url: String },
}

impl Source {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Wcc { .. } => "wcc",
            Self::Db { .. } => "db",
            Self::Silvermere { .. } => "silvermere",
        }
    }

    /// A short, stable name for it: the start of a hash of where it is.
    pub fn id(&self) -> String {
        let text = serde_json::to_string(self).unwrap_or_default();
        Sha256::digest(text.as_bytes())[..6]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Where it is, for a person: a folder, or the URL with its password masked.
    pub fn shown(&self) -> String {
        match self {
            Self::Wcc { dir } | Self::Db { dir } => dir.display().to_string(),
            Self::Silvermere { url } => daydream_silvermere::redact_url(url),
        }
    }

    /// From what a person typed: a folder (its files say which kind) or a database URL.
    pub fn typed(kind: &str, location: &str) -> Result<(Self, String, bool), String> {
        let location = location.trim();
        if location.is_empty() {
            return Err("give a folder, or silvermere's database".into());
        }
        if kind == "silvermere" {
            let url = daydream_silvermere::database_url(location);
            return Ok((Self::Silvermere { url }, "silvermere".into(), false));
        }
        let dir = discover::canonical(Path::new(location))
            .filter(|d| d.is_dir())
            .ok_or_else(|| format!("{location}: not a folder"))?;
        let names: Vec<String> = std::fs::read_dir(&dir)
            .map_err(|e| format!("{location}: {e}"))?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        let found = discover::classify(&dir, &names, "typed in")
            .into_iter()
            .find(|f| !matches!(f.source, Source::Silvermere { .. }))
            .ok_or_else(|| {
                format!(
                    "{}: no MajorMUD files (WCC*.DAT, WCC*.VIR or WCC*.db) in it",
                    dir.display()
                )
            })?;
        Ok((found.source, found.label, found.live))
    }

    /// Whether Daydream edits a working copy of it (else its own files).
    pub fn copied(&self) -> bool {
        !matches!(self, Self::Db { .. })
    }
}

/// The game open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub source: Source,
    pub label: String,
    /// A game runs on the realm's own files: saves are refused while it runs.
    pub live: bool,
}

impl Game {
    /// The game a realm folder given at start is: the source a working copy records, else
    /// the folder itself.
    pub fn of_realm(dir: &Path) -> Self {
        if let Some(saved) = workspace::read_saved(dir) {
            return Self {
                source: saved.source,
                label: saved.label,
                live: false,
            };
        }
        let dir = discover::canonical(dir).unwrap_or_else(|| dir.to_path_buf());
        let names: Vec<String> = std::fs::read_dir(&dir)
            .map(|r| {
                r.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let found = discover::classify(&dir, &names, "")
            .into_iter()
            .find(|f| matches!(f.source, Source::Db { .. }));
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            source: Source::Db { dir: dir.clone() },
            label: found
                .as_ref()
                .map_or_else(|| format!("{name} (realm folder)"), |f| f.label.clone()),
            live: found.is_some_and(|f| f.live),
        }
    }
}

/// How the MCP server looks at a realm's game: its name, whether the realm is its own files,
/// and what runs on them. Without a known game, the realm folder is looked at as one.
pub fn status_check(dir: &Path, game: Option<Game>) -> daydream_mcp::StatusCheck {
    let dir = dir.to_path_buf();
    let game = game.unwrap_or_else(|| Game::of_realm(&dir));
    daydream_mcp::StatusCheck::new(move || daydream_mcp::GameStatus {
        game: Some(game.label.clone()),
        live: game.live,
        running_by: if game.live {
            running::check(&dir).by
        } else {
            None
        },
    })
}

/// How games are found and opened.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Where working copies are kept; `None` is [`workspace::default_dir`].
    pub workspace: Option<PathBuf>,
    /// Folders to search besides the usual places.
    pub search: Vec<PathBuf>,
    /// Search the usual places ([`discover::Roots::usual`]).
    pub default_search: bool,
    /// Let a token holder type any folder or database URL (`serve --allow-any-source`).
    pub allow_any_source: bool,
    /// Folders the MCP server reads agents' edits files from, besides each realm's own
    /// `.daydream/batches`.
    pub batch_dirs: Vec<PathBuf>,
}

impl Config {
    pub fn workspace(&self) -> Workspace {
        Workspace::new(
            self.workspace
                .clone()
                .unwrap_or_else(workspace::default_dir),
        )
    }
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn conflict(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::CONFLICT, msg.into())
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// Refused to a token holder unless `--allow-any-source`.
pub(crate) fn not_allowed(what: &str) -> ApiError {
    ApiError(
        StatusCode::FORBIDDEN,
        format!(
            "{what}: this Daydream is reached with an access token, so it opens and writes only \
             the games it found itself. Start it with --allow-any-source to allow any, or add \
             the folder with --search"
        ),
    )
}

impl AppState {
    /// Whether a folder or URL a person typed may be used.
    pub(crate) fn custom_allowed(&self) -> bool {
        self.token.is_none() || self.games.allow_any_source
    }

    /// The games found (searching when not yet, or when `refresh`), and the working copies'
    /// sources, the game open and the one given at start: every game that may be opened.
    pub(crate) fn known(&self, refresh: bool) -> Vec<Found> {
        let ws = self.games.workspace();
        let mut found = {
            let mut cache = self.found.lock().unwrap_or_else(|e| e.into_inner());
            if refresh || cache.is_none() {
                let mut roots =
                    discover::Roots::usual(&self.games.search, self.games.default_search);
                roots.skip.push(ws.dir().to_path_buf());
                *cache = Some(discover::discover(&roots));
            }
            cache.clone().unwrap_or_default()
        };
        if let Some(url) = &self.silvermere {
            found.push(Found {
                source: Source::Silvermere { url: url.clone() },
                label: "silvermere".into(),
                found_by: "serve --silvermere".into(),
                live: false,
            });
        }
        for (_, saved) in ws.copies() {
            found.push(Found {
                source: saved.source,
                label: saved.label,
                found_by: "opened before".into(),
                live: false,
            });
        }
        if let Some(game) = &self.current().game {
            found.push(Found {
                source: game.source.clone(),
                label: game.label.clone(),
                found_by: "open now".into(),
                live: game.live,
            });
        }
        // the first of each source: what found it, rather than "opened before"
        let mut seen = std::collections::BTreeSet::new();
        found.retain(|f| seen.insert(f.source.clone()));
        found
    }

    /// The silvermere database a request names, or the open game's, or the one given at
    /// start; a typed one only where [`custom_allowed`](Self::custom_allowed) or known.
    pub(crate) fn silvermere_url(&self, given: Option<String>) -> Result<String, ApiError> {
        let given = given
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        let current = match &self.current().game {
            Some(Game {
                source: Source::Silvermere { url },
                ..
            }) => Some(url.clone()),
            _ => None,
        };
        match given {
            Some(d) => {
                let url = daydream_silvermere::database_url(&d);
                let known = || {
                    self.known(false).iter().any(
                        |f| matches!(&f.source, Source::Silvermere { url: u } if *u == url),
                    )
                };
                if !self.custom_allowed() && !known() {
                    return Err(not_allowed("that silvermere database"));
                }
                Ok(url)
            }
            None => current.or_else(|| self.silvermere.clone()).ok_or_else(|| {
                bad("give silvermere's database (its SQLite file or a URL), or open a silvermere game")
            }),
        }
    }

    /// The WCC folder `write-dat` writes: the one asked for, else the open game's. A token
    /// holder may name only a WCC game Daydream knows.
    pub(crate) fn wcc_target(&self, given: Option<&str>) -> Result<PathBuf, ApiError> {
        let current = match &self.current().game {
            Some(Game {
                source: Source::Wcc { dir },
                ..
            }) => Some(dir.clone()),
            Some(Game {
                source: Source::Db { .. },
                live: true,
                ..
            }) if given.is_none() => {
                return Err(bad(
                    "this game's own .db files are open: every save is already in the game, \
                     so there is nothing to write back",
                ));
            }
            _ => None,
        };
        let Some(given) = given.map(str::trim).filter(|g| !g.is_empty()) else {
            return current.ok_or_else(|| {
                bad("give the WCC folder to write (the realm was not opened from one)")
            });
        };
        let dir = PathBuf::from(given);
        if !dir.is_dir() {
            return Err(bad(format!("{}: not a folder", dir.display())));
        }
        if !self.custom_allowed() {
            let canon = discover::canonical(&dir);
            let known = self.known(false).iter().any(|f| match &f.source {
                Source::Wcc { dir: d } => discover::canonical(d) == canon,
                _ => false,
            });
            if !known {
                return Err(not_allowed(&dir.display().to_string()));
            }
        }
        Ok(dir)
    }
}

/// One game, as the picker shows it.
fn game_json(f: &Found, ws: &Workspace, current: Option<&Game>) -> Value {
    let copy = f.source.copied().then(|| {
        let dir = ws.copy_dir(&f.source, &f.label);
        let saved = workspace::read_saved(&dir);
        saved.map(|s| json!({ "dir": dir, "pulled": s.pulled }))
    });
    json!({
        "id": f.source.id(),
        "kind": f.source.kind(),
        "label": f.label,
        "location": f.source.shown(),
        "found_by": f.found_by,
        "live": f.live,
        "copy": copy.flatten(),
        "current": current.is_some_and(|g| g.source == f.source),
    })
}

/// `?refresh=`.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    refresh: bool,
}

pub(crate) async fn list(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || Ok(Json(list_games(&s, q.refresh)))).await
}

/// The games that may be opened, and the one open (`GET /api/games`, MCP `list_games`).
pub(crate) fn list_games(s: &AppState, refresh: bool) -> Value {
    let found = s.known(refresh);
    let ws = s.games.workspace();
    let cur = s.current();
    let games: Vec<Value> = found
        .iter()
        .map(|f| game_json(f, &ws, cur.game.as_ref()))
        .collect();
    json!({
        "current": current_json(&cur),
        "games": games,
        "custom": s.custom_allowed(),
        "workspace": ws.dir(),
    })
}

fn current_json(cur: &crate::Current) -> Value {
    match (&cur.realm_dir, &cur.game) {
        (Some(dir), Some(g)) => json!({
            "id": g.source.id(),
            "kind": g.source.kind(),
            "label": g.label,
            "location": g.source.shown(),
            "live": g.live,
            "realm_dir": dir,
        }),
        (Some(dir), None) => json!({ "realm_dir": dir }),
        _ => Value::Null,
    }
}

/// `?id=`.
#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    id: String,
}

/// Whether a game is running, as the picker shows it.
pub(crate) async fn status(
    State(s): State<Arc<AppState>>,
    Query(q): Query<StatusQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let source = if q.id == "current" {
            s.current().game.as_ref().map(|g| g.source.clone())
        } else {
            s.known(false)
                .into_iter()
                .find(|f| f.source.id() == q.id)
                .map(|f| f.source)
        }
        .ok_or_else(|| crate::not_found(format!("game {}", q.id)))?;
        Ok(Json(status_of(&source)))
    })
    .await
}

fn status_of(source: &Source) -> Value {
    match source {
        Source::Wcc { dir } | Source::Db { dir } => {
            let r = running::check(dir);
            json!({ "running": r.running(), "by": r.by, "elsewhere": r.elsewhere })
        }
        Source::Silvermere { url } => {
            match daydream_silvermere::block_on(daydream_silvermere::info(url)) {
                Ok(info) => json!({
                    "running": info.running.is_some(),
                    "by": info.running,
                    "elsewhere": [],
                    "tables": info.tables,
                }),
                Err(e) => json!({ "running": null, "error": e.to_string(), "elsewhere": [] }),
            }
        }
    }
}

/// The body of an open.
#[derive(Debug, Default, Deserialize)]
pub struct OpenJson {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    /// Read the game again into its working copy, replacing it.
    #[serde(default)]
    pub refresh: bool,
}

pub(crate) async fn open(
    State(s): State<Arc<AppState>>,
    Json(body): Json<OpenJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || open_game(&s, body).map(Json)).await
}

/// Opens a game (`POST /api/games/open`, MCP `open_game`).
pub(crate) fn open_game(s: &AppState, body: OpenJson) -> Result<Value, ApiError> {
    {
        let (source, label, live) = match (&body.id, &body.location) {
            (Some(id), _) => {
                let f = s
                    .known(false)
                    .into_iter()
                    .find(|f| f.source.id() == *id)
                    .ok_or_else(|| crate::not_found(format!("game {id}")))?;
                (f.source, f.label, f.live)
            }
            (None, Some(location)) => {
                if !s.custom_allowed() {
                    return Err(not_allowed(location));
                }
                Source::typed(body.kind.as_deref().unwrap_or("folder"), location).map_err(bad)?
            }
            _ => return Err(bad("give a game's id, or a folder or database to open")),
        };
        // no save may be half done in the realm being left
        let _writing = s.write.lock().map_err(internal)?;
        let realm_dir = prepare(&s.games.workspace(), &source, &label, body.refresh)?;
        let game = Game {
            source,
            label,
            live,
        };
        s.set_current(realm_dir.clone(), Some(game.clone()));
        let saved = s.games.workspace().save_last(&workspace::Last {
            realm_dir,
            game: Some(game),
        });
        if let Err(e) = saved {
            tracing::warn!("remembering the open game: {e}");
        }
        Ok(json!({ "current": current_json(&s.current()) }))
    }
}

/// The realm folder for `source`: its own folder, or its working copy (made, or made again
/// when `refresh`).
pub fn prepare(
    ws: &Workspace,
    source: &Source,
    label: &str,
    refresh: bool,
) -> Result<PathBuf, ApiError> {
    if let Source::Db { dir } = source {
        let realm = Realm::open(dir)?;
        if realm.files().is_empty() {
            return Err(bad(format!("{}: no .db files", dir.display())));
        }
        return Ok(dir.clone());
    }
    let copy = ws.copy_dir(source, label);
    if copy.is_dir() && !refresh {
        return Ok(copy);
    }
    let scratch = ws.scratch().map_err(internal)?;
    match source {
        Source::Wcc { dir } => {
            let converted = daydream_store::realm::convert_dir_matching(
                dir,
                scratch.path(),
                daydream_store::realm::is_wcc_stem,
            )?;
            if converted.is_empty() {
                return Err(bad(format!(
                    "{}: no WCC*.DAT or WCC*.VIR files",
                    dir.display()
                )));
            }
        }
        Source::Silvermere { url } => {
            // a realm's files come from the stock world's; silvermere's world goes over them
            let stock = daydream_wcc::stock_files::vir_dir(None).map_err(|e| {
                conflict(format!(
                    "a silvermere working copy is laid out from the stock 1.11p files, \
                     which could not be had: {e}"
                ))
            })?;
            daydream_store::realm::convert_dir_matching(
                &stock,
                scratch.path(),
                daydream_store::realm::is_wcc_stem,
            )?;
            let realm = Realm::open(scratch.path())?;
            daydream_silvermere::block_on(daydream_silvermere::import(
                &realm,
                url,
                &daydream_silvermere::ImportOptions::default(),
            ))?;
        }
        Source::Db { .. } => unreachable!(),
    }
    let saved = Saved {
        source: source.clone(),
        label: label.to_string(),
        pulled: jiff::Timestamp::now().to_string(),
    };
    workspace::write_saved(scratch.path(), &saved).map_err(internal)?;
    let fresh = scratch.keep();
    ws.replace(&fresh, &copy).map_err(internal)?;
    Ok(copy)
}
