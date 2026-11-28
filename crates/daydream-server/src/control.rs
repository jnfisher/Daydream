//! Running the game from Daydream: silvermere's admin API, and starting the game again
//! through what started it.
//!
//! **The admin API.** silvermere answers HTTP on its `admin_bind` (`[server] admin_bind` in
//! its `silvermere.toml`, `127.0.0.1:8080` when unset). `/healthz` is open; the rest needs a
//! bearer token silvermere knows: one made by `silvermere admin token new daydream --scopes
//! read,write,admin`, its `[[admin.tokens]]` block added to `silvermere.toml`, and silvermere
//! started again once, as it reads its tokens when it starts. With it Daydream sees who is
//! in the game, tells them something, and has silvermere warn them, count down and stop for
//! a publish ([`crate::publish`]).
//!
//! **Starting it again** needs no token. Daydream finds what runs silvermere ([`Starter`]):
//! its systemd unit (on Linux, from the running process's cgroup) or launchd agent (the
//! agent whose working folder is silvermere's), else the process itself, started by hand,
//! whose command line it runs again in the same folder. A command is never taken from a
//! request: Daydream runs only what it found, and only when the person lets it start the
//! game again (`autostart`: on for a service, off for a process started by hand, whose
//! output would no longer reach its terminal).
//!
//! Each game's settings are kept in `<workspace>/control/<source id>.json`, readable by its
//! owner only: the admin URL when it is not the one `silvermere.toml` names, the token, and
//! `autostart`.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/control` | the open game's: [`Live::json`] |
//! | `PUT /api/control` | `{admin?, token?, autostart?}`: changes them (`""` clears one); the same answer |
//! | `POST /api/control/say` | `{text}`: tells everyone in the game; `{delivered}` |
//!
//! With an access token, the admin URL is changed only where any source may be typed
//! (`serve --allow-any-source`): it is where Daydream sends the admin token. A new URL drops
//! the token, unless the same request gives one.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::games::discover::{canonical, toml_get};
use crate::games::workspace::{Workspace, write_private};
use crate::games::{Source, not_allowed};
use crate::{ApiError, AppState, blocking};

/// silvermere's admin API when its settings name none.
pub const DEFAULT_ADMIN_BIND: &str = "127.0.0.1:8080";

/// The longest message a shutdown carries (silvermere's own limit).
pub const MAX_MESSAGE: usize = 200;

/// The longest line told to everyone (silvermere's own limit).
pub const MAX_SAY: usize = 1000;

// ---- settings ------------------------------------------------------------------------------

/// What Daydream keeps about running one game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// The admin API's URL, when set by hand (else the one `silvermere.toml` names).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin: Option<String>,
    /// The admin token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Whether a publish starts the game again; `None`: as [`Starter::autostart`] says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
}

fn settings_path(ws: &Workspace, source: &Source) -> PathBuf {
    ws.dir()
        .join("control")
        .join(format!("{}.json", source.id()))
}

/// The settings kept for `source`; none yet, or unreadable, are the defaults.
pub fn load(ws: &Workspace, source: &Source) -> Settings {
    std::fs::read_to_string(settings_path(ws, source))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Keeps `settings` for `source`, readable by their owner only.
pub fn save(ws: &Workspace, source: &Source, settings: &Settings) -> std::io::Result<()> {
    let path = settings_path(ws, source);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    write_private(&path, &serde_json::to_vec_pretty(settings)?)
}

// ---- where silvermere is -------------------------------------------------------------------

/// silvermere's folder, for its SQLite database `url`: the one holding `silvermere.toml`,
/// the database's or the one above it, else the one above a `data` folder, else the
/// database's. `None` for PostgreSQL: the database says nothing of where silvermere runs.
pub fn silvermere_dir(url: &str) -> Option<PathBuf> {
    let db = Path::new(daydream_silvermere::sqlite_path(url)?);
    let folder = db.parent()?;
    let up = folder.parent().filter(|u| !u.as_os_str().is_empty());
    [Some(folder), up]
        .into_iter()
        .flatten()
        .find(|d| d.join("silvermere.toml").is_file())
        .or(match up {
            Some(up) if folder.file_name().is_some_and(|n| n == "data") => Some(up),
            _ => Some(folder),
        })
        .map(Path::to_path_buf)
}

/// The admin API a `silvermere.toml` names (`[server] admin_bind`; `None` for no file: the
/// default), as a URL. `None` when it is turned off (`admin_bind = ""`). An address that
/// listens everywhere is reached on this machine's loopback.
pub fn admin_url_of(toml: Option<&str>) -> Option<String> {
    let bind = toml
        .and_then(|t| toml_get(t, "server", "admin_bind"))
        .unwrap_or_else(|| DEFAULT_ADMIN_BIND.into());
    let (host, port) = bind.trim().rsplit_once(':')?;
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let host = match host {
        "" | "0.0.0.0" => "127.0.0.1",
        "[::]" | "::" => "[::1]",
        h => h,
    };
    Some(format!("http://{host}:{port}"))
}

/// The admin API's URL for silvermere on `url`, and where it came from: `set` by hand,
/// `silvermere.toml`, silvermere's `default`, or `unknown` (PostgreSQL, or turned off).
pub fn admin_of(url: &str, settings: &Settings) -> (Option<String>, &'static str) {
    if let Some(a) = settings.admin.as_ref().filter(|a| !a.is_empty()) {
        return (Some(a.clone()), "set");
    }
    let Some(dir) = silvermere_dir(url) else {
        return (None, "unknown");
    };
    match std::fs::read_to_string(dir.join("silvermere.toml")) {
        Ok(t) => match admin_url_of(Some(&t)) {
            Some(a) => (Some(a), "silvermere.toml"),
            None => (None, "unknown"),
        },
        Err(_) => (admin_url_of(None), "default"),
    }
}

/// An admin URL a person typed, made plain: `http(s)://host:port`, nothing after.
fn check_admin_url(given: &str) -> Result<String, ApiError> {
    let bad = || {
        ApiError(
            StatusCode::BAD_REQUEST,
            format!("{given}: give the admin API as http://host:port"),
        )
    };
    let rest = given
        .strip_prefix("http://")
        .or_else(|| given.strip_prefix("https://"))
        .ok_or_else(bad)?;
    let host = rest.trim_end_matches('/');
    if host.is_empty()
        || host.contains(['/', '@', '?', '#', ' '])
        || !host.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(bad());
    }
    let scheme = if given.starts_with("https") {
        "https"
    } else {
        "http"
    };
    Ok(format!("{scheme}://{host}"))
}

