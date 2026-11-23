//! Writing a realm back into its Btrieve `WCC*.DAT` files (M5).
//!
//! Each `.db` is compared with the Btrieve file it was converted from, found by its stem in
//! the WCC folder (`WCCITEMS.db` and `WCCITEMS.DAT`, or `WCCITEMS.VIR` where there is no
//! `.DAT`), and written the least intrusive way:
//!
//! - **unchanged**: the same records in the same order. The file is not written.
//! - **updated**: the same number of records and no key changed. The changed records are
//!   overwritten in their slots ([`update_in_place`]), so the file's bytes change only
//!   where its records did.
//! - **rebuilt**: records added, deleted or re-keyed. The file is written anew from the
//!   `.db`'s records, in their order, with the source's page size, keys and flags
//!   ([`build`]).
//!
//! A rebuilt record moves to a new position, which only a running game would notice: the
//! realm must be stopped while its files are written. A source that changed since its
//! `.db` was converted (the game ran on it) is refused, since writing the `.db` over it
//! would lose the game's changes.

use std::path::{Path, PathBuf};

use daydream_wcc::btrieve::build::{BuildError, build};
use daydream_wcc::btrieve::update::{UpdateError, update_in_place};
use daydream_wcc::btrieve::{BtrieveFile, OpenOptions, RecordAddress};

use crate::db::{Fingerprint, StoreError, WbtrvDb};
use crate::realm::Realm;

/// What writing one file did, or would do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The records are those of the Btrieve file: nothing written.
    Unchanged,
    /// This many records overwritten in place.
    Updated(usize),
    /// Written anew with this many records.
    Rebuilt(usize),
}

/// One `.db` written back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub stem: String,
    /// The Btrieve file the `.db` was compared with.
    pub source: PathBuf,
    /// The file written (or that would be). A `.VIR` source is written as a `.DAT`.
    pub dest: PathBuf,
    pub action: Action,
}

/// Why a realm could not be written back.
#[derive(Debug, thiserror::Error)]
pub enum DatError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Btrieve(#[from] daydream_wcc::btrieve::BtrieveError),
    #[error("{}: {source}", file.display())]
    Build {
        file: PathBuf,
        #[source]
        source: BuildError,
    },
    #[error(transparent)]
    Update(UpdateError),
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{stem}: no {stem}.DAT or {stem}.VIR in {}", dir.display())]
    NoSource { stem: String, dir: PathBuf },
    #[error(
        "{}: changed since the realm's {stem}.db was converted from it (the game ran on it?); \
         writing the realm would lose those changes",
        file.display()
    )]
    SourceChanged { stem: String, file: PathBuf },
}

/// Where and how to write.
#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    /// Write the files here instead of over the WCC folder. Files that need no change are
    /// copied, so the folder ends up a complete set.
    pub out: Option<PathBuf>,
    /// Writing over the WCC folder, copy each file to this folder before changing it.
    pub backup: Option<PathBuf>,
    /// Only report what would be done.
    pub dry_run: bool,
    /// Write even over a Btrieve file that changed since its `.db` was converted.
    pub force: bool,
}

/// The Btrieve file of `stem` in `dir`: its `.DAT`, else its `.VIR`.
fn source_of(dir: &Path, stem: &str) -> Result<Option<PathBuf>, DatError> {
    let io = |source| DatError::Io {
        path: dir.to_path_buf(),
        source,
    };
    let (mut dat, mut vir) = (None, None);
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let entry = entry.map_err(io)?;
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if let Some((s, ext)) = name.rsplit_once('.')
            && s == stem
        {
            match ext {
                "DAT" => dat = Some(entry.path()),
                "VIR" => vir = Some(entry.path()),
                _ => {}
            }
        }
    }
    Ok(dat.or(vir))
}

/// The file a source is written as: itself for a `.DAT`, the `.DAT` beside it for a `.VIR`.
fn dat_name(source: &Path) -> PathBuf {
    let is_vir = source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("vir"));
    if is_vir {
        source.with_extension("DAT")
    } else {
        source.to_path_buf()
    }
}

fn read_records(file: &mut BtrieveFile) -> Result<Vec<(RecordAddress, Vec<u8>)>, DatError> {
    let mut out = Vec::new();
    let mut cursor = file.records();
    while let Some(record) = cursor.next_record() {
        let record = record?;
        out.push((RecordAddress::of(&record), record.data.to_vec()));
    }
    Ok(out)
}

/// Whether `records` can go in place of `old`: the same number, fixed-length, every key
/// as it was. The changed records with their positions, or `None` for a rebuild.
fn in_place(
    file: &BtrieveFile,
    old: &[(RecordAddress, Vec<u8>)],
    records: &[Vec<u8>],
) -> Option<Vec<(RecordAddress, Vec<u8>)>> {
    if file.header.is_variable_length() || old.len() != records.len() {
        return None;
    }
    let len = usize::from(file.header.record_length);
    let mut changed = Vec::new();
    for ((at, before), after) in old.iter().zip(records) {
        if before == after {
            continue;
        }
        if after.len() != len
            || file
                .keys
                .iter()
                .any(|k| k.extract(before) != k.extract(after))
        {
            return None;
        }
        changed.push((*at, after.clone()));
    }
    Some(changed)
}

