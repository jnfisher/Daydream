//! Publishing: the open world's changes into its game, one step at a time, as the Publish
//! page shows them.
//!
//! Each engine has its ways ([`Way`]), and each way its steps:
//!
//! | Engine | Way | Steps |
//! |---|---|---|
//! | Worldgroup, The Major BBS (`WCC*.DAT`) | `update`: at the next start | check, back up, write the update file, live at the next start or cleanup |
//! | | `now`: with the game stopped | check, stop the game (Daydream waits), back up, write, start it again |
//! | MBBSEmu (`WCC*.db`) | `now`: with the game stopped | check, stop it (Daydream waits), back up, write its `.db` files, start it again |
//! | silvermere | `now` | check, warn the players (with an admin token), stop silvermere (Daydream waits for its writer lock), back up, write, start it again |
//!
//! A realm folder with no game, edited where it is, has nothing to publish.
//!
//! The update way needs no stop. MajorMUD merges its update file into its world at every start
//! and nightly cleanup, keeping what play changed (floors, shop stock, kill timers), so this
//! publish writes the update file alone ([`daydream_store::dat::write_update_file`]). It
//! carries edits and additions of the ten kinds of record the file holds, not deletions,
//! renumbering, players or what play changes ([`daydream_store::dat::not_carried`]); a world
//! with such changes publishes now.
//!
//! MBBSEmu keeps its world in wbtrv32's `.db` files and writes them as it runs, so its world is
//! edited in a copy taken while it runs, and published into them with it stopped: each edit
//! onto the record as play left it, where it is ([`daydream_store::game_db`]).
//!
//! Daydream cannot stop or start Worldgroup, The Major BBS or MBBSEmu, so publishing now waits
//! while the sysop stops the game, carries on by itself once it has, and leaves starting it
//! again to the sysop. silvermere has a remote control ([`crate::control`]): with an admin token
//! Daydream has it warn the players, count down (`warn.in_secs`, at once when nobody is in
//! the game; Cancel calls it off and silvermere tells the players) and stop, and it starts
//! silvermere again the way it was started, then waits until it answers. Without a token
//! the sysop stops it, and Daydream still starts it again. A silvermere that was not running
//! stays stopped. One publish runs at a time, in the background, and the page follows it.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/publish` | the plan: `{engine, game, location, running, changes, since, ways: [{id, title, summary, ready, why, not_carried, steps, button}], way, checks, control, job}` (`control`: silvermere's, as `GET /api/control` answers) |
//! | `POST /api/publish` | `{way, warn?: {in_secs, message}}`: starts publishing that way; the job |
//! | `GET /api/publish/job` | the job under way or the last one (`null` before any) |
//! | `POST /api/publish/cancel` | calls off a job while its step may be (`cancel`): a countdown, or waiting for the sysop |
//!
//! A job is `{id, way, engine, game, state, started, finished, steps: [{id, title, detail,
//! state, log, until?, cancel?}], skipped, error, result}`: `state` is `running`, `done`,
//! `failed` or `cancelled`, a step's is `waiting`, `running`, `done`, `failed`, `skipped` or
//! `yours` (for the sysop to do), `until` is when a countdown ends (seconds since 1970) and
//! `cancel` says the step may be called off now; `skipped` lists the edits that could not be
//! written, which stay among the changes. A job that wrote everything is `done` even when
//! starting the game again failed: that step says so, and so does `result`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use daydream_store::Realm;
use daydream_store::dat::{Action, DatError, WriteOptions, Written};
use daydream_types::records::layout::LayoutSet;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::control::{self, Admin, Live, Starter, Token};
use crate::games::{Game, Source, running};
use crate::{ApiError, AppState, blocking};

/// How often a waiting step looks at the game again.
const POLL: Duration = Duration::from_secs(2);

/// How long silvermere has to stop once its countdown is over.
const STOP_WITHIN: Duration = Duration::from_secs(120);

/// How long silvermere has to answer once it was started again.
const ANSWER_WITHIN: Duration = Duration::from_secs(120);

/// The longest countdown a publish asks for.
const MAX_WARN: u64 = 3600;

/// Why silvermere closes, when the page gives no reason.
pub const DEFAULT_MESSAGE: &str = "a world update, back in a minute or two";

/// What the open world publishes into.
#[derive(Debug, Clone)]
enum Engine {
    /// A folder of `WCC*.DAT` files: Worldgroup's (`WCCUPDA2`) or The Major BBS's (`WCCUPDAT`).
    Wcc { dir: PathBuf, worldgroup: bool },
    /// A game's own `.db` files (MBBSEmu's, or wbtrv32's beside a Worldgroup), worked on in a
    /// copy. `name`: the game, as its label says.
    Db {
        dir: PathBuf,
        name: &'static str,
        worldgroup: bool,
    },
    /// silvermere's database.
    Silvermere { url: String },
    /// Its own files, edited where they are: nothing to publish.
    InPlace,
}

impl Engine {
    /// What the game open publishes into, its world worked on in `realm_dir`.
    fn of(game: Option<&Game>, set: LayoutSet, realm_dir: &Path) -> Self {
        let worldgroup = set == LayoutSet::Worldgroup;
        match game {
            Some(Game {
                source: Source::Wcc { dir },
                ..
            }) => Self::Wcc {
                dir: dir.clone(),
                worldgroup,
            },
            // the game's own files, unless the world is worked on where they are
            Some(
                g @ Game {
                    source: Source::Db { dir },
                    label,
                    ..
                },
            ) if g.copied_into(realm_dir) => Self::Db {
                dir: dir.clone(),
                name: if label.contains("MBBSEmu") {
                    "MBBSEmu"
                } else if label.contains("Worldgroup") {
                    "Worldgroup"
                } else {
                    "the BBS"
                },
                worldgroup,
            },
            Some(Game {
                source: Source::Silvermere { url },
                ..
            }) => Self::Silvermere { url: url.clone() },
            _ => Self::InPlace,
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::Wcc {
                worldgroup: true, ..
            } => "worldgroup",
            Self::Wcc { .. } => "majorbbs",
            Self::Db {
                name: "MBBSEmu", ..
            } => "mbbsemu",
            Self::Db { .. } => "db",
            Self::Silvermere { .. } => "silvermere",
            Self::InPlace => "files",
        }
    }

