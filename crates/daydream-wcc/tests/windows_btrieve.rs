//! Daydream's Btrieve reader, index walk and in-place writer against a real Btrieve engine.
//!
//! On Windows, with `DAYDREAM_BTRIEVE_DIR` naming a directory holding `WBTRV32.DLL` and the
//! 6.15 workstation engine `W32MKDE.EXE` (Nightmare Redux ships both; the `windows` workflow
//! checks out a pinned copy), every stock file is copied as a `.DAT`, every fixed-length one
//! has all its non-key bytes rewritten by `update_in_place`, and `tools/btrieve/
//! btrieve-dump.ps1` reads each back through Btrieve, key by key. Btrieve must return the
//! records Daydream expects, in the order Daydream's index walk gives. Elsewhere the test
//! does nothing; in CI on Windows the engine is required.

use std::path::{Path, PathBuf};

use daydream_wcc::btrieve::update::update_in_place;
use daydream_wcc::btrieve::{BtrieveFile, OpenOptions, RecordAddress};

use crate::stock as stock_data;

fn engine() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    match std::env::var_os("DAYDREAM_BTRIEVE_DIR").filter(|v| !v.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None if std::env::var_os("CI").is_some() => {
            panic!("CI on Windows needs DAYDREAM_BTRIEVE_DIR (WBTRV32.DLL and W32MKDE.EXE)")
        }
        None => {
            eprintln!("skipping the Btrieve engine test: DAYDREAM_BTRIEVE_DIR is not set");
            None
        }
    }
}

fn open(path: &Path) -> BtrieveFile {
    OpenOptions::new()
        .variable_length(true)
        .open(path)
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Flips every byte outside the keys of every record, in place.
fn rewrite_non_key_bytes(path: &Path) {
    let mut file = open(path);
    let mut is_key = vec![false; usize::from(file.header.record_length)];
    for segment in file.keys.iter().flat_map(|k| &k.segments) {
        let at = usize::from(segment.offset);
        is_key[at..at + usize::from(segment.length)].fill(true);
    }
    let mut updates = Vec::new();
    let mut records = file.records();
    while let Some(record) = records.next_record() {
        let record = record.unwrap();
        let mut data = record.data.to_vec();
        for (b, key) in data.iter_mut().zip(&is_key) {
            if !key {
                *b ^= 0x5A;
            }
        }
        updates.push((RecordAddress::of(&record), data));
    }
    drop(file);
    update_in_place(path, &updates).unwrap_or_else(|e| panic!("{e}"));
}

/// The records of one key's dump: each a little-endian u32 length and the bytes.
fn read_dump(path: &Path) -> Vec<Vec<u8>> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        out.push(bytes[at + 4..at + 4 + len].to_vec());
        at += 4 + len;
    }
    out
}

/// Runs the dump script on `file` into `out`.
fn dump(engine: &Path, file: &Path, out: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/btrieve/btrieve-dump.ps1");
    let result = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg(file)
        .arg(out)
        .arg(engine)
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&result.stdout).trim());
    assert!(
        result.status.success(),
        "{}: {}{}",
        file.display(),
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Checks Btrieve's view of `path` against Daydream's.
fn compare(engine: &Path, path: &Path) {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let out = tempfile::tempdir().unwrap();
    dump(engine, path, out.path());

    let mut file = open(path);
    let mut payloads = std::collections::HashMap::new();
    {
        let mut records = file.records();
        while let Some(record) = records.next_record() {
            let record = record.unwrap();
            payloads.insert(RecordAddress::of(&record), record.data.to_vec());
        }
    }
    let stat = std::fs::read_to_string(out.path().join("stat.txt")).unwrap();
    assert!(
        stat.contains(&format!("records {}\n", payloads.len())),
        "{name}: Btrieve's Stat says {stat:?}, Daydream reads {} records",
        payloads.len()
    );
    for k in 0..file.keys.len() {
        let expected: Vec<&Vec<u8>> = file
            .index_entries(k)
            .unwrap_or_else(|e| panic!("{e}"))
            .iter()
            .flat_map(|e| e.records.clone())
            .map(|a| &payloads[&a])
            .collect();
        let theirs = read_dump(&out.path().join(format!("key{k}.bin")));
        assert_eq!(theirs.len(), expected.len(), "{name} key {k}: record count");
        for (i, (t, e)) in theirs.iter().zip(&expected).enumerate() {
            assert!(
                t == *e,
                "{name} key {k}: record {i} in key order differs (Btrieve {} bytes, Daydream {})",
                t.len(),
                e.len()
            );
        }
        println!("{name}: key {k}: {} records match Btrieve", theirs.len());
    }
}

#[test]
fn btrieve_reads_the_stock_files_as_daydream_does_and_writes_them() {
    let Some(engine) = engine() else { return };
    let dir = stock_data::dir();
    let tmp = tempfile::tempdir().unwrap();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_uppercase().ends_with(".VIR"))
        .collect();
    names.sort();
    for name in names {
        // as untouched copies first, then (fixed-length files) after Daydream's rewrite
        let stem = name.trim_end_matches(".VIR").trim_end_matches(".vir");
        let copy = tmp.path().join(format!("{stem}.DAT"));
        std::fs::copy(dir.join(&name), &copy).unwrap();
        compare(&engine, &copy);
        if !open(&copy).header.is_variable_length() {
            rewrite_non_key_bytes(&copy);
            compare(&engine, &copy);
        }
    }
}
