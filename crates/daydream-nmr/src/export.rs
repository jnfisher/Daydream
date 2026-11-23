//! A realm's records as NMR's Access export writes them.

use std::collections::HashMap;

use daydream_store::{Realm, StoreError, WbtrvDb};
use daydream_types::records::LayoutSet;

use crate::columns::TABLES;
use crate::data::{NmrData, NmrRows};
use crate::schema::{NmrTable, Source, Value, gather, layout_of, read_unsigned};
use crate::text;

/// NMR's version strings, as its `Info` table carries them. NMR's import warns when they
/// differ from its own, so Daydream writes those of the NMR it targets.
pub const NMR_VERSION: &str = "v1.8.3";
/// `FriendlyDatVersion` of 1.11p.
pub const DAT_VERSION: &str = "v1.11p";

/// What the export writes.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// NMR's "zero user interaction" option, on by default there: room cash and items on
    /// the floor, shop stock and monster kill state are written as 0, so the file carries
    /// the world's design and not a running game's state. Hidden items of type 3 stay.
    pub zero_runtime: bool,
    /// The `Info` row's Custom text.
    pub custom: String,
    /// The `Info` row's Date and Time, as VB's `Date` and `Time` print them.
    pub date: String,
    pub time: String,
    /// The realm's record layouts.
    pub layout: LayoutSet,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            zero_runtime: true,
            custom: String::new(),
            date: String::new(),
            time: String::new(),
            layout: LayoutSet::Dos,
        }
    }
}

/// Why an export failed.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("{stem} record {id}: {len} bytes, expected {expected}")]
    Length {
        stem: String,
        id: i64,
        len: usize,
        expected: usize,
    },
}

/// The row of one record.
pub fn row_of(t: &NmrTable, layout: &daydream_types::records::Layout, record: &[u8]) -> Vec<Value> {
    t.columns
        .iter()
        .map(|c| match &c.source {
            Source::Element { vb, pieces } => vb.read(&gather(layout, pieces, record)),
            Source::Unsigned(pieces) => read_unsigned(&gather(layout, pieces, record)),
            Source::Zero => Value::Long(0),
            Source::TextblockData => {
                let f = layout.field("text").expect("textblk has text");
                Value::Text(text::decrypt_textblock(
                    &record[f.offset..f.offset + f.size],
                ))
            }
            Source::Info => Value::Null,
        })
        .collect()
}

/// A record padded to the layout's length (a WCCTEXT record is variable-length).
fn full_record(
    stem: &str,
    id: i64,
    mut data: Vec<u8>,
    len: usize,
    variable: bool,
) -> Result<Vec<u8>, ExportError> {
    if data.len() == len || (variable && data.len() < len) {
        data.resize(len, 0);
        Ok(data)
    } else {
        Err(ExportError::Length {
            stem: stem.to_string(),
            id,
            len: data.len(),
            expected: len,
        })
    }
}

/// Every table of the realm as NMR exports it, all records, rows in index order.
pub fn export(realm: &Realm, opts: &ExportOptions) -> Result<NmrData, ExportError> {
    let mut data = NmrData::default();
    for t in TABLES {
        let rows = match t.table {
            Some(_) => export_table(realm, t, opts.layout)?,
            None => NmrRows {
                name: t.name.to_string(),
                columns: t.columns.iter().map(|c| c.name.to_string()).collect(),
                rows: vec![vec![
                    Value::Text(NMR_VERSION.to_string()),
                    Value::Text(DAT_VERSION.to_string()),
                    Value::Text(opts.date.clone()),
                    Value::Text(opts.time.clone()),
                    Value::Text(opts.custom.clone()),
                ]],
            },
        };
        data.tables.push(rows);
    }
    if opts.zero_runtime {
        zero_runtime(&mut data);
    }
    Ok(data)
}

/// One world table of the realm as NMR exports it (no runtime zeroing), rows in index
/// order. The `Info` table has no records and comes back empty.
pub fn export_table(
    realm: &Realm,
    t: &NmrTable,
    layout: LayoutSet,
) -> Result<NmrRows, ExportError> {
    let mut rows = Vec::new();
    if let Some(table) = t.table {
        let layout = layout_of(table, layout);
        for file in realm.files_of(table) {
            let db = WbtrvDb::open_read_only(&file.path)?;
            for (id, raw) in db.records()? {
                let record =
                    full_record(&file.stem, id, raw, layout.len, table.is_variable_length())?;
                rows.push(row_of(t, layout, &record));
            }
        }
        sort_by_index(t, &mut rows);
    }
    Ok(NmrRows {
        name: t.name.to_string(),
        columns: t.columns.iter().map(|c| c.name.to_string()).collect(),
        rows,
    })
}

