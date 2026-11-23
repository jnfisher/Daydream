//! Btrieve index trees, read back to check them against the records (Daydream addition:
//! wbtrv32 rebuilds its indexes from the records and never reads these pages).
//!
//! The layout was decoded from wbtrv32's sample files (v5 and v6, one to three levels) and
//! is checked against every stock MajorMUD file in the tests. It is the groundwork for a
//! writer that inserts, deletes or re-keys records; today the only writer
//! ([`super::update`]) leaves the trees alone.
//!
//! # Layout
//!
//! - Each key's first segment definition starts with the root page pointer (4 bytes) and
//!   the key's entry count (a high word then a low word).
//! - A page pointer is three bytes of logical page number, stored like a PAT entry: the
//!   high byte, a flag byte (0x80 on v6), then the low word. `FF FF FF FF` is none.
//! - A record pointer is a byte address in logical page space (`page * page_length +
//!   offset`, the offset pointing at the slot), stored as a high word then a low word.
//! - An index page (`I` pages: PAT type 0x80 on v6) is an 8-byte header (page id, a
//!   usage count, the entry count), the pointer to the page holding the keys above the
//!   last entry, then the entries in key order: the pointer to the page of smaller keys,
//!   the key bytes and the record pointer. Every key value is in exactly one entry, on an
//!   inner page or a leaf (a B-tree, not a B+ tree); leaves have no child pointers.
//! - A key with linked duplicates has one entry per value, with the first and the last
//!   record of the chain; each record carries, for every such key in key order, a
//!   previous and a next record pointer at the end of its slot. A key with repeating
//!   duplicates has one entry per record.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use super::error::{BtrieveError, Fault, ReadError};
use super::header::{Version, hi_lo_at, u16_at};
use super::keys::{Key, KeyAttributes, KeyDataType, KeyDef};
use super::pages::PAGE_HEADER_LENGTH;
use super::pat::kind;
use super::{BtrieveFile, Record};

/// Bytes in front of an index page's first entry: the header and the last-child pointer.
const INDEX_HEADER_LENGTH: usize = 12;

/// A tree deeper than this is corrupt (Btrieve's own limit is far lower).
const MAX_DEPTH: usize = 32;

/// A key's index tree as its definition gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Root {
    pub page: Option<u32>,
    pub entries: u32,
}

impl Root {
    /// From the key's first segment definition.
    pub(crate) fn parse(def: &[u8]) -> Self {
        Self {
            page: page_pointer(&def[0..4]),
            entries: hi_lo_at(def, 4),
        }
    }
}

fn page_pointer(b: &[u8]) -> Option<u32> {
    (b[..4] != [0xFF; 4]).then(|| u32::from(b[0]) << 16 | u32::from(u16_at(b, 2)))
}

fn record_pointer(b: &[u8], at: usize) -> Option<u32> {
    let v = hi_lo_at(b, at);
    (v != u32::MAX).then_some(v)
}

/// Where a record lives: its logical page and slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordAddress {
    pub page: u32,
    pub slot: u16,
}

impl RecordAddress {
    pub fn of(record: &Record<'_>) -> Self {
        Self {
            page: record.page,
            slot: record.slot,
        }
    }
}

impl std::fmt::Display for RecordAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "page {} slot {}", self.page, self.slot)
    }
}

/// One index entry: a key value and its records, in the index's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub key: Vec<u8>,
    pub records: Vec<RecordAddress>,
}

/// What [`BtrieveFile::verify_indexes`] found, key by key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexReport {
    pub keys: Vec<KeyIndexReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyIndexReport {
    pub key: u16,
    /// Index pages in the tree.
    pub pages: usize,
    /// Levels, 1 for a lone leaf, 0 for an empty tree.
    pub depth: usize,
    pub entries: usize,
    pub records: usize,
    /// False when a segment's type has no comparison here, so the order went unchecked.
    pub order_checked: bool,
}

/// A walk's state for one key.
struct Walk {
    key: u16,
    entry_len: usize,
    linked: bool,
    visited: HashSet<u32>,
    depth: usize,
    entries: Vec<(Vec<u8>, u32, Option<u32>)>,
}

impl BtrieveFile {
    /// The entries of key `key`'s index, in index order, with linked duplicate chains
    /// followed through the records.
    pub fn index_entries(&mut self, key: usize) -> Result<Vec<IndexEntry>, BtrieveError> {
        self.index_entries_inner(key)
            .map(|(entries, _)| entries)
            .map_err(|e| e.at(&self.path))
    }

