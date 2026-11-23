//! Building a whole Btrieve 6 file from records, for the `.DAT` writer (Daydream addition).
//!
//! A changed file is rebuilt rather than patched: the records go onto data pages in the
//! order given (so the reader's ordinals follow that order), every key's index is laid out
//! at once from the sorted records ([`super::btree`]), and the File Control Record and the
//! page allocation table are written as Btrieve 6.15 writes them for a file it creates.
//! The output is always a v6 file, whatever the source's version, as Btrieve's own
//! `BUTIL -CLONE` makes one.
//!
//! # Layout
//!
//! The format was decoded from files Btrieve 6.15 created and filled in CI (the Windows
//! job's `btrieve_clones_the_stock_files`), and from Worldgroup's own v6 files.
//!
//! - Physical pages 0 and 1 are the two FCR copies (the first active, the second blank
//!   but for its magic, page length and version byte); 2 and 3 the first PAT pair (the
//!   first active). Logical page `L` is physical `L + 3` plus two for every further PAT
//!   pair before it; pair `p` sits at physical `2 + p * page_length / 4`, and each maps
//!   `page_length / 4 - 2` logical pages. Every entry of a PAT page is pre-mapped, with
//!   type 0 where the page is not in use, as Btrieve leaves a new file.
//! - Logical pages 1.. hold each key's index pages in key order, then the data pages.
//!   An empty file is a lone empty leaf per key and one data page of free slots, which is
//!   what Btrieve's own Create writes.
//! - A data page is a 6-byte header (page id, usage `0x8000`) and slots of the physical
//!   record length: a usage count of 1, the record, and for each key with linked
//!   duplicates a previous and a next record pointer. The unused slots on the last data
//!   page form the free-slot chain the FCR points at.
//! - Index pages hold at most the key definition's maximum entries (what fits in a page)
//!   and, but for the root, at least half of it, as Btrieve keeps them.
//!
//! The FCR's creation time and Btrieve's running counter at +0x68 vary between files
//! Btrieve creates; the time is the caller's and the counter is 0.

use std::cmp::Ordering;

use super::BtrieveFile;
use super::btree::{self, Tree};
use super::index::{compare, is_null};
use super::keys::{KEY_DEF_LENGTH, Key, KeyAttributes};
use super::pat::kind;

/// Bytes in front of a data page's first slot.
const DATA_HEADER: usize = 6;
/// Bytes in front of an index page's first entry: the page header and the last child.
const INDEX_HEADER: usize = 12;
/// Where the key definitions start in the FCR.
const KEY_DEFS: usize = 0x110;
/// A record pointer or page pointer that points nowhere.
const NONE: [u8; 4] = [0xFF; 4];

/// Why a file could not be built. Nothing is written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildError {
    #[error("{0}")]
    Unsupported(String),
    #[error("record {index}: {len} bytes for {expected}-byte records")]
    Length {
        index: usize,
        len: usize,
        expected: usize,
    },
    #[error("records {first} and {second} have the same value for key {key}, which is unique")]
    Duplicate {
        key: u16,
        first: usize,
        second: usize,
    },
}

/// The KAT's offset in the FCR and the most key segments a page length allows, as
/// Btrieve 6.15 sets them; `None` for a page length it does not support.
fn kat_layout(page_length: usize) -> Option<(usize, usize)> {
    let max = match page_length {
        512 => return Some((0x7A, 8)),
        1024 => 23,
        1536 => 24,
        2048 | 2560 | 3072 | 3584 => 54,
        4096 => 119,
        _ => return None,
    };
    Some((page_length - (2 * max + 2), max))
}

/// A page pointer: `[page high][flag][page low word]`, the flag `0x80 | key` for an
/// index page.
fn page_pointer(page: u32, flag: u8) -> [u8; 4] {
    let lo = (page as u16).to_le_bytes();
    [(page >> 16) as u8, flag, lo[0], lo[1]]
}

/// A 32-bit value as a high word then a low word.
fn hi_lo(v: u32) -> [u8; 4] {
    let (hi, lo) = ((v >> 16) as u16, v as u16);
    let (hi, lo) = (hi.to_le_bytes(), lo.to_le_bytes());
    [hi[0], hi[1], lo[0], lo[1]]
}

