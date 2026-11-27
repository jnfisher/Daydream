//! A realm's records as the rows of NMR's tables, one row at a time: what the browser's
//! editors and the MCP server both edit through.
//!
//! Each table is one of NMR's `.mdb` tables, with its columns as NMR's forms label them,
//! and each record is a row keyed by the table's index columns: the number, map and room
//! for rooms, number and part for text blocks, the name for actions. Reads come from
//! [`export_table`](crate::export_table), and every write goes through
//! [`import`](crate::import) with its default (strict) rules, so an edit lands in the
//! record exactly as the same edit made in Access and imported would.
//!
//! An [`Edit`] can be previewed (every value it changes, checked by a dry-run import) and
//! applied; applying returns the records it changed, byte for byte, which [`revert`] puts
//! back.
//!
//! Text is shown up to its first NUL, as NMR's forms show it. Saved back unchanged, it
//! leaves the record's bytes alone.

use std::collections::BTreeMap;

use daydream_store::{Realm, StoreError, WbtrvDb};
use daydream_types::records::LayoutSet;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json, json};

use crate::modify::Change;
use crate::schema::{self, ColumnType, NmrTable, Value};
use crate::{ImportOptions, NmrData, NmrRows, TableReport};

/// Why a row could not be read or written. Nothing is written.
#[derive(Debug, thiserror::Error)]
pub enum RowError {
    /// The request itself is wrong: an unknown column, a value of the wrong type.
    #[error("{0}")]
    Bad(String),
    #[error("{0} not found")]
    NotFound(String),
    /// The key is taken, or the record changed since it was read.
    #[error("{0}")]
    Conflict(String),
    /// The import refused the row (its errors).
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("{0}")]
    Other(String),
}

fn bad(msg: impl Into<String>) -> RowError {
    RowError::Bad(msg.into())
}

fn other(e: impl std::fmt::Display) -> RowError {
    RowError::Other(e.to_string())
}

/// The columns a row list shows besides the key, the first one a table has.
pub const TITLE_COLUMNS: &[&str] = &[
    "Name",
    "Line 1",
    "Data",
    "Single to User",
    "Display Name",
    "First Name",
];

/// The most characters of a title a row list shows.
const TITLE_LEN: usize = 60;

/// Every table that holds records (every one but `Info`), NMR's then the player tables.
pub fn record_tables() -> impl Iterator<Item = &'static NmrTable> {
    crate::columns::TABLES
        .iter()
        .chain(crate::players::TABLES)
        .filter(|t| t.table.is_some())
}

/// An NMR table that holds records.
pub fn record_table(name: &str) -> Result<&'static NmrTable, RowError> {
    schema::table(name)
        .filter(|t| t.table.is_some())
        .ok_or_else(|| RowError::NotFound(format!("table {name}")))
}

/// The columns a row is keyed by, in the index's order.
pub fn index_columns(t: &NmrTable) -> Vec<&'static str> {
    t.index
        .as_ref()
        .map(|i| i.columns.to_vec())
        .unwrap_or_default()
}

/// The columns a record's members are, as the editors label them: `maxdmg` is `Max Hit`.
/// A member that is several numbered columns (`desc`: `Desc 0` to `Desc 6`) is their name
/// without the number. A member no column shows is left out; each label comes once.
pub fn member_labels(t: &NmrTable, members: &[&str]) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    for member in members {
        let cols: Vec<&str> = t
            .columns
            .iter()
            .filter(|c| match c.source {
                schema::Source::Element { pieces, .. } | schema::Source::Unsigned(pieces) => {
                    pieces.iter().any(|p| p.member == *member)
                }
                _ => false,
            })
            .map(|c| c.name)
            .collect();
        let label = match cols.as_slice() {
            [] => continue,
            [one] => (*one).to_string(),
            many => many[0]
                .trim_end_matches(|ch: char| ch.is_ascii_digit() || ch == ' ' || ch == '#')
                .to_string(),
        };
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    labels
}

