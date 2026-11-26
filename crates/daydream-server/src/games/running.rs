//! Whether a game is running on a folder: its files must not be written while it does.
//!
//! A MajorMUD game is a process: Worldgroup's `WGSERVER.EXE` (`WGSSRV.EXE` when it runs as a
//! Windows service, `WGSAPPGO.EXE` for its console), The Major BBS, or MBBSEmu (`MBBSEmu`, or
//! `dotnet MBBSEmu.dll`). A process is tied to a folder when its program,
//! its working folder or a path on its command line is in that folder. One that cannot be
//! tied (another BBS on the machine, or a process whose paths the system does not show) is
//! reported as a warning, not as running here. A folder's `WCC*` files are also looked at
//! themselves, which ties a game however it was started:
//!
//! - on Windows, each is tried for writing: a file another program holds open without sharing
//!   (the Btrieve engine) is in use;
//! - on Linux, a file another process has open (`/proc/<pid>/fd`, for the processes this user
//!   may look into) or holds a lock on (`/proc/locks`, every process's: SQLite's while it has a
//!   `.db` open in WAL mode, a game's in a container too) is in use.
//!
//! silvermere is not looked for here: its writer lock says whether it runs
//! ([`daydream_silvermere::info`]).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Program names of MajorMUD's hosts (lower case, no `.exe`).
const HOSTS: &[&str] = &[
    "wgserver", "wgserv", "wgssrv", "wgsappgo", "majorbbs", "bbsv10", "mbbsemu",
];

/// A game process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameProcess {
    pub pid: u32,
    pub name: String,
    /// Its program, working folder and command line's paths.
    #[serde(skip)]
    pub paths: Vec<PathBuf>,
    /// MBBSEmu ([`is_mbbsemu`]).
    #[serde(skip)]
    pub mbbsemu: bool,
}

/// What is known about a folder's game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Running {
    /// Who runs on the folder, as it reads in brackets: `MBBSEmu, process 1234`, or
    /// `another program has WCCITEMS.DAT open`.
    pub by: Option<String>,
    /// Game processes that could not be tied to the folder.
    pub elsewhere: Vec<String>,
    /// Of those, the MBBSEmu ones: one in a container may run this folder's `.db` files.
    #[serde(skip)]
    pub mbbsemu_elsewhere: Vec<String>,
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
            mbbsemu: is_mbbsemu(&name, &cmd),
            name,
            paths,
        });
    }
    out.sort_by_key(|p| p.pid);
    *cache = Some((Instant::now(), out.clone()));
    out
}

/// A program's name, lower case, without its folder or `.exe` / `.dll`.
fn stem(s: &str) -> String {
    let s = s
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(s)
        .to_ascii_lowercase();
    s.strip_suffix(".exe")
        .or_else(|| s.strip_suffix(".dll"))
        .map(str::to_string)
        .unwrap_or(s)
}

/// Whether a process (by name and command line) is one of MajorMUD's hosts.
pub fn is_host(name: &str, cmd: &[String]) -> bool {
    HOSTS.contains(&stem(name).as_str()) || is_mbbsemu(name, cmd)
}

/// Whether a process is MBBSEmu, which runs a game from its `.db` files (Worldgroup and The
/// Major BBS run theirs from its `.DAT` files).
pub fn is_mbbsemu(name: &str, cmd: &[String]) -> bool {
    let n = stem(name);
    n == "mbbsemu" || ((n == "dotnet" || n == "mono") && cmd.iter().any(|a| stem(a) == "mbbsemu"))
}

/// Whether a game runs on `dir`.
pub fn check(dir: &Path) -> Running {
    let dir = super::discover::canonical(dir).unwrap_or_else(|| dir.to_path_buf());
    let mut out = Running::default();
    for p in game_processes() {
        let shown = format!("{}, process {}", p.name, p.pid);
        let here = p.paths.iter().any(|q| {
            let q = super::discover::canonical(q).unwrap_or_else(|| q.clone());
            q.starts_with(&dir)
        });
        if here && out.by.is_none() {
            out.by = Some(shown);
        } else if !here {
            if p.mbbsemu {
                out.mbbsemu_elsewhere.push(shown.clone());
            }
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
            return Some(format!("another program has {name} open"));
        }
    }
    None
}

