//! Merging one file's records into another, as NMR's Merge Users tool does.
//!
//! Every record of the source whose first key (a user's BBS id, for `WCCUSER`) the
//! destination lacks is copied byte for byte; a record the destination already has is
//! skipped. The files must have the same record length.

use std::collections::HashSet;

use crate::{StoreError, WbtrvDb};

/// What a merge did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeReport {
    pub copied: usize,
    /// Records whose key the destination already had.
    pub skipped: Vec<Vec<u8>>,
}

/// The bytes of a record's first key.
fn key_bytes(db: &WbtrvDb, record: &[u8]) -> Vec<u8> {
    let mut key = Vec::new();
    if let Some(k) = db.keys().first() {
        for s in &k.segments {
            let (at, len) = (usize::from(s.offset), usize::from(s.length));
            let end = (at + len).min(record.len());
            key.extend_from_slice(record.get(at..end).unwrap_or(&[]));
        }
    }
    key
}

/// Copies `from`'s records into `into`, skipping keys `into` has. With `dry_run` nothing is
/// written.
pub fn merge(from: &WbtrvDb, into: &mut WbtrvDb, dry_run: bool) -> Result<MergeReport, StoreError> {
    let (a, b) = (from.metadata(), into.metadata());
    if a.record_length != b.record_length {
        return Err(StoreError::RecordLength {
            path: from.path().to_path_buf(),
            expected: b.record_length,
            found: a.record_length,
        });
    }
    let mut have: HashSet<Vec<u8>> = into
        .records()?
        .iter()
        .map(|(_, r)| key_bytes(into, r))
        .collect();
    let mut report = MergeReport::default();
    for (_, record) in from.records()? {
        let key = key_bytes(from, &record);
        if !have.insert(key.clone()) {
            report.skipped.push(key);
            continue;
        }
        if !dry_run {
            into.insert(&record)?;
        }
        report.copied += 1;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Metadata;
    use crate::key::{Key, Segment, ty};

    fn db(dir: &std::path::Path, name: &str, ids: &[u16]) -> WbtrvDb {
        let keys = vec![Key {
            segments: vec![Segment {
                number: 0,
                index: 0,
                attributes: 0,
                data_type: ty::UNSIGNED_BINARY,
                offset: 0,
                length: 2,
                null_value: 0,
                acs: None,
            }],
            acs_name: None,
            acs: None,
        }];
        let meta = Metadata {
            record_length: 8,
            physical_record_length: 12,
            page_length: 1024,
            variable_length_records: false,
            version: 4,
        };
        let mut db = WbtrvDb::create(&dir.join(name), meta, keys).unwrap();
        for &id in ids {
            let mut r = vec![0u8; 8];
            r[..2].copy_from_slice(&id.to_le_bytes());
            r[2..].copy_from_slice(name.as_bytes().get(..6).unwrap_or(b"xxxxxx"));
            db.insert(&r).unwrap();
        }
        db
    }

    #[test]
    fn new_keys_are_copied_and_known_ones_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let from = db(dir.path(), "FROMDB.db", &[1, 2, 3]);
        let mut into = db(dir.path(), "INTODB.db", &[2, 9]);
        let dry = merge(&from, &mut into, true).unwrap();
        assert_eq!(dry.copied, 2);
        assert_eq!(into.count().unwrap(), 2);
        let r = merge(&from, &mut into, false).unwrap();
        assert_eq!(r.copied, 2);
        assert_eq!(r.skipped, vec![2u16.to_le_bytes().to_vec()]);
        let mut ids: Vec<u16> = into
            .records()
            .unwrap()
            .iter()
            .map(|(_, r)| u16::from_le_bytes([r[0], r[1]]))
            .collect();
        ids.sort();
        assert_eq!(ids, [1, 2, 3, 9]);
        // record 2 kept the destination's bytes
        let two = into
            .records()
            .unwrap()
            .into_iter()
            .find(|(_, r)| r[0] == 2)
            .unwrap()
            .1;
        assert_eq!(&two[2..], b"INTODB");
    }
}
