//! Finding the games on this machine: folders of MajorMUD's `WCC*` files and silvermere
//! databases.
//!
//! Daydream is assumed to run on the machine that hosts the game, so it looks where games
//! are installed: each drive's top folders on Windows (`C:\WGSERV`, `C:\MBBSEmu`,
//! `C:\Silvermere`), the user's home folder, the folder Daydream was started in, `/opt` and
//! `/srv`, and any folder given with `--search`. It goes a few levels down from each, never
//! into hidden or system folders, and stops after [`MAX_DIRS`] folders, so a search takes a
//! moment and never wanders the whole disk.
//!
//! A folder is a game when it holds MajorMUD's item file and one of its other world files:
//!
//! - **`WCC*.db`** (MBBSEmu's own store, or wbtrv32's beside a Worldgroup): [`Source::Db`],
//!   edited where it is. It is *live* when a game runs on it (Btrieve files or MBBSEmu
//!   beside it), and not when it is a realm folder Daydream made (`daydream convert`).
//! - **`WCC*.DAT` or `WCC*.VIR`** (Worldgroup, The Major BBS, MBBSEmu before it converts
//!   them): [`Source::Wcc`], edited in a working copy and written back with `write-dat`.
//!
//! silvermere is found by its settings file (`silvermere.toml`: `[database] url`, else
//! `<data_dir>/silvermere.db`), its database file (`silvermere.db`), its service's
//! environment file (`/etc/silvermere/env`) and `SILVERMERE_DATABASE_URL`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use daydream_store::realm::stem_of_db;
use daydream_wcc::tables::{Table, table_for_stem};
use serde::Serialize;

use super::Source;

/// The most folders one search looks in.
pub const MAX_DIRS: usize = 20_000;

/// Folder names never searched (compared without case): the system's own, package caches,
/// build output.
const SKIP: &[&str] = &[
    "windows",
    "$recycle.bin",
    "system volume information",
    "program files",
    "program files (x86)",
    "programdata",
    "appdata",
    "library",
    "applications",
    "node_modules",
    "target",
    "proc",
    "sys",
    "dev",
    "snap",
];

/// A game found on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Found {
    pub source: Source,
    /// What it looks like: `Worldgroup`, `MBBSEmu`, `silvermere`, ...
    pub label: String,
    /// How it was found: the folder searched, or the setting that names it.
    pub found_by: String,
    /// The game runs on these very files, so editing them edits the game ([`Source::Db`]).
    pub live: bool,
}

/// Where to look, and how deep.
#[derive(Debug, Clone, Default)]
pub struct Roots {
    pub dirs: Vec<(PathBuf, usize)>,
    /// Folders not to look in (Daydream's own working copies).
    pub skip: Vec<PathBuf>,
}

impl Roots {
    /// The usual places games are installed on this machine, and `extra` (searched deeper).
    pub fn usual(extra: &[PathBuf], with_defaults: bool) -> Self {
        let mut dirs: Vec<(PathBuf, usize)> = extra.iter().map(|d| (d.clone(), 4)).collect();
        if with_defaults {
            if let Ok(cwd) = std::env::current_dir() {
                dirs.push((cwd, 2));
            }
            if let Some(home) = home() {
                dirs.push((home, 3));
            }
            if cfg!(windows) {
                for letter in b'C'..=b'Z' {
                    let root = PathBuf::from(format!("{}:\\", letter as char));
                    if root.is_dir() {
                        dirs.push((root, 2));
                    }
                }
            } else {
                for d in ["/opt", "/srv", "/data"] {
                    dirs.push((PathBuf::from(d), 3));
                }
            }
        }
        Self {
            dirs,
            skip: Vec::new(),
        }
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// Every game under `roots`, and the silvermere databases the environment names.
pub fn discover(roots: &Roots) -> Vec<Found> {
    let mut out = Vec::new();
    let skip: Vec<PathBuf> = roots.skip.iter().filter_map(|d| canonical(d)).collect();
    let mut seen = BTreeSet::new();
    let mut budget = MAX_DIRS;
    for (root, depth) in &roots.dirs {
        let Some(root) = canonical(root) else {
            continue;
        };
        walk(
            &root,
            *depth,
            &skip,
            &mut seen,
            &mut budget,
            &mut out,
            &root,
        );
    }
    if let Ok(url) = std::env::var("SILVERMERE_DATABASE_URL")
        && !url.trim().is_empty()
    {
        push_silvermere(&mut out, url.trim(), "SILVERMERE_DATABASE_URL".into());
    }
    let env = Path::new("/etc/silvermere/env");
    if let Some(url) = std::fs::read_to_string(env)
        .ok()
        .and_then(|t| env_value(&t, "SILVERMERE_DATABASE_URL"))
    {
        push_silvermere(&mut out, &url, env.display().to_string());
    }
    // a database found by its settings file and again as a file: the first finding
    let mut seen = BTreeSet::new();
    out.retain(|f| seen.insert(f.source.clone()));
    out
}

/// The folder's real path, when it exists.
pub fn canonical(p: &Path) -> Option<PathBuf> {
    let c = std::fs::canonicalize(p).ok()?;
    Some(strip_verbatim(c))
}

/// `\\?\C:\x` as `C:\x`: what Windows' canonical paths look like to a person.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => p,
    }
}

