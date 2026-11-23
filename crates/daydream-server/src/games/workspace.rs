//! Daydream's workspace: the working copies it edits, one folder per game, and which game
//! was open last.
//!
//! ```text
//! <workspace>/last.json                 the game open last (`daydream serve` with no REALM)
//! <workspace>/realms/<name>-<id>/       a working copy: the game's world as .db files
//!     .daydream/source.json             where it came from, and when
//! <workspace>/realms/<name>-<id>.previous/   the copy a refresh replaced
//! ```
//!
//! The workspace is `DAYDREAM_HOME`, else `%LOCALAPPDATA%\daydream` on Windows, else
//! `$XDG_DATA_HOME/daydream`, else `~/.local/share/daydream`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Game, Source};

/// Overrides the workspace folder.
pub const HOME_ENV: &str = "DAYDREAM_HOME";

/// What a working copy records about itself (`.daydream/source.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub source: Source,
    pub label: String,
    /// When it was last read from the game (RFC 3339).
    pub pulled: String,
}

/// The game open last.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Last {
    pub realm_dir: PathBuf,
    pub game: Option<Game>,
}

/// The workspace folder (see the module docs).
pub fn default_dir() -> PathBuf {
    let set = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if let Some(d) = set(HOME_ENV) {
        return d;
    }
    if cfg!(windows)
        && let Some(d) = set("LOCALAPPDATA")
    {
        return d.join("daydream");
    }
    set("XDG_DATA_HOME")
        .map(|d| d.join("daydream"))
        .or_else(|| set("HOME").map(|d| d.join(".local/share/daydream")))
        .or_else(|| set("USERPROFILE").map(|d| d.join(".daydream")))
        .unwrap_or_else(|| std::env::temp_dir().join("daydream"))
}

/// The record a working copy keeps of its source.
pub fn saved_path(realm_dir: &Path) -> PathBuf {
    realm_dir.join(".daydream").join("source.json")
}

/// What `realm_dir` records of its source, if it is a working copy.
pub fn read_saved(realm_dir: &Path) -> Option<Saved> {
    let text = std::fs::read_to_string(saved_path(realm_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Records `saved` in the working copy at `realm_dir`.
pub fn write_saved(realm_dir: &Path, saved: &Saved) -> std::io::Result<()> {
    let path = saved_path(realm_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_private(&path, &serde_json::to_vec_pretty(saved)?)
}

/// Writes a file only its owner may read (a silvermere URL can hold a password).
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// A workspace folder.
#[derive(Debug, Clone)]
pub struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn realms(&self) -> PathBuf {
        self.dir.join("realms")
    }

    /// The working copy of `source`, named after its label so a person can find it.
    pub fn copy_dir(&self, source: &Source, label: &str) -> PathBuf {
        let mut name: String = label
            .split(" (")
            .next()
            .unwrap_or(label)
            .chars()
            .filter_map(|c| {
                if c.is_ascii_alphanumeric() {
                    Some(c.to_ascii_lowercase())
                } else if c == '-' || c == '_' || c == ' ' {
                    Some('-')
                } else {
                    None
                }
            })
            .take(24)
            .collect();
        if name.is_empty() {
            name = source.kind().to_string();
        }
        // a copy made before under another label is still this source's
        if let Some(existing) = self.copies().into_iter().find(|(_, s)| s.source == *source) {
            return existing.0;
        }
        self.realms().join(format!("{name}-{}", source.id()))
    }

    /// Every working copy, with its record.
    pub fn copies(&self) -> Vec<(PathBuf, Saved)> {
        let Ok(entries) = std::fs::read_dir(self.realms()) else {
            return Vec::new();
        };
        let mut out: Vec<(PathBuf, Saved)> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.extension().is_none_or(|e| e != "previous"))
            .filter_map(|p| read_saved(&p).map(|s| (p, s)))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Replaces `copy` with the finished folder `fresh`, keeping the copy it replaces as
    /// `<copy>.previous` (one only).
    pub fn replace(&self, fresh: &Path, copy: &Path) -> std::io::Result<()> {
        if copy.exists() {
            let previous = copy.with_extension("previous");
            if previous.exists() {
                std::fs::remove_dir_all(&previous)?;
            }
            std::fs::rename(copy, &previous)?;
        }
        std::fs::rename(fresh, copy)
    }

    /// A new, empty folder beside the copies to build one in.
    pub fn scratch(&self) -> std::io::Result<tempfile::TempDir> {
        std::fs::create_dir_all(self.realms())?;
        tempfile::Builder::new()
            .prefix(".building-")
            .tempdir_in(self.realms())
    }

    pub fn last(&self) -> Option<Last> {
        let text = std::fs::read_to_string(self.dir.join("last.json")).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save_last(&self, last: &Last) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        write_private(
            &self.dir.join("last.json"),
            &serde_json::to_vec_pretty(last)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_are_named_for_people_and_found_again() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::new(tmp.path().to_path_buf());
        let source = Source::Wcc {
            dir: PathBuf::from("/bbs/WGSERV"),
        };
        let dir = ws.copy_dir(&source, "WGSERV (Worldgroup)");
        assert_eq!(
            dir.file_name().unwrap().to_string_lossy(),
            format!("wgserv-{}", source.id())
        );
        std::fs::create_dir_all(&dir).unwrap();
        let saved = Saved {
            source: source.clone(),
            label: "WGSERV (Worldgroup)".into(),
            pulled: "2026-11-23T00:00:00Z".into(),
        };
        write_saved(&dir, &saved).unwrap();
        assert_eq!(read_saved(&dir), Some(saved));
        assert_eq!(ws.copy_dir(&source, "renamed"), dir);
        assert_eq!(ws.copies().len(), 1);
    }

    #[test]
    fn replacing_keeps_one_previous_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::new(tmp.path().to_path_buf());
        let copy = ws.realms().join("x-1");
        for round in 0..3 {
            let fresh = ws.scratch().unwrap().keep();
            std::fs::write(fresh.join("n"), round.to_string()).unwrap();
            ws.replace(&fresh, &copy).unwrap();
        }
        assert_eq!(std::fs::read_to_string(copy.join("n")).unwrap(), "2");
        let previous = copy.with_extension("previous");
        assert_eq!(std::fs::read_to_string(previous.join("n")).unwrap(), "1");
    }
}