fn put(buf: &mut [u8], at: usize, bytes: &[u8]) {
    buf[at..at + bytes.len()].copy_from_slice(bytes);
}

fn put_u16(buf: &mut [u8], at: usize, v: u16) {
    put(buf, at, &v.to_le_bytes());
}

/// One key's index: its entries in key order, each the records holding its value (one
/// but for linked duplicates, in record order), and the laid-out tree with its pages'
/// logical numbers.
struct Index {
    entries: Vec<Vec<usize>>,
    entry_len: usize,
    max: usize,
    tree: Tree,
    /// Logical page of each of `tree.pages`.
    logical: Vec<u32>,
}

/// Where the records and pages go.
struct Plan {
    page_length: usize,
    physical_len: usize,
    per_page: usize,
    first_data: u32,
    data_pages: u32,
    /// Logical pages in use, page 0 (the FCR) included.
    page_count: u32,
    pages_per_pat: u32,
    pats: u32,
}

impl Plan {
    /// Physical page of logical page `l` (1 or more).
    fn physical(&self, l: u32) -> u32 {
        l + 3 + 2 * ((l - 1) / self.pages_per_pat)
    }

    fn pat_physical(&self, p: u32) -> u32 {
        2 + p * (self.page_length as u32 / 4)
    }

    fn record_pointer(&self, index: usize) -> u32 {
        let page = self.first_data + (index / self.per_page) as u32;
        let slot = index % self.per_page;
        page * self.page_length as u32 + (DATA_HEADER + slot * self.physical_len) as u32
    }
}

/// Builds a Btrieve 6 file with `file`'s page length, record length and keys holding
/// `records` in the order given. `created` is the creation time the FCR records (Unix
/// seconds). Variable-length files and keys with a collating sequence are refused.
pub fn build(
    file: &BtrieveFile,
    records: &[impl AsRef<[u8]>],
    created: u32,
) -> Result<Vec<u8>, BuildError> {
    let unsupported = |s: String| Err(BuildError::Unsupported(s));
    let h = &file.header;
    if h.is_variable_length() {
        return unsupported("variable-length records are not supported yet".into());
    }
    let page_length = usize::from(h.page_length);
    let Some((kat, max_segments)) = kat_layout(page_length) else {
        return unsupported(format!("page length {page_length}"));
    };
    let keys = &file.keys;
    let segments: usize = keys.iter().map(|k| k.segments.len()).sum();
    if segments > max_segments {
        return unsupported(format!(
            "{segments} key segments, more than a {page_length}-byte page holds"
        ));
    }
    if let Some(k) = keys
        .iter()
        .find(|k| k.segments.iter().any(|s| s.requires_acs()))
    {
        return unsupported(format!(
            "key {} uses a collating sequence, which is not supported yet",
            k.number
        ));
    }
    let record_length = usize::from(h.record_length);
    for (index, r) in records.iter().enumerate() {
        let len = r.as_ref().len();
        if len != record_length {
            return Err(BuildError::Length {
                index,
                len,
                expected: record_length,
            });
        }
    }

    let linked: Vec<bool> = keys.iter().map(is_linked).collect();
    let linked_count = linked.iter().filter(|&&l| l).count();
    let physical_len = record_length + 2 + 8 * linked_count;
    if physical_len > page_length - DATA_HEADER {
        return unsupported(format!(
            "{physical_len}-byte slots do not fit a {page_length}-byte page"
        ));
    }

    // every key's entries, sorted
    let mut indexes = Vec::with_capacity(keys.len());
    for (k, key) in keys.iter().enumerate() {
        let entry_len = 4 + key.length() + 4 + if linked[k] { 4 } else { 0 };
        let max = (page_length - INDEX_HEADER) / entry_len;
        if max < 2 {
            return unsupported(format!(
                "key {}: {entry_len}-byte entries do not fit two to a page",
                key.number
            ));
        }
        let entries = sorted_entries(key, records, linked[k])?;
        let tree = btree::layout(entries.len(), max);
        indexes.push(Index {
            entries,
            entry_len,
            max,
            logical: Vec::new(),
            tree,
        });
    }

    // logical pages: the index pages key by key, then the data pages
    let mut next: u32 = 1;
    for index in &mut indexes {
        index.logical = (0..index.tree.pages.len() as u32)
            .map(|i| next + i)
            .collect();
        next += index.tree.pages.len() as u32;
    }
    let per_page = (page_length - DATA_HEADER) / physical_len;
    let data_pages = records.len().div_ceil(per_page).max(1) as u32;
    let pages_per_pat = (page_length / 4 - 2) as u32;
    let page_count = next + data_pages;
    let plan = Plan {
        page_length,
        physical_len,
        per_page,
        first_data: next,
        data_pages,
        page_count,
        pages_per_pat,
        pats: (page_count - 1).div_ceil(pages_per_pat).max(1),
    };
    let physical_pages = plan.physical(page_count - 1) as usize + 1;
    let mut out = vec![0u8; physical_pages * page_length];

    // PAT: what each logical page is
    let mut kinds = vec![0u8; page_count as usize];
    for (k, index) in indexes.iter().enumerate() {
        for &l in &index.logical {
            kinds[l as usize] = kind::INDEX_BIT | k as u8;
        }
    }
    for l in plan.first_data..page_count {
        kinds[l as usize] = kind::DATA;
    }
    write_pats(&mut out, &plan, &kinds);

    // index pages
    for (k, index) in indexes.iter().enumerate() {
        write_index(
            &mut out,
            &plan,
            k as u8,
            key_records(&keys[k], records),
            index,
        );
    }

    // data pages and the free-slot chain
    let tails: Vec<(usize, &Index)> = linked
        .iter()
        .zip(&indexes)
        .filter(|(l, _)| **l)
        .map(|(_, i)| i)
        .enumerate()
        .map(|(n, i)| (record_length + 2 + 8 * n, i))
        .collect();
    let free = write_data(&mut out, &plan, records, &tails);

    write_fcr(
        &mut out,
        &plan,
        file,
        &indexes,
        Fcr {
            records: records.len() as u32,
            linked: linked_count,
            segments,
            kat,
            free,
            created,
        },
    );
    Ok(out)
}

