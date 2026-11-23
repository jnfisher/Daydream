//! Daydream's Btrieve reader, index walk and writers against a real Btrieve engine.
//!
//! On Windows, with `DAYDREAM_BTRIEVE_DIR` naming a directory holding `WBTRV32.DLL` and the
//! 6.15 workstation engine `W32MKDE.EXE` (Nightmare Redux ships both; the `windows` workflow
//! checks out a pinned copy), every stock file is copied as a `.DAT`; every fixed-length one
//! also has all its non-key bytes rewritten by `update_in_place`, and is rebuilt whole from
//! its records by the `.DAT` builder. `tools/btrieve/btrieve-dump.ps1` reads each back
//! through Btrieve, key by key: Btrieve must return the records Daydream expects, in the
//! order Daydream's index walk gives. Btrieve then deletes and inserts records in a copy of
//! each rebuilt file (`btrieve-churn.ps1`), and the result must still check out, in
//! Daydream and in Btrieve. Elsewhere the tests do nothing; in CI on Windows the engine is
//! required.

use std::path::{Path, PathBuf};

use daydream_wcc::btrieve::build::build;
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

/// Every record's payload, in the reader's order.
fn payloads(path: &Path) -> Vec<Vec<u8>> {
    let mut file = open(path);
    let mut out = Vec::new();
    let mut records = file.records();
    while let Some(record) = records.next_record() {
        out.push(record.unwrap().data.to_vec());
    }
    out
}

/// Runs a script of `tools/btrieve` once, with `args` and then `files`.
fn script(name: &str, args: &[&Path], files: &[PathBuf]) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/btrieve")
        .join(name);
    let result = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .args(args)
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
    if let Ok(error) = std::fs::read_to_string(out.join("error.txt")) {
        return vec![format!("{name}: Btrieve failed: {}", error.trim())];
    }
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

/// Runs the dump script once over all `files`, into `out`.
fn dump(engine: &Path, files: &[PathBuf], out: &Path) {
    script("btrieve-dump.ps1", &[out, engine], files);
}

