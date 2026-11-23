//! The editors' API: a realm's records as the rows of NMR's tables.
//!
//! The editors work in NMR's terms. Each table is one of NMR's `.mdb` tables, with its
//! columns (names as NMR's forms label them, types and sizes as its Access export makes
//! them), and each record is a row keyed by the table's index columns: the number, map
//! and room for rooms, number and part for text blocks, the name for actions. Reads come
//! from [`daydream_nmr::export_table`], and every write goes through
//! [`daydream_nmr::import`] with its default (strict) rules, so an edit made in the browser
//! lands in the record exactly as the same edit made in Access and imported would.
//!
//! Text is served up to its first NUL, as NMR's forms show it. Saved back unchanged, it
//! leaves the record's bytes alone.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/tables` | every table: name, index columns, columns with type and size |
//! | `GET /api/tables/{table}/rows` | every row's key and title (`Name`, `Line 1`, ...) |
//! | `GET /api/tables/{table}/row?key=[...]` | one row: `{key, values: {column: value}}` |
//! | `PUT /api/tables/{table}/row?key=[...]` | `{values}` over the row; the import's report |
//! | `POST /api/tables/{table}/row` | `{values}` as a new row; the import's report |
//! | `DELETE /api/tables/{table}/row?key=[...]` | removes the row; the report |
//!
//! A key is a JSON array of the index columns' values, in the index's order. A `PUT` may
//! not change the key (that would be a new record); a `POST` of a key the realm already
//! has is refused.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::StatusCode;
use daydream_nmr::schema::{self, ColumnType, NmrTable};
use daydream_nmr::{ImportOptions, NmrData, NmrRows, TableReport, Value as Cell};
use daydream_store::Realm;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::{ApiError, AppState, blocking, not_found};

/// The columns a row list shows besides the key, the first one a table has.
const TITLE_COLUMNS: &[&str] = &["Name", "Line 1", "Data", "Single to User"];

/// The most characters of a title a row list shows.
const TITLE_LEN: usize = 60;

/// A cell as a row list's title: text's first line, without ANSI escapes, and short.
fn title_json(v: &Cell) -> Value {
    let Cell::Text(s) = v else {
        return cell_json(v);
    };
    let line = s
        .split(['\0', '\n', '\r'])
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

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// An NMR table that holds records (every one but `Info`).
fn record_table(name: &str) -> Result<&'static NmrTable, ApiError> {
    schema::table(name)
        .filter(|t| t.table.is_some())
        .ok_or_else(|| not_found(format!("table {name}")))
}

fn index_columns(t: &NmrTable) -> Vec<&'static str> {
    t.index
        .as_ref()
        .map(|i| i.columns.to_vec())
        .unwrap_or_default()
}

/// A cell as the editors see it: text up to its first NUL.
fn cell_json(v: &Cell) -> Value {
    match v {
        Cell::Null => Value::Null,
        Cell::Long(n) => json!(n),
        Cell::Double(d) => json!(d),
        Cell::Text(s) => json!(s.split('\0').next().unwrap_or("")),
    }
}

/// A JSON value as a cell of `ty`.
fn cell_of(column: &str, ty: ColumnType, v: &Value) -> Result<Cell, ApiError> {
    let wrong = || bad(format!("{column}: {v} is not a {ty:?}"));
    Ok(match (ty, v) {
        (_, Value::Null) => Cell::Null,
        (ColumnType::Long, Value::Number(n)) => Cell::Long(
            n.as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or_else(wrong)?,
        ),
        (ColumnType::Double, Value::Number(n)) => Cell::Double(n.as_f64().ok_or_else(wrong)?),
        (ColumnType::Long, Value::String(s)) => Cell::Long(s.trim().parse().map_err(|_| wrong())?),
        (ColumnType::Double, Value::String(s)) => {
            Cell::Double(s.trim().parse().map_err(|_| wrong())?)
        }
        (ColumnType::Text(n), Value::String(s)) => {
            if s.chars().count() > usize::from(n) {
                return Err(bad(format!("{column}: longer than {n} characters")));
            }
            Cell::Text(s.clone())
        }
        (ColumnType::Memo, Value::String(s)) => Cell::Text(s.clone()),
        _ => return Err(wrong()),
    })
}

/// A cell's value when a new row leaves its column out.
fn blank(ty: ColumnType) -> Cell {
    match ty {
        ColumnType::Long => Cell::Long(0),
        ColumnType::Double => Cell::Double(0.0),
        ColumnType::Text(_) | ColumnType::Memo => Cell::Text(String::new()),
    }
}

