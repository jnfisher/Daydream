//! The index checker and the in-place record writer (the direct `.DAT` writer spike).
//!
//! The checker runs over the fixtures here, wbtrv32's sample files (its checkout, from
//! `tools/wbtrv32/build-oracle.sh`, required in CI) and every stock MajorMUD file. The
//! writer rewrites every non-key byte of a file and the indexes must still check out.

use std::path::{Path, PathBuf};

use daydream_wcc::btrieve::update::{UpdateError, update_in_place};
use daydream_wcc::btrieve::{BtrieveFile, OpenOptions, RecordAddress};

use crate::stock as stock_data;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/btrieve")
        .join(name)
}

fn open(path: &Path) -> BtrieveFile {
    OpenOptions::new()
        .variable_length(true)
        .open(path)
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Checks a file's indexes and prints what they hold.
fn verify(path: &Path) {
    let report = open(path)
        .verify_indexes()
        .unwrap_or_else(|e| panic!("{e}"));
    for k in &report.keys {
        println!(
            "{:<14} key {} pages {:>4} depth {} entries {:>6} records {:>6}{}",
            path.file_name().unwrap().to_string_lossy(),
            k.key,
            k.pages,
            k.depth,
            k.entries,
            k.records,
            if k.order_checked {
                ""
            } else {
                " (order unchecked)"
            }
        );
    }
}

/// wbtrv32's sample files, from its checkout.
fn wbtrv32_assets() -> Option<Vec<PathBuf>> {
    let Some(dir) = std::env::var_os("DAYDREAM_WBTRV32_DIR").filter(|v| !v.is_empty()) else {
        if std::env::var_os("CI").is_some() {
            panic!("CI needs the wbtrv32 checkout: eval \"$(tools/wbtrv32/build-oracle.sh)\"");
        }
        eprintln!("skipping wbtrv32's sample files: DAYDREAM_WBTRV32_DIR is not set");
        return None;
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(PathBuf::from(dir).join("assets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dat")))
        .collect();
    files.sort();
    Some(files)
}

/// Every record: address and payload.
fn records(path: &Path) -> Vec<(RecordAddress, Vec<u8>)> {
    let mut file = open(path);
    let mut out = Vec::new();
    let mut records = file.records();
    while let Some(record) = records.next_record() {
        let record = record.unwrap();
        out.push((RecordAddress::of(&record), record.data.to_vec()));
    }
    out
}

/// A copy of `path` in a new temporary directory.
fn copy(path: &Path) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join(path.file_name().unwrap());
    std::fs::copy(path, &to).unwrap();
    (dir, to)
}