fn walk(
    dir: &Path,
    depth: usize,
    skip: &[PathBuf],
    seen: &mut BTreeSet<PathBuf>,
    budget: &mut usize,
    out: &mut Vec<Found>,
    root: &Path,
) {
    if *budget == 0 || skip.iter().any(|s| dir.starts_with(s)) || !seen.insert(dir.to_path_buf()) {
        return;
    }
    *budget -= 1;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names = Vec::new();
    let mut subdirs = Vec::new();
    for e in entries.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        let name = e.file_name().to_string_lossy().into_owned();
        if ft.is_dir() {
            // no symlinks: they can loop, and lead off the roots
            let lower = name.to_ascii_lowercase();
            if !name.starts_with('.') && !SKIP.contains(&lower.as_str()) {
                subdirs.push(e.path());
            }
        } else if ft.is_file() {
            names.push(name);
        }
    }
    let found_by = root.display().to_string();
    out.extend(classify(dir, &names, &found_by));
    if depth == 0 {
        return;
    }
    subdirs.sort();
    for d in subdirs {
        walk(&d, depth - 1, skip, seen, budget, out, root);
    }
}

/// The MajorMUD file a name is (`WCCITEMS.DAT` -> its table and extension), when it is one.
fn wcc_file(name: &str) -> Option<(Table, String)> {
    let upper = name.to_ascii_uppercase();
    if upper.ends_with(".DB") {
        let stem = stem_of_db(name)?;
        return table_for_stem(&stem).map(|t| (t, "DB".into()));
    }
    let (stem, ext) = upper.rsplit_once('.')?;
    if ext != "DAT" && ext != "VIR" {
        return None;
    }
    table_for_stem(stem).map(|t| (t, ext.to_string()))
}

/// What the files of one folder make it, if anything.
pub fn classify(dir: &Path, names: &[String], found_by: &str) -> Vec<Found> {
    let mut out = Vec::new();
    let has = |n: &str| names.iter().any(|x| x.eq_ignore_ascii_case(n));
    let (mut db, mut btrieve): (Vec<Table>, Vec<Table>) = (Vec::new(), Vec::new());
    for n in names {
        if let Some((t, ext)) = wcc_file(n) {
            let set = if ext == "DB" { &mut db } else { &mut btrieve };
            if !set.contains(&t) {
                set.push(t);
            }
        }
    }
    // the item file and another of the world's: one stray file is not a game
    let world = |s: &Vec<Table>| s.contains(&Table::Items) && s.len() >= 2;
    let mbbsemu = names.iter().any(|n| {
        let l = n.to_ascii_lowercase();
        l.starts_with("mbbsemu") || l == "appsettings.json"
    });
    let worldgroup = has("WGSERVER.EXE");
    let worldgroup3 = names
        .iter()
        .any(|n| n.to_ascii_uppercase().starts_with("WCCITEM2."));
    let engine = if mbbsemu {
        "MBBSEmu"
    } else if worldgroup || worldgroup3 {
        "Worldgroup"
    } else {
        "MajorMUD"
    };
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string());
    if world(&db) {
        let made_by_daydream = dir.join(".daydream").join("source.json").is_file();
        let live = !made_by_daydream && (!btrieve.is_empty() || mbbsemu);
        let label = if live {
            format!("{name} ({engine}, .db files)")
        } else {
            format!("{name} (realm folder)")
        };
        out.push(Found {
            source: Source::Db {
                dir: dir.to_path_buf(),
            },
            label,
            found_by: found_by.to_string(),
            live,
        });
    } else if world(&btrieve) {
        out.push(Found {
            source: Source::Wcc {
                dir: dir.to_path_buf(),
            },
            label: format!("{name} ({engine})"),
            found_by: found_by.to_string(),
            live: false,
        });
    }
    if let Some(toml) = names
        .iter()
        .find(|n| n.eq_ignore_ascii_case("silvermere.toml"))
    {
        let path = dir.join(toml);
        if let Some(url) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| silvermere_url(&t, dir))
        {
            out.push(silvermere(&url, path.display().to_string()));
        }
    }
    if let Some(file) = names
        .iter()
        .find(|n| n.eq_ignore_ascii_case("silvermere.db"))
    {
        let url = daydream_silvermere::database_url(&dir.join(file).display().to_string());
        out.push(silvermere(&url, found_by.to_string()));
    }
    out
}