// One engine run over every case: each stock file as an untouched copy, and each
// fixed-length one again as a copy Daydream rewrote and as a file Daydream rebuilt (each a
// separate file, so the engine has nothing of it cached). Then Btrieve churns a copy of
// each rebuilt file, and a second run reads those.
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
    let mut churned = Vec::new();
    for name in names {
        let stem = name.trim_end_matches(".VIR").trim_end_matches(".vir");
        let copy = tmp.path().join(format!("{stem}.DAT"));
        std::fs::copy(dir.join(&name), &copy).unwrap();
        if !open(&copy).header.is_variable_length() {
            let rewritten = tmp.path().join(format!("{stem}_W.DAT"));
            std::fs::copy(&copy, &rewritten).unwrap();
            rewrite_non_key_bytes(&rewritten);
            files.push(rewritten);
            let bytes = build(&open(&copy), &payloads(&copy), 0x6B04_459B)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            for (suffix, list) in [("R", &mut files), ("C", &mut churned)] {
                let rebuilt = tmp.path().join(format!("{stem}_{suffix}.DAT"));
                std::fs::write(&rebuilt, &bytes).unwrap();
                list.push(rebuilt);
            }
        }
        files.push(copy);
    }
    let out = tmp.path().join("dumps");
    dump(&engine, &files, &out);
    let mut problems: Vec<String> = files.iter().flat_map(|f| compare(f, &out)).collect();

    // Btrieve deletes and inserts in the rebuilt files: they keep their records, and
    // Daydream and Btrieve still agree on them
    script("btrieve-churn.ps1", &[&engine], &churned);
    for f in &churned {
        let rebuilt = tmp.path().join(
            f.file_name()
                .unwrap()
                .to_string_lossy()
                .replace("_C.DAT", "_R.DAT"),
        );
        let error = PathBuf::from(format!("{}.error.txt", f.display()));
        if let Ok(error) = std::fs::read_to_string(&error) {
            problems.push(format!("{}: Btrieve failed: {}", f.display(), error.trim()));
            continue;
        }
        let (mut before, mut after) = (payloads(&rebuilt), payloads(f));
        before.sort();
        after.sort();
        if before != after {
            problems.push(format!("{}: the records changed", f.display()));
        }
        if let Err(e) = open(f).verify_indexes() {
            problems.push(format!("after Btrieve's deletes and inserts: {e}"));
        }
    }
    let out = tmp.path().join("churned");
    dump(&engine, &churned, &out);
    problems.extend(churned.iter().flat_map(|f| compare(f, &out)));
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
        // the small files; the message file, whose 512-byte pages need more than 30 page
        // tables filled (past which Btrieve leaves a 4096-byte gap); and the text file, the
        // variable-length one
        let big = std::fs::metadata(&path).unwrap().len() > 256 * 1024;
        if big
            && !["WCCMSG.VIR", "WCCTEXT.VIR"]
                .iter()
                .any(|n| name.eq_ignore_ascii_case(n))
        {
            continue;
        }
        let stem = name.trim_end_matches(".VIR").trim_end_matches(".vir");
        let copy = tmp.path().join(format!("{stem}.DAT"));
        std::fs::copy(&path, &copy).unwrap();
        files.push(copy);
    }
    files.sort();
    let out = tmp.path().join("clones");
    script("btrieve-clone.ps1", &[&out, &engine], &files);
    let mut unread = Vec::new();
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
            // of a large file, the first two pages of each kind whole (by the page id's
            // type byte: index, data, variable)
            let mut shown = std::collections::HashMap::new();
            for (p, chunk) in bytes.chunks(page).enumerate().skip(fcr_pages) {
                let seen = shown.entry(chunk[1]).or_insert(0);
                *seen += 1;
                if bytes.len() <= 16 * 1024 || (p > 3 && *seen <= 2) {
                    // small files whole, so every byte of Btrieve's layout is on record
                    println!("  page {p}:");
                    print!("{}", nonzero_rows(chunk));
                } else if bytes.len() <= 256 * 1024 {
                    let hex: Vec<String> = chunk[..16].iter().map(|b| format!("{b:02x}")).collect();
                    println!("  page {p}: {}", hex.join(" "));
                }
            }
            if what != "original" {
                // Daydream must read Btrieve's own files, indexes included
                match OpenOptions::new()
                    .variable_length(true)
                    .open(&path)
                    .and_then(|mut file| file.verify_indexes())
                {
                    Ok(report) => println!("  indexes: {report:?}"),
                    Err(e) => {
                        println!("  Daydream fails: {e}");
                        unread.push(format!("{stem} {what}: {e}"));
                    }
                }
            }
        }
    }
    // the .DAT builder writes an empty file as Btrieve's Create does, byte for byte but
    // for the creation time (taken from Btrieve's) and Btrieve's running counter at +0x68
    let mut differ = Vec::new();
    for f in &files {
        let source = open(f);
        if source.header.is_variable_length() {
            continue;
        }
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let theirs = std::fs::read(out.join(format!("{stem}.EMPTY.DAT"))).unwrap();
        let created = u32::from_le_bytes(theirs[0x56..0x5A].try_into().unwrap());
        let ours = build(&source, &Vec::<Vec<u8>>::new(), created)
            .unwrap_or_else(|e| panic!("{stem}: {e}"));
        let bytes: Vec<usize> = (0..ours.len().max(theirs.len()))
            .filter(|&i| ours.get(i) != theirs.get(i) && !(0x68..0x6A).contains(&i))
            .collect();
        if bytes.is_empty() {
            println!("{stem}: the builder's empty file is Btrieve's");
        } else {
            differ.push(format!(
                "{stem}: {} bytes against Btrieve's {}, {} differ, first at {:#x}",
                ours.len(),
                theirs.len(),
                bytes.len(),
                bytes[0]
            ));
        }
    }
    differ.extend(unread);
    assert!(differ.is_empty(), "{}", differ.join("\n"));
}