    /// The game, as a sysop calls it.
    fn name(&self) -> &'static str {
        match self {
            Self::Wcc {
                worldgroup: true, ..
            } => "Worldgroup",
            Self::Wcc { .. } => "The Major BBS",
            Self::Db { name, .. } => name,
            Self::Silvermere { .. } => "silvermere",
            Self::InPlace => "the game",
        }
    }

    /// The game's update file.
    fn update_file(&self) -> &'static str {
        match self {
            Self::Wcc {
                worldgroup: true, ..
            } => "WCCUPDA2.DAT",
            Self::Db {
                worldgroup: true, ..
            } => "WCCUPDA2.DB",
            Self::Db { .. } => "WCCUPDAT.DB",
            _ => "WCCUPDAT.DAT",
        }
    }

    fn ways(&self) -> &'static [Way] {
        match self {
            Self::Wcc { .. } => &[Way::Update, Way::Now],
            Self::Db { .. } | Self::Silvermere { .. } => &[Way::Now],
            Self::InPlace => &[],
        }
    }

    /// The folder a game runs on, for the ones Daydream looks for as processes.
    fn game_dir(&self) -> Option<&Path> {
        match self {
            Self::Wcc { dir, .. } | Self::Db { dir, .. } => Some(dir),
            _ => None,
        }
    }
}

/// How a publish reaches the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Way {
    /// Through the update file, which the game merges at its next start or cleanup.
    Update,
    /// Into the game's files, with the game stopped.
    Now,
}

/// One step, as the page lists it and the job runs it.
#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub id: &'static str,
    pub title: String,
    pub detail: String,
    /// `waiting`, `running`, `done`, `failed`, `skipped`, or `yours` (for the sysop to do).
    pub state: &'static str,
    pub log: Vec<String>,
    /// When the countdown under way ends, in seconds since 1970.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
    /// The step may be called off now.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cancel: bool,
}

fn step(id: &'static str, title: impl Into<String>, detail: impl Into<String>) -> Step {
    Step {
        id,
        title: title.into(),
        detail: detail.into(),
        state: "waiting",
        log: Vec::new(),
        until: None,
        cancel: false,
    }
}

/// Whether the world has an update file, which publishing now writes too.
fn has_update(realm: &Realm) -> bool {
    matches!(daydream_store::update::UpdateFile::find(realm), Ok(Some(_)))
}

/// The steps of publishing `way` into `engine`; `update`: the world has an update file;
/// `live`: what Daydream knows of a silvermere now.
fn steps(engine: &Engine, way: Way, update: bool, live: Option<&Live>) -> Vec<Step> {
    let name = engine.name();
    let upd = engine.update_file();
    let check = step(
        "check",
        "Check",
        "The changes are read again, and checked against what this way can carry.",
    );
    match (engine, way) {
        (Engine::Wcc { .. }, Way::Update) => vec![
            check,
            step(
                "backup",
                "Back up",
                format!("The game's {upd} is copied to a daydream-backup folder beside it first."),
            ),
            step(
                "write",
                "Write the update file",
                format!(
                    "Your changes go into {upd}. The game's other files are left as they are, \
                     so it can keep running."
                ),
            ),
            step(
                "live",
                "Live after the next start or cleanup",
                format!(
                    "MajorMUD merges {upd} at every start and at its nightly cleanup, and keeps \
                     what play changed: floors, shop stock, kill timers. Restart the BBS to \
                     apply it sooner."
                ),
            ),
        ],
        (Engine::Wcc { worldgroup, .. }, _) => vec![
            check,
            step(
                "stop",
                format!("Stop {name}"),
                if *worldgroup {
                    "Stop it from its console, the Remote Sysop SHUTDOWN menu or the \
                     WGSCONTROL service. Daydream waits, and carries on once it has stopped."
                } else {
                    "Stop the BBS from its console. Daydream waits, and carries on once it \
                     has stopped."
                },
            ),
            step(
                "backup",
                "Back up",
                "Each file about to be written is copied to a daydream-backup folder beside \
                 it first.",
            ),
            step(
                "write",
                "Write the changes",
                if update {
                    format!(
                        "Only the records you changed go in, onto the files as play left them, \
                         and {upd} with them, so the next start keeps your edits."
                    )
                } else {
                    "Only the records you changed go in, onto the files as play left them.".into()
                },
            ),
            step(
                "start",
                format!("Start {name} again"),
                "Start it the way you usually do.",
            ),
        ],
        (Engine::Db { .. }, _) => vec![
            check,
            step(
                "stop",
                format!("Stop {name}"),
                "Stop it the way you usually do: its console, or its service. Daydream waits, \
                 and carries on once it has stopped.",
            ),
            step(
                "backup",
                "Back up",
                "Each .db file about to be written is copied to a daydream-backup folder beside \
                 it first.",
            ),
            step(
                "write",
                "Write the changes",
                if update {
                    format!(
                        "Only the records you changed go in, each onto the game's record as \
                         play left it, where it is, and {upd} with them, so the next start \
                         keeps your edits."
                    )
                } else {
                    "Only the records you changed go in, each onto the game's record as play \
                     left it, where it is."
                        .into()
                },
            ),
            step(
                "start",
                format!("Start {name} again"),
                "Start it the way you usually do.",
            ),
        ],
        (Engine::Silvermere { url }, _) => {
            let stopper = live.is_some_and(|l| l.stopper().is_some());
            let mut out = vec![check];
            if stopper {
                out.push(step(
                    "warn",
                    "Warn the players",
                    "silvermere tells everyone in the game when it closes, and again as the \
                     time comes closer. With nobody in the game it stops at once.",
                ));
            }
            out.push(if stopper {
                step(
                    "stop",
                    "Shut down",
                    "silvermere saves every character and stops. Daydream waits for its \
                     writer lock to be let go.",
                )
            } else {
                step(
                    "stop",
                    "Stop silvermere",
                    "Stop it with silvermere sysop shutdown, daydream-stack edit, or its \
                     service. Daydream waits for its writer lock, and carries on once it is \
                     free.",
                )
            });
            out.push(step(
                "backup",
                "Back up",
                if daydream_silvermere::sqlite_path(url).is_some() {
                    "The database file is copied to a daydream-backup folder beside it first."
                } else {
                    "Back up PostgreSQL your usual way: Daydream copies only SQLite files."
                },
            ));
            out.push(step(
                "write",
                "Write the world",
                "The world's tables are written from the working copy. Players, what lies on \
                 the floors and the rest of what play changed stay as they are.",
            ));
            let starter = live.and_then(|l| l.starter.as_ref());
            let auto = live.is_some_and(|l| l.autostart);
            out.push(step(
                "start",
                "Start silvermere again",
                match starter {
                    Some(Starter::Process { .. }) if auto => "Daydream runs the command line it \
                                                              was started with again, in its \
                                                              folder, then waits until it answers."
                        .into(),
                    Some(st) if auto => format!(
                        "Daydream starts it with {}, then waits until it answers.",
                        st.command()
                    ),
                    Some(st) => format!(
                        "Start it with {}, or let Daydream do it: switch that on under Server.",
                        st.command()
                    ),
                    None => "Start it the way you usually do.".into(),
                },
            ));
            out
        }
        (Engine::InPlace, _) => Vec::new(),
    }
}