    /// Walks every key's index tree and checks it against the records: every live record
    /// is in each index exactly once (a null key value aside), under the key bytes the
    /// record holds, the entries are in key order, and the entry count matches the key
    /// definition's.
    pub fn verify_indexes(&mut self) -> Result<IndexReport, BtrieveError> {
        self.verify_inner().map_err(|e| e.at(&self.path))
    }

    fn verify_inner(&mut self) -> Result<IndexReport, ReadError> {
        let mut records: HashMap<RecordAddress, Vec<u8>> = HashMap::new();
        {
            let mut cursor = self.records();
            while let Some(record) = cursor.next_record() {
                let record = record.map_err(|e| Fault::corrupt(0, e.to_string()))?;
                records.insert(RecordAddress::of(&record), record.data.to_vec());
            }
        }
        let mut report = IndexReport { keys: Vec::new() };
        for k in 0..self.keys.len() {
            let key = self.keys[k].clone();
            let bad = |reason: String| Fault::Index {
                key: key.number,
                reason,
            };
            let (entries, (pages, depth)) = self.index_entries_inner(k)?;
            let expected = self.roots[k].entries as usize;
            if entries.len() != expected {
                return Err(bad(format!(
                    "{} entries in the tree, the key definition says {expected}",
                    entries.len()
                ))
                .into());
            }
            let mut seen = HashSet::new();
            let mut order_checked = true;
            let mut previous: Option<&[u8]> = None;
            for entry in &entries {
                if let Some(prev) = previous {
                    match compare(&key, prev, &entry.key) {
                        Some(Ordering::Less) => {}
                        Some(Ordering::Equal) if repeating(&key) => {}
                        Some(_) => {
                            return Err(bad(format!(
                                "entry {:02x?} follows {prev:02x?}",
                                entry.key
                            ))
                            .into());
                        }
                        None => order_checked = false,
                    }
                }
                previous = Some(&entry.key);
                for address in &entry.records {
                    let Some(data) = records.get(address) else {
                        return Err(
                            bad(format!("points at {address}, which holds no record")).into()
                        );
                    };
                    if !same_key(&key, key.extract(data).as_deref(), &entry.key) {
                        return Err(bad(format!(
                            "entry {:02x?} points at {address}, whose key differs",
                            entry.key
                        ))
                        .into());
                    }
                    if !seen.insert(*address) {
                        return Err(bad(format!("{address} is indexed twice")).into());
                    }
                }
            }
            let indexed = records
                .iter()
                .filter(|(_, data)| !is_null(&key, data))
                .count();
            if seen.len() != indexed {
                let missing = records
                    .iter()
                    .filter(|(a, d)| !seen.contains(a) && !is_null(&key, d))
                    .map(|(a, _)| *a)
                    .min();
                return Err(bad(format!(
                    "{} of {indexed} records indexed; first missing: {missing:?}",
                    seen.len()
                ))
                .into());
            }
            report.keys.push(KeyIndexReport {
                key: key.number,
                pages,
                depth,
                entries: entries.len(),
                records: seen.len(),
                order_checked,
            });
        }
        Ok(report)
    }

    /// The entries and the tree's (pages, depth).
    fn index_entries_inner(
        &mut self,
        k: usize,
    ) -> Result<(Vec<IndexEntry>, (usize, usize)), ReadError> {
        let key = self.keys[k].clone();
        let linked = linked(&key);
        let mut walk = Walk {
            key: key.number,
            entry_len: 4 + key.length() + 4 + if linked { 4 } else { 0 },
            linked,
            visited: HashSet::new(),
            depth: 0,
            entries: Vec::new(),
        };
        if let Some(root) = self.roots[k].page {
            self.walk(&mut walk, root, 1)?;
        }
        let tail = self.dup_tail_offset(k);
        let mut out = Vec::with_capacity(walk.entries.len());
        for (bytes, first, last) in std::mem::take(&mut walk.entries) {
            let mut records = vec![self.address(walk.key, first)?];
            if let (Some(last), Some(tail)) = (last, tail) {
                // follow the chain's next pointers from the first record to the last
                let mut at = first;
                while at != last {
                    if records.len() > usize::try_from(self.header.record_count).unwrap_or(0) {
                        return Err(Fault::Index {
                            key: walk.key,
                            reason: format!(
                                "duplicate chain from {first:#x} does not reach {last:#x}"
                            ),
                        }
                        .into());
                    }
                    let slot = self.slot_bytes(self.address(walk.key, at)?)?;
                    at = record_pointer(&slot, tail + 4).ok_or_else(|| Fault::Index {
                        key: walk.key,
                        reason: format!("duplicate chain from {first:#x} ends before {last:#x}"),
                    })?;
                    records.push(self.address(walk.key, at)?);
                }
            }
            out.push(IndexEntry {
                key: bytes,
                records,
            });
        }
        Ok((out, (walk.visited.len(), walk.depth)))
    }