fn is_linked(key: &Key) -> bool {
    let a = key.primary().attributes;
    a.has(KeyAttributes::DUPLICATES) && !a.has(KeyAttributes::REPEATING_DUPLICATES)
}

/// The key's entries in key order: each the records holding a value (one record each but
/// for linked duplicates). Records with a null key are left out; equal keys keep the
/// records' order.
fn sorted_entries(
    key: &Key,
    records: &[impl AsRef<[u8]>],
    linked: bool,
) -> Result<Vec<Vec<usize>>, BuildError> {
    let values = key_records(key, records);
    let mut order: Vec<usize> = (0..records.len())
        .filter(|&i| !is_null(key, records[i].as_ref()))
        .collect();
    if let Some(&first) = order.first()
        && compare(key, &values[first], &values[first]).is_none()
    {
        return Err(BuildError::Unsupported(format!(
            "key {}: a segment type with no known order",
            key.number
        )));
    }
    let cmp =
        |a: &usize, b: &usize| compare(key, &values[*a], &values[*b]).unwrap_or(Ordering::Equal);
    order.sort_by(cmp);
    let duplicates = key.allows_duplicates();
    let mut entries: Vec<Vec<usize>> = Vec::with_capacity(order.len());
    for i in order {
        if let Some(last) = entries.last_mut()
            && let Some(&previous) = last.last()
            && cmp(&previous, &i) == Ordering::Equal
        {
            if !duplicates {
                return Err(BuildError::Duplicate {
                    key: key.number,
                    first: previous,
                    second: i,
                });
            }
            if linked {
                last.push(i);
                continue;
            }
        }
        entries.push(vec![i]);
    }
    Ok(entries)
}

/// Every record's key bytes.
fn key_records(key: &Key, records: &[impl AsRef<[u8]>]) -> Vec<Vec<u8>> {
    records
        .iter()
        .map(|r| {
            key.extract(r.as_ref())
                .expect("records are the record length")
        })
        .collect()
}

