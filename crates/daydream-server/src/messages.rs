//! Messages as the editors share them: who uses each one, and a record's own copy of one it
//! shares ([`daydream_nmr::messages`]).
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/messages/users?numbers=1,2&table=&key=[...]` | `{users: {"1": {others, tables: {table: count}, some: [{table, key, title}]}}}`: for each message, the records besides the one asking (`table` and `key`, when given) that use it, a record once however many of its columns name it (a text block whose script names it too): how many, by table, and the first [`LISTED`] |
//! | `POST /api/messages/own` | `{table, key, number, dry_run?}`: a new message copied from `number` for the record, with the names of its other users written in its lines swapped for the record's own (a monster's), and the record's columns that named `number` pointed at the copy, as one save: `{number, lines, columns, swapped, report}` |
//!
//! At most [`MAX_NUMBERS`] numbers are looked up at a time; the rest are left out. Who uses
//! each message is read once and kept until the realm's files change
//! ([`messages::users_index`]).

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use daydream_nmr::messages;
use daydream_nmr::refs;
use daydream_nmr::rows::{self, Edit, cell_json, record_table, report_json, title_json};
use daydream_store::Realm;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::{ApiError, AppState, blocking};

/// The most messages one `users` asks about: a record card's, many times over.
pub const MAX_NUMBERS: usize = 64;
/// The most users of a message listed by name; the rest are counted.
pub const LISTED: usize = 24;

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

/// `?numbers=`, comma separated; `table` and `key`, the record asking, left out of the answer.
#[derive(Debug, Deserialize)]
pub struct UsersQuery {
    numbers: String,
    table: Option<String>,
    key: Option<String>,
}

/// The records that use each message, besides the one asking.
pub(crate) async fn users(
    State(s): State<Arc<AppState>>,
    Query(q): Query<UsersQuery>,
) -> Result<Json<Value>, ApiError> {
    let mut numbers = Vec::new();
    for part in q
        .numbers
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let n: i64 = part
            .parse()
            .map_err(|_| bad(format!("{part} is not a message number")))?;
        if n > 0 && !numbers.contains(&n) && numbers.len() < MAX_NUMBERS {
            numbers.push(n);
        }
    }
    let me = match (&q.table, &q.key) {
        (Some(t), Some(k)) => {
            let t = record_table(t)?;
            let k: Value =
                serde_json::from_str(k).map_err(|e| bad(format!("key is not JSON: {e}")))?;
            Some((t.name, rows::parse_key(t, &k)?))
        }
        (None, None) => None,
        _ => return Err(bad("table and key go together")),
    };
    blocking(move || {
        let index = messages::users_index(&s.realm_dir()?)?;
        let users: Map<String, Value> = numbers
            .iter()
            .map(|n| {
                let others: Vec<&messages::User> = index
                    .get(n)
                    .into_iter()
                    .flatten()
                    .filter(|u| {
                        !me.as_ref()
                            .is_some_and(|(t, k)| u.table == *t && rows::same_key(&u.key, k))
                    })
                    .collect();
                let mut tables: BTreeMap<&str, usize> = BTreeMap::new();
                for u in &others {
                    *tables.entry(u.table).or_default() += 1;
                }
                let some: Vec<Value> = others
                    .iter()
                    .take(LISTED)
                    .map(|u| {
                        json!({
                            "table": u.table,
                            "key": u.key.iter().map(cell_json).collect::<Vec<_>>(),
                            "title": u.title.as_ref().map(title_json),
                        })
                    })
                    .collect();
                (
                    n.to_string(),
                    json!({ "others": others.len(), "tables": tables, "some": some }),
                )
            })
            .collect();
        Ok(Json(json!({ "users": users })))
    })
    .await
}

/// The body of `own`.
#[derive(Debug, Deserialize)]
pub struct OwnBody {
    table: String,
    key: Value,
    number: i64,
    #[serde(default)]
    dry_run: bool,
}

/// Gives a record its own copy of a message it shares.
pub(crate) async fn own(
    State(s): State<Arc<AppState>>,
    Json(b): Json<OwnBody>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let t = record_table(&b.table)?;
        let key = rows::parse_key(t, &b.key)?;
        // as the history lists it: `Give Monsters 2 its own message 9`
        let shown: Vec<String> = key
            .iter()
            .map(|v| match cell_json(v) {
                Value::String(s) => s,
                v => v.to_string(),
            })
            .collect();
        let label = format!(
            "Give {} {} its own message {}",
            t.name,
            shown.join("/"),
            b.number
        );
        let (_writing, dir) = s.lock_write(b.dry_run, &label)?;
        let realm = Realm::open(&dir)?;
        let mut tables = refs::Tables::new(&realm);
        let copy = messages::own_copy(&mut tables, t, &key, b.number)?;
        // the copy's number: after the highest
        let mt = record_table("Messages")?;
        let msgs = tables.rows(mt)?;
        let at = msgs.position("Number");
        let number = msgs
            .rows
            .iter()
            .filter_map(|r| at.and_then(|i| refs::num(&r[i])))
            .max()
            .unwrap_or(0)
            + 1;
        let answer = |report: Value| {
            json!({
                "number": number,
                "lines": copy.lines,
                "columns": copy.columns,
                "swapped": copy.swapped,
                "report": report,
            })
        };
        if b.dry_run {
            return Ok(Json(answer(Value::Null)));
        }
        let [l1, l2, l3] = copy.lines.clone();
        let insert = Edit::Insert {
            table: "Messages".into(),
            values: Map::from_iter([
                ("Number".to_string(), json!(number)),
                ("Line 1".to_string(), json!(l1)),
                ("Line 2".to_string(), json!(l2)),
                ("Line 3".to_string(), json!(l3)),
            ]),
        };
        let (added, changes) = rows::apply(&realm, &insert)?;
        if !added.errors.is_empty() {
            rows::revert(&realm, &changes)?;
            return Err(bad(added.errors.join("; ")));
        }
        let update = Edit::Update {
            table: t.name.to_string(),
            key: b.key.clone(),
            values: copy
                .columns
                .iter()
                .map(|c| (c.clone(), json!(number)))
                .collect(),
        };
        // the record is pointed at the copy, or the copy is taken back
        let report = match rows::commit(&realm, &update) {
            Ok(r) if r.errors.is_empty() => r,
            Ok(r) => {
                rows::revert(&realm, &changes)?;
                return Err(bad(r.errors.join("; ")));
            }
            Err(e) => {
                rows::revert(&realm, &changes)?;
                return Err(e.into());
            }
        };
        Ok(Json(answer(report_json(&report))))
    })
    .await
}
