//! Nightmare Redux's `.mdb` export format, the path between Daydream and legacy NMR.
//!
//! NMR exports a realm's world tables to an Access `.mdb` and imports such a file back
//! into a realm's `WCC*.DAT` files. Daydream does both with the same tables and columns,
//! so a realm can move between the two editors, and back to Btrieve files through NMR's
//! import while Daydream's own `.DAT` writer matures.
//!
//! - [`schema`] and the generated [`columns`]: NMR's tables, and where each column's value
//!   lives in a record, taken from NMR's own source by `tools/nmr/generate.sh`.
//! - [`export`]: a realm to [`NmrData`]; [`import`]: [`NmrData`] into a realm.
//! - [`mdb`]: reading a `.mdb` (pure Rust, any OS).
//! - [`jet`]: writing a `.mdb` through Windows' Jet engine, as NMR does.

#![forbid(unsafe_code)]

#[rustfmt::skip]
pub mod columns;
pub mod data;
pub mod export;
pub mod import;
pub mod jet;
pub mod map;
pub mod mdb;
pub mod modify;
pub mod presets;
pub mod renumber;
pub mod reports;
pub mod schema;
pub mod text;

pub use data::{NmrData, NmrRows};
pub use export::{ExportOptions, export, export_table, layout_of_realm};
pub use import::{ImportOptions, Range, RoomParts, TableReport, delete, delete_range, import};
pub use schema::Value;
