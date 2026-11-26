//! Publishing: the open world's changes into its game, one step at a time, as the Publish
//! page shows them.
//!
//! Each engine has its ways ([`Way`]), and each way its steps:
//!
//! | Engine | Way | Steps |
//! |---|---|---|
//! | Worldgroup, The Major BBS (`WCC*.DAT`) | `update`: at the next start | check, back up, write the update file, live at the next start or cleanup |
//! | | `now`: with the game stopped | check, stop the game (Daydream waits), back up, write, start it again |
//! | silvermere | `now` | check, stop silvermere (Daydream waits for its writer lock), back up, write, start it again |
//!
//! A world edited where it is (MBBSEmu's `.db` files) has nothing to publish.
//!
//! The update way needs no stop. MajorMUD merges its update file into its world at every start
//! and nightly cleanup, keeping what play changed (floors, shop stock, kill timers), so this
//! publish writes the update file alone ([`daydream_store::dat::write_update_file`]). It
//! carries edits and additions of the ten kinds of record the file holds, not deletions,
//! renumbering, players or what play changes ([`daydream_store::dat::not_carried`]); a world
//! with such changes publishes now.
//!
//! Daydream cannot stop or start Worldgroup or The Major BBS, so publishing now waits while the
//! sysop stops the game, carries on by itself once it has, and leaves starting it again to
//! the sysop. One publish runs at a time, in the background, and the page follows it.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/publish` | the plan: `{engine, game, location, running, changes, since, ways: [{id, title, summary, ready, why, not_carried, steps, button}], way, checks, job}` |
//! | `POST /api/publish` | `{way}`: starts publishing that way; the job |
//! | `GET /api/publish/job` | the job under way or the last one (`null` before any) |
//! | `POST /api/publish/cancel` | stops a job that is still waiting for the game to stop |
//!
//! A job is `{id, way, engine, game, state, started, finished, steps: [{id, title, detail,
//! state, log}], skipped, error, result}`: `state` is `running`, `done`, `failed` or
//! `cancelled`, a step's is `waiting`, `running`, `done`, `failed`, `skipped` or `yours` (for
//! the sysop to do), and `skipped` lists the edits that could not be written, which stay among
//! the changes.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use daydream_store::Realm;
use daydream_store::dat::{Action, DatError, WriteOptions, Written};
use daydream_types::records::layout::LayoutSet;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::games::{Game, Source, running};
use crate::{ApiError, AppState, blocking};

/// How often a waiting step looks at the game again.
const POLL: Duration = Duration::from_secs(2);

/// What the open world publishes into.
#[derive(Debug, Clone)]
enum Engine {
    /// A folder of `WCC*.DAT` files: Worldgroup's (`WCCUPDA2`) or The Major BBS's (`WCCUPDAT`).
    Wcc { dir: PathBuf, worldgroup: bool },
    /// silvermere's database.
    Silvermere { url: String },
    /// Its own files, edited where they are: nothing to publish.
    InPlace,
}

impl Engine {
    fn of(game: Option<&Game>, set: LayoutSet) -> Self {
        match game.map(|g| &g.source) {
            Some(Source::Wcc { dir }) => Self::Wcc {
                dir: dir.clone(),
                worldgroup: set == LayoutSet::Worldgroup,
            },
            Some(Source::Silvermere { url }) => Self::Silvermere { url: url.clone() },
            _ => Self::InPlace,
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::Wcc {
                worldgroup: true, ..
            } => "worldgroup",
            Self::Wcc { .. } => "majorbbs",
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
            _ => "WCCUPDAT.DAT",
        }
    }