// ---- the plan ------------------------------------------------------------------------------

/// Whether a game is running, as the plan shows it; `live`: silvermere's, looked at already.
fn running_json(engine: &Engine, live: Option<&Live>) -> Value {
    match (engine, live) {
        (Engine::Wcc { dir, .. } | Engine::Db { dir, .. }, _) => {
            let r = running::check(dir);
            json!({ "running": r.running(), "by": r.by, "elsewhere": r.elsewhere })
        }
        (Engine::Silvermere { .. }, Some(live)) => match &live.error {
            None => {
                let by = live
                    .running
                    .as_deref()
                    .map(|h| match control::holder_pid(h) {
                        Some(pid) => format!("pid {pid}"),
                        None => h.to_string(),
                    });
                json!({ "running": live.running.is_some(), "by": by, "elsewhere": [] })
            }
            Some(e) => json!({ "running": null, "error": e, "elsewhere": [] }),
        },
        _ => json!({ "running": null, "elsewhere": [] }),
    }
}

/// How the one way into silvermere goes, as its plan says.
fn silvermere_summary(live: Option<&Live>) -> String {
    let stopper = live.is_some_and(|l| l.stopper().is_some());
    let auto = live.is_some_and(|l| l.autostart && l.starter.is_some());
    match (stopper, auto) {
        (true, true) => "Daydream has silvermere warn the players and stop, writes its world from \
                         your working copy, and starts it again. A minute or two of downtime."
            .into(),
        (true, false) => {
            "Daydream has silvermere warn the players and stop, then writes its world \
                          from your working copy. You start it again."
                .into()
        }
        (false, true) => {
            "You stop silvermere while Daydream waits. Then Daydream writes its world \
                          from your working copy, and starts it again."
                .into()
        }
        (false, false) => "silvermere is stopped while Daydream waits, then its world is written \
                           from your working copy, and you start it again."
            .into(),
    }
}

/// The file of `dir` named `name`, in any case.
fn file_in(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        e.file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
            .then(|| e.path())
    })
}

/// Why the update way cannot be taken, if it cannot: the changes it cannot carry, or a
/// missing update file.
fn update_blocked(
    realm: &Realm,
    engine: &Engine,
) -> Result<(Option<String>, Vec<String>), ApiError> {
    let Engine::Wcc { dir, .. } = engine else {
        return Ok((
            Some("only MajorMUD's own files have an update file".into()),
            Vec::new(),
        ));
    };
    let upd = engine.update_file();
    if daydream_store::update::UpdateFile::find(realm)?.is_none() {
        return Ok((
            Some(format!(
                "the working copy has no update file: the game folder had no {upd} when it was read"
            )),
            Vec::new(),
        ));
    }
    if file_in(dir, upd).is_none() {
        return Ok((Some(format!("the game folder has no {upd}")), Vec::new()));
    }
    let not = daydream_store::dat::not_carried(realm).map_err(dat_error)?;
    let why = (!not.is_empty()).then(|| {
        format!(
            "{} of the changes {} not carried by the update file",
            not.len(),
            if not.len() == 1 { "is" } else { "are" }
        )
    });
    Ok((why, not))
}

/// What the Publish page shows before anything is published, and what a publish starts
/// from.
struct Plan {
    json: Value,
    engine: Engine,
    /// silvermere as it is now.
    live: Option<Live>,
    /// The world has an update file.
    update: bool,
}

