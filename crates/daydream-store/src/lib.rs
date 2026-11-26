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
//! - [`dat`]: writing a realm back into its Btrieve `WCC*.DAT` files.
//! - [`journal`]: what Daydream changed in a realm, record by record.
//! - [`update`]: MajorMUD's update file, kept in step with the realm's edits.
//! - [`three_way`]: an edit merged onto a record as the game has it now.
//! - [`changes`]: the realm's changes since it was read from its game, listed and taken back.
//!
//! Writes assume nothing else has the file open: stop MBBSEmu (or the realm) before
//! editing, until live writes are tested.

#![forbid(unsafe_code)]

pub mod changes;
pub mod dat;
pub mod db;
pub mod journal;
pub mod key;
pub mod merge;
pub mod realm;
pub mod three_way;
pub mod update;

pub use db::{ConvertReport, Fingerprint, Metadata, StoreError, WbtrvDb};
pub use realm::{Realm, RealmFile};