/// The NMR table a WCC record type's rows are in, and a record's key there: what the
/// editors open it by (`Items`, `[868]`). `None` for a record type no NMR table holds.
pub fn record_key(
    table: daydream_wcc::tables::Table,
    set: LayoutSet,
    record: &[u8],
) -> Option<(&'static NmrTable, Vec<Value>)> {
    let t = record_tables().find(|t| t.table == Some(table))?;
    let layout = schema::layout_of(table, set);
    let mut full = record.to_vec();
    // a variable-length record (a text block) is short of its layout
    if full.len() < layout.len {
        full.resize(layout.len, 0);
    }
    let row = crate::export::row_of(t, layout, &full);
    let key = index_columns(t)
        .iter()
        .map(|c| {
            t.columns
                .iter()
                .position(|col| col.name == *c)
                .map_or(Value::Null, |i| row[i].clone())
        })
        .collect();
    Some((t, key))
}

/// The table's title column (`Name`, ...), if it has one.
pub fn title_column(t: &NmrTable) -> Option<&'static str> {
    TITLE_COLUMNS
        .iter()
        .copied()
        .find(|c| t.column(c).is_some())
}

/// A column's type as JSON names it: `long`, `double`, `text` (with its size) or `memo`.
pub fn column_type(ty: ColumnType) -> (&'static str, Option<u16>) {
    match ty {
        ColumnType::Long => ("long", None),
        ColumnType::Double => ("double", None),
        ColumnType::Text(n) => ("text", Some(n)),
        ColumnType::Memo => ("memo", None),
    }
}

/// The whole numbers a column holds, as its record element stores them: `None` for text, and
/// for the columns NMR clamps (an unsigned 32-bit value) or makes up. A value outside it, or
/// one with a fraction, is refused when it is written.
pub fn column_range(c: &schema::Column) -> Option<(i64, i64)> {
    use schema::{Source, Vb};
    match c.source {
        Source::Element { vb: Vb::Long, .. } => Some((i32::MIN.into(), i32::MAX.into())),
        Source::Element {
            vb: Vb::Integer, ..
        } => Some((i16::MIN.into(), i16::MAX.into())),
        Source::Element { vb: Vb::Byte, .. } => Some((0, 255)),
        _ => None,
    }
}

/// A cell as the editors see it: text up to its first NUL.
pub fn cell_json(v: &Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Long(n) => json!(n),
        Value::Double(d) => json!(d),
        Value::Text(s) => json!(s.split('\0').next().unwrap_or("")),
    }
}

/// A cell as a row list's title: text's first line, without ANSI escapes, and short. A NUL
/// ends the text, as it does for the editors: what follows one is an older value's leftovers.
pub fn title_json(v: &Value) -> Json {
    let Value::Text(s) = v else {
        return cell_json(v);
    };
    let line = s
        .split('\0')
        .next()
        .unwrap_or("")
        .split(['\n', '\r'])
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    let mut out = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // ESC [ parameters, up to the final letter
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    let out = out.trim();
    match out.char_indices().nth(TITLE_LEN) {
        Some((i, _)) => json!(format!("{}...", &out[..i])),
        None => json!(out),
    }
}

/// A JSON value as a cell of `ty`.
pub fn cell_of(column: &str, ty: ColumnType, v: &Json) -> Result<Value, RowError> {
    let wrong = || bad(format!("{column}: {v} is not a {ty:?}"));
    Ok(match (ty, v) {
        (_, Json::Null) => Value::Null,
        (ColumnType::Long, Json::Number(n)) => Value::Long(
            n.as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or_else(wrong)?,
        ),
        (ColumnType::Double, Json::Number(n)) => Value::Double(n.as_f64().ok_or_else(wrong)?),
        (ColumnType::Long, Json::String(s)) => Value::Long(s.trim().parse().map_err(|_| wrong())?),
        (ColumnType::Double, Json::String(s)) => {
            Value::Double(s.trim().parse().map_err(|_| wrong())?)
        }
        (ColumnType::Text(n), Json::String(s)) => {
            if s.chars().count() > usize::from(n) {
                return Err(bad(format!("{column}: longer than {n} characters")));
            }
            Value::Text(s.clone())
        }
        (ColumnType::Memo, Json::String(s)) => Value::Text(s.clone()),
        _ => return Err(wrong()),
    })
}

