//! The stock distribution for tests: downloaded once into the shared cache, checked against
//! its pinned sha256 and unpacked ([`daydream_wcc::stock_files`]). With no cached copy and no
//! network a test fails here with a message saying what to fetch; it never skips.

use std::path::PathBuf;
use std::sync::OnceLock;

/// The unpacked `.VIR` files and `WCCUPDAT.DAT`.
pub fn dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| {
            panic!(
                "the tests need the stock distribution and could not get it: {e}\n\
                 (cache directory {}; set {} to use another)",
                daydream_wcc::stock_files::cache_dir().display(),
                daydream_wcc::stock_files::CACHE_ENV
            )
        })
    })
    .clone()
}

/// The `.VIR` files (the same directory: only they and `WCCUPDAT.DAT` are unpacked).
pub fn vir_dir() -> PathBuf {
    dir()
}

/// One file of the unpacked package, by name (`WCCMP001.VIR`).
pub fn file(name: &str) -> PathBuf {
    let path = dir().join(name);
    assert!(path.is_file(), "the stock distribution has no {name}");
    path
}
