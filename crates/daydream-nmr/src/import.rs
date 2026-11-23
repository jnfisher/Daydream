//! Applying NMR `.mdb` rows to a realm, as NMR's import does.
//!
//! Each row is matched to a record by the table's index columns (the record number; map
//! and room for rooms; number and part for text blocks; the action's name). A matched
//! record gets the row's values over its own bytes, so the bytes no column covers are
//! kept; a new record starts from zeros. Where NMR differs, Daydream is stricter:
//!
//! - A value that already reads back the same from the record is not written, so an
//!   unedited row changes nothing. Text without a NUL is compared with the stored text up
//!   to its first NUL, as NMR's `ClipNull` shows it, so the bytes after a stored text's NUL
//!   survive an edit made in Access.
//! - A number that does not fit its element is an error for that row, where VB would
//!   raise an overflow.
//! - Text is written NUL-filled, where NMR's fixed-length VB strings pad with spaces.
//!
//! A monster table without `Exp Multiplier` imports with a multiplier of 1, the answer
//! NMR offers.

use std::collections::HashMap;
use std::path::PathBuf;

use daydream_store::{Realm, StoreError, WbtrvDb};
use daydream_types::records::{Layout, LayoutSet};
use daydream_wcc::tables::Table;

use crate::data::{NmrData, NmrRows};
use crate::export::{key_text, row_of};
use crate::schema::{
    self, NmrTable, Source, Value, ValueError, gather, layout_of, read_unsigned, scatter,
    write_unsigned,
};
use crate::text;

/// How to import.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Leave records that already exist alone (NMR's "skip existing").
    pub skip_existing: bool,
    /// Report what would change without writing.
    pub dry_run: bool,
    /// The realm's record layouts.
    pub layout: LayoutSet,
}

/// What an import did to one table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableReport {
    pub table: String,
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub skipped: usize,
    /// One line per row that could not be imported.
    pub errors: Vec<String>,
}

/// Why an import stopped.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("{table}: missing column {column}")]
    MissingColumn { table: String, column: String },
    #[error("{table}: the realm has no file for these records")]
    NoFile { table: String },
}

/// Applies every table of `data` NMR knows to the realm.
pub fn import(
    realm: &Realm,
    data: &NmrData,
    opts: &ImportOptions,
) -> Result<Vec<TableReport>, ImportError> {
    let mut reports = Vec::new();
    for rows in &data.tables {
        let Some(t) = schema::table(&rows.name) else {
            continue;
        };
        let Some(table) = t.table else { continue };
        reports.push(import_table(realm, t, table, rows, opts)?);
    }
    Ok(reports)
}

/// A row's identity: its index columns, numbers as they are and text as [`key_text`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum KeyPart {
    Number(i64),
    Text(String),
}

fn key_of(values: impl Iterator<Item = Value>) -> Vec<KeyPart> {
    values
        .map(|v| match v {
            Value::Long(n) => KeyPart::Number(i64::from(n)),
            Value::Double(d) => KeyPart::Number(d as i64),
            Value::Text(s) => KeyPart::Text(key_text(&s)),
            Value::Null => KeyPart::Number(0),
        })
        .collect()
}

struct Target {
    path: PathBuf,
    db: Option<WbtrvDb>,
}

