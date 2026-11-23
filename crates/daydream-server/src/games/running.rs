//! Whether a game is running on a folder: its files must not be written while it does.
//!
//! A MajorMUD game is a process: Worldgroup's `WGSERVER.EXE`, The Major BBS, or MBBSEmu
//! (`MBBSEmu`, or `dotnet MBBSEmu.dll`). A process is tied to a folder when its program,
//! its working folder or a path on its command line is in that folder. One that cannot be
//! tied (another BBS on the machine, or a process whose paths the system does not show) is
//! reported as a warning, not as running here. On Windows a folder's `WCC*` files are also
//! tried for writing: a file another program holds open without sharing (the Btrieve
//! engine) is in use.
//!
//! silvermere is not looked for here: its writer lock says whether it runs
//! ([`daydream_silvermere::info`]).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Program names of MajorMUD's hosts (lower case, no `.exe`).
const HOSTS: &[&str] = &["wgserver", "wgserv", "majorbbs", "bbsv10", "mbbsemu"];

/// A game process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameProcess {
    pub pid: u32,
    pub name: String,
    /// Its program, working folder and command line's paths.
    #[serde(skip)]
    pub paths: Vec<PathBuf>,
}

/// What is known about a folder's game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Running {
    /// The process running on the folder, or the file found in use.
    pub by: Option<String>,
    /// Game processes that could not be tied to the folder.
    pub elsewhere: Vec<String>,
}

impl Running {
    pub fn running(&self) -> bool {
        self.by.is_some()
    }
}

/// How long one look at the processes is reused.
const FRESH: Duration = Duration::from_secs(2);

static CACHE: Mutex<Option<(Instant, Vec<GameProcess>)>> = Mutex::new(None);

/// The MajorMUD hosts running on this machine (looked at once every [`FRESH`]).
pub fn game_processes() -> Vec<GameProcess> {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, procs)) = cache.as_ref()
        && at.elapsed() < FRESH
    {
        return procs.clone();
    }
    let Ok(mut sys) = System::new() else {
        // the system will not say: nothing found, and the folder checks still run
        return Vec::new();
    };
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cwd(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet),
    );
    let mut out = Vec::new();
    for (pid, p) in sys.processes() {
        let name = p.name().to_string_lossy().to_string();
        let cmd: Vec<String> = p
            .cmd()
            .iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        if !is_host(&name, &cmd) {
            continue;
        }
        let mut paths: Vec<PathBuf> = p.exe().into_iter().map(Path::to_path_buf).collect();
        paths.extend(p.cwd().map(Path::to_path_buf));
        paths.extend(cmd.iter().map(PathBuf::from).filter(|a| a.is_absolute()));
        out.push(GameProcess {
            pid: pid.as_u32(),
            name,
            paths,
        });
    }
    out.sort_by_key(|p| p.pid);
    *cache = Some((Instant::now(), out.clone()));
    out
}

/// Whether a process (by name and command line) is one of MajorMUD's hosts.
pub fn is_host(name: &str, cmd: &[String]) -> bool {
    let stem = |s: &str| {
        let s = s
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(s)
            .to_ascii_lowercase();
        s.strip_suffix(".exe")
            .or_else(|| s.strip_suffix(".dll"))
            .map(str::to_string)
            .unwrap_or(s)
    };
    let n = stem(name);
    HOSTS.contains(&n.as_str())
        || ((n == "dotnet" || n == "mono") && cmd.iter().any(|a| stem(a) == "mbbsemu"))
}

/// Whether a game runs on `dir`.
pub fn check(dir: &Path) -> Running {
    let dir = super::discover::canonical(dir).unwrap_or_else(|| dir.to_path_buf());
    let mut out = Running::default();
    for p in game_processes() {
        let shown = format!("{} (process {})", p.name, p.pid);
        let here = p.paths.iter().any(|q| {
            let q = super::discover::canonical(q).unwrap_or_else(|| q.clone());
            q.starts_with(&dir)
        });
        if here && out.by.is_none() {
            out.by = Some(shown);
        } else if !here {
            out.elsewhere.push(shown);
        }
    }
    if out.by.is_none() {
        out.by = file_in_use(&dir);
    }
    out
}

/// A `WCC*` file of `dir` that another program holds open without sharing it (Windows).
#[cfg(windows)]
fn file_in_use(dir: &Path) -> Option<String> {
    use std::os::windows::fs::OpenOptionsExt;
    const SHARING_VIOLATION: i32 = 32;
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_ascii_uppercase();
        let interesting =
            name.starts_with("WCCITEM") && (name.ends_with(".DAT") || name.ends_with(".DB"));
        if !interesting {
            continue;
        }
        let r = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(e.path());
        if let Err(err) = r
            && err.raw_os_error() == Some(SHARING_VIOLATION)
        {
            return Some(format!("{name} is open in another program"));
        }
    }
    None
}

#[cfg(not(windows))]
fn file_in_use(_dir: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_by_name() {
        assert!(is_host("WGSERVER.EXE", &[]));
        assert!(is_host("MBBSEmu", &[]));
        assert!(is_host("mbbsemu.exe", &[]));
        assert!(is_host(
            "dotnet",
            &["dotnet".into(), "/srv/emu/MBBSEmu.dll".into(), "-S".into()]
        ));
        assert!(!is_host("dotnet", &["dotnet".into(), "other.dll".into()]));
        assert!(!is_host("silvermere", &[]));
        assert!(!is_host("bash", &[]));
    }

    #[test]
    fn nothing_runs_on_an_empty_folder() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!check(tmp.path()).running());
    }
}