/// A cell's value when a new row leaves its column out.
pub fn blank(ty: ColumnType) -> Value {
    match ty {
        ColumnType::Long => Value::Long(0),
        ColumnType::Double => Value::Double(0.0),
        ColumnType::Text(_) | ColumnType::Memo => Value::Text(String::new()),
    }
}

/// Comparable form of a key: numbers as numbers, text as NMR matches it.
fn key_form(v: &Value) -> Json {
    match v {
        Value::Null => json!(0),
        Value::Long(n) => json!(n),
        Value::Double(d) => json!(*d as i64),
        Value::Text(s) => json!(crate::export::key_text(s)),
    }
}

/// A row's key: its index columns' values.
pub fn key_of_row(rows: &NmrRows, row: &[Value], index: &[&str]) -> Vec<Value> {
    index
        .iter()
        .map(|c| {
            rows.position(c)
                .map(|i| row[i].clone())
                .unwrap_or(Value::Null)
        })
        .collect()
}

/// A key given as JSON (an array of the index columns' values, in order), as cells.
pub fn parse_key(t: &NmrTable, key: &Json) -> Result<Vec<Value>, RowError> {
    let Json::Array(parts) = key else {
        return Err(bad("key must be an array"));
    };
    let index = index_columns(t);
    if parts.len() != index.len() {
        return Err(bad(format!(
            "{} keys are {} values ({})",
            t.name,
            index.len(),
            index.join(", ")
        )));
    }
    index
        .iter()
        .zip(parts)
        .map(|(c, v)| {
            let col = t.column(c).expect("index columns are columns");
            cell_of(c, col.ty, v)
        })
        .collect()
}

/// Whether two keys name the same record, as NMR matches them.
pub fn same_key(a: &[Value], b: &[Value]) -> bool {
    a.iter().map(key_form).eq(b.iter().map(key_form))
}

/// The realm's layouts and every row of one table.
pub fn load(realm: &Realm, t: &NmrTable) -> Result<(LayoutSet, NmrRows), RowError> {
    let layout = crate::layout_of_realm(realm);
    let rows = crate::export_table(realm, t, layout).map_err(other)?;
    Ok((layout, rows))
}

/// The row with this key.
pub fn find_row<'a>(rows: &'a NmrRows, index: &[&str], key: &[Value]) -> Option<&'a Vec<Value>> {
    rows.rows
        .iter()
        .find(|r| same_key(&key_of_row(rows, r, index), key))
}

/// What a row was when the realm was last read from its game or written back into it (its
/// last publish).
#[derive(Debug, Clone, PartialEq)]
pub enum Published {
    /// As it is now; also when the realm's changes are not known (a realm edited before
    /// Daydream journaled its edits).
    Same,
    /// It is new since.
    Added,
    /// It changed since: the row as it was.
    Was(Vec<Value>),
}

/// What the row of `t` keyed `key` was at the realm's last publish: found among the records
/// its base journal holds ([`daydream_store::journal::BASE`]), by the key they have now, so
/// a renumbered record's was is its old number.
pub fn published(realm: &Realm, t: &NmrTable, key: &[Value]) -> Result<Published, RowError> {
    use daydream_store::journal;
    let Some(table) = t.table else {
        return Ok(Published::Same);
    };
    if journal::established(realm.dir())?.is_none() {
        return Ok(Published::Same);
    }
    let layout = schema::layout_of(table, realm.layout_set());
    let index = index_columns(t);
    let positions: Vec<Option<usize>> = index
        .iter()
        .map(|c| t.columns.iter().position(|col| col.name == *c))
        .collect();
    let row = |bytes: &[u8]| {
        let mut full = bytes.to_vec();
        // a variable-length record (a text block) is short of its layout
        if full.len() < layout.len {
            full.resize(layout.len, 0);
        }
        crate::export::row_of(t, layout, &full)
    };
    for file in realm.files_of(table) {
        let entries = journal::entries_of(realm.dir(), journal::BASE, &file.stem)?;
        if entries.is_empty() {
            continue;
        }
        let db = WbtrvDb::open_read_only(&file.path)?;
        for e in entries {
            let Some(now) = db.get(e.id)? else {
                continue;
            };
            let now_row = row(&now);
            let now_key: Vec<Value> = positions
                .iter()
                .map(|p| p.map_or(Value::Null, |i| now_row[i].clone()))
                .collect();
            if !same_key(&now_key, key) {
                continue;
            }
            return Ok(match e.before {
                None => Published::Added,
                Some(b) if b == now => Published::Same,
                Some(b) => Published::Was(row(&b)),
            });
        }
    }
    Ok(Published::Same)
}

