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
//! | `GET /api/tables/{table}/rows?columns=` | every row's key and title (`Name`, `Line 1`, ...), and the values of `columns` (comma separated) when asked |
//! | `GET /api/tables/{table}/row?key=[...]` | one row: `{key, values: {column: value}, published}` |
//! | `GET /api/references?table=&key=[...]` | the rows that name a record: `{references: [{table, key, title, column, by}], more}` |
//! | `PUT /api/tables/{table}/row?key=[...]` | `{values}` over the row; the import's report |
//! | `POST /api/tables/{table}/row` | `{values}` as a new row; the import's report |
//! | `DELETE /api/tables/{table}/row?key=[...]` | removes the row; the report |
//!
//! A key is a JSON array of the index columns' values, in the index's order. A `PUT` may
//! not change the key (that would be a new record); a `POST` of a key the realm already
//! has is refused.
//!
//! A row's `published` is what it was when the realm was last read from its game or written
//! back into it (its last publish): `null` when it is as it was then (or the realm's changes
//! are not known), `{added: true}` when it is new since, and `{was: {column: value}}` with the
//! columns that changed since, as they were.
//!
//! The rows that name a record ([`daydream_nmr::refs`]) are those of every table: a column
//! that holds its number, an exit's parameter, an exit into a room, a room that spawns a
//! monster (with no `column`). `by` says how, for a person; at most
//! [`daydream_nmr::refs::MAX`] come back, and `more` says whether there were more.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::StatusCode;
use daydream_nmr::refs;
use daydream_nmr::rows::{
    self, Edit, Published, RowError, cell_json, find_row, index_columns, key_of_row, record_table,
    report_json, title_json,
};
use daydream_store::Realm;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::{ApiError, AppState, blocking, not_found};

impl From<RowError> for ApiError {
    fn from(e: RowError) -> Self {
        let status = match &e {
            RowError::Bad(_) => StatusCode::BAD_REQUEST,
            RowError::NotFound(_) => StatusCode::NOT_FOUND,
            RowError::Conflict(_) => StatusCode::CONFLICT,
            RowError::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            RowError::Store(_) | RowError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiError(status, e.to_string())
    }
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

/// The key in a `?key=` parameter, as cells of the index columns' types.
fn parse_key(
    t: &daydream_nmr::schema::NmrTable,
    raw: &str,
) -> Result<Vec<daydream_nmr::Value>, ApiError> {
    let v: Value = serde_json::from_str(raw).map_err(|e| bad(format!("key is not JSON: {e}")))?;
    Ok(rows::parse_key(t, &v)?)
}

pub(crate) async fn tables() -> Json<Value> {
    let tables: Vec<Value> = rows::record_tables()
        .map(|t| {
            let columns: Vec<Value> = t
                .columns
                .iter()
                .map(|c| {
                    let (ty, size) = rows::column_type(c.ty);
                    json!({ "name": c.name, "type": ty, "size": size })
                })
                .collect();
            json!({
                "name": t.name,
                "key": index_columns(t),
                "title": rows::title_column(t),
                "columns": columns,
            })
        })
        .collect();
    Json(json!({ "tables": tables }))
}

/// `?columns=`: more columns for each row of a row list, comma separated.
#[derive(Debug, Default, Deserialize)]
pub struct RowsQuery {
    columns: Option<String>,
}

pub(crate) async fn rows(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Query(q): Query<RowsQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let t = record_table(&table)?;
        let realm = Realm::open(&s.realm_dir()?)?;
        let (_, rows) = rows::load(&realm, t)?;
        let index = index_columns(t);
        let title = rows::TITLE_COLUMNS.iter().find_map(|c| rows.position(c));
        let more: Vec<usize> = q
            .columns
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|c| !c.is_empty())
            .map(|c| {
                rows.position(c)
                    .ok_or_else(|| bad(format!("{} has no column {c}", t.name)))
            })
            .collect::<Result<_, _>>()?;
        let list: Vec<Value> = rows
            .rows
            .iter()
            .map(|r| {
                let key: Vec<Value> = key_of_row(&rows, r, &index).iter().map(cell_json).collect();
                let title = title.map(|i| title_json(&r[i]));
                if more.is_empty() {
                    json!([key, title])
                } else {
                    json!([
                        key,
                        title,
                        more.iter().map(|&i| cell_json(&r[i])).collect::<Vec<_>>()
                    ])
                }
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
        let realm = Realm::open(&s.realm_dir()?)?;
        let (_, rows) = rows::load(&realm, t)?;
        let row = find_row(&rows, &index_columns(t), &key)
            .ok_or_else(|| not_found(format!("{} {}", t.name, q.key)))?;
        let published = match rows::published(&realm, t, &key)? {
            Published::Same => Value::Null,
            Published::Added => json!({ "added": true }),
            Published::Was(was) => {
                // the columns that look different: text up to its NUL, as the editors show it
                let changed: Map<String, Value> = rows
                    .columns
                    .iter()
                    .zip(row.iter().zip(&was))
                    .filter(|(_, (now, then))| cell_json(now) != cell_json(then))
                    .map(|(c, (_, then))| (c.clone(), cell_json(then)))
                    .collect();
                if changed.is_empty() {
                    Value::Null
                } else {
                    json!({ "was": changed })
                }
            }
        };
        Ok(Json(json!({
            "table": t.name,
            "key": key.iter().map(cell_json).collect::<Vec<_>>(),
            "values": rows::row_json(&rows, row),
            "published": published,
        })))
    })
    .await
}

/// `?table=&key=`.
#[derive(Debug, Deserialize)]
pub struct RecordQuery {
    table: String,
    key: String,
}

/// The rows of every table that name a record.
pub(crate) async fn references(
    State(s): State<Arc<AppState>>,
    Query(q): Query<RecordQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let t = record_table(&q.table)?;
        let key = parse_key(t, &q.key)?;
        let realm = Realm::open(&s.realm_dir()?)?;
        let mut tables = refs::Tables::new(&realm);
        if find_row(tables.rows(t)?, &index_columns(t), &key).is_none() {
            return Err(not_found(format!("{} {}", t.name, q.key)));
        }
        let search: Vec<_> = rows::record_tables().collect();
        let found = refs::pointing_at(&mut tables, &search, t, &key)?;
        let list: Vec<Value> = found
            .list
            .iter()
            .map(|r| {
                json!({
                    "table": r.table,
                    "key": r.key.iter().map(cell_json).collect::<Vec<_>>(),
                    "title": r.title.as_ref().map(title_json),
                    "column": r.column,
                    "by": r.by,
                })
            })
            .collect();
        Ok(Json(json!({
            "table": t.name,
            "key": key.iter().map(cell_json).collect::<Vec<_>>(),
            "references": list,
            "more": found.more,
        })))
    })
    .await
}