/// A token as silvermere makes them: visible ASCII, no spaces.
fn check_token(token: &str) -> Result<(), ApiError> {
    if token.len() > 512 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "that is not an admin token: paste the line silvermere printed after \
             \"Token (shown once, store it now):\""
                .into(),
        ));
    }
    Ok(())
}

/// A line for the players: one line of what a keyboard types, at most `max` characters.
pub fn check_line(text: &str, max: usize, what: &str) -> Result<(), String> {
    if text.chars().count() > max {
        return Err(format!("the {what} is longer than {max} characters"));
    }
    if text.chars().any(char::is_control) {
        return Err(format!("the {what} must be one line"));
    }
    Ok(())
}

// ---- the admin API -------------------------------------------------------------------------

/// Why silvermere's admin API did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminError {
    /// Nothing answers there.
    Unreachable(String),
    /// silvermere does not know the token (or none was given).
    Unauthorized,
    /// The token lacks the scope.
    Forbidden,
    /// Any other answer.
    Refused(u16, String),
}

impl AdminError {
    /// Whether it may pass in a moment: nothing answered, or silvermere was too busy to.
    pub fn passing(&self) -> bool {
        matches!(self, Self::Unreachable(_) | Self::Refused(500..=599, _))
    }
}

/// How long a quick failure may take and still be tried [`again`], and the pause before it.
const AGAIN_WITHIN: Duration = Duration::from_secs(2);
const AGAIN_AFTER: Duration = Duration::from_millis(300);

/// `f`, tried twice more a moment apart when it failed quickly with what may pass
/// (`passing`): silvermere busy for a moment, or its database, should not change what a
/// publish does. A slow failure is not waited for again.
pub(crate) fn again<T, E>(
    mut f: impl FnMut() -> Result<T, E>,
    passing: impl Fn(&E) -> bool,
) -> Result<T, E> {
    let t0 = Instant::now();
    let mut tries = 1;
    loop {
        match f() {
            Err(e) if tries < 3 && passing(&e) && t0.elapsed() < AGAIN_WITHIN => {
                tries += 1;
                std::thread::sleep(AGAIN_AFTER);
            }
            done => return done,
        }
    }
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(e) => write!(f, "its admin API does not answer ({e})"),
            Self::Unauthorized => f.write_str("silvermere does not know the admin token"),
            Self::Forbidden => f.write_str("the admin token lacks the scope this needs"),
            Self::Refused(code, msg) => write!(f, "silvermere answered {code}: {msg}"),
        }
    }
}

/// silvermere's admin API at one URL.
#[derive(Debug, Clone)]
pub struct Admin {
    base: String,
    token: Option<String>,
}

impl Admin {
    pub fn new(base: &str, token: Option<&str>) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            token: token.map(str::to_string),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, AdminError> {
        self.call_within(method, path, body, Duration::from_secs(8))
    }

    /// [`Self::call`], waiting up to `within` for the answer.
    fn call_within(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        within: Duration,
    ) -> Result<Value, AdminError> {
        // silvermere listens on this machine: never through a proxy
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .proxy(None)
            .timeout_connect(Some(Duration::from_secs(2)))
            .timeout_global(Some(within))
            .build()
            .into();
        let url = format!("{}{path}", self.base);
        let auth = self.token.as_ref().map(|t| format!("Bearer {t}"));
        let sent = match method {
            "GET" | "DELETE" => {
                let mut r = if method == "GET" {
                    agent.get(&url)
                } else {
                    agent.delete(&url)
                };
                if let Some(a) = &auth {
                    r = r.header("Authorization", a);
                }
                r.call()
            }
            _ => {
                let mut r = agent.post(&url).header("Content-Type", "application/json");
                if let Some(a) = &auth {
                    r = r.header("Authorization", a);
                }
                r.send(body.map(Value::to_string).unwrap_or_else(|| "{}".into()))
            }
        };
        let mut res = sent.map_err(|e| AdminError::Unreachable(e.to_string()))?;
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().unwrap_or_default();
        let json: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        match status {
            200..=299 => Ok(json),
            401 => Err(AdminError::Unauthorized),
            403 => Err(AdminError::Forbidden),
            _ => Err(AdminError::Refused(
                status,
                json["message"]
                    .as_str()
                    .or_else(|| json["error"].as_str())
                    .map_or_else(|| text.chars().take(200).collect(), str::to_string),
            )),
        }
    }

    /// `/healthz`: silvermere answers, and is well.
    pub fn health(&self) -> Result<Value, AdminError> {
        self.call("GET", "/healthz", None)
    }

