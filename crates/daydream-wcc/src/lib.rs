//! MajorMUD 1.11p WCC data: the Btrieve reader and the record decoders and encoders.
//!
//! Copied from silvermere's `silvermere-import` crate (see `UPSTREAM.md` at the repository
//! root for the commit and what was left out). The modules are kept as close to upstream as
//! possible so fixes can be carried across in either direction.
//!
//! - [`btrieve`]: reads Btrieve 5.x/6.x `.DAT` and virgin `.VIR` files.
//! - [`decode`]: one flat struct per record type, member for member with `unk_*` spans,
//!   with `decode`, `encode` and `to_json`.
//! - [`json`]: the decoded records as JSON objects.
//! - [`stock`], [`stock_files`] and [`zip`]: the stock 1.11p distribution, downloaded on
//!   demand, checked and unpacked.
//! - [`tables`]: which WCC file holds which record type.

#![forbid(unsafe_code)]

pub mod btrieve;
pub mod decode;
pub mod dos_time;
pub mod json;
pub mod stock;
pub mod stock_files;
pub mod tables;
pub mod zip;