/// Comparable form of a key: numbers as numbers, text as NMR matches it.
fn key_form(v: &Cell) -> Value {
    match v {
        Cell::Null => json!(0),
        Cell::Long(n) => json!(n),
        Cell::Double(d) => json!(*d as i64),
        Cell::Text(s) => json!(daydream_nmr::export::key_text(s)),
    }
}

fn key_of_row(rows: &NmrRows, row: &[Cell], index: &[&str]) -> Vec<Cell> {
    index
        .iter()
        .map(|c| {
            rows.position(c)
                .map(|i| row[i].clone())
                .unwrap_or(Cell::Null)
        })
        .collect()
}

/// The key in a `?key=` parameter, as cells of the index columns' types.
fn parse_key(t: &NmrTable, raw: &str) -> Result<Vec<Cell>, ApiError> {
    let v: Value = serde_json::from_str(raw).map_err(|e| bad(format!("key is not JSON: {e}")))?;
    let Value::Array(parts) = v else {
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
        .zip(&parts)
        .map(|(c, v)| {
            let col = t.column(c).expect("index columns are columns");
            cell_of(c, col.ty, v)
        })
        .collect()
}

fn same_key(a: &[Cell], b: &[Cell]) -> bool {
    a.iter().map(key_form).eq(b.iter().map(key_form))
}

/// The realm and its rows of one table.
fn load(
    s: &AppState,
    t: &NmrTable,
) -> Result<(Realm, daydream_types::records::LayoutSet, NmrRows), ApiError> {
    let realm = Realm::open(&s.realm_dir)?;
    let layout = daydream_nmr::layout_of_realm(&realm);
    let rows = daydream_nmr::export_table(&realm, t, layout).map_err(internal)?;
    Ok((realm, layout, rows))
}

fn find_row<'a>(rows: &'a NmrRows, index: &[&str], key: &[Cell]) -> Option<&'a Vec<Cell>> {
    rows.rows
        .iter()
        .find(|r| same_key(&key_of_row(rows, r, index), key))
}

pub(crate) fn report_json(r: &TableReport) -> Value {
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

/// Imports one row into the realm with the default (strict) import.
fn write_row(
    realm: &Realm,
    layout: daydream_types::records::LayoutSet,
    t: &NmrTable,
    row: Vec<Cell>,
) -> Result<Value, ApiError> {
    let data = NmrData {
        tables: vec![NmrRows {
            name: t.name.to_string(),
            columns: t.columns.iter().map(|c| c.name.to_string()).collect(),
            rows: vec![row],
        }],
    };
    let opts = ImportOptions {
        layout,
        ..ImportOptions::default()
    };
    let reports = daydream_nmr::import(realm, &data, &opts).map_err(internal)?;
    let report = reports.into_iter().next().unwrap_or_default();
    if !report.errors.is_empty() {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            report.errors.join("; "),
        ));
    }
    Ok(report_json(&report))
}

/// `values` from a request body, over `row`.
fn apply_values(
    t: &NmrTable,
    row: &mut [Cell],
    values: &Map<String, Value>,
) -> Result<(), ApiError> {
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

pub(crate) async fn tables() -> Json<Value> {
    let tables: Vec<Value> = daydream_nmr::columns::TABLES
        .iter()
        .filter(|t| t.table.is_some())
        .map(|t| {
            let columns: Vec<Value> = t
                .columns
                .iter()
                .map(|c| {
                    let (ty, size) = match c.ty {
                        ColumnType::Long => ("long", None),
                        ColumnType::Double => ("double", None),
                        ColumnType::Text(n) => ("text", Some(n)),
                        ColumnType::Memo => ("memo", None),
                    };
                    json!({ "name": c.name, "type": ty, "size": size })
                })
                .collect();
            json!({
                "name": t.name,
                "key": index_columns(t),
                "title": TITLE_COLUMNS.iter().find(|c| t.column(c).is_some()),
                "columns": columns,
            })
        })
        .collect();
    Json(json!({ "tables": tables }))
}

pub(crate) async fn rows(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let t = record_table(&table)?;
        let (_, _, rows) = load(&s, t)?;
        let index = index_columns(t);
        let title = TITLE_COLUMNS.iter().find_map(|c| rows.position(c));
        let list: Vec<Value> = rows
            .rows
            .iter()
            .map(|r| {
                let key: Vec<Value> = key_of_row(&rows, r, &index).iter().map(cell_json).collect();
                json!([key, title.map(|i| title_json(&r[i]))])
            })
            .collect();
        Ok(Json(json!({ "table": t.name, "key": index, "rows": list })))
    })
    .await
}