/// Flips every byte outside the file's keys in every record, writes that in place and
/// checks the result: the records read back changed, the slots' other bytes and every
/// other page did not, and the indexes still match.
fn rewrite_every_record(path: &Path) {
    let (_dir, copy) = copy(path);
    let file = open(&copy);
    let mut is_key = vec![false; usize::from(file.header.record_length)];
    for segment in file.keys.iter().flat_map(|k| &k.segments) {
        let at = usize::from(segment.offset);
        is_key[at..at + usize::from(segment.length)].fill(true);
    }
    drop(file);
    let before = records(&copy);
    let updates: Vec<_> = before
        .iter()
        .map(|(at, data)| {
            let mut data = data.clone();
            for (b, key) in data.iter_mut().zip(&is_key) {
                if !key {
                    *b ^= 0x5A;
                }
            }
            (*at, data)
        })
        .collect();
    let old_bytes = std::fs::read(&copy).unwrap();
    let written = update_in_place(&copy, &updates).unwrap_or_else(|e| panic!("{e}"));
    let changed_records = is_key.iter().any(|k| !k) as usize * before.len();
    assert_eq!(written, changed_records);
    assert_eq!(records(&copy), updates, "{}", path.display());

    // only record bytes changed
    let new_bytes = std::fs::read(&copy).unwrap();
    assert_eq!(old_bytes.len(), new_bytes.len());
    let mut file = open(&copy);
    let prefix = if file.header.version == daydream_wcc::btrieve::Version::V6 {
        2
    } else {
        0
    };
    let physical = usize::from(file.header.physical_record_length);
    let page_length = usize::from(file.header.page_length);
    let changed: Vec<usize> = (0..old_bytes.len())
        .filter(|&i| old_bytes[i] != new_bytes[i])
        .collect();
    let mut slots = std::collections::HashSet::new();
    for &i in &changed {
        let within = i % page_length;
        assert!(within >= 6, "byte {i:#x}: a page header changed");
        let in_slot = (within - 6) % physical;
        assert!(
            in_slot >= prefix && in_slot < prefix + is_key.len() && !is_key[in_slot - prefix],
            "byte {i:#x}: changed outside a record's non-key bytes"
        );
        slots.insert((i / page_length, (within - 6) / physical));
    }
    assert!(slots.len() <= before.len());
    file.verify_indexes().unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn fixtures_indexes_match_their_records() {
    for name in ["MBBSEMU.DAT", "MULTIACS.DAT", "VARIABLE.DAT"] {
        verify(&fixture(name));
    }
}

#[test]
fn wbtrv32_samples_indexes_match_their_records() {
    for path in wbtrv32_assets().unwrap_or_default() {
        verify(&path);
    }
}

#[test]
fn every_stock_file_indexes_match_their_records() {
    let dir = stock_data::dir();
    let mut names: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("vir") || e.eq_ignore_ascii_case("dat"))
        })
        .collect();
    names.sort();
    assert!(names.len() >= 17, "{names:?}");
    for path in names {
        verify(&path);
    }
}

#[test]
fn rewriting_non_key_bytes_keeps_the_indexes() {
    rewrite_every_record(&fixture("MBBSEMU.DAT"));
    if let Some(assets) = wbtrv32_assets() {
        for path in assets {
            // the writer refuses variable-length files
            if !open(&path).header.is_variable_length() {
                rewrite_every_record(&path);
            }
        }
    }
}

#[test]
fn rewriting_the_stock_items_keeps_the_indexes() {
    for name in [
        "WCCITEMS.VIR",
        "WCCKNMSR.VIR",
        "WCCMP001.VIR",
        "WCCSPELS.VIR",
    ] {
        rewrite_every_record(&stock_data::file(name));
    }
}

#[test]
fn updates_that_change_a_key_or_miss_are_refused_and_write_nothing() {
    let (_dir, path) = copy(&fixture("MBBSEMU.DAT"));
    let original = std::fs::read(&path).unwrap();
    let all = records(&path);
    let (at, data) = all[0].clone();
    let file = open(&path);
    let key = &file.keys[1].segments[0];
    let mut rekeyed = data.clone();
    rekeyed[usize::from(key.offset)] ^= 1;
    drop(file);

    // a good update first, so a refusal must also hold back the good one
    let mut fine = all[1].1.clone();
    fine[0] ^= 1; // bytes 0-1 are in no key
    let good = (all[1].0, fine);

    let err = update_in_place(&path, &[good.clone(), (at, rekeyed)]).unwrap_err();
    assert!(
        matches!(err, UpdateError::KeyChanged { key: 1, .. }),
        "{err}"
    );
    let missing = RecordAddress { page: 5, slot: 4 };
    let err = update_in_place(&path, &[good.clone(), (missing, data.clone())]).unwrap_err();
    assert!(matches!(err, UpdateError::NoRecord { .. }), "{err}");
    let err = update_in_place(&path, &[good.clone(), (at, data[1..].to_vec())]).unwrap_err();
    assert!(matches!(err, UpdateError::Length { .. }), "{err}");
    let err = update_in_place(&path, &[good.clone(), good.clone()]).unwrap_err();
    assert!(matches!(err, UpdateError::Twice { .. }), "{err}");
    assert_eq!(std::fs::read(&path).unwrap(), original);

    // unchanged records are not written
    assert_eq!(update_in_place(&path, &[(at, data)]).unwrap(), 0);
    assert_eq!(update_in_place(&path, &[good]).unwrap(), 1);
}