    /// `/v1/who`: the players in the game (`read`).
    pub fn who(&self) -> Result<Vec<Value>, AdminError> {
        Ok(self.call("GET", "/v1/who", None)?["players"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// `/v1/shutdown`: the shutdown planned, if any (`read`).
    pub fn planned(&self) -> Result<Option<Value>, AdminError> {
        Ok(self
            .call("GET", "/v1/shutdown", None)?
            .get("planned")
            .filter(|p| !p.is_null())
            .cloned())
    }

    /// Tells every player in the game `text` (`write`); how many were told.
    pub fn broadcast(&self, text: &str) -> Result<u64, AdminError> {
        Ok(
            self.call("POST", "/v1/broadcast", Some(&json!({ "text": text })))?["delivered"]
                .as_u64()
                .unwrap_or(0),
        )
    }

    /// Plans a shutdown in `in_secs` (`admin`): silvermere warns the players as the time
    /// comes closer, then saves every character and stops.
    pub fn shutdown(&self, in_secs: u64, message: &str) -> Result<Value, AdminError> {
        self.call(
            "POST",
            "/v1/shutdown",
            Some(&json!({ "in_secs": in_secs, "message": message })),
        )
    }

    /// Whether this silvermere can hold its writes for a publish while it runs
    /// (`/v1/world-hold`, silvermere 1.0.5 and later; `read`): an older one has no route.
    pub fn holds(&self) -> Result<bool, AdminError> {
        match self.call("GET", "/v1/world-hold", None) {
            Ok(_) => Ok(true),
            Err(AdminError::Refused(404 | 405, _)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Holds silvermere's own writes for `secs` at most (`admin`); the hold's id. It answers
    /// once everything silvermere had queued is written.
    pub fn hold(&self, secs: u64) -> Result<String, AdminError> {
        let v = self.call_within(
            "POST",
            "/v1/world-hold",
            Some(&json!({ "secs": secs })),
            Duration::from_secs(40),
        )?;
        v["hold"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| AdminError::Refused(200, "no hold id in the answer".into()))
    }

    /// Has silvermere reload its world, then ends the hold `id` (`admin`): the reload's
    /// answer, `{"reloaded": {...}, "ms": t}`.
    pub fn reload_held(&self, id: &str) -> Result<Value, AdminError> {
        self.call_within(
            "POST",
            &format!("/v1/world-hold/{id}/reload"),
            None,
            Duration::from_secs(60),
        )
    }

    /// Has silvermere take one of its own backups now (`admin`): `{"file": ..., ...}`, or
    /// `backup_unavailable` when it takes none.
    pub fn backup(&self) -> Result<Value, AdminError> {
        self.call_within("POST", "/v1/backup", None, Duration::from_secs(120))
    }

    /// Ends the hold `id` with no reload (`admin`).
    pub fn end_hold(&self, id: &str) -> Result<(), AdminError> {
        self.call("DELETE", &format!("/v1/world-hold/{id}"), None)
            .map(drop)
    }

    /// Calls off a planned shutdown (`admin`), telling the players; whether one was.
    pub fn cancel_shutdown(&self) -> Result<bool, AdminError> {
        Ok(self.call("DELETE", "/v1/shutdown", None)?["cancelled"]
            .as_bool()
            .unwrap_or(false))
    }
}

// ---- what runs it --------------------------------------------------------------------------

/// What runs a game, so Daydream can start it again the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Starter {
    /// A systemd unit: this user's (`systemctl --user`), or the system's.
    Systemd { unit: String, user: bool },
    /// A launchd agent of this user's, and the file that defines it.
    Launchd {
        label: String,
        plist: Option<PathBuf>,
    },
    /// A process started by hand: its program and arguments, run again in its folder with
    /// its environment.
    Process {
        exe: PathBuf,
        args: Vec<OsString>,
        cwd: PathBuf,
        env: Vec<(OsString, OsString)>,
    },
}

/// silvermere's commands other than serving: a process running one of them is not the game.
const NOT_SERVING: &[&str] = &[
    "import",
    "migrate",
    "healthcheck",
    "account",
    "character",
    "backup",
    "restore",
    "admin",
    "sysop",
    "setup",
    "config",
    "web",
    "service",
    "help",
];

/// Whether `exe` with `args` (the arguments after the program) is silvermere serving: its
/// `serve`, or no command at all, which serves too.
fn serves(exe: &Path, args: &[String]) -> bool {
    let name = exe
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    name.starts_with("silvermere")
        && (args.iter().any(|a| a == "serve")
            || !args.iter().any(|a| NOT_SERVING.contains(&a.as_str())))
}

impl Starter {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Systemd { user: true, .. } => "systemd-user",
            Self::Systemd { .. } => "systemd",
            Self::Launchd { .. } => "launchd",
            Self::Process { .. } => "process",
        }
    }

    /// The command that starts it, as a person would type it.
    pub fn command(&self) -> String {
        match self {
            Self::Systemd { unit, user: true } => format!("systemctl --user start {unit}"),
            Self::Systemd { unit, .. } => format!("sudo systemctl start {unit}"),
            Self::Launchd { label, .. } => match my_uid() {
                Some(uid) => format!("launchctl kickstart gui/{uid}/{label}"),
                None => format!("launchctl kickstart gui/$UID/{label}"),
            },
            Self::Process { exe, args, cwd, .. } => {
                let mut words = vec![quote(&exe.to_string_lossy())];
                words.extend(args.iter().map(|a| quote(&unsecret(&a.to_string_lossy()))));
                format!(
                    "cd {} && {}",
                    quote(&cwd.to_string_lossy()),
                    words.join(" ")
                )
            }
        }
    }

    /// Whether Daydream can start it: a system unit needs root.
    pub fn can(&self) -> bool {
        match self {
            Self::Systemd { user: false, .. } => my_uid() == Some(0),
            _ => true,
        }
    }

    /// Whether a publish starts it again unless told otherwise: a service yes; a process
    /// started by hand no, as its output would no longer reach the terminal it ran in.
    pub fn autostart(&self) -> bool {
        self.can() && !matches!(self, Self::Process { .. })
    }

    /// Starts the game; what went wrong when it did not. A process started by hand writes
    /// its output to `log`.
    pub fn start(&self, log: &Path) -> Result<(), String> {
        match self {
            Self::Systemd { unit, user } => {
                let mut c = Command::new("systemctl");
                c.arg(if *user { "--user" } else { "--no-ask-password" })
                    .args(["start", unit]);
                run(c)
            }
            Self::Launchd { label, plist } => {
                let uid = my_uid().ok_or("Daydream could not tell which user it runs as")?;
                let mut c = Command::new("launchctl");
                c.arg("kickstart").arg(format!("gui/{uid}/{label}"));
                match (run(c), plist) {
                    (Ok(()), _) => Ok(()),
                    // an agent not loaded: load it, and it starts as it does at login
                    (Err(_), Some(plist)) => {
                        let mut c = Command::new("launchctl");
                        c.arg("bootstrap").arg(format!("gui/{uid}")).arg(plist);
                        run(c)
                    }
                    (Err(e), None) => Err(e),
                }
            }
            Self::Process {
                exe,
                args,
                cwd,
                env,
            } => {
                if let Some(dir) = log.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                }
                let mut options = std::fs::OpenOptions::new();
                options.create(true).append(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let out = options
                    .open(log)
                    .map_err(|e| format!("{}: {e}", log.display()))?;
                let err = out.try_clone().map_err(|e| e.to_string())?;
                let mut c = Command::new(exe);
                c.args(args)
                    .current_dir(cwd)
                    .stdin(Stdio::null())
                    .stdout(out)
                    .stderr(err);
                if !env.is_empty() {
                    c.env_clear().envs(env.iter().map(|(k, v)| (k, v)));
                }
                #[cfg(unix)]
                {
                    // its own process group: a Ctrl-C meant for Daydream does not reach it
                    use std::os::unix::process::CommandExt;
                    c.process_group(0);
                }
                let mut child = c.spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
                // reaped when it ends, so it leaves no zombie behind
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                Ok(())
            }
        }
    }
}

/// A command-line word as it may be shown: a database URL's password masked
/// (`--database-url postgres://me:secret@db/sm`, or `--database-url=…`).
fn unsecret(word: &str) -> String {
    if !word.contains("://") {
        return word.to_string();
    }
    match word.split_once('=').filter(|(k, _)| !k.contains("://")) {
        Some((k, url)) => format!("{k}={}", daydream_silvermere::redact_url(url)),
        None => daydream_silvermere::redact_url(word),
    }
}

/// A word for a shell, quoted when it needs to be.
fn quote(w: &str) -> String {
    if !w.is_empty()
        && w.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./=:,+@%".contains(&b))
    {
        w.to_string()
    } else {
        format!("'{}'", w.replace('\'', r"'\''"))
    }
}

/// Runs a short command (systemctl, launchctl), for at most a minute.
fn run(mut c: Command) -> Result<(), String> {
    let shown = format!("{c:?}").replace('"', "");
    let mut child = c
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{shown}: {e}"))?;
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{shown}: no answer within a minute"));
            }
            Err(e) => return Err(format!("{shown}: {e}")),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr);
    let said = said.trim();
    Err(if said.is_empty() {
        format!("{shown}: {}", out.status)
    } else {
        format!("{shown}: {said}")
    })
}

/// This process's user id, on Unix.
fn my_uid() -> Option<u32> {
    #[cfg(unix)]
    {
        use std::sync::OnceLock;
        static UID: OnceLock<Option<u32>> = OnceLock::new();
        *UID.get_or_init(|| {
            use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
            let me = sysinfo::get_current_pid().ok()?;
            let mut sys = System::new().ok()?;
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[me]),
                true,
                ProcessRefreshKind::nothing().with_user(UpdateKind::Always),
            );
            sys.process(me)?.user_id().map(|u| **u)
        })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// The pid in a writer lock's holder, as [`daydream_silvermere::info`] reports it
/// (`lock file … is held (pid 4242)`).
pub fn holder_pid(holder: &str) -> Option<u32> {
    let (_, rest) = holder.rsplit_once("(pid ")?;
    rest.trim_end_matches(')').trim().parse().ok()
}

/// What runs silvermere on `url`. `holder`: the pid holding its writer lock, when it runs.
pub fn find_starter(url: &str, holder: Option<u32>) -> Option<Starter> {
    let dir = silvermere_dir(url);
    match holder {
        Some(pid) => of_process(pid, dir.as_deref()),
        None => dir.as_deref().and_then(of_services),
    }
}

/// What runs process `pid`, when it is silvermere serving, as this user.
fn of_process(pid: u32, dir: Option<&Path>) -> Option<Starter> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let me = sysinfo::get_current_pid().ok()?;
    let pid = Pid::from_u32(pid);
    let mut sys = System::new().ok()?;
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid, me]),
        true,
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_environ(UpdateKind::Always)
            .with_user(UpdateKind::Always),
    );
    let p = sys.process(pid)?;
    let exe = p.exe()?.to_path_buf();
    let args: Vec<OsString> = p.cmd().iter().skip(1).cloned().collect();
    let words: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    if !serves(&exe, &words) {
        return None;
    }
    // a game of another user's is not this one's to start
    if p.user_id()? != sys.process(me)?.user_id()? {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        // the unit it runs in, when silvermere is that unit's own program: one started by
        // hand from inside another service (a CI runner, a terminal multiplexer's) is not
        let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap_or_default();
        if let Some(Starter::Systemd { unit, user }) = unit_of_cgroup(&cgroup)
            && main_pid(&unit, user) == Some(pid.as_u32())
        {
            return Some(Starter::Systemd { unit, user });
        }
    }
    #[cfg(target_os = "macos")]
    {
        // launchd's own children: the agent whose working folder is silvermere's
        if p.parent().map(|p| p.as_u32()) == Some(1)
            && let Some(agent) = dir.and_then(of_services)
        {
            return Some(agent);
        }
    }
    let _ = dir;
    let env = p
        .environ()
        .iter()
        .filter_map(|kv| {
            let kv = kv.to_str()?;
            let (k, v) = kv.split_once('=')?;
            Some((OsString::from(k), OsString::from(v)))
        })
        .collect();
    Some(Starter::Process {
        exe,
        args,
        cwd: p.cwd()?.to_path_buf(),
        env,
    })
}