    fn walk(&mut self, walk: &mut Walk, logical: u32, depth: usize) -> Result<(), ReadError> {
        let key = walk.key;
        let bad = |reason: String| Fault::Index {
            key,
            reason: format!("index page {logical}: {reason}"),
        };
        if depth > MAX_DEPTH {
            return Err(bad("the tree is too deep".into()).into());
        }
        if !walk.visited.insert(logical) {
            return Err(bad("reached twice".into()).into());
        }
        walk.depth = walk.depth.max(depth);
        let entry = self.lookup(logical)?;
        if self.header.version == Version::V6 && entry.kind & kind::INDEX_BIT == 0 {
            return Err(bad(format!("PAT type {:#04x} is not an index page", entry.kind)).into());
        }
        let offset = entry.offset.ok_or_else(|| bad("not mapped".into()))?;
        let mut page = vec![0u8; usize::from(self.header.page_length)];
        self.reader.read_at(offset, &mut page)?;
        let count = usize::from(u16_at(&page, 6));
        if INDEX_HEADER_LENGTH + count * walk.entry_len > page.len() {
            return Err(bad(format!("{count} entries do not fit")).into());
        }
        let key_len = walk.entry_len - 8 - if walk.linked { 4 } else { 0 };
        for i in 0..count {
            let at = INDEX_HEADER_LENGTH + i * walk.entry_len;
            if let Some(child) = page_pointer(&page[at..]) {
                self.walk(walk, child, depth + 1)?;
            }
            let key = page[at + 4..at + 4 + key_len].to_vec();
            let first = record_pointer(&page, at + 4 + key_len)
                .ok_or_else(|| bad(format!("entry {i} has no record")))?;
            let last = if walk.linked {
                record_pointer(&page, at + 8 + key_len)
            } else {
                None
            };
            walk.entries.push((key, first, last));
        }
        if let Some(child) = page_pointer(&page[8..]) {
            self.walk(walk, child, depth + 1)?;
        }
        Ok(())
    }

    /// The slot a record pointer names.
    fn address(&self, key: u16, pointer: u32) -> Result<RecordAddress, Fault> {
        let page_length = u32::from(self.header.page_length);
        let physical = u32::from(self.header.physical_record_length);
        let (page, offset) = (pointer / page_length, pointer % page_length);
        let header = PAGE_HEADER_LENGTH as u32;
        if offset < header || !(offset - header).is_multiple_of(physical) {
            return Err(Fault::Index {
                key,
                reason: format!("record pointer {pointer:#x} is not at a slot"),
            });
        }
        Ok(RecordAddress {
            page,
            slot: ((offset - header) / physical) as u16,
        })
    }

    /// A whole slot: the v6 usage count, the record and its tail.
    pub(crate) fn slot_bytes(&mut self, at: RecordAddress) -> Result<Vec<u8>, ReadError> {
        let (offset, len) = self.slot_offset(at)?;
        let mut slot = vec![0u8; len];
        self.reader.read_at(offset, &mut slot)?;
        Ok(slot)
    }

    /// The file offset and length of a slot.
    pub(crate) fn slot_offset(&mut self, at: RecordAddress) -> Result<(u64, usize), ReadError> {
        let physical = usize::from(self.header.physical_record_length);
        if usize::from(at.slot) >= self.header.records_per_page() {
            return Err(Fault::corrupt(at.page, format!("no slot {}", at.slot)).into());
        }
        let entry = self.lookup(at.page)?;
        let page = entry
            .offset
            .ok_or_else(|| Fault::corrupt(at.page, "page is not mapped"))?;
        Ok((
            page + (PAGE_HEADER_LENGTH + usize::from(at.slot) * physical) as u64,
            physical,
        ))
    }

    /// Where key `k`'s previous/next pointers sit in a slot, if it has linked duplicates:
    /// one pair per such key, in key order, ending the slot.
    fn dup_tail_offset(&self, k: usize) -> Option<usize> {
        if !linked(&self.keys[k]) {
            return None;
        }
        let total = self.keys.iter().filter(|key| linked(key)).count();
        let before = self.keys[..k].iter().filter(|key| linked(key)).count();
        Some(usize::from(self.header.physical_record_length) - 8 * (total - before))
    }
}