/// A `WCC*` file of `dir` that another process has open or locked (Linux).
#[cfg(target_os = "linux")]
fn file_in_use(dir: &Path) -> Option<String> {
    use std::collections::HashMap;
    use std::os::unix::fs::MetadataExt;
    let mut files: HashMap<(u64, u64), String> = HashMap::new();
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.to_ascii_uppercase().starts_with("WCC") {
            continue;
        }
        if let Some(m) = e.metadata().ok().filter(|m| m.is_file()) {
            files.insert((m.dev(), m.ino()), name);
        }
    }
    if files.is_empty() {
        return None;
    }
    let me = std::process::id();
    let comm = |pid: u32| {
        std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .map(|c| c.trim().to_string())
            .unwrap_or_else(|_| "another program".into())
    };
    for p in std::fs::read_dir("/proc").ok()?.flatten() {
        let Some(pid) = p.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        if pid == me {
            continue;
        }
        // another user's process: its files are not ours to see, but its locks are below
        let Ok(fds) = std::fs::read_dir(p.path().join("fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if let Ok(m) = std::fs::metadata(fd.path())
                && let Some(name) = files.get(&(m.dev(), m.ino()))
            {
                return Some(format!("{}, process {pid}, has {name} open", comm(pid)));
            }
        }
    }
    let locks = std::fs::read_to_string("/proc/locks").ok()?;
    let (pid, name) = locked(&locks, &files, me)?;
    Some(format!(
        "{}, process {pid}, holds a lock on {name}",
        comm(pid)
    ))
}