fn write_pats(out: &mut [u8], plan: &Plan, kinds: &[u8]) {
    let len = plan.page_length;
    for p in 0..plan.pats {
        let at = plan.pat_physical(p) as usize * len;
        for (copy, usage) in [(0, 1u32), (1, 0)] {
            let page = &mut out[at + copy * len..at + (copy + 1) * len];
            put(page, 0, b"PP");
            put_u16(page, 2, p as u16 + 1);
            put(page, 4, &usage.to_le_bytes());
            for j in 1..=plan.pages_per_pat {
                let l = p * plan.pages_per_pat + j;
                let kind = match (copy, kinds.get(l as usize)) {
                    (0, Some(&k)) => k,
                    _ => 0,
                };
                put(
                    page,
                    4 + 4 * j as usize,
                    &page_pointer(plan.physical(l), kind),
                );
            }
        }
    }
}

fn write_index(out: &mut [u8], plan: &Plan, k: u8, values: Vec<Vec<u8>>, index: &Index) {
    let flag = kind::INDEX_BIT | k;
    let child = |c: usize| page_pointer(index.logical[c], flag);
    let key_len = values.first().map_or(0, Vec::len);
    for (p, page) in index.tree.pages.iter().enumerate() {
        let l = index.logical[p];
        let at = plan.physical(l) as usize * plan.page_length;
        let buf = &mut out[at..at + plan.page_length];
        put(buf, 0, &page_pointer(l, flag));
        put_u16(buf, 6, page.entries.len() as u16);
        put(buf, 8, &page.children.last().map_or(NONE, |&c| child(c)));
        for (i, &e) in page.entries.iter().enumerate() {
            let at = INDEX_HEADER + i * index.entry_len;
            let records = &index.entries[e];
            let (first, last) = (records[0], records[records.len() - 1]);
            put(buf, at, &page.children.get(i).map_or(NONE, |&c| child(c)));
            put(buf, at + 4, &values[first]);
            put(buf, at + 4 + key_len, &hi_lo(plan.record_pointer(first)));
            if index.entry_len == 4 + key_len + 8 {
                put(buf, at + 8 + key_len, &hi_lo(plan.record_pointer(last)));
            }
        }
    }
}

/// Writes the data pages; returns the head of the free-slot chain.
fn write_data(
    out: &mut [u8],
    plan: &Plan,
    records: &[impl AsRef<[u8]>],
    tails: &[(usize, &Index)],
) -> u32 {
    let len = plan.page_length;
    for d in 0..plan.data_pages {
        let l = plan.first_data + d;
        let at = plan.physical(l) as usize * len;
        put(&mut out[at..], 0, &page_pointer(l, kind::DATA));
        put_u16(&mut out[at..], 4, 0x8000);
    }
    let slot_at = |i: usize| {
        let ptr = plan.record_pointer(i) as usize;
        plan.physical(ptr as u32 / len as u32) as usize * len + ptr % len
    };
    for (i, r) in records.iter().enumerate() {
        let at = slot_at(i);
        put(out, at, &[1, 0]);
        put(out, at + 2, r.as_ref());
    }
    // linked duplicates: each record's previous and next in its value's chain
    for &(offset, index) in tails {
        let mut links = vec![(NONE, NONE); records.len()];
        for chain in &index.entries {
            for w in chain.windows(2) {
                links[w[0]].1 = hi_lo(plan.record_pointer(w[1]));
                links[w[1]].0 = hi_lo(plan.record_pointer(w[0]));
            }
        }
        for (i, (previous, next)) in links.iter().enumerate() {
            let at = slot_at(i) + offset;
            put(out, at, previous);
            put(out, at + 4, next);
        }
    }
    // the unused slots of the last data page
    let used = records.len();
    let total = plan.data_pages as usize * plan.per_page;
    for i in used..total {
        let at = slot_at(i);
        let next = if i + 1 < total {
            hi_lo(plan.record_pointer(i + 1))
        } else {
            NONE
        };
        put(out, at + 2, &next);
    }
    if used < total {
        plan.record_pointer(used)
    } else {
        u32::MAX
    }
}

