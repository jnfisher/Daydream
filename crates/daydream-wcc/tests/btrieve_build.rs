//! The `.DAT` builder: a rebuilt file reads back with the same records in the same order,
//! and every index checks out against them and holds what the source's index holds.

use std::path::{Path, PathBuf};

use daydream_wcc::btrieve::build::{BuildError, build};
use daydream_wcc::btrieve::{BtrieveFile, KeyDataType, OpenOptions, RecordAddress, Version};

use crate::stock as stock_data;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/btrieve")
        .join(name)
}

fn records(file: &mut BtrieveFile) -> Vec<(RecordAddress, Vec<u8>)> {
    let mut out = Vec::new();
    let mut cursor = file.records();
    while let Some(record) = cursor.next_record() {
        let record = record.unwrap_or_else(|e| panic!("{e}"));
        out.push((RecordAddress::of(&record), record.data.to_vec()));
    }
    out
}

/// Each key's index as the records' positions in `records`: one set per entry.
fn index_by_position(
    file: &mut BtrieveFile,
    records: &[(RecordAddress, Vec<u8>)],
) -> Vec<Vec<Vec<usize>>> {
    let position = |a: &RecordAddress| records.iter().position(|(r, _)| r == a).unwrap();
    (0..file.keys.len())
        .map(|k| {
            file.index_entries(k)
                .unwrap_or_else(|e| panic!("{e}"))
                .iter()
                .map(|e| {
                    let mut p: Vec<usize> = e.records.iter().map(position).collect();
                    p.sort();
                    p
                })
                .collect()
        })
        .collect()
}

/// Builds `data` with `source`'s layout and checks the result; returns it opened.
fn rebuilt(source: &BtrieveFile, data: &[Vec<u8>]) -> BtrieveFile {
    let bytes = build(source, data, 0x6B04_459B).unwrap_or_else(|e| panic!("{e}"));
    let mut file = OpenOptions::new()
        .open_bytes("rebuilt.dat", bytes)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(file.header.version, Version::V6);
    assert_eq!(file.header.page_length, source.header.page_length);
    assert_eq!(file.header.record_length, source.header.record_length);
    assert_eq!(file.header.record_count as usize, data.len());
    assert_eq!(file.keys, source.keys);
    let read: Vec<Vec<u8>> = records(&mut file).into_iter().map(|(_, d)| d).collect();
    assert!(read == data, "the records read back differ");
    file.verify_indexes().unwrap_or_else(|e| panic!("{e}"));
    file
}

/// Rebuilds a file from its own records: the same records in the same order, and each
/// index holds the same records under each value as the source's.
fn rebuild(path: &Path) {
    let mut source = OpenOptions::new()
        .open(path)
        .unwrap_or_else(|e| panic!("{e}"));
    let before = records(&mut source);
    let data: Vec<Vec<u8>> = before.iter().map(|(_, d)| d.clone()).collect();
    let mut file = rebuilt(&source, &data);
    let after = records(&mut file);
    assert_eq!(
        index_by_position(&mut file, &after),
        index_by_position(&mut source, &before),
        "{}",
        path.display()
    );
}

/// Records for `source`'s keys: every segment written from a number, unique keys from a
/// shuffle of `0..n` and keys with duplicates from a few values.
fn synthetic(source: &BtrieveFile, n: usize) -> Vec<Vec<u8>> {
    let len = usize::from(source.header.record_length);
    (0..n)
        .map(|i| {
            let mut r: Vec<u8> = (0..len).map(|b| (b * 7 + i) as u8).collect();
            for key in &source.keys {
                let v = if key.allows_duplicates() {
                    (i * 31) % 17
                } else {
                    (i * 7919) % n
                };
                for s in &key.segments {
                    let at = usize::from(s.offset);
                    let field = &mut r[at..at + usize::from(s.length)];
                    match s.data_type {
                        KeyDataType::Zstring | KeyDataType::String | KeyDataType::OldAscii => {
                            let text = format!("{v:0w$}", w = field.len());
                            let digits = &text.as_bytes()[text.len() + 1 - field.len()..];
                            field[..digits.len()].copy_from_slice(digits);
                            field[field.len() - 1] = 0;
                        }
                        _ => {
                            field.fill(0);
                            let b = (v as u64).to_le_bytes();
                            let k = field.len().min(8);
                            field[..k].copy_from_slice(&b[..k]);
                        }
                    }
                }
            }
            r
        })
        .collect()
}

#[test]
fn a_rebuilt_fixture_holds_the_same_records_and_indexes() {
    rebuild(&fixture("MBBSEMU.DAT"));
}

#[test]
fn every_stock_file_rebuilds() {
    let dir = stock_data::dir();
    let mut names: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("vir")))
        .collect();
    names.sort();
    for path in names {
        let source = OpenOptions::new()
            .variable_length(true)
            .open(&path)
            .unwrap();
        if source.header.is_variable_length() {
            continue; // WCCTEXT: not yet
        }
        drop(source);
        rebuild(&path);
    }
}

#[test]
fn large_files_get_deep_trees_and_more_page_tables() {
    let source = BtrieveFile::open(&fixture("MBBSEMU.DAT")).unwrap();
    for n in [0, 1, 5, 6, 400, 3000] {
        let data = synthetic(&source, n);
        let mut file = rebuilt(&source, &data);
        let report = file.verify_indexes().unwrap();
        if n == 3000 {
            assert!(report.keys.iter().any(|k| k.depth >= 3), "{report:?}");
            // 512-byte pages: a page table maps 126 pages
            assert!(file.header.page_count > 2 * 126);
        }
    }
}

#[test]
fn a_unique_key_with_two_records_is_refused() {
    let source = BtrieveFile::open(&fixture("MBBSEMU.DAT")).unwrap();
    let mut data = synthetic(&source, 3);
    let unique = source
        .keys
        .iter()
        .find(|k| !k.allows_duplicates())
        .expect("a unique key");
    let s = &unique.segments[0];
    let at = usize::from(s.offset)..usize::from(s.offset + s.length);
    let value = data[0][at.clone()].to_vec();
    data[2][at].copy_from_slice(&value);
    assert!(matches!(
        build(&source, &data, 0),
        Err(BuildError::Duplicate {
            first: 0,
            second: 2,
            ..
        })
    ));
    data[1].pop();
    assert!(matches!(
        build(&source, &data, 0),
        Err(BuildError::Length { index: 1, .. })
    ));
}