/// A row as `{column: value}`, as the editors show it.
pub fn row_json(rows: &NmrRows, row: &[Value]) -> Map<String, Json> {
    rows.columns
        .iter()
        .zip(row)
        .map(|(c, v)| (c.clone(), cell_json(v)))
        .collect()
}

/// An import report as JSON.
pub fn report_json(r: &TableReport) -> Json {
    json!({
        "table": r.table,
        "inserted": r.inserted,
        "updated": r.updated,
        "unchanged": r.unchanged,
        "skipped": r.skipped,
        "deleted": r.deleted,
        "errors": r.errors,
    })
}

/// Imports one row with the default (strict) import; `dry_run` only checks it.
pub fn write_row(
    realm: &Realm,
    layout: LayoutSet,
    t: &NmrTable,
    row: Vec<Value>,
    dry_run: bool,
) -> Result<TableReport, RowError> {
    let data = NmrData {
        tables: vec![NmrRows {
            name: t.name.to_string(),
            columns: t.columns.iter().map(|c| c.name.to_string()).collect(),
            rows: vec![row],
        }],
    };
    let opts = ImportOptions {
        layout,
        dry_run,
        ..ImportOptions::default()
    };
    let reports = crate::import(realm, &data, &opts).map_err(other)?;
    let report = reports.into_iter().next().unwrap_or_default();
    if !report.errors.is_empty() {
        return Err(RowError::Invalid(report.errors.join("; ")));
    }
    Ok(report)
}

/// `values` (`{column: value}`, any case) over `row`.
pub fn apply_values(
    t: &NmrTable,
    row: &mut [Value],
    values: &Map<String, Json>,
) -> Result<(), RowError> {
    for (name, v) in values {
        let (i, col) = t
            .columns
            .iter()
            .enumerate()
            .find(|(_, c)| c.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| bad(format!("{} has no column {name}", t.name)))?;
        row[i] = cell_of(col.name, col.ty, v)?;
    }
    Ok(())
}

/// One edit of one row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Edit {
    /// A new row: its key columns and any others (the rest start blank).
    Insert {
        table: String,
        values: Map<String, Json>,
    },
    /// Some columns of an existing row; the key cannot change.
    Update {
        table: String,
        key: Json,
        values: Map<String, Json>,
    },
    /// Removes a row.
    Delete { table: String, key: Json },
}

impl Edit {
    pub fn table(&self) -> &str {
        match self {
            Edit::Insert { table, .. }
            | Edit::Update { table, .. }
            | Edit::Delete { table, .. } => table,
        }
    }
}

/// What an edit would do.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Preview {
    pub table: String,
    /// The row's key.
    pub key: Vec<Value>,
    /// Every value that changes; an insert's start from null, a delete's end at null.
    pub changes: Vec<Change>,
}

/// The rows an edit works on, worked out without writing.
struct Planned {
    t: &'static NmrTable,
    layout: LayoutSet,
    key: Vec<Value>,
    before: Option<Vec<Value>>,
    /// `None` for a delete.
    after: Option<Vec<Value>>,
    columns: Vec<String>,
}