fn import_table(
    realm: &Realm,
    t: &'static NmrTable,
    table: Table,
    rows: &NmrRows,
    opts: &ImportOptions,
) -> Result<TableReport, ImportError> {
    let layout = layout_of(table, opts.layout);
    let mut report = TableReport {
        table: t.name.to_string(),
        ..TableReport::default()
    };
    let missing = |column: &str| ImportError::MissingColumn {
        table: t.name.to_string(),
        column: column.to_string(),
    };
    // where each schema column is in the rows
    let at: Vec<Option<usize>> = t.columns.iter().map(|c| rows.position(c.name)).collect();
    for (c, a) in t.columns.iter().zip(&at) {
        if a.is_none() && !c.optional && !matches!(c.source, Source::Zero) {
            return Err(missing(c.name));
        }
    }
    let index_cols: Vec<usize> = t
        .index
        .as_ref()
        .map(|i| {
            i.columns
                .iter()
                .map(|c| t.columns.iter().position(|col| col.name == *c).unwrap())
                .collect()
        })
        .unwrap_or_default();

    // the realm's records by key
    let mut targets: Vec<Target> = realm
        .files_of(table)
        .map(|f| Target {
            path: f.path.clone(),
            db: None,
        })
        .collect();
    if targets.is_empty() {
        return Err(ImportError::NoFile {
            table: t.name.to_string(),
        });
    }
    let mut existing: HashMap<Vec<KeyPart>, (usize, i64, Vec<u8>, usize)> = HashMap::new();
    for (fi, target) in targets.iter().enumerate() {
        let db = WbtrvDb::open_read_only(&target.path)?;
        for (id, mut raw) in db.records()? {
            let stored_len = raw.len();
            raw.resize(layout.len.max(raw.len()), 0);
            let row = row_of(t, layout, &raw);
            let key = key_of(index_cols.iter().map(|&i| row[i].clone()));
            existing.entry(key).or_insert((fi, id, raw, stored_len));
        }
    }

    for (n, row) in rows.rows.iter().enumerate() {
        let key = key_of(
            index_cols
                .iter()
                .map(|&i| at[i].map(|a| row[a].clone()).unwrap_or(Value::Null)),
        );
        let (fi, id, before, stored_len) = match existing.get(&key) {
            Some((fi, id, raw, len)) => {
                if opts.skip_existing {
                    report.skipped += 1;
                    continue;
                }
                (*fi, Some(*id), raw.clone(), *len)
            }
            None => (0, None, vec![0; layout.len], 0),
        };
        let mut record = before.clone();
        if let Err((column, e)) = apply(t, layout, &at, row, &mut record) {
            report
                .errors
                .push(format!("{} row {}: {column}: {e}", t.name, n + 1));
            continue;
        }
        if id.is_some() && record == before {
            report.unchanged += 1;
            continue;
        }
        let image = if table.is_variable_length() {
            textblock_image(layout, &record, stored_len)
        } else {
            record.clone()
        };
        if opts.dry_run {
            if id.is_some() {
                report.updated += 1;
            } else {
                report.inserted += 1;
            }
            continue;
        }
        let target = &mut targets[fi];
        if target.db.is_none() {
            target.db = Some(WbtrvDb::open(&target.path)?);
        }
        let db = target.db.as_mut().unwrap();
        let result = match id {
            Some(id) => db.update(id, &image).map(|()| id),
            None => db.insert(&image),
        };
        match result {
            Ok(new_id) => {
                if id.is_some() {
                    report.updated += 1;
                } else {
                    report.inserted += 1;
                    existing.insert(key, (fi, new_id, record, image.len()));
                }
            }
            Err(e) => report.errors.push(format!("{} row {}: {e}", t.name, n + 1)),
        }
    }
    Ok(report)
}

/// A WCCTEXT record's stored image: the fixed part and the text through its terminator,
/// never shorter than it was.
fn textblock_image(layout: &Layout, record: &[u8], stored_len: usize) -> Vec<u8> {
    let f = layout.field("text").expect("textblk has text");
    let text = &record[f.offset..f.offset + f.size];
    let used = text.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let len = (f.offset + used + 1).max(stored_len).min(layout.len);
    record[..len].to_vec()
}

/// Writes the row's values into the record.
fn apply(
    t: &NmrTable,
    layout: &Layout,
    at: &[Option<usize>],
    row: &[Value],
    record: &mut [u8],
) -> Result<(), (&'static str, ValueError)> {
    for (c, a) in t.columns.iter().zip(at) {
        let value = match a {
            Some(a) => row[*a].clone(),
            None if c.optional => Value::Long(1),
            None => continue,
        };
        let err = |e| (c.name, e);
        match &c.source {
            Source::Element { vb, pieces } => {
                let len = pieces.iter().map(|p| p.len).sum();
                let current = vb.read(&gather(layout, pieces, record));
                if same(&current, &value) {
                    continue;
                }
                let bytes = vb.write(&value, len).map_err(err)?;
                scatter(layout, pieces, &bytes, record);
            }
            Source::Unsigned(pieces) => {
                let current = read_unsigned(&gather(layout, pieces, record));
                if same(&current, &value) {
                    continue;
                }
                let bytes = write_unsigned(&value).map_err(err)?;
                scatter(layout, pieces, &bytes, record);
            }
            Source::TextblockData => {
                let Value::Text(s) = &value else {
                    return Err(err(ValueError::NotText(value)));
                };
                let f = layout.field("text").expect("textblk has text");
                let span = f.offset..f.offset + f.size;
                if text::decrypt_textblock(&record[span.clone()]) == *s {
                    continue;
                }
                let bytes =
                    text::encrypt_textblock(s, f.size).map_err(|e| err(ValueError::Text(e)))?;
                record[span].copy_from_slice(&bytes);
            }
            Source::Zero | Source::Info => {}
        }
    }
    Ok(())
}

/// Whether a stored value and an incoming one are the same. Text that carries its NULs (a
/// whole fixed-length string, as NMR exports it) must match every character; text without
/// one matches the stored text up to its first NUL.
fn same(current: &Value, incoming: &Value) -> bool {
    match (current, incoming) {
        (Value::Text(a), Value::Text(b)) if b.contains('\0') => a == b,
        (Value::Text(a), Value::Text(b)) => clip(a) == b,
        (a, b) => match (schema::number(a), schema::number(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        },
    }
}

fn clip(s: &str) -> &str {
    s.split('\0').next().unwrap_or("")
}