/// The record layouts of a realm's files: Worldgroup 3 when it holds any Worldgroup file
/// (`WCCITEM2`, ...), else DOS.
pub fn layout_of_realm(realm: &Realm) -> LayoutSet {
    let wg = realm.files().iter().any(|f| {
        daydream_wcc::tables::WORLDGROUP_FILES
            .iter()
            .any(|w| w.stem == f.stem)
    });
    if wg {
        LayoutSet::Worldgroup
    } else {
        LayoutSet::Dos
    }
}

/// The rows of a table sorted by its index columns, as NMR walks the Btrieve file.
pub(crate) fn sort_by_index(t: &NmrTable, rows: &mut [Vec<Value>]) {
    let Some(index) = &t.index else { return };
    let at: Vec<usize> = index
        .columns
        .iter()
        .map(|c| t.columns.iter().position(|col| col.name == *c).unwrap())
        .collect();
    rows.sort_by(|a, b| {
        for &i in &at {
            let o = match (&a[i], &b[i]) {
                (Value::Long(x), Value::Long(y)) => x.cmp(y),
                (Value::Text(x), Value::Text(y)) => key_text(x).cmp(&key_text(y)),
                _ => std::cmp::Ordering::Equal,
            };
            if o.is_ne() {
                return o;
            }
        }
        std::cmp::Ordering::Equal
    });
}

/// A text key as compared: up to its first NUL, without trailing spaces, any case.
pub fn key_text(s: &str) -> String {
    let s = s.split('\0').next().unwrap_or("");
    s.trim_end().to_ascii_lowercase()
}

/// NMR's "zero user interaction" export.
fn zero_runtime(data: &mut NmrData) {
    let item_types: HashMap<i32, i32> = data
        .table("Items")
        .map(|t| {
            let (n, ty) = (t.position("Number").unwrap(), t.position("Type").unwrap());
            t.rows
                .iter()
                .filter_map(|r| match (&r[n], &r[ty]) {
                    (Value::Long(n), Value::Long(t)) => Some((*n, *t)),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let zero = |t: &mut NmrRows, names: &[String]| {
        let at: Vec<usize> = names.iter().filter_map(|n| t.position(n)).collect();
        for row in &mut t.rows {
            for &i in &at {
                row[i] = Value::Long(0);
            }
        }
    };
    if let Some(t) = data.table_mut("Rooms") {
        let mut names: Vec<String> = [
            "Runic",
            "Platinum",
            "Gold",
            "Silver",
            "Copper",
            "InvisRunic",
            "InvisPlatinum",
            "InvisGold",
            "InvisSilver",
            "InvisCopper",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for x in 0..17 {
            names.push(format!("Room Item {x}"));
            names.push(format!("Room Item {x} QTY"));
            names.push(format!("Room Item {x} USES"));
        }
        zero(t, &names);
        // hidden items stay only when they are of type 3 (GetItemTypeNumber = 3)
        for x in 0..15 {
            let cols: Vec<usize> = ["", " QTY", " USES"]
                .iter()
                .filter_map(|s| t.position(&format!("Hidden Item {x}{s}")))
                .collect();
            let item = cols[0];
            for row in &mut t.rows {
                let keep = matches!(&row[item], Value::Long(n) if *n > 0
                    && item_types.get(n) == Some(&3));
                if !keep {
                    for &i in &cols {
                        row[i] = Value::Long(0);
                    }
                }
            }
        }
    }
    if let Some(t) = data.table_mut("Shops") {
        // stock on hand: the regen number when the slot regenerates, else 0
        for x in 0..20 {
            let (Some(normal), Some(number), Some(pct)) = (
                t.position(&format!("Normal {x}")),
                t.position(&format!("Regen Number{x}")),
                t.position(&format!("Regen %{x}")),
            ) else {
                continue;
            };
            for row in &mut t.rows {
                let regen = matches!(row[pct], Value::Long(p) if p > 0);
                row[normal] = if regen {
                    row[number].clone()
                } else {
                    Value::Long(0)
                };
            }
        }
    }
    if let Some(t) = data.table_mut("Monsters") {
        let names = ["Active", "Date Killed", "Time Killed"].map(String::from);
        zero(t, &names);
    }
}