/// Text cells clipped at their NUL, as the editors send them back: the import leaves such
/// text alone.
fn clipped(mut row: Vec<Value>) -> Vec<Value> {
    for v in row.iter_mut() {
        if let Value::Text(s) = v {
            s.truncate(s.find('\0').unwrap_or(s.len()));
        }
    }
    row
}

fn plan(realm: &Realm, edit: &Edit) -> Result<Planned, RowError> {
    let t = record_table(edit.table())?;
    let (layout, rows) = load(realm, t)?;
    let index = index_columns(t);
    let columns = rows.columns.clone();
    let found = |key: &[Value]| find_row(&rows, &index, key).cloned();
    Ok(match edit {
        Edit::Insert { values, .. } => {
            for c in &index {
                if !values.keys().any(|k| k.eq_ignore_ascii_case(c)) {
                    return Err(bad(format!("a new {} row needs {c}", t.name)));
                }
            }
            let mut row: Vec<Value> = t.columns.iter().map(|c| blank(c.ty)).collect();
            apply_values(t, &mut row, values)?;
            let key = key_of_row(&rows, &row, &index);
            if found(&key).is_some() {
                return Err(RowError::Conflict(format!(
                    "{} already has {}",
                    t.name,
                    key_display(&key)
                )));
            }
            Planned {
                t,
                layout,
                key,
                before: None,
                after: Some(row),
                columns,
            }
        }
        Edit::Update { key, values, .. } => {
            let key = parse_key(t, key)?;
            let before = found(&key)
                .ok_or_else(|| RowError::NotFound(format!("{} {}", t.name, key_display(&key))))?;
            let mut row = clipped(before.clone());
            apply_values(t, &mut row, values)?;
            if !same_key(&key_of_row(&rows, &row, &index), &key) {
                return Err(bad(
                    "an update cannot change the key; insert a new record instead",
                ));
            }
            Planned {
                t,
                layout,
                key,
                before: Some(before),
                after: Some(row),
                columns,
            }
        }
        Edit::Delete { key, .. } => {
            let key = parse_key(t, key)?;
            let before = found(&key)
                .ok_or_else(|| RowError::NotFound(format!("{} {}", t.name, key_display(&key))))?;
            Planned {
                t,
                layout,
                key,
                before: Some(before),
                after: None,
                columns,
            }
        }
    })
}

/// A key as people write it: `[1, "x"]`.
pub fn key_display(key: &[Value]) -> String {
    Json::Array(key.iter().map(cell_json).collect()).to_string()
}

fn changes(p: &Planned) -> Vec<Change> {
    let shown = |r: &Option<Vec<Value>>, i: usize| match r {
        Some(r) => match &r[i] {
            Value::Text(s) => Value::Text(s.split('\0').next().unwrap_or("").to_string()),
            v => v.clone(),
        },
        None => Value::Null,
    };
    (0..p.columns.len())
        .filter_map(|i| {
            let (before, after) = (shown(&p.before, i), shown(&p.after, i));
            (before != after).then(|| Change {
                key: p.key.clone(),
                column: p.columns[i].clone(),
                before,
                after,
            })
        })
        .collect()
}

/// Every value `edit` would change, after a dry-run import has accepted it.
pub fn preview(realm: &Realm, edit: &Edit) -> Result<Preview, RowError> {
    let p = plan(realm, edit)?;
    if let Some(row) = &p.after {
        write_row(realm, p.layout, p.t, row.clone(), true)?;
    }
    Ok(Preview {
        table: p.t.name.to_string(),
        key: p.key.clone(),
        changes: changes(&p),
    })
}

/// One record an edit changed, as its `.db` holds it: `before` is `None` for a record the
/// edit added, `after` `None` for one it removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordChange {
    /// The file's stem (`WCCITEMS`).
    pub stem: String,
    pub id: i64,
    #[serde(with = "opt_bytes")]
    pub before: Option<Vec<u8>>,
    #[serde(with = "opt_bytes")]
    pub after: Option<Vec<u8>>,
}