/// The first of `files` (by device and inode) that a process other than `me` holds a lock
/// on, as `/proc/locks` lists them (`1: POSIX ADVISORY READ 1234 08:01:5678 128 128`, a
/// waiter's with `->` after the number): that process, and the file's name.
#[cfg(target_os = "linux")]
fn locked<'a>(
    locks: &str,
    files: &'a std::collections::HashMap<(u64, u64), String>,
    me: u32,
) -> Option<(u32, &'a str)> {
    let by_ino: std::collections::HashMap<u64, (u64, &str)> = files
        .iter()
        .map(|(&(dev, ino), name)| (ino, (dev, name.as_str())))
        .collect();
    for line in locks.lines() {
        let words: Vec<&str> = line.split_whitespace().filter(|w| *w != "->").collect();
        // number, kind, advisory, mode, pid, major:minor:inode, start, end
        let (Some(pid), Some(at)) = (words.get(4), words.get(5)) else {
            continue;
        };
        let Ok(pid) = pid.parse::<u32>() else {
            continue;
        };
        let mut parts = at.split(':');
        let (Some(major), Some(minor), Some(ino)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (Ok(major), Ok(minor), Ok(ino)) = (
            u64::from_str_radix(major, 16),
            u64::from_str_radix(minor, 16),
            ino.parse::<u64>(),
        ) else {
            continue;
        };
        let Some(&(dev, name)) = by_ino.get(&ino) else {
            continue;
        };
        // dev_t as glibc packs it
        let (dev_major, dev_minor) = (
            ((dev >> 8) & 0xfff) | ((dev >> 32) & !0xfff),
            (dev & 0xff) | ((dev >> 12) & !0xff),
        );
        if (dev_major, dev_minor) == (major, minor) && pid != me {
            return Some((pid, name));
        }
    }
    None
}

#[cfg(not(any(windows, target_os = "linux")))]
fn file_in_use(_dir: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_by_name() {
        assert!(is_host("WGSERVER.EXE", &[]));
        // Worldgroup as a Windows service, and its console
        assert!(is_host("WGSSRV.EXE", &[]));
        assert!(is_host("wgsappgo.exe", &[]));
        assert!(is_host("MBBSEmu", &[]));
        assert!(is_host("mbbsemu.exe", &[]));
        assert!(is_host(
            "dotnet",
            &["dotnet".into(), "/srv/emu/MBBSEmu.dll".into(), "-S".into()]
        ));
        assert!(!is_host("dotnet", &["dotnet".into(), "other.dll".into()]));
        assert!(!is_host("silvermere", &[]));
        assert!(!is_host("bash", &[]));
        // MBBSEmu runs a game's .db files; Worldgroup its .DAT files
        assert!(is_mbbsemu("MBBSEmu", &[]));
        assert!(is_mbbsemu("mono", &["mono".into(), "MBBSEmu.exe".into()]));
        assert!(!is_mbbsemu("WGSERVER.EXE", &[]));
    }

    #[test]
    fn nothing_runs_on_an_empty_folder() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!check(tmp.path()).running());
    }

    /// A game's file another process holds open, as MBBSEmu holds its `.DB` files.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_another_process_has_open_is_in_use() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("WCCITEMS.DB");
        std::fs::write(&file, b"x").unwrap();
        std::fs::write(tmp.path().join("README.txt"), b"x").unwrap();
        assert_eq!(file_in_use(tmp.path()), None);
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::fs::File::open(&file).unwrap())
            .spawn()
            .unwrap();
        let found = file_in_use(tmp.path());
        let _ = child.kill();
        let _ = child.wait();
        let found = found.expect("in use");
        assert!(
            found.starts_with("sleep, process ") && found.ends_with(", has WCCITEMS.DB open"),
            "{found}"
        );
        assert_eq!(file_in_use(tmp.path()), None);
    }

    /// An MBBSEmu in another folder does not run here, but is named: one in a container may
    /// run this folder's game.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_mbbsemu_elsewhere_is_named() {
        let tmp = tempfile::tempdir().unwrap();
        let (here, there) = (tmp.path().join("here"), tmp.path().join("there"));
        std::fs::create_dir_all(&here).unwrap();
        std::fs::create_dir_all(&there).unwrap();
        // a stand-in for MBBSEmu: sleep, under its name (told it is sleep, since sleep can be
        // one program of many that goes by the name it is called)
        let Some(sleep) = ["/usr/bin/sleep", "/bin/sleep"]
            .into_iter()
            .map(Path::new)
            .find(|p| p.is_file())
        else {
            return;
        };
        let emu = there.join("MBBSEmu");
        std::fs::copy(sleep, &emu).unwrap();
        let spawn = || {
            use std::os::unix::process::CommandExt;
            std::process::Command::new(&emu)
                .arg0("sleep")
                .arg("30")
                .current_dir(&there)
                .spawn()
        };
        // a program just written can be busy a moment while another test starts a process
        let mut child = spawn()
            .or_else(|_| {
                std::thread::sleep(Duration::from_millis(200));
                spawn()
            })
            .unwrap();
        let shown = format!("MBBSEmu, process {}", child.id());
        // the processes are looked at afresh once FRESH has passed
        let deadline = Instant::now() + FRESH * 3;
        let mut seen = (check(&here), check(&there));
        while seen.1.by.as_deref() != Some(shown.as_str()) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(200));
            seen = (check(&here), check(&there));
        }
        let _ = child.kill();
        let _ = child.wait();
        let (here, there) = seen;
        assert_eq!(there.by.as_deref(), Some(shown.as_str()), "{there:?}");
        assert!(!here.running(), "{here:?}");
        assert!(here.elsewhere.contains(&shown), "{here:?}");
        assert!(here.mbbsemu_elsewhere.contains(&shown), "{here:?}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_locks_list_names_the_holder() {
        let files: std::collections::HashMap<(u64, u64), String> =
            [((0xfe00, 835_694), "WCCITEMS.DB-shm".to_string())].into();
        let locks = "1: FLOCK  ADVISORY  WRITE 185 fe:00:688131 0 EOF\n\
                     2: POSIX  ADVISORY  READ 3293 fe:00:835694 128 128\n\
                     3: -> POSIX  ADVISORY  WRITE 4000 fe:00:835694 0 EOF\n";
        assert_eq!(locked(locks, &files, 1), Some((3293, "WCCITEMS.DB-shm")));
        // its own locks are not another's
        assert_eq!(
            locked(&locks.replace("3293", "1"), &files, 1),
            Some((4000, "WCCITEMS.DB-shm"))
        );
        // the same inode on another device is another file
        assert_eq!(
            locked(&locks.replace("fe:00:835694", "08:01:835694"), &files, 1),
            None
        );
    }
}
