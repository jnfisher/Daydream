//! Daydream's realm store: wbtrv32's SQLite `.db` files.
//!
//! A realm is a directory of `.db` files, one per WCC Btrieve file, in the format of
//! Paladine's wbtrv32 (the `WBTRV32.DLL` replacement MBBSEmu uses): each record's raw
//! bytes plus its Btrieve key columns. That is lossless by construction, it is what
//! MBBSEmu-hosted realms already run on, and legacy NMR can open it through the same DLL.
//!
//! - [`db`]: one `.db`: convert a `.DAT`/`.VIR` into it, read, insert, update, delete.
//! - [`key`]: wbtrv32's key columns, ported so Daydream's writes index exactly as wbtrv32's.
//! - [`realm`]: a realm directory and its tables.
//!
//! Writes assume nothing else has the file open: stop MBBSEmu (or the realm) before
//! editing, until live writes are tested.

#![forbid(unsafe_code)]

pub mod db;
pub mod key;
pub mod realm;

pub use db::{ConvertReport, Fingerprint, Metadata, StoreError, WbtrvDb};
pub use realm::{Realm, RealmFile};