/// Bytes as hex in JSON, so a journal is plain text.
mod opt_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_some(&b.iter().map(|x| format!("{x:02x}")).collect::<String>()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        let Some(hex) = Option::<String>::deserialize(d)? else {
            return Ok(None);
        };
        if hex.len() % 2 != 0 {
            return Err(serde::de::Error::custom("odd hex length"));
        }
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
            .map(Some)
            .map_err(serde::de::Error::custom)
    }
}

/// Every record of the files holding `t`, by file and id.
fn snapshot(
    realm: &Realm,
    t: &NmrTable,
) -> Result<BTreeMap<String, BTreeMap<i64, Vec<u8>>>, RowError> {
    let mut out = BTreeMap::new();
    let table = t.table.expect("record tables have a file");
    for f in realm.files_of(table) {
        let db = WbtrvDb::open_read_only(&f.path)?;
        out.insert(f.stem.clone(), db.records()?.into_iter().collect());
    }
    Ok(out)
}

fn diff(
    before: &BTreeMap<String, BTreeMap<i64, Vec<u8>>>,
    after: &BTreeMap<String, BTreeMap<i64, Vec<u8>>>,
) -> Vec<RecordChange> {
    let empty = BTreeMap::new();
    let mut out = Vec::new();
    for stem in before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let (b, a) = (
            before.get(stem).unwrap_or(&empty),
            after.get(stem).unwrap_or(&empty),
        );
        for id in b
            .keys()
            .chain(a.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            let (old, new) = (b.get(id), a.get(id));
            if old != new {
                out.push(RecordChange {
                    stem: stem.clone(),
                    id: *id,
                    before: old.cloned(),
                    after: new.cloned(),
                });
            }
        }
    }
    out
}

/// Makes `edit` and returns the import's report (the editors' save).
pub fn commit(realm: &Realm, edit: &Edit) -> Result<TableReport, RowError> {
    let p = plan(realm, edit)?;
    write_planned(realm, &p)
}

/// Makes `edit`. Returns the import's report and every record it changed, byte for byte,
/// for [`revert`].
pub fn apply(realm: &Realm, edit: &Edit) -> Result<(TableReport, Vec<RecordChange>), RowError> {
    let p = plan(realm, edit)?;
    if let Some(row) = &p.after {
        // checked before anything is written
        write_row(realm, p.layout, p.t, row.clone(), true)?;
    }
    let before = snapshot(realm, p.t)?;
    let report = write_planned(realm, &p)?;
    let after = snapshot(realm, p.t)?;
    Ok((report, diff(&before, &after)))
}

fn write_planned(realm: &Realm, p: &Planned) -> Result<TableReport, RowError> {
    Ok(match &p.after {
        Some(row) => write_row(realm, p.layout, p.t, row.clone(), false)?,
        None => {
            let r = crate::delete(realm, p.t.name, std::slice::from_ref(&p.key), p.layout)
                .map_err(other)?;
            if r.deleted == 0 {
                return Err(RowError::NotFound(format!(
                    "{} {}",
                    p.t.name,
                    key_display(&p.key)
                )));
            }
            r
        }
    })
}