fn plan(s: &AppState) -> Result<Plan, ApiError> {
    let cur = s.current();
    let dir = s.realm_dir()?;
    let realm = Realm::open(&dir)?;
    let engine = Engine::of(cur.game.as_ref(), realm.layout_set(), &dir);
    let changes = daydream_store::changes::list(&realm)?;
    let n = changes.changes.len();
    let empty = n == 0 && changes.since.is_some();
    let live = match (&engine, cur.game.as_ref()) {
        (Engine::Silvermere { url }, Some(game)) => Some(Live::look(
            url,
            &control::load(&s.games.workspace(), &game.source),
        )),
        _ => None,
    };
    let running = running_json(&engine, live.as_ref());
    let is_running = running["running"].as_bool() == Some(true);

    let mut checks: Vec<Value> = Vec::new();
    let mut check =
        |level: &str, text: String| checks.push(json!({ "level": level, "text": text }));
    match &engine {
        Engine::InPlace if cur.game.is_some() => check(
            "ok",
            "This world is the game's own files: every save is already in the game.".into(),
        ),
        Engine::InPlace => check(
            "ok",
            "This realm folder is edited where it is: there is no game to publish into.".into(),
        ),
        _ if empty => check("ok", "Nothing is waiting to publish.".into()),
        _ => {}
    }
    match &engine {
        Engine::InPlace => {}
        _ if changes.since.is_some() => {}
        // only the edits go into a game's .db files, so they must be known
        Engine::Db { .. } => check(
            "block",
            "This working copy has no list of its changes, so they cannot be told from what \
             play changed. Read the game again to make a fresh copy."
                .into(),
        ),
        _ => check(
            "note",
            "This world was edited before Daydream kept a list of its changes, so publishing \
             writes it whole, once."
                .into(),
        ),
    }
    if let Some(by) = running["by"].as_str().filter(|_| is_running) {
        check(
            "note",
            match (&engine, live.as_ref()) {
                (Engine::Wcc { .. }, _) => format!(
                    "{} is running ({by}). Publishing at the next start needs no stop; \
                     publishing now waits for it to stop.",
                    engine.name()
                ),
                (Engine::Db { .. }, _) => format!(
                    "{} is running ({by}): you go on editing the copy while it runs, and \
                     publishing waits for you to stop it.",
                    engine.name()
                ),
                (_, Some(l)) if l.stopper().is_some() => match l.players.as_ref().map(Vec::len) {
                    Some(0) => "silvermere is running, with nobody in the game: publishing \
                                stops it at once."
                        .into(),
                    Some(k) => format!(
                        "silvermere is running, with {k} player{} in the game: publishing \
                         warns them, then stops it.",
                        if k == 1 { "" } else { "s" }
                    ),
                    None => {
                        "silvermere is running: publishing warns the players, then stops it.".into()
                    }
                },
                _ => format!("silvermere is running ({by}): publishing waits for you to stop it."),
            },
        );
    }
    if let Some(l) = &live {
        match l.token {
            Token::Unknown => check(
                "warn",
                "silvermere does not know Daydream's admin token yet: it reads its tokens when \
                 it starts. This publish waits while you stop silvermere, and the next one can \
                 do it all."
                    .into(),
            ),
            Token::None if is_running => check(
                "note",
                "Daydream has no admin token for silvermere, so you stop it yourself. Add one \
                 under Server to let Daydream warn the players and stop it."
                    .into(),
            ),
            _ => {}
        }
        if let Some(admin) = l.admin.as_ref().filter(|_| is_running && !l.answers) {
            check(
                "note",
                format!(
                    "silvermere's admin API does not answer at {}, so you stop silvermere yourself.",
                    admin.base()
                ),
            );
        }
        if let Some(planned) = &l.planned {
            let when = planned["at"]
                .as_str()
                .and_then(|at| at.parse::<jiff::Timestamp>().ok())
                .map(|at| (at.as_second() - jiff::Timestamp::now().as_second()).max(0))
                .map_or_else(
                    || "soon".to_string(),
                    // whole minutes are close enough this far off
                    |secs| {
                        let secs = secs as u64;
                        let secs = if secs >= 120 {
                            (secs + 30) / 60 * 60
                        } else {
                            secs
                        };
                        format!("in {}", how_long(secs))
                    },
                );
            check(
                "note",
                format!(
                    "silvermere plans to shut down already, {when}: publishing replaces that \
                     with its own countdown."
                ),
            );
        }
    }
    if let Some(e) = running["error"].as_str() {
        check("warn", format!("Daydream could not look at the game: {e}"));
    }
    if let Some(others) = running["elsewhere"].as_array().filter(|a| !a.is_empty()) {
        let others: Vec<&str> = others.iter().filter_map(Value::as_str).collect();
        check(
            "note",
            format!(
                "Other games run on this computer ({}). Daydream cannot tell whether they use \
                 this folder.",
                others.join(", ")
            ),
        );
    }

    let blocked = empty || checks.iter().any(|c| c["level"] == "block");
    let update = has_update(&realm);
    let mut ways = Vec::new();
    for &way in engine.ways() {
        let (why, not_carried) = match way {
            Way::Update => update_blocked(&realm, &engine)?,
            Way::Now => (None, Vec::new()),
        };
        let (title, summary, button) = match way {
            Way::Update => (
                "At the next start".to_string(),
                format!(
                    "Your changes go into the game's update file, {}. MajorMUD merges it at its \
                     next start or nightly cleanup, and keeps what play changed. Nobody has to \
                     stop the BBS.",
                    engine.update_file()
                ),
                "Hold to write the update file".to_string(),
            ),
            Way::Now => (
                format!("Now, with {} stopped", engine.name()),
                match &engine {
                    Engine::Silvermere { .. } => silvermere_summary(live.as_ref()),
                    Engine::Db { .. } => format!(
                        "{} is stopped the way you usually do while Daydream waits, then your \
                         changes go into its .db files, each onto the record as play left it.",
                        engine.name()
                    ),
                    _ => format!(
                        "{} is stopped from its console while Daydream waits, then the \
                         changes go straight into its files. Deletions, renumbering and \
                         players need this way.",
                        engine.name()
                    ),
                },
                "Hold to publish".to_string(),
            ),
        };
        ways.push(json!({
            "id": way,
            "title": title,
            "summary": summary,
            "button": button,
            "ready": why.is_none() && !blocked,
            "why": why,
            "not_carried": not_carried,
            "steps": steps(&engine, way, update, live.as_ref()),
        }));
    }
    let way = ways
        .iter()
        .find(|w| w["ready"] == true)
        .or(ways.last())
        .map(|w| w["id"].clone());
    let label = cur.game.as_ref().map(|g| g.label.clone());
    let location = cur.game.as_ref().map(|g| g.source.shown());
    let json = json!({
        "engine": engine.id(),
        "name": engine.name(),
        "game": label,
        "location": location,
        "running": running,
        "changes": n,
        "since": changes.since,
        "ways": ways,
        "way": way,
        "checks": checks,
        "control": live.as_ref().map(|l| l.json(s.custom_allowed())),
        "job": s.publish.snapshot(),
    });
    Ok(Plan {
        json,
        engine,
        live,
        update,
    })
}

// ---- the job -----------------------------------------------------------------------------

/// A publish under way, or the last one.
#[derive(Debug, Clone, Serialize)]
pub struct JobState {
    pub id: u64,
    pub way: Way,
    pub engine: &'static str,
    pub game: String,
    /// `running`, `done`, `failed` or `cancelled`.
    pub state: &'static str,
    /// Seconds since 1970.
    pub started: u64,
    pub finished: Option<u64>,
    pub steps: Vec<Step>,
    /// Edits that could not be written, and why: they stay among the changes.
    pub skipped: Vec<String>,
    pub error: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Job {
    cancel: AtomicBool,
    state: Mutex<JobState>,
}

impl Job {
    fn lock(&self) -> MutexGuard<'_, JobState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn step(&self, id: &str, f: impl FnOnce(&mut Step)) {
        if let Some(s) = self.lock().steps.iter_mut().find(|s| s.id == id) {
            f(s);
        }
    }

    fn set(&self, id: &str, state: &'static str) {
        self.step(id, |s| s.state = state);
    }

    fn has(&self, id: &str) -> bool {
        self.lock().steps.iter().any(|s| s.id == id)
    }

    fn state_of(&self, id: &str) -> Option<&'static str> {
        self.lock()
            .steps
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.state)
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn log(&self, id: &str, line: impl Into<String>) {
        let line = line.into();
        tracing::info!("publish, {id}: {line}");
        self.step(id, |s| s.log.push(line));
    }
}

/// The publishes of a server: the one under way, or the last.
#[derive(Debug, Default)]
pub(crate) struct Jobs {
    current: Mutex<Option<Arc<Job>>>,
    next: AtomicU64,
}