    fn ways(&self) -> &'static [Way] {
        match self {
            Self::Wcc { .. } => &[Way::Update, Way::Now],
            Self::Silvermere { .. } => &[Way::Now],
            Self::InPlace => &[],
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
}

fn step(id: &'static str, title: impl Into<String>, detail: impl Into<String>) -> Step {
    Step {
        id,
        title: title.into(),
        detail: detail.into(),
        state: "waiting",
        log: Vec::new(),
    }
}

/// Whether the world has an update file, which publishing now writes too.
fn has_update(realm: &Realm) -> bool {
    matches!(daydream_store::update::UpdateFile::find(realm), Ok(Some(_)))
}

/// The steps of publishing `way` into `engine`; `update`: the world has an update file.
fn steps(engine: &Engine, way: Way, update: bool) -> Vec<Step> {
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
        (Engine::Silvermere { url }, _) => vec![
            check,
            step(
                "stop",
                "Stop silvermere",
                "Stop it with silvermere sysop shutdown, daydream-stack edit, or its service. \
                 Daydream waits for its writer lock, and carries on once it is free.",
            ),
            step(
                "backup",
                "Back up",
                if daydream_silvermere::sqlite_path(url).is_some() {
                    "The database file is copied to a daydream-backup folder beside it first."
                } else {
                    "Back up PostgreSQL your usual way: Daydream copies only SQLite files."
                },
            ),
            step(
                "write",
                "Write the world",
                "The world's tables are written from the working copy. Players, what lies on \
                 the floors and the rest of what play changed stay as they are.",
            ),
            step(
                "start",
                "Start silvermere again",
                "Start it with daydream-stack play, or its service.",
            ),
        ],
        (Engine::InPlace, _) => Vec::new(),
    }
}

// ---- the plan ------------------------------------------------------------------------------

/// Whether a game is running, as the plan shows it.
fn running_json(engine: &Engine) -> Value {
    match engine {
        Engine::Wcc { dir, .. } => {
            let r = running::check(dir);
            json!({ "running": r.running(), "by": r.by, "elsewhere": r.elsewhere })
        }
        Engine::Silvermere { url } => {
            match daydream_silvermere::block_on(daydream_silvermere::info(url)) {
                Ok(info) => {
                    json!({ "running": info.running.is_some(), "by": info.running, "elsewhere": [] })
                }
                Err(e) => json!({ "running": null, "error": e.to_string(), "elsewhere": [] }),
            }
        }
        Engine::InPlace => json!({ "running": null, "elsewhere": [] }),
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

/// What the Publish page shows before anything is published.
fn plan(s: &AppState) -> Result<Value, ApiError> {
    let cur = s.current();
    let dir = s.realm_dir()?;
    let realm = Realm::open(&dir)?;
    let engine = Engine::of(cur.game.as_ref(), realm.layout_set());
    let changes = daydream_store::changes::list(&realm)?;
    let n = changes.changes.len();
    let empty = n == 0 && changes.since.is_some();
    let running = running_json(&engine);
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
    if changes.since.is_none() && !matches!(engine, Engine::InPlace) {
        check(
            "note",
            "This world was edited before Daydream kept a list of its changes, so publishing \
             writes it whole, once."
                .into(),
        );
    }
    if let Some(by) = running["by"].as_str().filter(|_| is_running) {
        check(
            "note",
            match &engine {
                Engine::Wcc { .. } => format!(
                    "{} is running ({by}). Publishing at the next start needs no stop; \
                     publishing now waits for it to stop.",
                    engine.name()
                ),
                _ => format!("silvermere is running ({by}): publishing waits for it to stop."),
            },
        );
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
                    Engine::Silvermere { .. } => "silvermere is stopped, its world written, and \
                                                  started again."
                        .to_string(),
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
            "steps": steps(&engine, way, update),
        }));
    }
    let way = ways
        .iter()
        .find(|w| w["ready"] == true)
        .or(ways.last())
        .map(|w| w["id"].clone());
    let label = cur.game.as_ref().map(|g| g.label.clone());
    let location = cur.game.as_ref().map(|g| g.source.shown());
    Ok(json!({
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
        "job": s.publish.snapshot(),
    }))
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
            Engine::Wcc { dir, .. } => running::check(dir).by,
            Engine::Silvermere { url } => {
                daydream_silvermere::block_on(daydream_silvermere::info(url))?.running
            }
            Engine::InPlace => None,
        };
        let Some(by) = by else {
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
                format!("Waiting for {by} to stop. Daydream carries on by itself once it has."),
            );
            told = true;
        }
        if job.cancel.load(Ordering::Relaxed) {
            return Err(Stop::Cancelled);
        }
        std::thread::sleep(POLL);
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

    // stop: wait for the sysop
    if ctx.way == Way::Now {
        job.set("stop", "running");
        wait_stopped(job, &ctx.engine)?;
        job.set("stop", "done");
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
                &realm,
                url,
                &daydream_silvermere::ExportOptions::default(),
            ))?;
            for t in &tables {
                if t.inserted + t.deleted > 0 || t.updated > 0 {
                    job.log(
                        "write",
                        format!(
                            "{}: {} written, {} new, {} removed",
                            t.table, t.updated, t.inserted, t.deleted
                        ),
                    );
                }
            }
            // silvermere now holds the working copy's world: its changes count from here
            daydream_store::journal::establish(&ctx.realm_dir, &[])?;
            job.set("write", "done");
            job.set("start", "yours");
            "Your changes are in silvermere's world. Start it again when you are ready.".into()
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
    blocking(move || plan(&s).map(Json)).await
}