/// A record's key bytes against an index entry's. Btrieve copies a zstring segment into
/// the index only up to its NUL, leaving whatever was in the page after it, so the types
/// with a comparison compare by it.
fn same_key(key: &Key, record: Option<&[u8]>, entry: &[u8]) -> bool {
    record.is_some_and(|r| match compare(key, r, entry) {
        Some(order) => order == Ordering::Equal,
        None => r == entry,
    })
}

fn linked(key: &Key) -> bool {
    let a = key.primary().attributes;
    a.has(KeyAttributes::DUPLICATES) && !a.has(KeyAttributes::REPEATING_DUPLICATES)
}

fn repeating(key: &Key) -> bool {
    key.primary()
        .attributes
        .has(KeyAttributes::REPEATING_DUPLICATES)
}

/// True when a record's key is null and so left out of the index.
fn is_null(key: &Key, record: &[u8]) -> bool {
    let a = key.primary().attributes;
    let null = |s: &KeyDef| {
        s.bytes(record)
            .is_some_and(|b| b.iter().all(|&c| c == s.null_value))
    };
    if a.has(KeyAttributes::NULL_ALL_SEGMENTS) {
        key.segments.iter().all(null)
    } else if a.has(KeyAttributes::NULL_ANY_SEGMENT) {
        key.segments.iter().any(null)
    } else {
        false
    }
}

/// Btrieve's order of two key values, segment by segment; `None` for a type compared
/// nowhere here.
pub(crate) fn compare(key: &Key, a: &[u8], b: &[u8]) -> Option<Ordering> {
    let mut at = 0;
    for s in &key.segments {
        let len = usize::from(s.length);
        let (x, y) = (&a[at..at + len], &b[at..at + len]);
        at += len;
        let order = match s.data_type {
            KeyDataType::Integer | KeyDataType::AutoInc => signed(x).cmp(&signed(y)),
            KeyDataType::Unsigned | KeyDataType::UnsignedBinary | KeyDataType::OldBinary => {
                unsigned(x).cmp(&unsigned(y))
            }
            KeyDataType::Zstring => text(s, until_nul(x)).cmp(&text(s, until_nul(y))),
            KeyDataType::String | KeyDataType::OldAscii => text(s, x).cmp(&text(s, y)),
            KeyDataType::Lstring => text(s, lstring(x)).cmp(&text(s, lstring(y))),
            _ => return None,
        };
        let order = if s.attributes.has(KeyAttributes::DESCENDING) {
            order.reverse()
        } else {
            order
        };
        if order != Ordering::Equal {
            return Some(order);
        }
    }
    Some(Ordering::Equal)
}

fn signed(v: &[u8]) -> i128 {
    let mut bytes = [if v.last().is_some_and(|b| b & 0x80 != 0) {
        0xFF
    } else {
        0
    }; 16];
    bytes[..v.len()].copy_from_slice(v);
    i128::from_le_bytes(bytes)
}

fn unsigned(v: &[u8]) -> u128 {
    let mut bytes = [0u8; 16];
    bytes[..v.len()].copy_from_slice(v);
    u128::from_le_bytes(bytes)
}

fn lstring(v: &[u8]) -> &[u8] {
    &v[1..1 + usize::from(v[0]).min(v.len() - 1)]
}

fn until_nul(v: &[u8]) -> &[u8] {
    &v[..v.iter().position(|&b| b == 0).unwrap_or(v.len())]
}

fn text(s: &KeyDef, v: &[u8]) -> Vec<u8> {
    match &s.acs {
        Some(table) => v.iter().map(|&b| table[usize::from(b)]).collect(),
        None => v.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointers_decode_as_stored() {
        assert_eq!(page_pointer(&[0x00, 0x80, 0x28, 0x00]), Some(0x28));
        assert_eq!(page_pointer(&[0x01, 0x80, 0x28, 0x00]), Some(0x01_0028));
        assert_eq!(page_pointer(&[0xFF; 4]), None);
        assert_eq!(
            record_pointer(&[0x30, 0x00, 0x06, 0x80], 0),
            Some(0x0030_8006)
        );
    }

    #[test]
    fn integers_compare_by_sign() {
        assert!(signed(&[0xFF, 0xFF]) < signed(&[0x01, 0x00]));
        assert!(unsigned(&[0xFF, 0xFF]) > unsigned(&[0x01, 0x00]));
    }
}
