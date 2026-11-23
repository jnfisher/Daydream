//! The stock distribution's Btrieve files, unpacked once into a cache shared by every
//! checkout: `DAYDREAM_STOCK_CACHE`, else `$XDG_CACHE_HOME/daydream/stock`, else
//! `~/.cache/daydream/stock`. The package itself comes from [`crate::stock`] and is checked
//! against its pinned sha256.

use std::path::{Path, PathBuf};

use crate::stock::{self, FetchError, SHA256};
use crate::zip::Archive;

/// Overrides the cache directory.
pub const CACHE_ENV: &str = "DAYDREAM_STOCK_CACHE";

/// The cache directory (see the module docs).
pub fn cache_dir() -> PathBuf {
    let set = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    set(CACHE_ENV)
        .or_else(|| set("XDG_CACHE_HOME").map(|d| d.join("daydream/stock")))
        .or_else(|| set("HOME").map(|d| d.join(".cache/daydream/stock")))
        .or_else(|| set("USERPROFILE").map(|d| d.join(".cache/daydream/stock")))
        .unwrap_or_else(|| std::env::temp_dir().join("daydream-stock"))
}

/// What getting the stock files can fail on.
#[derive(Debug, thiserror::Error)]
pub enum StockError {
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error("unpacking {}: {reason}", zip.display())]
    Unpack { zip: PathBuf, reason: String },
}

/// The directory holding the package's `WCC*.VIR` files and `WCCUPDAT.DAT`, flat, unpacked
/// from `zip` (or from the cached package, downloaded when missing, if `zip` is `None`).
/// Read only: copy what you change.
pub fn vir_dir(zip: Option<&Path>) -> Result<PathBuf, StockError> {
    let root = cache_dir();
    let zip = match zip {
        Some(z) => z.to_path_buf(),
        None => stock::cached(&root, false)?.0,
    };
    let dir = root.join(format!("vir-1-{}", &SHA256[..12]));
    let done = dir.join(".complete");
    if done.exists() {
        return Ok(dir);
    }
    let unpack_err = |reason: String| StockError::Unpack {
        zip: zip.clone(),
        reason,
    };
    std::fs::create_dir_all(&root).map_err(|e| unpack_err(e.to_string()))?;
    let lock_path = root.join(".unpack.lock");
    let lock = std::fs::File::create(&lock_path).map_err(|e| unpack_err(e.to_string()))?;
    lock.lock().map_err(|e| unpack_err(e.to_string()))?;
    if !done.exists() {
        let bytes = std::fs::read(&zip).map_err(|e| unpack_err(e.to_string()))?;
        if !stock::verify(&bytes) {
            return Err(unpack_err(format!(
                "not the stock package (sha256 {SHA256})"
            )));
        }
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| unpack_err(e.to_string()))?;
        let archive =
            Archive::new(stock::FILE_NAME, &bytes).map_err(|e| unpack_err(e.to_string()))?;
        for e in archive.entries().iter().filter(|e| !e.is_dir()) {
            let name = e.file_name().to_ascii_uppercase();
            if name.ends_with(".VIR") || name == "WCCUPDAT.DAT" {
                let data = archive.read(e).map_err(|e| unpack_err(e.to_string()))?;
                std::fs::write(dir.join(&name), data).map_err(|e| unpack_err(e.to_string()))?;
            }
        }
        std::fs::write(&done, b"").map_err(|e| unpack_err(e.to_string()))?;
    }
    Ok(dir)
}