fn silvermere(url: &str, found_by: String) -> Found {
    let url = daydream_silvermere::database_url(url);
    // a hosted database by its host (`silvermere on ep-x.neon.tech`)
    let host = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))
        .map(|rest| {
            let rest = rest.split('/').next().unwrap_or(rest);
            let host = rest.rsplit('@').next().unwrap_or(rest);
            host.split(':').next().unwrap_or(host).to_string()
        });
    Found {
        label: match host {
            Some(h) if !h.is_empty() => format!("silvermere on {h}"),
            _ => "silvermere".into(),
        },
        source: Source::Silvermere { url },
        found_by,
        live: false,
    }
}

fn push_silvermere(out: &mut Vec<Found>, url: &str, found_by: String) {
    out.push(silvermere(url, found_by));
}

/// The database a `silvermere.toml` in `dir` names: `[database] url`, else `silvermere.db`
/// in its data folder (`data_dir`, else `./data`, else the file's own folder), when that
/// file exists.
pub fn silvermere_url(toml: &str, dir: &Path) -> Option<String> {
    let mut section = String::new();
    let (mut url, mut data_dir) = (None, None);
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line
                .trim_matches(|c| c == '[' || c == ']')
                .trim()
                .to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), toml_string(v));
        match (section.as_str(), k) {
            ("database", "url") => url = v,
            ("", "data_dir") => data_dir = v,
            _ => {}
        }
    }
    if let Some(u) = url.filter(|u| !u.is_empty()) {
        // a relative SQLite path is relative to where silvermere runs: the file's folder
        if let Some(rest) = u
            .strip_prefix("sqlite://")
            .or_else(|| u.strip_prefix("sqlite:"))
            && !rest.is_empty()
            && Path::new(rest.split('?').next().unwrap_or(rest)).is_relative()
            && !rest.contains(":memory:")
        {
            return Some(format!("sqlite:{}", dir.join(rest).display()));
        }
        return Some(u);
    }
    let candidates = match data_dir {
        Some(d) => vec![dir.join(d)],
        None => vec![dir.join("data"), dir.to_path_buf()],
    };
    candidates
        .into_iter()
        .map(|d| d.join("silvermere.db"))
        .find(|f| f.is_file())
        .map(|f| format!("sqlite:{}", f.display()))
}