/// `?key=`.
#[derive(Debug, Deserialize)]
pub struct KeyQuery {
    key: String,
}

pub(crate) async fn row(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Query(q): Query<KeyQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let t = record_table(&table)?;
        let key = parse_key(t, &q.key)?;
        let (_, _, rows) = load(&s, t)?;
        let row = find_row(&rows, &index_columns(t), &key)
            .ok_or_else(|| not_found(format!("{} {}", t.name, q.key)))?;
        let values: Map<String, Value> = rows
            .columns
            .iter()
            .zip(row)
            .map(|(c, v)| (c.clone(), cell_json(v)))
            .collect();
        Ok(Json(json!({
            "table": t.name,
            "key": key.iter().map(cell_json).collect::<Vec<_>>(),
            "values": values,
        })))
    })
    .await
}

/// The body of a `PUT` or `POST`.
#[derive(Debug, Deserialize)]
pub struct RowBody {
    values: Map<String, Value>,
}

pub(crate) async fn update(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Query(q): Query<KeyQuery>,
    Json(body): Json<RowBody>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let _writing = s.write.lock().map_err(internal)?;
        let t = record_table(&table)?;
        let key = parse_key(t, &q.key)?;
        let (realm, layout, rows) = load(&s, t)?;
        let index = index_columns(t);
        let mut row = find_row(&rows, &index, &key)
            .ok_or_else(|| not_found(format!("{} {}", t.name, q.key)))?
            .clone();
        // text the editor did not touch comes back clipped at its NUL, as it was served,
        // and the import leaves such text alone
        for v in row.iter_mut() {
            if let Cell::Text(s) = v {
                s.truncate(s.find('\0').unwrap_or(s.len()));
            }
        }
        apply_values(t, &mut row, &body.values)?;
        if !same_key(&key_of_row(&rows, &row, &index), &key) {
            return Err(bad(
                "a save cannot change the key; insert a new record instead",
            ));
        }
        Ok(Json(write_row(&realm, layout, t, row)?))
    })
    .await
}

pub(crate) async fn insert(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Json(body): Json<RowBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    blocking(move || {
        let _writing = s.write.lock().map_err(internal)?;
        let t = record_table(&table)?;
        let (realm, layout, rows) = load(&s, t)?;
        let index = index_columns(t);
        for c in &index {
            if !body.values.keys().any(|k| k.eq_ignore_ascii_case(c)) {
                return Err(bad(format!("a new {} row needs {c}", t.name)));
            }
        }
        let mut row: Vec<Cell> = t.columns.iter().map(|c| blank(c.ty)).collect();
        apply_values(t, &mut row, &body.values)?;
        let key = key_of_row(&rows, &row, &index);
        if find_row(&rows, &index, &key).is_some() {
            return Err(ApiError(
                StatusCode::CONFLICT,
                format!("{} already has that record", t.name),
            ));
        }
        Ok((
            StatusCode::CREATED,
            Json(write_row(&realm, layout, t, row)?),
        ))
    })
    .await
}

pub(crate) async fn remove(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Query(q): Query<KeyQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let _writing = s.write.lock().map_err(internal)?;
        let t = record_table(&table)?;
        let key = parse_key(t, &q.key)?;
        let realm = Realm::open(&s.realm_dir)?;
        let layout = daydream_nmr::layout_of_realm(&realm);
        let report = daydream_nmr::delete(&realm, t.name, &[key], layout).map_err(internal)?;
        if report.deleted == 0 {
            return Err(not_found(format!("{} {}", t.name, q.key)));
        }
        Ok(Json(report_json(&report)))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_one_clean_short_line() {
        let t = |s: &str| title_json(&Cell::Text(s.into()));
        assert_eq!(
            t("\n  \n\u{1b}[1;33mThe %s\u{1b}[0m greets you\nmore"),
            json!("The %s greets you")
        );
        assert_eq!(t("grey robes\0junk"), json!("grey robes"));
        let long = "x".repeat(70);
        assert_eq!(t(&long), json!(format!("{}...", "x".repeat(60))));
        assert_eq!(title_json(&Cell::Long(7)), json!(7));
    }
}