fn copy(from: &Path, to: &Path) -> Result<(), DatError> {
    std::fs::copy(from, to)
        .map(drop)
        .map_err(|source| DatError::Io {
            path: to.to_path_buf(),
            source,
        })
}

/// Writes `bytes` to `path` through a temporary file beside it, so a failure leaves the
/// old file whole.
fn replace(path: &Path, bytes: &[u8]) -> Result<(), DatError> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".daydream-tmp");
    let tmp = PathBuf::from(tmp);
    let io = |source| DatError::Io {
        path: path.to_path_buf(),
        source,
    };
    std::fs::write(&tmp, bytes).map_err(io)?;
    std::fs::rename(&tmp, path).map_err(io)
}

fn now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as u32)
}

/// Writes every `.db` of `realm` back into the Btrieve files of the WCC folder `wcc` (or
/// into `options.out`). Every file is checked before any is written, so a refusal leaves
/// everything as it was.
pub fn write_dat(
    realm: &Realm,
    wcc: &Path,
    options: &WriteOptions,
) -> Result<Vec<Written>, DatError> {
    struct Plan {
        written: Written,
        db: PathBuf,
        bytes: Option<Vec<u8>>,
        updates: Vec<(RecordAddress, Vec<u8>)>,
    }
    let mut plans = Vec::new();
    for f in realm.files() {
        let source = source_of(wcc, &f.stem)?.ok_or_else(|| DatError::NoSource {
            stem: f.stem.clone(),
            dir: wcc.to_path_buf(),
        })?;
        let db = WbtrvDb::open_read_only(&f.path)?;
        if !options.force
            && let Some(recorded) = db.source_fingerprint()?
        {
            let now = Fingerprint::of(&source).map_err(|e| DatError::Io {
                path: source.clone(),
                source: e,
            })?;
            if (now.size, now.hash) != (recorded.size, recorded.hash) {
                return Err(DatError::SourceChanged {
                    stem: f.stem.clone(),
                    file: source,
                });
            }
        }
        let records: Vec<Vec<u8>> = db.records()?.into_iter().map(|(_, r)| r).collect();
        let mut file = OpenOptions::new().variable_length(true).open(&source)?;
        let old = read_records(&mut file)?;
        let unchanged =
            old.len() == records.len() && old.iter().zip(&records).all(|(a, b)| a.1 == *b);
        let (action, bytes, updates) = if unchanged {
            (Action::Unchanged, None, Vec::new())
        } else if let Some(updates) = in_place(&file, &old, &records) {
            (Action::Updated(updates.len()), None, updates)
        } else {
            let bytes = build(&file, &records, now()).map_err(|source| DatError::Build {
                file: f.path.clone(),
                source,
            })?;
            (Action::Rebuilt(records.len()), Some(bytes), Vec::new())
        };
        let name = dat_name(&source);
        let dest = match &options.out {
            Some(out) => out.join(name.file_name().unwrap()),
            None => name,
        };
        plans.push(Plan {
            written: Written {
                stem: f.stem.clone(),
                source,
                dest,
                action,
            },
            db: f.path.clone(),
            bytes,
            updates,
        });
    }
    if options.dry_run {
        return Ok(plans.into_iter().map(|p| p.written).collect());
    }

    let create = |dir: &PathBuf| {
        std::fs::create_dir_all(dir).map_err(|source| DatError::Io {
            path: dir.clone(),
            source,
        })
    };
    if let Some(out) = &options.out {
        create(out)?;
    }
    for p in &plans {
        let w = &p.written;
        let over_source = w.dest == w.source;
        if options.out.is_none() && w.action == Action::Unchanged {
            continue;
        }
        if options.out.is_none()
            && let Some(backup) = &options.backup
            && w.dest.exists()
        {
            create(backup)?;
            copy(&w.dest, &backup.join(w.dest.file_name().unwrap()))?;
        }
        match (&p.bytes, w.action) {
            (Some(bytes), _) => replace(&w.dest, bytes)?,
            (None, _) if !over_source => {
                copy(&w.source, &w.dest)?;
                if !p.updates.is_empty() {
                    update_in_place(&w.dest, &p.updates).map_err(DatError::Update)?;
                }
            }
            (None, _) => {
                update_in_place(&w.dest, &p.updates).map_err(DatError::Update)?;
            }
        }
        // the .db now holds what its WCC file holds: record that, so the next write
        // compares with the file just written
        if options.out.is_none() {
            let fp = Fingerprint::of(&w.dest).map_err(|source| DatError::Io {
                path: w.dest.clone(),
                source,
            })?;
            WbtrvDb::open(&p.db)?.set_source_fingerprint(fp)?;
        }
    }
    Ok(plans.into_iter().map(|p| p.written).collect())
}