/// The body of a `PUT` or `POST`.
#[derive(Debug, Deserialize)]
pub struct RowBody {
    values: Map<String, Value>,
}

/// `?key=` as the JSON an [`Edit`] takes.
fn key_json(raw: &str) -> Result<Value, ApiError> {
    serde_json::from_str(raw).map_err(|e| bad(format!("key is not JSON: {e}")))
}

/// A key for a person: `12`, `1/5`.
fn key_text(key: &Value) -> String {
    let one = |v: &Value| match v {
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    match key {
        Value::Array(parts) => parts.iter().map(one).collect::<Vec<_>>().join("/"),
        v => one(v),
    }
}

/// What a save is, for the history: `Save Items 12`.
fn label(edit: &Edit) -> String {
    match edit {
        Edit::Update { table, key, .. } => format!("Save {table} {}", key_text(key)),
        Edit::Insert { table, .. } => format!("Add to {table}"),
        Edit::Delete { table, key } => format!("Delete {table} {}", key_text(key)),
    }
}

/// Makes an edit under the write lock: one step of the history.
fn commit(s: &AppState, edit: &Edit) -> Result<Value, ApiError> {
    let (_writing, dir) = s.lock_write(false, &label(edit))?;
    let realm = Realm::open(&dir)?;
    Ok(report_json(&rows::commit(&realm, edit)?))
}

pub(crate) async fn update(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Query(q): Query<KeyQuery>,
    Json(body): Json<RowBody>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let edit = Edit::Update {
            table,
            key: key_json(&q.key)?,
            values: body.values,
        };
        Ok(Json(commit(&s, &edit)?))
    })
    .await
}

pub(crate) async fn insert(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Json(body): Json<RowBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    blocking(move || {
        let edit = Edit::Insert {
            table,
            values: body.values,
        };
        Ok((StatusCode::CREATED, Json(commit(&s, &edit)?)))
    })
    .await
}

pub(crate) async fn remove(
    State(s): State<Arc<AppState>>,
    UrlPath(table): UrlPath<String>,
    Query(q): Query<KeyQuery>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let edit = Edit::Delete {
            table,
            key: key_json(&q.key)?,
        };
        Ok(Json(commit(&s, &edit)?))
    })
    .await
}