/// The main process of a systemd unit, as systemd says.
#[cfg(target_os = "linux")]
fn main_pid(unit: &str, user: bool) -> Option<u32> {
    let mut c = Command::new("systemctl");
    if user {
        c.arg("--user");
    }
    let out = c
        .args(["show", "--property=MainPID", "--value", unit])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// The systemd unit a process runs in, from its `/proc/<pid>/cgroup`: the last `.service`
/// on its path, unless that is the user's manager itself (a process a person started) or a
/// passing one (`systemd-run`'s `run-…`, gone once it stops).
pub fn unit_of_cgroup(text: &str) -> Option<Starter> {
    let path = text
        .lines()
        .find_map(|l| l.strip_prefix("0::"))
        .or_else(|| {
            text.lines()
                .find_map(|l| l.split_once(":name=systemd:").map(|(_, p)| p))
        })?;
    let unit = path.rsplit('/').find(|c| c.ends_with(".service"))?;
    if unit.starts_with("user@") || unit.starts_with("run-") {
        return None;
    }
    Some(Starter::Systemd {
        unit: unit.trim_end_matches(".service").to_string(),
        user: path.contains("/user@"),
    })
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let c = |p: &Path| canonical(p).unwrap_or_else(|| p.to_path_buf());
    c(a) == c(b)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// The service set up for silvermere in `dir`: on Linux this user's systemd units, then the
/// system's; on macOS this user's launchd agents.
fn of_services(dir: &Path) -> Option<Starter> {
    #[cfg(target_os = "linux")]
    {
        let user_dir = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join(".config")))
            .map(|c| c.join("systemd/user"));
        let places = [
            (user_dir, true),
            (Some(PathBuf::from("/etc/systemd/system")), false),
        ];
        for (place, user) in places {
            let Some(place) = place else { continue };
            for (name, text) in files_in(&place, "service") {
                if unit_runs_in(&text, dir, home().as_deref()) {
                    let unit = name.trim_end_matches(".service").to_string();
                    return Some(Starter::Systemd { unit, user });
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let agents = home()?.join("Library/LaunchAgents");
        for (name, text) in files_in(&agents, "plist") {
            if let Some(job) = plist_job(&text)
                && job.runs_in(dir)
            {
                return Some(Starter::Launchd {
                    label: job.label,
                    plist: Some(agents.join(name)),
                });
            }
        }
    }
    let _ = dir;
    None
}

/// The files of `dir` ending in `.ext`, with their text (a few hundred at most).
#[cfg_attr(not(any(target_os = "linux", target_os = "macos")), allow(dead_code))]
fn files_in(dir: &Path, ext: &str) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = entries
        .flatten()
        .take(500)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            (path.extension().is_some_and(|x| x == ext) && path.is_file())
                .then(|| std::fs::read_to_string(&path).ok().map(|t| (name, t)))
                .flatten()
        })
        .collect();
    out.sort();
    out
}