/// The FCR fields that are not in the plan.
struct Fcr {
    records: u32,
    linked: usize,
    segments: usize,
    kat: usize,
    free: u32,
    created: u32,
}

fn write_fcr(out: &mut [u8], plan: &Plan, file: &BtrieveFile, indexes: &[Index], f: Fcr) {
    let len = plan.page_length;
    let h = &file.header;
    let record_length = h.record_length;
    // the second copy: blank but for its magic, page length and version byte
    let copy1 = &mut out[len..2 * len];
    put(copy1, 0, b"FC\0\0");
    put_u16(copy1, 0x08, len as u16);
    copy1[0x4B] = 6;

    let fcr = &mut out[..len];
    put(fcr, 0, b"FC\0\0");
    put(fcr, 0x04, &1u32.to_le_bytes());
    put_u16(fcr, 0x08, len as u16);
    put(fcr, 0x0C, &NONE);
    put(fcr, 0x10, &NONE);
    put_u16(fcr, 0x14, file.keys.len() as u16);
    put_u16(fcr, 0x16, record_length);
    put_u16(fcr, 0x18, plan.physical_len as u16);
    put(fcr, 0x1A, &hi_lo(f.records));
    put(fcr, 0x22, &[0xFF, 0xFF]);
    put_u16(fcr, 0x24, 1);
    put(fcr, 0x26, &hi_lo(plan.page_count));
    fcr[0x4B] = 6;
    let in_last_pat = plan.page_count - plan.pages_per_pat * (plan.pats - 1);
    put_u16(fcr, 0x4C, plan.pats as u16);
    put_u16(fcr, 0x4E, (4 + 4 * in_last_pat) as u16);
    put_u16(fcr, 0x50, plan.pats as u16);
    put(fcr, 0x52, &plan.page_count.to_le_bytes());
    put(fcr, 0x56, &f.created.to_le_bytes());
    put(fcr, 0x5A, &[0xFF; 8]);
    put(fcr, 0x63, &[0xFF; 3]);
    if f.linked > 0 {
        put_u16(fcr, 0x72, record_length + 2);
    }
    put_u16(fcr, 0x74, f.linked as u16);
    fcr[0x76] = file.keys.len() as u8;
    fcr[0x77] = f.segments as u8;
    put_u16(fcr, 0x78, f.kat as u16);
    put(fcr, 0x98, &NONE);
    put(fcr, 0x9C, &hi_lo(f.free));
    for i in 0..5 {
        let at = 0xA0 + 12 * i;
        let end = (at + 12).min(0xD4);
        fcr[at + 1..end].fill(0xFF);
    }

    // key definitions, and the KAT pointing at each key's first segment
    let mut at = KEY_DEFS;
    let mut linked_before = 0;
    for (k, (key, index)) in file.keys.iter().zip(indexes).enumerate() {
        put_u16(fcr, f.kat + 2 * k, at as u16);
        let linked = is_linked(key);
        for (s, segment) in key.segments.iter().enumerate() {
            let def = &mut fcr[at..at + KEY_DEF_LENGTH];
            if s == 0 {
                let root = index.logical[index.tree.root];
                put(def, 0x00, &page_pointer(root, kind::INDEX_BIT | k as u8));
                put(def, 0x04, &hi_lo(index.entries.len() as u32));
                put_u16(def, 0x0A, key.length() as u16);
                put_u16(def, 0x0C, index.entry_len as u16);
                put_u16(def, 0x0E, index.max as u16);
                put_u16(def, 0x10, (index.max / 2) as u16);
                if linked {
                    put_u16(def, 0x12, record_length + 2 + 8 * linked_before);
                }
                def[0x18] = kind::INDEX_BIT | k as u8;
            }
            put_u16(def, 0x08, segment.attributes.0);
            put_u16(def, 0x14, segment.offset + 2);
            put_u16(def, 0x16, segment.length);
            if segment.attributes.has(KeyAttributes::EXTENDED_TYPE) {
                def[0x1C] = segment.data_type.code();
            }
            def[0x1D] = segment.null_value;
            at += KEY_DEF_LENGTH;
        }
        if linked {
            linked_before += 1;
        }
    }
}
