//! Writes the stock 1.11p world as an NMR `.mdb` through Windows' Jet, for driving NMR's own
//! Database Importer with it (`tools/nmr/nmr-import.ps1`):
//!
//! ```text
//! cargo run -p daydream-nmr --example stock_mdb -- OUT.mdb
//! ```

use daydream_nmr::{ExportOptions, export, jet};
use daydream_store::Realm;

fn main() {
    let out = std::env::args_os()
        .nth(1)
        .expect("usage: stock_mdb OUT.mdb");
    let vir = daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| panic!("{e}"));
    let dir = tempfile::tempdir().unwrap();
    daydream_store::realm::convert_dir(&vir, dir.path()).unwrap();
    let realm = Realm::open(dir.path()).unwrap();
    let opts = ExportOptions {
        zero_runtime: false,
        custom: "Daydream".into(),
        date: "11/22/2026".into(),
        time: "7:50:00 PM".into(),
        ..ExportOptions::default()
    };
    let data = export(&realm, &opts).unwrap();
    jet::write_mdb(&data, std::path::Path::new(&out)).unwrap();
}