/// Whether a systemd unit file runs silvermere serving in `dir` (`WorkingDirectory=`;
/// `%h` is `home`).
pub fn unit_runs_in(text: &str, dir: &Path, home: Option<&Path>) -> bool {
    let (mut wd, mut exec) = (None, None);
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("WorkingDirectory=") {
            wd = Some(
                v.trim()
                    .trim_start_matches('-')
                    .trim_matches('"')
                    .to_string(),
            );
        } else if let Some(v) = line.strip_prefix("ExecStart=") {
            exec = Some(v.trim().to_string());
        }
    }
    let (Some(wd), Some(exec)) = (wd, exec) else {
        return false;
    };
    // `ExecStart=-/path …`, `@`, `+` and the like prefix the program
    let mut words = exec
        .split_whitespace()
        .map(|w| w.trim_matches('"').to_string());
    let Some(program) = words.next() else {
        return false;
    };
    let program = program.trim_start_matches(['-', '@', '+', '!', ':']);
    let args: Vec<String> = words.collect();
    if !serves(Path::new(program), &args) {
        return false;
    }
    let wd = match (wd.strip_prefix("%h"), home) {
        (Some(rest), Some(h)) => h.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(wd),
    };
    same_dir(&wd, dir)
}

/// A launchd job, as its plist defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlistJob {
    pub label: String,
    pub working_directory: Option<String>,
    pub arguments: Vec<String>,
}

impl PlistJob {
    /// Whether it runs silvermere serving in `dir`.
    pub fn runs_in(&self, dir: &Path) -> bool {
        let Some((program, args)) = self.arguments.split_first() else {
            return false;
        };
        serves(Path::new(program), args)
            && self
                .working_directory
                .as_deref()
                .is_some_and(|wd| same_dir(Path::new(wd), dir))
    }
}

/// The job a launchd plist (XML) defines: its label, working folder and program.
pub fn plist_job(text: &str) -> Option<PlistJob> {
    fn unescape(s: &str) -> String {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    }
    // the <string> right after <key>KEY</key>
    let string_of = |key: &str| -> Option<String> {
        let at = text.find(&format!("<key>{key}</key>"))? + key.len() + 11;
        let rest = text[at..].trim_start();
        let rest = rest.strip_prefix("<string>")?;
        Some(unescape(&rest[..rest.find("</string>")?]))
    };
    let label = string_of("Label")?;
    let mut arguments = Vec::new();
    if let Some(at) = text.find("<key>ProgramArguments</key>") {
        let rest = &text[at..];
        let end = rest.find("</array>").unwrap_or(rest.len());
        arguments = rest[..end]
            .split("<string>")
            .skip(1)
            .filter_map(|p| p.split_once("</string>").map(|(v, _)| unescape(v)))
            .collect();
    }
    if arguments.is_empty()
        && let Some(program) = string_of("Program")
    {
        arguments.push(program);
    }
    Some(PlistJob {
        label,
        working_directory: string_of("WorkingDirectory"),
        arguments,
    })
}

// ---- what Daydream knows now ---------------------------------------------------------------

/// Whether silvermere takes the admin token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    /// None given.
    None,
    /// Not asked: silvermere is not running, or its admin API does not answer.
    Unchecked,
    /// silvermere knows it.
    Ok,
    /// silvermere does not know it (it reads its tokens when it starts).
    Unknown,
    /// silvermere knows it, but it cannot read who is in the game.
    NoRead,
}

impl Token {
    fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Unchecked => "unchecked",
            Self::Ok => "ok",
            Self::Unknown => "unknown",
            Self::NoRead => "no-read",
        }
    }

    /// Whether a publish asks silvermere to warn the players and stop: it may work.
    pub fn usable(self) -> bool {
        matches!(self, Self::Ok | Self::NoRead)
    }
}

/// What Daydream knows of a silvermere now: whether it runs, its admin API, its players,
/// and what starts it.
#[derive(Debug, Clone)]
pub struct Live {
    /// Its admin API, with the token when there is one.
    pub admin: Option<Admin>,
    pub admin_from: &'static str,
    /// The writer lock's holder while it runs.
    pub running: Option<String>,
    /// Why Daydream could not look at the database.
    pub error: Option<String>,
    /// Its `/healthz` answered.
    pub answers: bool,
    pub token: Token,
    /// The players in the game, when the token may ask.
    pub players: Option<Vec<Value>>,
    /// A shutdown planned already.
    pub planned: Option<Value>,
    /// It can hold its writes while a publish writes its world, so it need not stop
    /// (`/v1/world-hold`).
    pub holds: bool,
    pub starter: Option<Starter>,
    /// Whether a publish starts it again.
    pub autostart: bool,
}

impl Live {
    /// Looks at silvermere on `url`, with `settings`.
    pub fn look(url: &str, settings: &Settings) -> Self {
        let (running, error) = match again(
            || daydream_silvermere::block_on(daydream_silvermere::info(url)),
            |e| matches!(e, daydream_silvermere::Error::Db(_)),
        ) {
            Ok(i) => (i.running, None),
            Err(e) => (None, Some(e.to_string())),
        };
        let (base, admin_from) = admin_of(url, settings);
        let admin = base.map(|b| Admin::new(&b, settings.token.as_deref()));
        let mut live = Live {
            admin,
            admin_from,
            running,
            error,
            answers: false,
            token: if settings.token.is_some() {
                Token::Unchecked
            } else {
                Token::None
            },
            players: None,
            planned: None,
            holds: false,
            starter: None,
            autostart: false,
        };
        if let (Some(_), Some(admin)) = (&live.running, &live.admin) {
            live.answers = again(|| admin.health(), AdminError::passing).is_ok();
            if live.answers && admin.has_token() {
                match again(|| admin.who(), AdminError::passing) {
                    Ok(players) => {
                        live.token = Token::Ok;
                        live.players = Some(players);
                        live.planned = admin.planned().ok().flatten();
                        live.holds = admin.holds().unwrap_or(false);
                    }
                    Err(AdminError::Unauthorized) => live.token = Token::Unknown,
                    Err(AdminError::Forbidden) => live.token = Token::NoRead,
                    Err(_) => {}
                }
            }
        }
        let holder = live.running.as_deref().and_then(holder_pid);
        live.starter = find_starter(url, holder);
        live.autostart = live
            .starter
            .as_ref()
            .is_some_and(|s| s.can() && settings.autostart.unwrap_or_else(|| s.autostart()));
        live
    }

