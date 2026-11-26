//! A realm: a directory of wbtrv32 `.db` files.
//!
//! Each file is named after the Btrieve file it holds: `WCCITEMS.db` for `WCCITEMS.DAT`,
//! as wbtrv32 names it beside a running realm's `.DAT`. wbtrv32 names the conversion of a
//! virgin file `WCCITEMS.VIR.db`; Daydream reads both, and writes the first form so MBBSEmu
//! finds the files under the names the game opens.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use daydream_types::records::layout::{self as l, Layout, LayoutSet};
use daydream_wcc::tables::{self, Table};

use crate::db::{ConvertReport, StoreError, WbtrvDb};

/// One `.db` of a realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealmFile {
    /// The Btrieve file's stem, upper case (`WCCITEMS`).
    pub stem: String,
    pub path: PathBuf,
    /// The record type, for a file Daydream decodes.
    pub table: Option<Table>,
}

/// A realm directory.
#[derive(Debug, Clone)]
pub struct Realm {
    dir: PathBuf,
    files: Vec<RealmFile>,
}

/// The Btrieve stem of a `.db` file name (`WCCITEMS.db`, `WCCITEMS.VIR.db`), or `None`
/// when it is not a `.db`.
pub fn stem_of_db(name: &str) -> Option<String> {
    let (rest, ext) = name.rsplit_once('.')?;
    if !ext.eq_ignore_ascii_case("db") {
        return None;
    }
    let stem = match rest.rsplit_once('.') {
        Some((s, e)) if e.eq_ignore_ascii_case("vir") || e.eq_ignore_ascii_case("dat") => s,
        _ => rest,
    };
    Some(stem.to_ascii_uppercase())
}

impl Realm {
    /// Opens a realm directory: every `.db` in it (not its subdirectories).
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        let io = |source| StoreError::Io {
            path: dir.to_path_buf(),
            source,
        };
        let mut by_stem: BTreeMap<String, RealmFile> = BTreeMap::new();
        for entry in std::fs::read_dir(dir).map_err(io)? {
            let entry = entry.map_err(io)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(stem) = stem_of_db(&name) else {
                continue;
            };
            if !entry.file_type().map_err(io)?.is_file() {
                continue;
            }
            let file = RealmFile {
                table: tables::table_for_stem(&stem),
                stem: stem.clone(),
                path: entry.path(),
            };
            // a running realm's WCCITEMS.db wins over a converted WCCITEMS.VIR.db
            let is_vir = name.to_ascii_uppercase().ends_with(".VIR.DB");
            match by_stem.get(&stem) {
                Some(_) if is_vir => {}
                _ => {
                    by_stem.insert(stem, file);
                }
            }
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            files: by_stem.into_values().collect(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every `.db`, by stem.
    pub fn files(&self) -> &[RealmFile] {
        &self.files
    }

    /// The file with this stem (any case).
    pub fn file(&self, stem: &str) -> Option<&RealmFile> {
        self.files
            .iter()
            .find(|f| f.stem.eq_ignore_ascii_case(stem))
    }

    /// The files holding `table` (several for rooms: one per map file).
    pub fn files_of(&self, table: Table) -> impl Iterator<Item = &RealmFile> {
        self.files.iter().filter(move |f| f.table == Some(table))
    }

    /// The build whose layouts its records have: Worldgroup 3's when it holds any of that
    /// build's files (`WCCITEM2`, ...).
    pub fn layout_set(&self) -> LayoutSet {
        let wg = self
            .files
            .iter()
            .any(|f| tables::WORLDGROUP_FILES.iter().any(|w| w.stem == f.stem));
        if wg {
            LayoutSet::Worldgroup
        } else {
            LayoutSet::Dos
        }
    }
}

/// The layout of `table`'s records in `set`.
pub fn layout_of(table: Table, set: LayoutSet) -> &'static Layout {
    let dos: &'static Layout = match table {
        Table::Rooms => &l::ROOM,
        Table::Items => &l::ITEM,
        Table::Monsters => &l::MONSTER,
        Table::ActiveMonsters => &l::ACTMON,
        Table::Spells => &l::SPELL,
        Table::Classes => &l::CLASS,
        Table::Races => &l::RACE,
        Table::Shops => &l::SHOP,
        Table::Messages => &l::MUDMSG,
        Table::TextBlocks => &l::TEXTBLK,
        Table::Actions => &l::ACTION,
        Table::Gangs => &l::GANG,
        Table::BankBooks => &l::BANKBOOK,
        Table::ItemOwnership => &l::ITEMOWN,
        Table::Users => &l::PLAYER,
    };
    set.layout(dos)
}

/// One converted file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub report: ConvertReport,
}

/// Converts every Btrieve file in `src` (`*.DAT`, and `*.VIR` where no `.DAT` of the same
/// stem exists) into `<STEM>.db` in `dest`, replacing what is there.
pub fn convert_dir(src: &Path, dest: &Path) -> Result<Vec<Converted>, StoreError> {
    convert_dir_matching(src, dest, |_| true)
}

/// Whether a Btrieve stem is one of MajorMUD's files (`WCC*`): what a game folder's working
/// copy holds, leaving out the BBS's own files beside them (its user accounts, other modules).
pub fn is_wcc_stem(stem: &str) -> bool {
    stem.len() > 3 && stem[..3].eq_ignore_ascii_case("WCC")
}

/// [`convert_dir`], for the stems `keep` takes (upper case) only.
pub fn convert_dir_matching(
    src: &Path,
    dest: &Path,
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<Converted>, StoreError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| StoreError::Io { path, source }
    };
    std::fs::create_dir_all(dest).map_err(io(dest))?;
    let mut sources: BTreeMap<String, (bool, PathBuf)> = BTreeMap::new();
    for entry in std::fs::read_dir(src).map_err(io(src))? {
        let entry = entry.map_err(io(src))?;
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        let Some((stem, ext)) = name.rsplit_once('.') else {
            continue;
        };
        let is_dat = match ext {
            "DAT" => true,
            "VIR" => false,
            _ => continue,
        };
        if !keep(stem) {
            continue;
        }
        match sources.get(stem) {
            Some((true, _)) => {}
            _ => {
                sources.insert(stem.to_string(), (is_dat, entry.path()));
            }
        }
    }
    let mut out = Vec::new();
    for (stem, (_, source)) in sources {
        let db = dest.join(format!("{stem}.db"));
        let (_, report) = WbtrvDb::convert(&source, &db)?;
        out.push(Converted {
            source,
            dest: db,
            report,
        });
    }
    // the realm holds what the files hold: its changes count from here
    crate::journal::reset(dest)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_names_give_btrieve_stems() {
        assert_eq!(stem_of_db("WCCITEMS.db").as_deref(), Some("WCCITEMS"));
        assert_eq!(stem_of_db("wccitems.VIR.DB").as_deref(), Some("WCCITEMS"));
        assert_eq!(stem_of_db("WCCITEMS.DAT.db").as_deref(), Some("WCCITEMS"));
        assert_eq!(stem_of_db("WCCITEMS.DAT"), None);
    }
}
