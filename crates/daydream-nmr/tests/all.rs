//! NMR `.mdb` export and import.
//!
//! - `synthetic`: realms of random records, every NMR table: export, import back, edit.
//! - `stock`: the stock 1.11p world round-trips through NMR's tables (needs the stock
//!   distribution; see daydream-wcc's tests).
//! - `windows`: the same through a real `.mdb` written by Windows' Jet (Windows only).

mod support;

mod mdb_read;
mod stock;
mod synthetic;
mod windows;