    /// The admin API, when Daydream may ask it to warn the players and stop.
    pub fn stopper(&self) -> Option<&Admin> {
        self.admin
            .as_ref()
            .filter(|_| self.running.is_some() && self.answers && self.token.usable())
    }

    /// The admin API, when a publish can go in with silvermere running (`/v1/world-hold`).
    pub fn holder(&self) -> Option<&Admin> {
        self.stopper().filter(|_| self.holds)
    }

    /// Why Daydream cannot warn the players and stop silvermere now, while it may be
    /// running: `None` when it can, or when silvermere is stopped.
    pub fn why_no_stopper(&self) -> Option<String> {
        if self.stopper().is_some() || (self.running.is_none() && self.error.is_none()) {
            return None;
        }
        Some(match (&self.error, &self.admin) {
            (Some(e), _) => {
                format!("Daydream could not look at silvermere's database just now ({e})")
            }
            (None, None) => "Daydream does not know where silvermere's admin API is".into(),
            (None, Some(admin)) if !self.answers => {
                format!(
                    "silvermere's admin API did not answer at {} just now",
                    admin.base()
                )
            }
            (None, Some(_)) => match self.token {
                Token::None => "Daydream has no admin token for silvermere".into(),
                Token::Unknown => "silvermere does not know Daydream's admin token yet".into(),
                _ => "silvermere did not say who is in the game just now".into(),
            },
        })
    }

    /// What the token's state means, for the Server card.
    fn token_text(&self) -> String {
        let admin = self.admin.as_ref().map_or("", Admin::base);
        match self.token {
            Token::None => "No admin token yet: you stop silvermere yourself, and Daydream \
                            carries on once it has. With one, Daydream warns the players and \
                            stops it."
                .into(),
            Token::Unchecked if self.running.is_none() => {
                "Not checked: silvermere is not running.".into()
            }
            Token::Unchecked if self.admin.is_none() => {
                "Daydream does not know where silvermere's admin API is.".into()
            }
            Token::Unchecked => {
                format!("Not checked: silvermere's admin API does not answer at {admin}.")
            }
            Token::Ok => "silvermere takes it: Daydream can warn the players and stop it.".into(),
            Token::Unknown => "silvermere does not know this token yet. Add its [[admin.tokens]] \
                               block to silvermere.toml and start silvermere again once: it \
                               reads its tokens when it starts."
                .into(),
            Token::NoRead => "silvermere knows this token, but it cannot read who is in the \
                              game. Give it the read, write and admin scopes."
                .into(),
        }
    }

    /// The Server card's view: `{running, error, admin, admin_from, answers, token,
    /// token_state, token_text, players, planned, start, autostart, custom}`.
    pub fn json(&self, custom: bool) -> Value {
        let players = self.players.as_ref().map(|p| {
            p.iter()
                .map(|p| {
                    let name = format!(
                        "{} {}",
                        p["first_name"].as_str().unwrap_or_default(),
                        p["last_name"].as_str().unwrap_or_default()
                    );
                    json!({
                        "name": name.trim(),
                        "level": p["level"],
                        "class": p["class"],
                        "race": p["race"],
                        "idle_secs": p["idle_secs"],
                    })
                })
                .collect::<Vec<_>>()
        });
        json!({
            "engine": "silvermere",
            "running": self.running.is_some(),
            "error": self.error,
            "admin": self.admin.as_ref().map(Admin::base),
            "admin_from": self.admin_from,
            "answers": self.answers,
            "token": self.token != Token::None,
            "token_state": self.token.id(),
            "token_text": self.token_text(),
            "players": players,
            "planned": self.planned,
            "holds": self.holds,
            "start": self.starter.as_ref().map(|s| json!({
                "kind": s.kind(),
                "command": s.command(),
                "can": s.can(),
            })),
            "autostart": self.autostart,
            "custom": custom,
        })
    }
}

/// The open game's silvermere database and settings, or why there is none.
fn open_silvermere(s: &AppState) -> Result<(Source, String, Settings), ApiError> {
    let cur = s.current();
    let game = cur.game.as_ref().ok_or_else(|| {
        ApiError(
            StatusCode::CONFLICT,
            "no game is open: choose one with the game picker at the top".into(),
        )
    })?;
    let Source::Silvermere { url } = &game.source else {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "only silvermere has a remote control Daydream can use".into(),
        ));
    };
    let settings = load(&s.games.workspace(), &game.source);
    Ok((game.source.clone(), url.clone(), settings))
}

/// The open game's control, as `GET /api/control` answers.
fn status(s: &AppState) -> Value {
    match open_silvermere(s) {
        Ok((_, url, settings)) => Live::look(&url, &settings).json(s.custom_allowed()),
        Err(_) => json!({
            "engine": s.current().game.as_ref().map(|g| g.source.kind()),
        }),
    }
}

/// Where Daydream writes the output of a game it started by its command line.
pub fn process_log(s: &AppState, source: &Source) -> PathBuf {
    s.games
        .workspace()
        .dir()
        .join("control")
        .join(format!("{}.log", source.id()))
}

// ---- routes --------------------------------------------------------------------------------

pub(crate) async fn get(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    blocking(move || Ok(Json(status(&s)))).await
}

/// The body of `PUT /api/control`.
#[derive(Debug, Deserialize)]
pub struct PutJson {
    #[serde(default)]
    admin: Option<String>,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    autostart: Option<bool>,
}

