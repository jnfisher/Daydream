//! NMR `.mdb` export and import.
//!
//! - `synthetic`: realms of random records, every NMR table: export, import back, edit.
//! - `stock`: the stock 1.11p world round-trips through NMR's tables (needs the stock
//!   distribution; see daydream-wcc's tests).
//! - `windows`: the same through a real `.mdb` written by Windows' Jet (Windows only).
//! - `options`: the importer's options (tables, ranges, Skip, a room's items, text block parts).
//! - `cases`: the batch of edits and new records the NMR job imports, through Daydream alone.
//! - `windows_nmr`: real Nightmare Redux imports that batch exactly as Daydream does
//!   (Windows only, with NMR set up as the `windows` workflow does).

mod support;

mod cases;
mod mdb_read;
mod modify;
mod options;
mod presets;
mod stock;
mod synthetic;
mod windows;
mod windows_nmr;
