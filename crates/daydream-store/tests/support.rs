//! Shared helpers: the Btrieve fixtures, the stock files and table dumps.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use rusqlite::types::ValueRef;

/// wbtrv32's synthetic Btrieve test files, as daydream-wcc keeps them.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../daydream-wcc/tests/fixtures/btrieve")
        .join(name)
}

/// The stock `.VIR` files (downloaded once and cached; see daydream-wcc's `stock_files`).
pub fn stock_dir() -> PathBuf {
    daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| {
        panic!(
            "the tests need the stock distribution and could not get it: {e}\n\
             (cache directory {}; set {} to use another)",
            daydream_wcc::stock_files::cache_dir().display(),
            daydream_wcc::stock_files::CACHE_ENV
        )
    })
}

/// Copies `src` into `dir` as `name`.
pub fn copy_as(src: &Path, dir: &Path, name: &str) -> PathBuf {
    let dest = dir.join(name);
    std::fs::copy(src, &dest).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
    dest
}

/// A cell as SQLite stores it: type and bytes (TEXT may hold CP437, so it stays bytes).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Null,
    Integer(i64),
    Real(f64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}

/// Every row of `sql`, every column as SQLite returns it (type included).
pub fn rows(conn: &Connection, sql: &str) -> Vec<Vec<Cell>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let n = stmt.column_count();
    stmt.query_map([], |r| {
        (0..n)
            .map(|i| {
                Ok(match r.get_ref(i)? {
                    ValueRef::Null => Cell::Null,
                    ValueRef::Integer(v) => Cell::Integer(v),
                    ValueRef::Real(v) => Cell::Real(v),
                    ValueRef::Text(t) => Cell::Text(t.to_vec()),
                    ValueRef::Blob(b) => Cell::Blob(b.to_vec()),
                })
            })
            .collect()
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}