/// A TOML value as a string: a quoted string's contents (comment after it dropped).
fn toml_string(v: &str) -> Option<String> {
    let v = v.trim();
    let quote = v.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let rest = &v[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

/// `KEY=value` (or `KEY='value'`) from an environment file.
pub fn env_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let l = l.trim().strip_prefix("export ").unwrap_or(l.trim());
        let (k, v) = l.split_once('=')?;
        if k.trim() != key {
            return None;
        }
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
        (!v.is_empty()).then_some(v)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn btrieve_folders_are_wcc_games() {
        let f = classify(
            Path::new("/bbs/WGSERV"),
            &names(&["WGSERVER.EXE", "WCCITEM2.DAT", "WCCKNMS2.DAT", "BBSUSR.DAT"]),
            "/bbs",
        );
        assert_eq!(f.len(), 1);
        assert!(matches!(f[0].source, Source::Wcc { .. }));
        assert_eq!(f[0].label, "WGSERV (Worldgroup)");
        assert!(!f[0].live);
    }

    #[test]
    fn one_stray_file_is_not_a_game() {
        assert!(classify(Path::new("/x"), &names(&["WCCITEMS.DAT"]), "/").is_empty());
        assert!(
            classify(
                Path::new("/x"),
                &names(&["WCCKNMSR.DAT", "WCCSPELS.DAT"]),
                "/"
            )
            .is_empty()
        );
    }

    #[test]
    fn db_files_beside_btrieve_files_are_live() {
        let f = classify(
            Path::new("/emu"),
            &names(&["WCCITEMS.DAT", "WCCITEMS.DB", "WCCKNMSR.DB", "MBBSEmu.exe"]),
            "/",
        );
        assert_eq!(f.len(), 1);
        assert!(matches!(f[0].source, Source::Db { .. }));
        assert!(f[0].live);
        assert_eq!(f[0].label, "emu (MBBSEmu, .db files)");
        let realm = classify(
            Path::new("/realm"),
            &names(&["WCCITEMS.db", "WCCKNMSR.db"]),
            "/",
        );
        assert!(!realm[0].live);
        assert_eq!(realm[0].label, "realm (realm folder)");
    }

    #[test]
    fn silvermere_settings_name_the_database() {
        let dir = Path::new("/srv/sm");
        assert_eq!(
            silvermere_url("[database]\nurl = \"postgres://u:p@h/db\"  # neon\n", dir).as_deref(),
            Some("postgres://u:p@h/db")
        );
        assert_eq!(
            silvermere_url("[database]\nurl = \"sqlite://data/world.db\"\n", dir).as_deref(),
            Some("sqlite:/srv/sm/data/world.db")
        );
        // the default: data/silvermere.db, only when it is there
        assert_eq!(silvermere_url("[database]\nurl = \"\"\n", dir), None);
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("data")).unwrap();
        std::fs::write(tmp.path().join("data/silvermere.db"), b"").unwrap();
        let url = silvermere_url("[server]\nurl = \"no\"\n", tmp.path()).unwrap();
        assert!(url.ends_with("silvermere.db"), "{url}");
    }

    #[test]
    fn hosted_databases_are_named_by_host() {
        let f = silvermere(
            "postgres://u:secret@ep-x.neon.tech:5432/db?sslmode=require",
            "env".into(),
        );
        assert_eq!(f.label, "silvermere on ep-x.neon.tech");
        assert_eq!(
            f.source.shown(),
            "postgres://u:****@ep-x.neon.tech:5432/db?sslmode=require"
        );
        assert_eq!(
            silvermere("/srv/sm/data/silvermere.db", "x".into()).label,
            "silvermere"
        );
    }

    #[test]
    fn environment_files() {
        let t = "# x\nSILVERMERE_DATABASE_URL='postgres://a@b/c?sslmode=require'\n";
        assert_eq!(
            env_value(t, "SILVERMERE_DATABASE_URL").as_deref(),
            Some("postgres://a@b/c?sslmode=require")
        );
        assert_eq!(env_value("OTHER=1", "SILVERMERE_DATABASE_URL"), None);
    }

    #[test]
    fn searching_finds_games_below_a_root() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("bbs").join("WGSERV");
        std::fs::create_dir_all(&game).unwrap();
        for f in ["WCCITEMS.DAT", "WCCKNMSR.DAT", "WCCMP001.DAT"] {
            std::fs::write(game.join(f), b"").unwrap();
        }
        let hidden = tmp.path().join(".hidden").join("game");
        std::fs::create_dir_all(&hidden).unwrap();
        for f in ["WCCITEMS.DAT", "WCCKNMSR.DAT"] {
            std::fs::write(hidden.join(f), b"").unwrap();
        }
        let roots = Roots {
            dirs: vec![(tmp.path().to_path_buf(), 3)],
            skip: Vec::new(),
        };
        let found: Vec<_> = discover(&roots)
            .into_iter()
            .filter(|f| !matches!(f.source, Source::Silvermere { .. }))
            .collect();
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            found[0].source,
            Source::Wcc {
                dir: canonical(&game).unwrap()
            }
        );
        // too shallow to reach it
        let roots = Roots {
            dirs: vec![(tmp.path().to_path_buf(), 1)],
            skip: Vec::new(),
        };
        assert!(
            discover(&roots)
                .iter()
                .all(|f| matches!(f.source, Source::Silvermere { .. }))
        );
    }
}