impl Jobs {
    fn current(&self) -> Option<Arc<Job>> {
        self.current
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub(crate) fn snapshot(&self) -> Option<JobState> {
        self.current().map(|j| j.lock().clone())
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Why a job stopped before its end.
enum Stop {
    Cancelled,
    Failed(String),
}

impl From<ApiError> for Stop {
    fn from(e: ApiError) -> Self {
        Stop::Failed(e.1)
    }
}

impl From<DatError> for Stop {
    fn from(e: DatError) -> Self {
        Stop::Failed(e.to_string())
    }
}

impl From<daydream_store::StoreError> for Stop {
    fn from(e: daydream_store::StoreError) -> Self {
        Stop::Failed(e.to_string())
    }
}

impl From<daydream_silvermere::Error> for Stop {
    fn from(e: daydream_silvermere::Error) -> Self {
        Stop::Failed(e.to_string())
    }
}

fn dat_error(e: DatError) -> ApiError {
    match e {
        DatError::Store(e) => e.into(),
        e => ApiError(StatusCode::CONFLICT, e.to_string()),
    }
}

/// What a job works on, fixed when it starts.
struct Ctx {
    realm_dir: PathBuf,
    engine: Engine,
    way: Way,
    /// silvermere as it was when the publish began.
    live: Option<Live>,
    /// How the players are warned.
    warn: Warn,
    /// Where a game Daydream starts by its command line writes its output.
    log: PathBuf,
}

fn backup_dir(dir: &Path) -> PathBuf {
    let at = jiff::Zoned::now().strftime("%Y%m%d-%H%M%S").to_string();
    dir.join(format!("daydream-backup-{at}"))
}

fn copy_into(file: &Path, dir: &Path) -> Result<(), Stop> {
    std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::copy(file, dir.join(file.file_name().unwrap_or_default())))
        .map(drop)
        .map_err(|e| Stop::Failed(format!("backing up {}: {e}", file.display())))
}

/// `WCCITEM2.DAT: 1 record updated`, for a file written.
fn written_line(w: &Written) -> Option<String> {
    let name = w.dest.file_name()?.to_string_lossy().into_owned();
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    match w.action {
        Action::Unchanged => None,
        Action::Updated(n) => Some(format!("{name}: {n} record{} written in place", plural(n))),
        Action::Rebuilt(n) => Some(format!("{name}: written anew, {n} record{}", plural(n))),
        Action::Rows {
            changed,
            added,
            deleted,
        } => {
            let mut parts = Vec::new();
            for (n, what) in [(changed, "changed"), (added, "added"), (deleted, "deleted")] {
                if n > 0 {
                    parts.push(format!("{n} {what}"));
                }
            }
            Some(format!("{name}: {}", parts.join(", ")))
        }
    }
}

impl AppState {
    /// The open realm, if it is still the one the job started on.
    fn same_realm(&self, ctx: &Ctx) -> Result<Realm, Stop> {
        if self.current().realm_dir.as_deref() != Some(ctx.realm_dir.as_path()) {
            return Err(Stop::Failed(
                "another world was opened meanwhile: publish it from its own page".into(),
            ));
        }
        Ok(Realm::open(&ctx.realm_dir)?)
    }
}

/// Waits until `engine` has stopped, or the job is cancelled.
fn wait_stopped(job: &Job, engine: &Engine) -> Result<(), Stop> {
    let mut told = false;
    loop {
        let by = match engine {
            Engine::Silvermere { url } => {
                daydream_silvermere::block_on(daydream_silvermere::info(url))?.running
            }
            _ => engine.game_dir().and_then(|dir| running::check(dir).by),
        };
        let Some(by) = by else {
            job.step("stop", |s| s.cancel = false);
            job.log(
                "stop",
                if told {
                    format!("{} has stopped.", engine.name())
                } else {
                    format!("{} is not running.", engine.name())
                },
            );
            return Ok(());
        };
        if !told {
            job.log(
                "stop",
                format!(
                    "Waiting for {} to stop ({by}). Daydream carries on by itself once it has.",
                    engine.name()
                ),
            );
            job.step("stop", |s| s.cancel = true);
            told = true;
        }
        if job.cancelled() {
            return Err(Stop::Cancelled);
        }
        std::thread::sleep(POLL);
    }
}

/// `5 minutes`, `1 minute 30 seconds`, `10 seconds`.
fn how_long(secs: u64) -> String {
    let unit = |n: u64, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    let (m, s) = (secs / 60, secs % 60);
    match (m, s) {
        (0, s) => unit(s, "second"),
        (m, 0) => unit(m, "minute"),
        (m, s) => format!("{} {}", unit(m, "minute"), unit(s, "second")),
    }
}

/// silvermere stopped for a publish: with its admin API and a token it warns the players,
/// counts down and stops by itself; else Daydream waits while the sysop stops it. Whether
/// it was running.
fn stop_silvermere(job: &Job, url: &str, live: &Live, warn: &Warn) -> Result<bool, Stop> {
    let has_warn = job.has("warn");
    let holder = daydream_silvermere::block_on(daydream_silvermere::info(url))?.running;
    let Some(holder) = holder else {
        if has_warn {
            job.log("warn", "silvermere is not running: nobody to warn.");
            job.set("warn", "skipped");
        }
        job.set("stop", "running");
        job.log("stop", "silvermere is not running.");
        job.set("stop", "done");
        return Ok(false);
    };
    let pid = control::holder_pid(&holder);
    let asked = match live.admin.as_ref().filter(|_| has_warn) {
        Some(admin) => warn_players(job, admin, warn)?,
        None => false,
    };
    job.set("stop", "running");
    wait_released(job, url, pid, asked)?;
    job.set("stop", "done");
    Ok(true)
}

/// Has silvermere warn the players and stop: at once when nobody is in the game, else when
/// the countdown is over, which Cancel calls off. Whether silvermere took it; when it did
/// not, the sysop stops it.
fn warn_players(job: &Job, admin: &Admin, warn: &Warn) -> Result<bool, Stop> {
    job.set("warn", "running");
    let players = admin.who().ok().map(|p| p.len());
    let in_secs = if players == Some(0) { 0 } else { warn.in_secs };
    if in_secs == 0 && players != Some(0) && !warn.message.is_empty() {
        // no countdown, so silvermere warns nobody before its own notice: say why first
        let _ = admin.broadcast(&format!("The server shuts down now: {}", warn.message));
    }
    if let Err(e) = admin.shutdown(in_secs, &warn.message) {
        job.log(
            "warn",
            format!("silvermere did not take the shutdown: {e}. Stop it yourself: Daydream waits."),
        );
        job.set("warn", "skipped");
        return Ok(false);
    }
    job.log(
        "warn",
        match (players, in_secs) {
            (Some(0), _) => "Nobody is in the game: silvermere stops at once.".to_string(),
            (_, 0) => "silvermere stops now, and has told the players why.".to_string(),
            (Some(k), secs) => format!(
                "silvermere has told the {k} player{} in the game, and stops in {}.",
                if k == 1 { "" } else { "s" },
                how_long(secs)
            ),
            (None, secs) => format!(
                "silvermere has told the players, and stops in {}.",
                how_long(secs)
            ),
        },
    );
    if in_secs > 0 {
        let until = now_secs() + in_secs;
        job.step("warn", |s| {
            s.until = Some(until);
            s.cancel = true;
        });
        let end = Instant::now() + Duration::from_secs(in_secs);
        while Instant::now() < end {
            if job.cancelled() {
                job.step("warn", |s| {
                    s.until = None;
                    s.cancel = false;
                });
                match admin.cancel_shutdown() {
                    Ok(_) => job.log("warn", "Called off: silvermere told the players."),
                    Err(e) => job.log(
                        "warn",
                        format!(
                            "Daydream could not call it off ({e}): silvermere still stops when \
                             the time is up. Nothing is written."
                        ),
                    ),
                }
                return Err(Stop::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        job.step("warn", |s| s.cancel = false);
    }
    job.set("warn", "done");
    Ok(true)
}

/// Waits until silvermere lets its writer lock go. Asked to stop (`asked`), it has
/// [`STOP_WITHIN`]; else the sysop stops it, however long that takes, and Cancel calls the
/// wait off. silvermere started again meanwhile (another `pid` holds the lock) ends the
/// publish: what runs it restarts it, so Daydream would never have its turn.
fn wait_released(job: &Job, url: &str, pid: Option<u32>, asked: bool) -> Result<(), Stop> {
    let started = Instant::now();
    let mut told = false;
    loop {
        let holder = daydream_silvermere::block_on(daydream_silvermere::info(url))?.running;
        let Some(holder) = holder else {
            job.step("stop", |s| s.cancel = false);
            job.log(
                "stop",
                if asked {
                    "silvermere has stopped, every character saved."
                } else {
                    "silvermere has stopped."
                },
            );
            return Ok(());
        };
        if let (Some(was), Some(now)) = (pid, control::holder_pid(&holder))
            && was != now
        {
            return Err(Stop::Failed(format!(
                "silvermere started again by itself (pid {was}, now {now}): what runs it starts \
                 it again when it stops, as Docker's restart policy and NSSM do. Stop it there, \
                 then publish again. Nothing was written"
            )));
        }
        if asked {
            if started.elapsed() > STOP_WITHIN {
                return Err(Stop::Failed(
                    "silvermere has not stopped two minutes after it was asked to: look at its \
                     log. Nothing was written"
                        .into(),
                ));
            }
        } else {
            if !told {
                let pid = pid.map(|p| format!(" (pid {p})")).unwrap_or_default();
                job.log(
                    "stop",
                    format!(
                        "Waiting for silvermere{pid} to stop. Daydream carries on by itself \
                         once it has."
                    ),
                );
                job.step("stop", |s| s.cancel = true);
                told = true;
            }
            if job.cancelled() {
                return Err(Stop::Cancelled);
            }
        }
        std::thread::sleep(if asked {
            Duration::from_millis(500)
        } else {
            POLL
        });
    }
}

/// Starts silvermere again after a publish, as what ran it does, and waits until it answers.
/// What went wrong, to tell, when it did not start or answer.
fn start_silvermere(job: &Job, live: &Live, ran: bool, log: &Path) -> Option<String> {
    if !ran {
        job.log(
            "start",
            "silvermere was not running when the publish began, so it stays stopped.",
        );
        job.set("start", "skipped");
        return None;
    }
    let Some(starter) = live.starter.as_ref().filter(|_| live.autostart) else {
        job.set("start", "yours");
        return None;
    };
    job.set("start", "running");
    job.log("start", starter.command());
    if let Err(e) = starter.start(log) {
        job.log("start", e);
        job.set("start", "failed");
        return Some(format!(
            "Daydream could not start it again: start it with {}.",
            starter.command()
        ));
    }
    let Some(admin) = &live.admin else {
        job.log(
            "start",
            "Started. Daydream knows no admin API to ask whether it answers.",
        );
        job.set("start", "done");
        return None;
    };
    let t0 = Instant::now();
    loop {
        if admin.health().is_ok() {
            job.log(
                "start",
                format!(
                    "silvermere answers again, {:.1} s after it was started.",
                    t0.elapsed().as_secs_f32()
                ),
            );
            job.set("start", "done");
            return None;
        }
        if t0.elapsed() > ANSWER_WITHIN {
            job.log(
                "start",
                format!("No answer at {} for two minutes.", admin.base()),
            );
            job.set("start", "failed");
            return Some(match starter {
                Starter::Process { .. } => format!(
                    "it has not answered since Daydream started it: its output is in {}.",
                    log.display()
                ),
                _ => "it has not answered since Daydream started it: look at its log.".into(),
            });
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Runs a job's steps; the result line on success.
fn run_steps(s: &AppState, job: &Job, ctx: &Ctx) -> Result<String, Stop> {
    let upd = ctx.engine.update_file();

    // check
    job.set("check", "running");
    let realm = s.same_realm(ctx)?;
    let changes = daydream_store::changes::list(&realm)?;
    let n = changes.changes.len();
    if n == 0 && changes.since.is_some() {
        return Err(Stop::Failed("nothing is waiting to publish".into()));
    }
    if ctx.way == Way::Update {
        let (why, not) = update_blocked(&realm, &ctx.engine)?;
        if let Some(why) = why {
            return Err(Stop::Failed(format!("{why}: {}", not.join("; "))));
        }
    }
    job.log(
        "check",
        match changes.since {
            Some(_) => format!("{n} change{} to publish.", if n == 1 { "" } else { "s" }),
            None => "The whole world is written, once.".into(),
        },
    );
    job.set("check", "done");

    // stop: silvermere stops when asked to; the other games wait for the sysop
    let mut ran = false;
    if ctx.way == Way::Now {
        match (&ctx.engine, &ctx.live) {
            (Engine::Silvermere { url }, Some(live)) => {
                ran = stop_silvermere(job, url, live, &ctx.warn)?;
            }
            _ => {
                job.set("stop", "running");
                wait_stopped(job, &ctx.engine)?;
                job.set("stop", "done");
            }
        }
    }

    // back up and write, with no save in between
    let (_writing, _) = s.lock_write(true, "Publish")?;
    let realm = s.same_realm(ctx)?;
    let result = match &ctx.engine {
        Engine::Wcc { dir, .. } => {
            let backup = backup_dir(dir);
            job.set("backup", "running");
            let files: Vec<PathBuf> = match ctx.way {
                Way::Update => file_in(dir, upd).into_iter().collect(),
                Way::Now => {
                    let dry = WriteOptions {
                        dry_run: true,
                        ..WriteOptions::default()
                    };
                    daydream_store::dat::write_dat(&realm, dir, &dry)?
                        .into_iter()
                        .filter(|w| w.action != Action::Unchanged && w.dest.exists())
                        .map(|w| w.dest)
                        .collect()
                }
            };
            for f in &files {
                copy_into(f, &backup)?;
            }
            job.log(
                "backup",
                if files.is_empty() {
                    "Nothing to copy: no file the write changes exists yet.".to_string()
                } else {
                    format!(
                        "{} file{} copied to {}.",
                        files.len(),
                        if files.len() == 1 { "" } else { "s" },
                        backup.display()
                    )
                },
            );
            job.set("backup", "done");

            job.set("write", "running");
            let options = WriteOptions::default();
            let written = match ctx.way {
                Way::Update => vec![daydream_store::dat::write_update_file(
                    &realm, dir, &options,
                )?],
                Way::Now => daydream_store::dat::write_dat(&realm, dir, &options)?,
            };
            for w in &written {
                if let Some(line) = written_line(w) {
                    job.log("write", line);
                }
                job.lock().skipped.extend(w.skipped.iter().cloned());
            }
            if written.iter().all(|w| w.action == Action::Unchanged) {
                job.log("write", "Every file already held these records.");
            }
            job.set("write", "done");
            match ctx.way {
                Way::Update => {
                    job.set("live", "yours");
                    format!(
                        "Your changes are in {upd}. They go live at the game's next start or \
                         nightly cleanup."
                    )
                }
                Way::Now => {
                    job.set("start", "yours");
                    format!(
                        "Your changes are in the game's files. Start {} again when you are ready.",
                        ctx.engine.name()
                    )
                }
            }
        }
        Engine::Silvermere { url } => {
            let written = write_silvermere(job, ctx, url, &realm);
            // started again even when the write failed: the game is back as it was
            let trouble = match &ctx.live {
                Some(live) => start_silvermere(job, live, ran, &ctx.log),
                None => {
                    job.set("start", "yours");
                    None
                }
            };
            written?;
            match (trouble, job.state_of("start")) {
                (Some(t), _) => format!("Your changes are in silvermere's world, but {t}"),
                (None, Some("done")) => {
                    "Your changes are in silvermere's world, and it is running again.".into()
                }
                (None, Some("skipped")) => "Your changes are in silvermere's world. It was not \
                                            running, so it stays stopped."
                    .into(),
                _ => "Your changes are in silvermere's world. Start it again when you are ready."
                    .into(),
            }
        }
        Engine::Db { dir, .. } => {
            write_db(job, dir, &realm)?;
            job.set("start", "yours");
            format!(
                "Your changes are in {}'s files. Start it again when you are ready.",
                ctx.engine.name()
            )
        }
        Engine::InPlace => return Err(Stop::Failed("this world has nothing to publish".into())),
    };
    let skipped = job.lock().skipped.len();
    Ok(if skipped == 0 {
        result
    } else {
        format!(
            "{result} {skipped} edit{} could not be written and stay among the changes.",
            if skipped == 1 { "" } else { "s" }
        )
    })
}

/// A game's `.db` files backed up, and the working copy's edits written into them.
fn write_db(job: &Job, dir: &Path, realm: &Realm) -> Result<(), Stop> {
    job.set("backup", "running");
    let dry = WriteOptions {
        dry_run: true,
        ..WriteOptions::default()
    };
    let files: Vec<PathBuf> = daydream_store::game_db::write_db(realm, dir, &dry)?
        .into_iter()
        .filter(|w| w.action != Action::Unchanged)
        .map(|w| w.dest)
        .collect();
    let backup = backup_dir(dir);
    if !files.is_empty() {
        std::fs::create_dir_all(&backup)
            .map_err(|e| Stop::Failed(format!("backing up into {}: {e}", backup.display())))?;
    }
    for f in &files {
        // a consistent copy of the file, its write-ahead log included
        daydream_store::WbtrvDb::snapshot(f, &backup.join(f.file_name().unwrap_or_default()))?;
    }
    job.log(
        "backup",
        if files.is_empty() {
            "Nothing to copy: the game's files already hold these records.".to_string()
        } else {
            format!(
                "{} file{} copied to {}.",
                files.len(),
                if files.len() == 1 { "" } else { "s" },
                backup.display()
            )
        },
    );
    job.set("backup", "done");

    job.set("write", "running");
    let written = daydream_store::game_db::write_db(realm, dir, &WriteOptions::default())?;
    for w in &written {
        if let Some(line) = written_line(w) {
            job.log("write", line);
        }
        job.lock().skipped.extend(w.skipped.iter().cloned());
    }
    if written.iter().all(|w| w.action == Action::Unchanged) {
        job.log("write", "Every file already held these records.");
    }
    job.set("write", "done");
    Ok(())
}

/// silvermere's database backed up, and its world written from the working copy.
fn write_silvermere(job: &Job, ctx: &Ctx, url: &str, realm: &Realm) -> Result<(), Stop> {
    job.set("backup", "running");
    match daydream_silvermere::sqlite_path(url) {
        Some(p) => {
            let p = PathBuf::from(p);
            let backup = backup_dir(p.parent().unwrap_or(Path::new(".")));
            for f in [p.clone(), PathBuf::from(format!("{}-wal", p.display()))] {
                if f.exists() {
                    copy_into(&f, &backup)?;
                }
            }
            job.log("backup", format!("Copied to {}.", backup.display()));
            job.set("backup", "done");
        }
        None => {
            job.log("backup", "PostgreSQL: not copied by Daydream.");
            job.set("backup", "skipped");
        }
    }
    job.set("write", "running");
    let tables = daydream_silvermere::block_on(daydream_silvermere::export(
        realm,
        url,
        &daydream_silvermere::ExportOptions::default(),
    ))?;
    let rows: usize = tables.iter().map(|t| t.updated + t.inserted).sum();
    job.log(
        "write",
        format!(
            "{} tables written from the working copy, {rows} rows.",
            tables.len()
        ),
    );
    for t in tables.iter().filter(|t| t.inserted + t.deleted > 0) {
        job.log(
            "write",
            format!("{}: {} new, {} removed", t.table, t.inserted, t.deleted),
        );
    }
    // silvermere now holds the working copy's world: its changes count from here
    daydream_store::journal::establish(&ctx.realm_dir, &[])?;
    job.set("write", "done");
    Ok(())
}

fn run(s: Arc<AppState>, job: Arc<Job>, ctx: Ctx) {
    let outcome = run_steps(&s, &job, &ctx);
    let mut st = job.lock();
    st.finished = Some(now_secs());
    let open = st
        .steps
        .iter()
        .position(|s| s.state == "running")
        .or_else(|| st.steps.iter().position(|s| s.state == "waiting"));
    match outcome {
        Ok(result) => {
            st.state = "done";
            st.result = Some(result);
        }
        Err(Stop::Cancelled) => {
            st.state = "cancelled";
            if let Some(i) = open {
                st.steps[i].state = "skipped";
                st.steps[i]
                    .log
                    .push("Cancelled: nothing was written.".into());
            }
        }
        Err(Stop::Failed(e)) => {
            tracing::warn!("publish failed: {e}");
            st.state = "failed";
            if let Some(i) = open {
                st.steps[i].state = "failed";
            }
            st.error = Some(e);
        }
    }
    for s in &mut st.steps {
        if s.state == "waiting" || s.state == "running" {
            s.state = "skipped";
        }
    }
}

// ---- routes ------------------------------------------------------------------------------

pub(crate) async fn get(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    blocking(move || plan(&s).map(|p| Json(p.json))).await
}

pub(crate) async fn job(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!(s.publish.snapshot()))
}

/// How silvermere's players are warned before it stops for a publish.
#[derive(Debug, Clone, Deserialize)]
pub struct Warn {
    /// Seconds from the warning to the stop (at most an hour); at once with nobody in the
    /// game.
    #[serde(default = "Warn::default_in_secs")]
    pub in_secs: u64,
    /// Why it closes, as the players are told: `The server will shut down in 5 minutes:
    /// <message>`.
    #[serde(default)]
    pub message: String,
}

impl Warn {
    fn default_in_secs() -> u64 {
        300
    }
}

impl Default for Warn {
    fn default() -> Self {
        Self {
            in_secs: Self::default_in_secs(),
            message: DEFAULT_MESSAGE.into(),
        }
    }
}

/// The body of a publish.
#[derive(Debug, Deserialize)]
pub struct StartJson {
    way: Way,
    #[serde(default)]
    warn: Option<Warn>,
}

pub(crate) async fn start(
    State(s): State<Arc<AppState>>,
    Json(body): Json<StartJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let conflict = |m: String| ApiError(StatusCode::CONFLICT, m);
        let bad = |m: String| ApiError(StatusCode::BAD_REQUEST, m);
        let mut warn = body.warn.unwrap_or_default();
        warn.message = warn.message.trim().to_string();
        if warn.in_secs > MAX_WARN {
            return Err(bad("warn the players at most an hour ahead".into()));
        }
        control::check_line(&warn.message, control::MAX_MESSAGE, "message").map_err(bad)?;
        // the plan reads the job too: before the lock is taken
        let plan = plan(&s)?;
        let mut current = s.publish.current.lock().unwrap_or_else(|e| e.into_inner());
        if current
            .as_ref()
            .is_some_and(|j| j.lock().state == "running")
        {
            return Err(conflict("a publish is under way already".into()));
        }
        let way = plan.json["ways"]
            .as_array()
            .and_then(|w| w.iter().find(|w| w["id"] == json!(body.way)))
            .ok_or_else(|| conflict(format!("this world does not publish {:?}", body.way)))?;
        if way["ready"] != true {
            let block = plan.json["checks"]
                .as_array()
                .and_then(|c| c.iter().find(|c| c["level"] == "block"))
                .and_then(|c| c["text"].as_str());
            let why = match (way["why"].as_str(), block) {
                (Some(why), _) | (None, Some(why)) => why.to_string(),
                _ if plan.json["changes"] == 0 => "nothing is waiting to publish".into(),
                _ => "this way cannot be taken now".into(),
            };
            return Err(conflict(why));
        }
        let realm_dir = s.realm_dir()?;
        let Plan {
            json: plan_json,
            engine,
            live,
            update,
        } = plan;
        let state = JobState {
            id: s.publish.next.fetch_add(1, Ordering::Relaxed) + 1,
            way: body.way,
            engine: engine.id(),
            game: plan_json["game"].as_str().unwrap_or_default().to_string(),
            state: "running",
            started: now_secs(),
            finished: None,
            steps: steps(&engine, body.way, update, live.as_ref()),
            skipped: Vec::new(),
            error: None,
            result: None,
        };
        let job = Arc::new(Job {
            cancel: AtomicBool::new(false),
            state: Mutex::new(state.clone()),
        });
        *current = Some(job.clone());
        drop(current);
        let log = match s.current().game.as_ref() {
            Some(g) => control::process_log(&s, &g.source),
            None => realm_dir.join(".daydream").join("game.log"),
        };
        let ctx = Ctx {
            realm_dir,
            engine,
            way: body.way,
            live,
            warn,
            log,
        };
        let s2 = s.clone();
        std::thread::Builder::new()
            .name("publish".into())
            .spawn(move || run(s2, job, ctx))
            .map_err(crate::tools::internal)?;
        Ok(Json(json!(state)))
    })
    .await
}

/// Calls off a job while its step may be: silvermere's countdown (silvermere tells the
/// players), or the wait for the sysop to stop the game.
pub(crate) async fn cancel(State(s): State<Arc<AppState>>) -> Json<Value> {
    let job = s.publish.current();
    let may = job.as_ref().is_some_and(|j| {
        let st = j.lock();
        st.state == "running" && st.steps.iter().any(|s| s.state == "running" && s.cancel)
    });
    if let Some(j) = job.filter(|_| may) {
        j.cancel.store(true, Ordering::Relaxed);
    }
    Json(json!({ "cancelled": may }))
}
