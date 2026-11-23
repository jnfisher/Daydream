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
//!
//! [`ImportOptions::like_nmr`] does what NMR's Database Importer does instead, byte for
//! byte (the `windows_nmr` test holds it to that against real NMR):
//!
//! - Rows go in index order, and every value is written, changed or not.
//! - Text goes through VB's `Trim` (spaces off both ends) into a fixed-length string, which
//!   pads it with spaces.
//! - A new record starts from the record of the row before it in this table (NMR reuses its
//!   Btrieve buffer, which a failed Get Equal leaves alone), or from zeros for the first.
//!   A new room starts from zeros: only `ImportRooms` clears the buffer.
//! - A text block is always re-encrypted, and stored at its full length.
//!
//! The options of NMR's Database Importer are [`ImportOptions`]' too: which tables, a range
//! of record numbers per table (for rooms, of one map), "Skip" or "Update" for records the
//! realm has, a room's items only or all but its items, and deleting the parts of a text
//! block past the last part the file has (NMR always does; [`ImportOptions::extra_parts`]).
//! NMR mode keeps NMR's ways with a range too: it starts at the range's first record when
//! the file has it, stops at the first row past the range, and leaves the parts of the last
//! text block it met alone.

use std::collections::HashMap;
use std::path::PathBuf;

use daydream_store::{Realm, StoreError, WbtrvDb};
use daydream_types::records::{Layout, LayoutSet};
use daydream_wcc::tables::Table;

use crate::data::{NmrData, NmrRows};
use crate::export::{key_text, row_of, sort_by_index};
use crate::schema::{
    self, NmrTable, Source, Value, ValueError, Vb, gather, layout_of, read_unsigned, scatter,
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
    /// Do exactly what NMR's import does (see the module docs), not Daydream's stricter
    /// rules.
    pub like_nmr: bool,
    /// Only these tables (NMR's "Databases to Import"); `None` is all of them.
    pub tables: Option<Vec<String>>,
    /// Only these records of a table (NMR's range boxes); a table without one imports all.
    pub ranges: Vec<Range>,
    /// Which of a room's columns to import (NMR's "Only Items" and "Not Items").
    pub room_parts: RoomParts,
    /// Delete the parts of an imported text block beyond the last part the file has, as NMR
    /// does, so a text that got shorter loses its old tail.
    pub extra_parts: bool,
}

/// Records of one table by number: `from..=to` of the first index column, and for rooms
/// only those of map `map`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Range {
    #[serde(default)]
    pub table: String,
    #[serde(default)]
    pub map: Option<i64>,
    pub from: i64,
    pub to: i64,
}

/// Which of a room's columns an import writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RoomParts {
    #[default]
    All,
    /// Only the visible and hidden items (`Room Item n`, `Hidden Item n`, with their uses
    /// and quantities).
    OnlyItems,
    /// Everything but those.
    NotItems,
}

/// Whether a room column is one of the items "Only Items" means.
fn is_room_item(column: &str) -> bool {
    column.starts_with("Room Item ") || column.starts_with("Hidden Item ")
}

/// What an import did to one table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableReport {
    pub table: String,
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub skipped: usize,
    /// Records removed by [`delete`].
    pub deleted: usize,
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
    #[error("{table}: its records are not numbered, so they take no range")]
    Range { table: String },
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
        if let Some(only) = &opts.tables
            && !only.iter().any(|n| n.eq_ignore_ascii_case(t.name))
        {
            continue;
        }
        reports.push(import_table(realm, t, table, rows, opts)?);
    }
    Ok(reports)
}

