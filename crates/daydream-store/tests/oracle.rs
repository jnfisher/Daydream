//! Differential test against wbtrv32 itself: Daydream's conversion of a Btrieve file must be
//! the `.db` wbtrv32's own converter writes, schema, keys and every row's key columns
//! included, or MBBSEmu and legacy NMR would index Daydream's files differently.
//!
//! The converter is built by `tools/wbtrv32/build-oracle.sh` (cached by its commit), which
//! names it in `DAYDREAM_WBTRV32_CONVERTER` and its checkout in `DAYDREAM_WBTRV32_DIR`. In
//! CI (`CI` set) the oracle is required; elsewhere the test says how to get it and passes.

use std::path::{Path, PathBuf};
use std::process::Command;

use daydream_store::WbtrvDb;
use rusqlite::Connection;

use crate::support::{copy_as, fixture, rows, stock_dir};

pub(crate) struct Oracle {
    converter: PathBuf,
    checkout: PathBuf,
}

pub(crate) fn oracle() -> Option<Oracle> {
    let get = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    match (
        get("DAYDREAM_WBTRV32_CONVERTER"),
        get("DAYDREAM_WBTRV32_DIR"),
    ) {
        (Some(converter), Some(checkout)) => Some(Oracle {
            converter,
            checkout,
        }),
        _ if std::env::var_os("CI").is_some() => {
            panic!("CI needs the wbtrv32 oracle: eval \"$(tools/wbtrv32/build-oracle.sh)\"")
        }
        _ => {
            eprintln!(
                "skipping the wbtrv32 differential test: run \
                 `eval \"$(tools/wbtrv32/build-oracle.sh)\"; export DAYDREAM_WBTRV32_DIR DAYDREAM_WBTRV32_CONVERTER` first"
            );
            None
        }
    }
}

/// Converts `source` with wbtrv32 and with Daydream and compares the two databases.
pub(crate) fn compare(oracle: &Oracle, source: &Path) {
    let name = source
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_ascii_uppercase();
    let tmp = tempfile::tempdir().unwrap();
    let theirs_dir = tmp.path().join("wbtrv32");
    std::fs::create_dir(&theirs_dir).unwrap();
    let dat = copy_as(source, &theirs_dir, &format!("{name}.DAT"));
    let out = Command::new(&oracle.converter).arg(&dat).output().unwrap();
    let theirs = theirs_dir.join(format!("{name}.db"));
    assert!(
        out.status.success() && out.stderr.is_empty() && theirs.exists(),
        "wbtrv32 could not convert {}: {}",
        source.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let ours = tmp.path().join(format!("{name}.db"));
    let (_, report) = WbtrvDb::convert(&dat, &ours).unwrap_or_else(|e| panic!("{name}: {e}"));

    let a = Connection::open(&theirs).unwrap();
    let b = Connection::open(&ours).unwrap();
    for sql in [
        "SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY type, name",
        "SELECT * FROM metadata_t",
        "SELECT * FROM keys_t ORDER BY id",
        "SELECT * FROM data_t ORDER BY id",
        "SELECT size, hash FROM source_t",
    ] {
        let (x, y) = (rows(&a, sql), rows(&b, sql));
        assert_eq!(x.len(), y.len(), "{name}: `{sql}`: row counts differ");
        for (i, (rx, ry)) in x.iter().zip(&y).enumerate() {
            assert_eq!(
                rx, ry,
                "{name}: `{sql}`: row {i} differs (wbtrv32, Daydream)"
            );
        }
    }
    let journal: String = b
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(journal, "wal", "{name}");
    eprintln!(
        "{name}: {} records match wbtrv32 ({} skipped)",
        report.records, report.skipped
    );
}

#[test]
fn conversions_match_wbtrv32_on_its_own_test_files() {
    let Some(oracle) = oracle() else { return };
    let mut sources: Vec<PathBuf> = std::fs::read_dir(oracle.checkout.join("assets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dat")))
        .collect();
    sources.push(oracle.checkout.join("test/MBMGEUSR.DAT"));
    sources.sort();
    assert!(sources.len() >= 8, "wbtrv32's assets moved: {sources:?}");
    for s in sources {
        compare(&oracle, &s);
    }
    // and the copies daydream-wcc keeps
    for f in ["MBBSEMU.DAT", "MULTIACS.DAT", "VARIABLE.DAT"] {
        compare(&oracle, &fixture(f));
    }
}

#[test]
fn conversions_match_wbtrv32_on_the_stock_files() {
    let Some(oracle) = oracle() else { return };
    let dir = stock_dir();
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("vir")))
        .collect();
    sources.sort();
    assert_eq!(sources.len(), 16);
    for s in sources {
        compare(&oracle, &s);
    }
}