pub(crate) async fn put(
    State(s): State<Arc<AppState>>,
    Json(body): Json<PutJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let (source, _, mut settings) = open_silvermere(&s)?;
        if let Some(admin) = &body.admin {
            if !s.custom_allowed() {
                return Err(not_allowed("silvermere's admin API address"));
            }
            let admin = admin.trim();
            let new = if admin.is_empty() {
                None
            } else {
                Some(check_admin_url(admin)?)
            };
            if new != settings.admin {
                settings.admin = new;
                // a token goes only where it was given for
                if body.token.is_none() {
                    settings.token = None;
                }
            }
        }
        if let Some(token) = &body.token {
            let token = token.trim();
            settings.token = if token.is_empty() {
                None
            } else {
                check_token(token)?;
                Some(token.to_string())
            };
        }
        if let Some(on) = body.autostart {
            settings.autostart = Some(on);
        }
        save(&s.games.workspace(), &source, &settings).map_err(crate::tools::internal)?;
        Ok(Json(status(&s)))
    })
    .await
}

/// The body of `POST /api/control/say`.
#[derive(Debug, Deserialize)]
pub struct SayJson {
    text: String,
}

pub(crate) async fn say(
    State(s): State<Arc<AppState>>,
    Json(body): Json<SayJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let conflict = |m: String| ApiError(StatusCode::CONFLICT, m);
        let text = body.text.trim();
        if text.is_empty() {
            return Err(ApiError(StatusCode::BAD_REQUEST, "say something".into()));
        }
        check_line(text, MAX_SAY, "line").map_err(|e| ApiError(StatusCode::BAD_REQUEST, e))?;
        let (_, url, settings) = open_silvermere(&s)?;
        if settings.token.is_none() {
            return Err(conflict(
                "Daydream needs an admin token to talk to the players".into(),
            ));
        }
        let (Some(base), _) = admin_of(&url, &settings) else {
            return Err(conflict(
                "Daydream does not know where silvermere's admin API is".into(),
            ));
        };
        let delivered = Admin::new(&base, settings.token.as_deref())
            .broadcast(text)
            .map_err(|e| conflict(format!("silvermere did not pass it on: {e}")))?;
        Ok(Json(json!({ "delivered": delivered })))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// An admin API on this machine answering `statuses` in turn, a connection each, with
    /// no players; how many times it was asked.
    fn admin_answering(statuses: &'static [u16]) -> (Admin, Arc<AtomicUsize>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let asked = Arc::new(AtomicUsize::new(0));
        let n = asked.clone();
        std::thread::spawn(move || {
            for (stream, status) in listener.incoming().zip(statuses) {
                let mut stream = stream.unwrap();
                n.fetch_add(1, Ordering::SeqCst);
                let (mut head, mut buf) = (Vec::new(), [0u8; 1024]);
                while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(k) => head.extend_from_slice(&buf[..k]),
                    }
                }
                let body = r#"{"players":[]}"#;
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        (Admin::new(&base, Some("t")), asked)
    }

    #[test]
    fn a_quick_failure_is_asked_again() {
        // busy a moment, then well
        let (admin, asked) = admin_answering(&[503, 200]);
        assert_eq!(again(|| admin.who(), AdminError::passing), Ok(Vec::new()));
        assert_eq!(asked.load(Ordering::SeqCst), 2);
        // three times at most
        let (admin, asked) = admin_answering(&[504, 504, 504, 200]);
        let busy = again(|| admin.who(), AdminError::passing);
        assert!(matches!(busy, Err(AdminError::Refused(504, _))), "{busy:?}");
        assert_eq!(asked.load(Ordering::SeqCst), 3);
        // a token silvermere does not know is not asked again
        let (admin, asked) = admin_answering(&[401, 200]);
        assert_eq!(
            again(|| admin.who(), AdminError::passing),
            Err(AdminError::Unauthorized)
        );
        assert_eq!(asked.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_publish_is_told_why_daydream_cannot_stop_silvermere() {
        let live = |running: bool, answers: bool, token: Token| Live {
            admin: Some(Admin::new("http://127.0.0.1:8080", Some("t"))),
            admin_from: "default",
            running: running.then(|| "lock file x is held (pid 1)".to_string()),
            error: None,
            answers,
            token,
            players: None,
            planned: None,
            holds: false,
            starter: None,
            autostart: false,
        };
        assert_eq!(live(true, true, Token::Ok).why_no_stopper(), None);
        // stopped: there is nothing to stop
        assert_eq!(live(false, false, Token::Unchecked).why_no_stopper(), None);
        let why = |l: Live| l.why_no_stopper().unwrap_or_default();
        assert!(
            why(live(true, false, Token::Unchecked))
                .contains("did not answer at http://127.0.0.1:8080")
        );
        assert!(why(live(true, true, Token::Unchecked)).contains("who is in the game"));
        assert!(why(live(true, true, Token::Unknown)).contains("does not know"));
        assert!(why(live(true, true, Token::None)).contains("no admin token"));
        // its database could not be read: it may be running
        let mut unread = live(false, false, Token::Unchecked);
        unread.error = Some("database is locked".into());
        assert!(why(unread).contains("database is locked"));
    }

    #[test]
    fn the_admin_api_is_where_silvermere_toml_says() {
        assert_eq!(
            admin_url_of(Some("[server]\nadmin_bind = \"127.0.0.1:8081\"\n")).as_deref(),
            Some("http://127.0.0.1:8081")
        );
        // listening everywhere: reached on this machine
        assert_eq!(
            admin_url_of(Some("[server]\nadmin_bind = \"0.0.0.0:9000\" # all\n")).as_deref(),
            Some("http://127.0.0.1:9000")
        );
        assert_eq!(
            admin_url_of(Some("[server]\nadmin_bind = \"[::]:9000\"\n")).as_deref(),
            Some("http://[::1]:9000")
        );
        // unset: silvermere's default; turned off: none
        assert_eq!(
            admin_url_of(Some("[database]\nadmin_bind = \"1.2.3.4:5\"\n")).as_deref(),
            Some("http://127.0.0.1:8080")
        );
        assert_eq!(admin_url_of(None).as_deref(), Some("http://127.0.0.1:8080"));
        assert_eq!(admin_url_of(Some("[server]\nadmin_bind = \"\"\n")), None);
    }

    #[test]
    fn silvermere_is_found_beside_its_database() {
        let tmp = tempfile::tempdir().unwrap();
        let sm = tmp.path().join("sm");
        std::fs::create_dir_all(sm.join("data")).unwrap();
        let url = format!("sqlite:{}", sm.join("data/silvermere.db").display());
        // no settings file: the folder above data/
        assert_eq!(silvermere_dir(&url), Some(sm.clone()));
        assert_eq!(
            admin_of(&url, &Settings::default()),
            (Some("http://127.0.0.1:8080".into()), "default")
        );
        std::fs::write(
            sm.join("silvermere.toml"),
            "[server]\nadmin_bind = \"127.0.0.1:8091\"\n",
        )
        .unwrap();
        assert_eq!(
            admin_of(&url, &Settings::default()),
            (Some("http://127.0.0.1:8091".into()), "silvermere.toml")
        );
        let set = Settings {
            admin: Some("http://127.0.0.1:1".into()),
            ..Settings::default()
        };
        assert_eq!(admin_of(&url, &set).1, "set");
        assert_eq!(
            admin_of("postgres://u@h/db", &Settings::default()),
            (None, "unknown")
        );
    }

    #[test]
    fn typed_admin_urls_and_tokens_are_checked() {
        assert_eq!(
            check_admin_url("http://127.0.0.1:8081/").unwrap(),
            "http://127.0.0.1:8081"
        );
        for bad in [
            "127.0.0.1:8081",
            "http://",
            "http://u:p@host:1",
            "http://h:1/x",
            "file:///etc",
        ] {
            assert!(check_admin_url(bad).is_err(), "{bad}");
        }
        assert!(check_token("abc-DEF_123.~").is_ok());
        assert!(check_token("two words").is_err());
        assert!(check_line("back soon", MAX_MESSAGE, "message").is_ok());
        assert!(check_line("two\nlines", MAX_MESSAGE, "message").is_err());
        assert!(check_line(&"x".repeat(201), MAX_MESSAGE, "message").is_err());
    }

    #[test]
    fn the_lock_names_its_holder() {
        assert_eq!(
            holder_pid("lock file /sm/data/silvermere.db.lock is held (pid 4242)"),
            Some(4242)
        );
        assert_eq!(holder_pid("a PostgreSQL advisory lock is held"), None);
    }

    #[test]
    fn a_process_is_known_by_its_cgroup() {
        let user = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/daydream-stack-silvermere.service\n";
        assert_eq!(
            unit_of_cgroup(user),
            Some(Starter::Systemd {
                unit: "daydream-stack-silvermere".into(),
                user: true
            })
        );
        assert_eq!(
            unit_of_cgroup("0::/system.slice/silvermere.service\n"),
            Some(Starter::Systemd {
                unit: "silvermere".into(),
                user: false
            })
        );
        // cgroup v1
        assert_eq!(
            unit_of_cgroup("12:pids:/x\n1:name=systemd:/system.slice/silvermere.service\n"),
            Some(Starter::Systemd {
                unit: "silvermere".into(),
                user: false
            })
        );
        // started from a terminal, a session, a container, or systemd-run
        for none in [
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.gnome.Terminal.slice/vte-spawn-1.scope\n",
            "0::/user.slice/user-1000.slice/session-2.scope\n",
            "0::/system.slice/docker-0123abcd.scope\n",
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/run-u12.service\n",
        ] {
            assert_eq!(unit_of_cgroup(none), None, "{none}");
        }
    }

    #[test]
    fn units_and_agents_are_matched_by_their_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let sm = tmp.path().join("silvermere");
        std::fs::create_dir_all(&sm).unwrap();
        let unit = format!(
            "[Service]\nWorkingDirectory={}\nExecStart=\"{}/releases/current/silvermere\" serve\nRestart=on-failure\n",
            sm.display(),
            sm.display()
        );
        assert!(unit_runs_in(&unit, &sm, None));
        assert!(!unit_runs_in(&unit, tmp.path(), None));
        // the same folder, another of silvermere's commands
        let backup = unit.replace(" serve", " backup");
        assert!(!unit_runs_in(&backup, &sm, None));
        // %h is the home folder
        let home_unit =
            "[Service]\nWorkingDirectory=-%h/silvermere\nExecStart=/opt/sm/silvermere\n";
        assert!(unit_runs_in(home_unit, &sm, Some(tmp.path())));

        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
    <key>Label</key><string>io.github.jnfisher.daydream-stack.silvermere</string>
    <key>ProgramArguments</key><array>
        <string>{0}/releases/current/silvermere</string>
        <string>serve</string>
    </array>
    <key>WorkingDirectory</key>
    <string>{0}</string>
</dict></plist>"#,
            sm.display()
        );
        let job = plist_job(&plist).unwrap();
        assert_eq!(job.label, "io.github.jnfisher.daydream-stack.silvermere");
        assert_eq!(job.arguments.len(), 2);
        assert!(job.runs_in(&sm));
        assert!(!job.runs_in(tmp.path()));
    }

    #[test]
    fn silvermere_serving_is_told_from_its_other_commands() {
        let sm = Path::new("/opt/silvermere/silvermere");
        let words = |w: &[&str]| w.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(serves(sm, &words(&["serve"])));
        assert!(serves(sm, &words(&[])));
        assert!(serves(sm, &words(&["--config", "x.toml", "serve"])));
        assert!(!serves(sm, &words(&["import", "stock.zip"])));
        assert!(!serves(Path::new("/sbin/init"), &words(&[])));
        assert_eq!(
            Starter::Process {
                exe: sm.into(),
                args: vec!["serve".into()],
                cwd: "/srv/my game".into(),
                env: Vec::new(),
            }
            .command(),
            "cd '/srv/my game' && /opt/silvermere/silvermere serve"
        );
        // a password on its command line is not shown
        let shown = Starter::Process {
            exe: sm.into(),
            args: vec![
                "serve".into(),
                "--database-url".into(),
                "postgres://sm:hunter2@db/sm".into(),
                "--database-url=postgres://sm:hunter2@db/sm".into(),
            ],
            cwd: "/srv/sm".into(),
            env: Vec::new(),
        }
        .command();
        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(shown.contains("postgres://sm:"), "{shown}");
    }

    #[test]
    fn settings_are_kept_for_their_owner_only() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::new(tmp.path().to_path_buf());
        let source = Source::Silvermere {
            url: "sqlite:/srv/sm/data/silvermere.db".into(),
        };
        assert_eq!(load(&ws, &source), Settings::default());
        let settings = Settings {
            admin: None,
            token: Some("secret".into()),
            autostart: Some(false),
        };
        save(&ws, &source, &settings).unwrap();
        assert_eq!(load(&ws, &source), settings);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(settings_path(&ws, &source))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