/// Removes the records of NMR table `table` whose index values are `keys` (each in the
/// order of the table's index columns, as NMR's Delete Records tool picks them). A key no
/// record has is reported as an error.
pub fn delete(
    realm: &Realm,
    table: &str,
    keys: &[Vec<Value>],
    layout: LayoutSet,
) -> Result<TableReport, ImportError> {
    let t = schema::table(table).ok_or_else(|| ImportError::NoFile {
        table: table.to_string(),
    })?;
    let mut report = TableReport {
        table: t.name.to_string(),
        ..TableReport::default()
    };
    let Some(kind) = t.table else {
        return Ok(report);
    };
    let wanted: HashMap<Vec<KeyPart>, usize> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (key_of(k.iter().cloned()), i))
        .collect();
    let mut found = vec![false; keys.len()];
    let layout = layout_of(kind, layout);
    let index_cols = index_columns(t);
    for file in realm.files_of(kind) {
        let mut doomed = Vec::new();
        {
            let db = WbtrvDb::open_read_only(&file.path)?;
            for (id, mut raw) in db.records()? {
                raw.resize(layout.len.max(raw.len()), 0);
                let row = row_of(t, layout, &raw);
                let key = key_of(index_cols.iter().map(|&i| row[i].clone()));
                if let Some(&i) = wanted.get(&key) {
                    found[i] = true;
                    doomed.push(id);
                }
            }
        }
        if !doomed.is_empty() {
            let mut db = WbtrvDb::open(&file.path)?;
            for id in doomed {
                db.delete(id)?;
                report.deleted += 1;
            }
        }
    }
    for (k, f) in keys.iter().zip(found) {
        if !f {
            report.errors.push(format!("{}: no record {k:?}", t.name));
        }
    }
    Ok(report)
}

/// Removes every record of NMR table `table`, or with `range` only those in it, as NMR's
/// Database Deleter does ("All", or From and To, and for rooms a map).
pub fn delete_range(
    realm: &Realm,
    table: &str,
    range: Option<&Range>,
    layout: LayoutSet,
    dry_run: bool,
) -> Result<TableReport, ImportError> {
    let t = schema::table(table).ok_or_else(|| ImportError::NoFile {
        table: table.to_string(),
    })?;
    let mut report = TableReport {
        table: t.name.to_string(),
        ..TableReport::default()
    };
    let Some(kind) = t.table else {
        return Ok(report);
    };
    let index_cols = index_columns(t);
    if range.is_some() && !matches!(t.columns[index_cols[0]].ty, schema::ColumnType::Long) {
        return Err(ImportError::Range {
            table: t.name.to_string(),
        });
    }
    let layout = layout_of(kind, layout);
    let num = |v: &Value| schema::number(v).map_or(0, |n| n as i64);
    for file in realm.files_of(kind) {
        let mut doomed = Vec::new();
        {
            let db = WbtrvDb::open_read_only(&file.path)?;
            for (id, mut raw) in db.records()? {
                let Some(r) = range else {
                    doomed.push(id);
                    continue;
                };
                raw.resize(layout.len.max(raw.len()), 0);
                let row = row_of(t, layout, &raw);
                let (map, number) = if kind == Table::Rooms {
                    (Some(num(&row[index_cols[0]])), num(&row[index_cols[1]]))
                } else {
                    (None, num(&row[index_cols[0]]))
                };
                if r.map.is_none_or(|m| Some(m) == map) && (r.from..=r.to).contains(&number) {
                    doomed.push(id);
                }
            }
        }
        report.deleted += doomed.len();
        if !doomed.is_empty() && !dry_run {
            let mut db = WbtrvDb::open(&file.path)?;
            for id in doomed {
                db.delete(id)?;
            }
        }
    }
    Ok(report)
}