/// Puts back the records `changes` changed, as they were before. Refused, with nothing
/// written, if any of them has changed since.
pub fn revert(realm: &Realm, changes: &[RecordChange]) -> Result<(), RowError> {
    let mut dbs: BTreeMap<&str, WbtrvDb> = BTreeMap::new();
    for c in changes {
        if !dbs.contains_key(c.stem.as_str()) {
            let f = realm
                .file(&c.stem)
                .ok_or_else(|| RowError::NotFound(format!("file {}", c.stem)))?;
            dbs.insert(c.stem.as_str(), WbtrvDb::open(&f.path)?);
        }
        let now = dbs[c.stem.as_str()].get(c.id)?;
        if now != c.after {
            return Err(RowError::Conflict(format!(
                "{} record {} changed since; not undone",
                c.stem, c.id
            )));
        }
    }
    for c in changes {
        let db = dbs.get_mut(c.stem.as_str()).expect("opened above");
        match (&c.before, &c.after) {
            (Some(b), Some(_)) => db.update(c.id, b)?,
            (None, Some(_)) => db.delete(c.id)?,
            (Some(b), None) => db.restore(c.id, b)?,
            (None, None) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_one_clean_short_line() {
        let t = |s: &str| title_json(&Value::Text(s.into()));
        assert_eq!(
            t("\n  \n\u{1b}[1;33mThe %s\u{1b}[0m greets you\nmore"),
            json!("The %s greets you")
        );
        assert_eq!(t("grey robes\0junk"), json!("grey robes"));
        // an emptied message still holds its old words after the NUL
        assert_eq!(t("\0ou flake off pieces of the steak"), json!(""));
        let long = "x".repeat(70);
        assert_eq!(t(&long), json!(format!("{}...", "x".repeat(60))));
        assert_eq!(title_json(&Value::Long(7)), json!(7));
    }

    #[test]
    fn a_column_holds_what_its_element_does() {
        let range = |table: &str, col: &str| {
            let t = record_table(table).unwrap();
            column_range(t.columns.iter().find(|c| c.name == col).unwrap())
        };
        // a monster's Energy is a VB Integer, its Weapon Number a Long, Undead a Byte
        assert_eq!(range("Monsters", "Energy"), Some((-32768, 32767)));
        assert_eq!(
            range("Monsters", "Weapon Number"),
            Some((i32::MIN.into(), i32::MAX.into()))
        );
        assert_eq!(range("Monsters", "Undead"), Some((0, 255)));
        // NMR clamps an unsigned Experience; a name is text
        assert_eq!(range("Monsters", "Experience"), None);
        assert_eq!(range("Items", "Name"), None);
    }

    #[test]
    fn a_record_is_keyed_as_its_editor_keys_it() {
        use daydream_wcc::tables::Table;
        // each record type is in one table at most
        for t in record_tables() {
            assert_eq!(
                record_tables().filter(|o| o.table == t.table).count(),
                1,
                "{}",
                t.name
            );
        }
        let set = LayoutSet::Dos;
        let item = schema::layout_of(Table::Items, set);
        let mut r = vec![0u8; item.len];
        let f = item.field("number").unwrap();
        r[f.offset..f.offset + 2].copy_from_slice(&868u16.to_le_bytes());
        let (t, key) = record_key(Table::Items, set, &r).unwrap();
        assert_eq!((t.name, key), ("Items", vec![Value::Long(868)]));

        let room = schema::layout_of(Table::Rooms, set);
        let mut r = vec![0u8; room.len];
        for (name, n) in [("mapnum", 1u32), ("roomnum", 224)] {
            let f = room.field(name).unwrap();
            r[f.offset..f.offset + 4].copy_from_slice(&n.to_le_bytes());
        }
        let (t, key) = record_key(Table::Rooms, set, &r).unwrap();
        assert_eq!(t.name, "Rooms");
        assert_eq!(key, vec![Value::Long(1), Value::Long(224)]);
    }

    #[test]
    fn members_are_labelled_as_the_editors_label_their_columns() {
        let table = |name: &str| record_tables().find(|t| t.name == name).unwrap();
        assert_eq!(
            member_labels(table("Items"), &["maxdmg", "speed", "nothing-such"]),
            ["Max Hit", "Speed"]
        );
        // a member of several numbered columns, named once
        assert_eq!(
            member_labels(table("Rooms"), &["desc", "name", "desc"]),
            ["Desc", "Name"]
        );
    }

    #[test]
    fn record_changes_round_trip_through_json() {
        let c = RecordChange {
            stem: "WCCITEMS".into(),
            id: 3,
            before: Some(vec![0, 0xAB, 0xFF]),
            after: None,
        };
        let text = serde_json::to_string(&c).unwrap();
        assert!(text.contains("\"00abff\""), "{text}");
        assert_eq!(serde_json::from_str::<RecordChange>(&text).unwrap(), c);
    }
}