pub(crate) async fn job(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!(s.publish.snapshot()))
}

/// The body of a publish.
#[derive(Debug, Deserialize)]
pub struct StartJson {
    way: Way,
}

pub(crate) async fn start(
    State(s): State<Arc<AppState>>,
    Json(body): Json<StartJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let conflict = |m: String| ApiError(StatusCode::CONFLICT, m);
        // the plan reads the job too: before the lock is taken
        let plan = plan(&s)?;
        let mut current = s.publish.current.lock().unwrap_or_else(|e| e.into_inner());
        if current
            .as_ref()
            .is_some_and(|j| j.lock().state == "running")
        {
            return Err(conflict("a publish is under way already".into()));
        }
        let way = plan["ways"]
            .as_array()
            .and_then(|w| w.iter().find(|w| w["id"] == json!(body.way)))
            .ok_or_else(|| conflict(format!("this world does not publish {:?}", body.way)))?;
        if way["ready"] != true {
            let block = plan["checks"]
                .as_array()
                .and_then(|c| c.iter().find(|c| c["level"] == "block"))
                .and_then(|c| c["text"].as_str());
            let why = match (way["why"].as_str(), block) {
                (Some(why), _) | (None, Some(why)) => why.to_string(),
                _ if plan["changes"] == 0 => "nothing is waiting to publish".into(),
                _ => "this way cannot be taken now".into(),
            };
            return Err(conflict(why));
        }
        let realm_dir = s.realm_dir()?;
        let realm = Realm::open(&realm_dir)?;
        let engine = Engine::of(s.current().game.as_ref(), realm.layout_set());
        let state = JobState {
            id: s.publish.next.fetch_add(1, Ordering::Relaxed) + 1,
            way: body.way,
            engine: engine.id(),
            game: plan["game"].as_str().unwrap_or_default().to_string(),
            state: "running",
            started: now_secs(),
            finished: None,
            steps: steps(&engine, body.way, has_update(&realm)),
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
        let ctx = Ctx {
            realm_dir,
            engine,
            way: body.way,
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

pub(crate) async fn cancel(State(s): State<Arc<AppState>>) -> Json<Value> {
    let job = s.publish.current();
    let waiting = job.as_ref().is_some_and(|j| {
        let st = j.lock();
        st.state == "running"
            && st
                .steps
                .iter()
                .any(|s| s.id == "stop" && s.state == "running")
    });
    if let Some(j) = job.filter(|_| waiting) {
        j.cancel.store(true, Ordering::Relaxed);
    }
    Json(json!({ "cancelled": waiting }))
}