/// The positions of a table's index columns among its columns.
fn index_columns(t: &NmrTable) -> Vec<usize> {
    t.index
        .as_ref()
        .map(|i| {
            i.columns
                .iter()
                .map(|c| t.columns.iter().position(|col| col.name == *c).unwrap())
                .collect()
        })
        .unwrap_or_default()
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
    let index_cols = index_columns(t);

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

    let ordered;
    let rows: &[Vec<Value>] = if opts.like_nmr {
        let mut sorted = rows.rows.clone();
        sort_by_index(t, &mut sorted);
        ordered = sorted;
        &ordered
    } else {
        &rows.rows
    };
    let range = opts
        .ranges
        .iter()
        .find(|r| r.table.eq_ignore_ascii_case(t.name));
    if range.is_some() && !matches!(t.columns[index_cols[0]].ty, schema::ColumnType::Long) {
        return Err(ImportError::Range {
            table: t.name.to_string(),
        });
    }
    let room_parts = if table == Table::Rooms {
        opts.room_parts
    } else {
        RoomParts::All
    };
    // text block parts: the last part of each number the file has, in the order met
    let mut parts: Vec<(i64, i64)> = Vec::new();
    // NMR's Btrieve buffer: the record of the row before
    let mut last: Option<Vec<u8>> = None;
    // NMR seeks to the range's first record and starts there, or from the top when the
    // file lacks it
    let start = match range {
        Some(r) if opts.like_nmr => {
            let seek: Vec<KeyPart> = match table {
                Table::Rooms => vec![KeyPart::Number(r.map.unwrap_or(0)), KeyPart::Number(r.from)],
                Table::TextBlocks => vec![KeyPart::Number(r.from), KeyPart::Number(0)],
                _ => vec![KeyPart::Number(r.from)],
            };
            rows.iter()
                .position(|row| {
                    key_of(
                        index_cols
                            .iter()
                            .map(|&i| at[i].map(|a| row[a].clone()).unwrap_or(Value::Null)),
                    ) == seek
                })
                .unwrap_or(0)
        }
        _ => 0,
    };
    report.skipped += start;
    for (n, row) in rows.iter().enumerate().skip(start) {
        let cell = |i: usize| at[i].map(|a| row[a].clone()).unwrap_or(Value::Null);
        let key = key_of(index_cols.iter().map(|&i| cell(i)));
        let num = |i: usize| schema::number(&cell(i)).map_or(0, |n| n as i64);
        if let Some(r) = range {
            // rooms: the map, then the room number
            let (map, number) = if table == Table::Rooms {
                (Some(num(index_cols[0])), num(index_cols[1]))
            } else {
                (None, num(index_cols[0]))
            };
            if number > r.to && opts.like_nmr && r.map.is_none_or(|m| Some(m) == map) {
                // NMR stops at the first row past the range (its rows are in order)
                report.skipped += rows.len() - n;
                break;
            }
            if r.map.is_some_and(|m| Some(m) != map) || number < r.from || number > r.to {
                report.skipped += 1;
                // NMR still counts a skipped row as the text block it last met
                if table == Table::TextBlocks && opts.like_nmr {
                    parts.push((num(index_cols[0]), num(index_cols[1])));
                }
                continue;
            }
        }
        if table == Table::TextBlocks {
            parts.push((num(index_cols[0]), num(index_cols[1])));
        }
        let (fi, id, before, stored_len) = match existing.get(&key) {
            Some((fi, id, raw, len)) => {
                if opts.skip_existing {
                    report.skipped += 1;
                    continue;
                }
                (*fi, Some(*id), raw.clone(), *len)
            }
            None => {
                // NMR clears the buffer for a new room only
                let base = match &last {
                    Some(r) if opts.like_nmr && table != Table::Rooms => r.clone(),
                    _ => vec![0; layout.len],
                };
                (0, None, base, 0)
            }
        };
        let mut record = before.clone();
        let applied = apply(t, layout, &at, row, &mut record, opts.like_nmr, room_parts);
        if opts.like_nmr {
            last = Some(record.clone());
        }
        if let Err((column, e)) = applied {
            report
                .errors
                .push(format!("{} row {}: {column}: {e}", t.name, n + 1));
            continue;
        }
        if id.is_some() && record == before {
            report.unchanged += 1;
            continue;
        }
        let image = if table.is_variable_length() && !opts.like_nmr {
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
    if table == Table::TextBlocks && opts.extra_parts {
        drop_extra_parts(&mut targets, &existing, &parts, opts, &mut report)?;
    }
    Ok(report)
}

/// Deletes the text block parts past the last part of each number the import met. NMR does
/// it when it reaches the next number, so it never does it for the last number of the file
/// and deletes only parts that follow on one from another; NMR mode does the same, and
/// otherwise every later part of every number goes.
fn drop_extra_parts(
    targets: &mut [Target],
    existing: &HashMap<Vec<KeyPart>, (usize, i64, Vec<u8>, usize)>,
    parts: &[(i64, i64)],
    opts: &ImportOptions,
    report: &mut TableReport,
) -> Result<(), ImportError> {
    let mut last: Vec<(i64, i64)> = Vec::new();
    for &(number, part) in parts {
        match last.last_mut() {
            Some((n, p)) if *n == number => *p = part,
            _ => last.push((number, part)),
        }
    }
    if opts.like_nmr {
        last.pop();
    }
    let key = |n: i64, p: i64| vec![KeyPart::Number(n), KeyPart::Number(p)];
    let mut doomed: Vec<(usize, i64)> = Vec::new();
    for (number, part) in last {
        if opts.like_nmr {
            let mut p = part + 1;
            while let Some((fi, id, ..)) = existing.get(&key(number, p)) {
                doomed.push((*fi, *id));
                p += 1;
            }
        } else {
            for (k, (fi, id, ..)) in existing {
                if let [KeyPart::Number(n), KeyPart::Number(p)] = k.as_slice()
                    && *n == number
                    && *p > part
                {
                    doomed.push((*fi, *id));
                }
            }
        }
    }
    doomed.sort_unstable();
    doomed.dedup();
    for (fi, id) in doomed {
        if !opts.dry_run {
            let target = &mut targets[fi];
            if target.db.is_none() {
                target.db = Some(WbtrvDb::open(&target.path)?);
            }
            target.db.as_mut().unwrap().delete(id)?;
        }
        report.deleted += 1;
    }
    Ok(())
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
    like_nmr: bool,
    room_parts: RoomParts,
) -> Result<(), (&'static str, ValueError)> {
    for (c, a) in t.columns.iter().zip(at) {
        let wanted = match room_parts {
            RoomParts::All => true,
            // the key goes in either way
            RoomParts::OnlyItems => {
                is_room_item(c.name) || c.name == "Map Number" || c.name == "Room Number"
            }
            RoomParts::NotItems => !is_room_item(c.name),
        };
        if !wanted {
            continue;
        }
        let value = match a {
            Some(a) => row[*a].clone(),
            None if c.optional => Value::Long(1),
            None => continue,
        };
        let err = |e| (c.name, e);
        match &c.source {
            Source::Element {
                vb: Vb::Text,
                pieces,
            } if like_nmr => {
                let len = pieces.iter().map(|p| p.len).sum();
                let Value::Text(s) = &value else {
                    return Err(err(ValueError::NotText(value)));
                };
                let mut bytes = text::to_1252(s.trim_matches(' ')).map_err(|e| err(e.into()))?;
                bytes.resize(len, b' ');
                scatter(layout, pieces, &bytes, record);
            }
            Source::Element { vb, pieces } => {
                let len = pieces.iter().map(|p| p.len).sum();
                let current = vb.read(&gather(layout, pieces, record));
                if !like_nmr && same(&current, &value) {
                    continue;
                }
                let bytes = vb.write(&value, len).map_err(err)?;
                scatter(layout, pieces, &bytes, record);
            }
            Source::Unsigned(pieces) => {
                let current = read_unsigned(&gather(layout, pieces, record));
                if !like_nmr && same(&current, &value) {
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
                if !like_nmr && text::decrypt_textblock(&record[span.clone()]) == *s {
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
