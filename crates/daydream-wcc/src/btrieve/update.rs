//! Writing changed records back into a Btrieve file, in place (Daydream addition).
//!
//! The first step towards writing `WCC*.DAT` files directly. An update overwrites a
//! record's bytes in its slot and touches nothing else: not the index pages, the FCR, the
//! PAT, the slot's usage count or its duplicate-chain pointers. That is only sound while
//! every key's bytes stay the same, so an update that changes a key is refused, and the
//! indexes stay valid by construction ([`BtrieveFile::verify_indexes`] checks it in the
//! tests). Adding, deleting or re-keying records needs the index trees maintained, which
//! this does not do yet.
//!
//! The file must not be open in Btrieve or a running game while it is written.

use std::collections::{HashMap, HashSet};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::index::RecordAddress;
use super::{BtrieveError, BtrieveFile};

/// Why an update was refused. Nothing is written unless every update is acceptable.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error(transparent)]
    Btrieve(#[from] BtrieveError),
    #[error("{}: {source}", file.display())]
    Io {
        file: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{}: no record at {at}", file.display())]
    NoRecord { file: PathBuf, at: RecordAddress },
    #[error("{}: {at} is updated twice", file.display())]
    Twice { file: PathBuf, at: RecordAddress },
    #[error("{}: {at}: {len} bytes for {expected}-byte records", file.display())]
    Length {
        file: PathBuf,
        at: RecordAddress,
        len: usize,
        expected: usize,
    },
    #[error(
        "{}: {at}: key {key} would change, which needs the index updated (not supported yet)",
        file.display()
    )]
    KeyChanged {
        file: PathBuf,
        at: RecordAddress,
        key: u16,
    },
}

/// Overwrites the records at the given addresses with new bytes (each the file's record
/// length, keys unchanged). Returns how many records changed; identical ones are skipped.
/// Variable-length files are refused.
pub fn update_in_place(
    path: &Path,
    updates: &[(RecordAddress, Vec<u8>)],
) -> Result<usize, UpdateError> {
    let file_name = || path.to_path_buf();
    let mut file = BtrieveFile::open(path)?;
    let wanted: HashSet<RecordAddress> = updates.iter().map(|(a, _)| *a).collect();
    let mut current: HashMap<RecordAddress, Vec<u8>> = HashMap::new();
    {
        let mut records = file.records();
        while let Some(record) = records.next_record() {
            let record = record?;
            let at = RecordAddress::of(&record);
            if wanted.contains(&at) {
                current.insert(at, record.data.to_vec());
            }
        }
    }

    let expected = usize::from(file.header.record_length);
    let prefix = file.header.slot_prefix();
    let mut writes = Vec::new();
    let mut done = HashSet::new();
    for (at, data) in updates {
        if !done.insert(*at) {
            return Err(UpdateError::Twice {
                file: file_name(),
                at: *at,
            });
        }
        let Some(old) = current.get(at) else {
            return Err(UpdateError::NoRecord {
                file: file_name(),
                at: *at,
            });
        };
        if data.len() != expected {
            return Err(UpdateError::Length {
                file: file_name(),
                at: *at,
                len: data.len(),
                expected,
            });
        }
        if let Some(key) = file.keys.iter().find(|k| k.extract(old) != k.extract(data)) {
            return Err(UpdateError::KeyChanged {
                file: file_name(),
                at: *at,
                key: key.number,
            });
        }
        if old == data {
            continue;
        }
        let (offset, _) = file.slot_offset(*at).map_err(|e| e.at(path))?;
        writes.push((offset + prefix as u64, data));
    }

    let io = |source| UpdateError::Io {
        file: file_name(),
        source,
    };
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(io)?;
    for (offset, data) in &writes {
        out.seek(SeekFrom::Start(*offset)).map_err(io)?;
        out.write_all(data).map_err(io)?;
    }
    out.sync_all().map_err(io)?;
    Ok(writes.len())
}
