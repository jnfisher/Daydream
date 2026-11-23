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

/// Runs the dump script once over all `files`, into `out`.
fn dump(engine: &Path, files: &[PathBuf], out: &Path) {
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
        .arg(out)
        .arg(engine)
        .args(files)
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&result.stdout).trim());
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Btrieve's view of `path` (dumped into `out`) against Daydream's; every difference, as text.
fn compare(path: &Path, out: &Path) -> Vec<String> {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let out = out.join(&name);
    let mut problems = Vec::new();
    let mut file = open(path);
    let mut payloads = std::collections::HashMap::new();
    {
        let mut records = file.records();
        while let Some(record) = records.next_record() {
            let record = record.unwrap();
            payloads.insert(RecordAddress::of(&record), record.data.to_vec());
        }
    }
    let stat = std::fs::read_to_string(out.join("stat.txt")).unwrap_or_default();
    if !stat.contains(&format!("records {}\n", payloads.len())) {
        problems.push(format!(
            "{name}: Btrieve's Stat says {stat:?}, Daydream reads {} records",
            payloads.len()
        ));
    }
    for k in 0..file.keys.len() {
        let expected: Vec<&Vec<u8>> = file
            .index_entries(k)
            .unwrap_or_else(|e| panic!("{e}"))
            .iter()
            .flat_map(|e| e.records.clone())
            .map(|a| &payloads[&a])
            .collect();
        let theirs = read_dump(&out.join(format!("key{k}.bin")));
        if theirs.len() != expected.len() {
            problems.push(format!(
                "{name} key {k}: Btrieve returns {} records, Daydream {}",
                theirs.len(),
                expected.len()
            ));
        }
        let differ: Vec<usize> = theirs
            .iter()
            .zip(&expected)
            .enumerate()
            .filter(|(_, (t, e))| *t != **e)
            .map(|(i, _)| i)
            .collect();
        if let Some(&i) = differ.first() {
            let (t, e) = (&theirs[i], expected[i]);
            let at = t
                .iter()
                .zip(e.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(t.len().min(e.len()));
            let show = |v: &[u8]| format!("{:02x?}", &v[at.min(v.len())..v.len().min(at + 16)]);
            problems.push(format!(
                "{name} key {k}: {} of {} records differ; first is record {i} in key order \
                 ({} bytes from Btrieve, {} from Daydream), at byte {at}: Btrieve {} Daydream {}",
                differ.len(),
                theirs.len(),
                t.len(),
                e.len(),
                show(t),
                show(e)
            ));
        } else {
            println!("{name}: key {k}: {} records match Btrieve", theirs.len());
        }
    }
    problems
}

// One engine run over every case: each stock file as an untouched copy, and each
// fixed-length one again as a copy Daydream rewrote (a separate file, so the engine has
// nothing of it cached).
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
    let mut files = Vec::new();
    for name in names {
        let stem = name.trim_end_matches(".VIR").trim_end_matches(".vir");
        let copy = tmp.path().join(format!("{stem}.DAT"));
        std::fs::copy(dir.join(&name), &copy).unwrap();
        if !open(&copy).header.is_variable_length() {
            let rewritten = tmp.path().join(format!("{stem}_W.DAT"));
            std::fs::copy(&copy, &rewritten).unwrap();
            rewrite_non_key_bytes(&rewritten);
            files.push(rewritten);
        }
        files.push(copy);
    }
    let out = tmp.path().join("dumps");
    dump(&engine, &files, &out);
    let problems: Vec<String> = files.iter().flat_map(|f| compare(f, &out)).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Each 16-byte row of `bytes` that is not all zeros, as hex, with its offset.
fn nonzero_rows(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (i, row) in bytes.chunks(16).enumerate() {
        if row.iter().any(|&b| b != 0) {
            let hex: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
            out.push_str(&format!("    {:06x} {}\n", i * 16, hex.join(" ")));
        }
    }
    out
}

/// Has Btrieve clone the smaller stock files (empty, and with every record inserted) and
/// prints what it wrote: the FCR pages whole, and every page's first 16 bytes. This is the
/// reference the `.DAT` writer (M5) is built against.
#[test]
fn btrieve_clones_the_stock_files() {
    let Some(engine) = engine() else { return };
    let dir = stock_data::dir();
    let tmp = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.to_ascii_uppercase().ends_with(".VIR") {
            continue;
        }
        if std::fs::metadata(&path).unwrap().len() > 256 * 1024 {
            continue;
        }
        let stem = name.trim_end_matches(".VIR").trim_end_matches(".vir");
        let copy = tmp.path().join(format!("{stem}.DAT"));
        std::fs::copy(&path, &copy).unwrap();
        files.push(copy);
    }
    files.sort();
    let out = tmp.path().join("clones");
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/btrieve/btrieve-clone.ps1");
    let result = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg(&out)
        .arg(&engine)
        .args(&files)
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&result.stdout).trim());
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    for f in &files {
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        for (what, path) in [
            ("original", f.clone()),
            ("empty", out.join(format!("{stem}.EMPTY.DAT"))),
            ("full", out.join(format!("{stem}.FULL.DAT"))),
        ] {
            let bytes = std::fs::read(&path).unwrap();
            let page = usize::from(u16::from_le_bytes([bytes[8], bytes[9]]));
            let fcr_pages = if bytes[..4] == *b"FC\0\0" { 2 } else { 1 };
            println!(
                "== {stem} {what}: {} bytes, {} pages of {page}",
                bytes.len(),
                bytes.len() / page
            );
            for p in 0..fcr_pages {
                println!("  FCR page {p}:");
                print!("{}", nonzero_rows(&bytes[p * page..(p + 1) * page]));
            }
            for (p, chunk) in bytes.chunks(page).enumerate().skip(fcr_pages) {
                let hex: Vec<String> = chunk[..16].iter().map(|b| format!("{b:02x}")).collect();
                println!("  page {p}: {}", hex.join(" "));
            }
            if what != "original" {
                // Daydream must read Btrieve's own files, indexes included
                let mut file = open(&path);
                let report = file.verify_indexes().unwrap_or_else(|e| panic!("{e}"));
                println!("  indexes: {report:?}");
            }
        }
    }
}
