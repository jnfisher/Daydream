//! Checks that read the wiring, over the whole world ([`daydream_nmr::wiring`]): prices
//! against what is said, spelling, placeholders, spawns nobody reaches. The report is kept
//! until the realm's files change, so asking again is cheap.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /api/checks?check=&level=&offset=&limit=` | each check with its counts, and a page of its findings (warnings first) |
//! | `GET /api/checks/record?table=&key=` | the findings about one record or resting on it |
//! | `POST /api/checks/words` `{"word", "accept"}` | accepts a word the world uses on purpose, so spelling leaves it alone (or takes it back) |
//! | `GET /api/checks/health` | the health checks ([`daydream_nmr::health`]): each one's `{id, level, records, values, list, notes, fix}`, `fix` being the history's name for its fix (null when it has no sure fix) |
//! | `POST /api/checks/health/fix` `{"check", "dry_run"}` | puts a health check right as one step of the history: `{dry_run, check, records, values, tables, notes, reports, step}` (`tables`: each table's `{table, records, values}`) |
//!
//! A finding is `{check, level, about: {table, key}, column, bits, refs, fix, word, suggest,
//! text}`: `bits` are words and `{ref: {t, n, map?}}` records, as the cards' script lines are;
//! `fix` is the values that put it right, when there is a sure one.

use std::collections::HashSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use daydream_nmr::health;
use daydream_nmr::rows;
use daydream_nmr::wiring::{self, Bit, Check, Finding, Level, Rec, Report};
use daydream_store::Realm;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking};

/// The most findings one page gives.
pub const MAX_PAGE: usize = 500;

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn rec_json(x: &Rec) -> Value {
    json!({ "table": x.table, "key": x.key })
}

fn bit_json(b: &Bit) -> Value {
    match b {
        Bit::Text(s) => json!(s),
        Bit::Rec(x) if x.table == "Rooms" => {
            json!({ "ref": { "t": x.table, "n": x.number(), "map": x.key.first() } })
        }
        Bit::Rec(x) => json!({ "ref": { "t": x.table, "n": x.number() } }),
    }
}

/// A finding as the browser shows it.
pub fn finding_json(r: &Report, f: &Finding) -> Value {
    let fix: serde_json::Map<String, Value> = f
        .fix
        .iter()
        .map(|(c, v)| (c.clone(), json!(v.to_string())))
        .collect();
    json!({
        "check": f.check.id(),
        "level": f.level.id(),
        "about": rec_json(&f.about),
        "column": f.column,
        "bits": f.bits.iter().map(bit_json).collect::<Vec<_>>(),
        "refs": f.refs.iter().map(rec_json).collect::<Vec<_>>(),
        "fix": fix,
        "word": f.word,
        "suggest": f.suggest,
        "text": r.text(f),
    })
}

/// The report on the open realm, and its accepted words.
fn report(s: &AppState) -> Result<(Arc<Report>, HashSet<String>), ApiError> {
    let dir = s.realm_dir()?;
    Ok((wiring::report(&dir)?, wiring::accepted(&dir)))
}

/// `?check=&level=&offset=&limit=`.
#[derive(Debug, Deserialize)]
pub struct ChecksQuery {
    #[serde(default)]
    check: Option<String>,
    /// `warn`, `note`, `ok`, or several joined by commas.
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: Option<usize>,
}

fn levels(q: &Option<String>) -> Result<Vec<Level>, ApiError> {
    let all = [Level::Warn, Level::Note, Level::Ok];
    match q.as_deref().filter(|s| !s.is_empty()) {
        None => Ok(all.to_vec()),
        Some(s) => s
            .split(',')
            .map(|l| {
                all.into_iter()
                    .find(|x| x.id() == l.trim())
                    .ok_or_else(|| bad(format!("no level {l}: warn, note or ok")))
            })
            .collect(),
    }
}

/// `GET /api/checks`.
pub(crate) async fn list(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ChecksQuery>,
) -> Result<Json<Value>, ApiError> {
    let check = match q.check.as_deref().filter(|c| !c.is_empty()) {
        None => None,
        Some(c) => Some(Check::parse(c).ok_or_else(|| {
            bad(format!(
                "no check {c}: {}",
                Check::ALL.map(Check::id).join(", ")
            ))
        })?),
    };
    let levels = levels(&q.level)?;
    let limit = q.limit.unwrap_or(50).clamp(1, MAX_PAGE);
    blocking(move || {
        let (r, words) = report(&s)?;
        let live: Vec<&Finding> = r
            .findings
            .iter()
            .filter(|f| wiring::stays(f, &words))
            .collect();
        let checks: Vec<Value> = Check::ALL
            .iter()
            .map(|c| {
                let n = |l: Level| {
                    live.iter()
                        .filter(|f| f.check == *c && f.level == l)
                        .count()
                };
                json!({
                    "id": c.id(),
                    "title": c.title(),
                    "about": c.about(),
                    "warn": n(Level::Warn),
                    "note": n(Level::Note),
                    "ok": n(Level::Ok),
                })
            })
            .collect();
        let mut matching: Vec<&&Finding> = live
            .iter()
            .filter(|f| check.is_none_or(|c| f.check == c) && levels.contains(&f.level))
            .collect();
        matching.sort_by_key(|f| f.level);
        let mut accepted: Vec<&String> = words.iter().collect();
        accepted.sort();
        Ok(Json(json!({
            "checks": checks,
            "total": matching.len(),
            "findings": matching
                .iter()
                .skip(q.offset)
                .take(limit)
                .map(|f| finding_json(&r, f))
                .collect::<Vec<_>>(),
            "words": accepted,
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

/// `GET /api/checks/record`.
pub(crate) async fn record(
    State(s): State<Arc<AppState>>,
    Query(q): Query<RecordQuery>,
) -> Result<Json<Value>, ApiError> {
    let table = rows::record_table(&q.table)?.name;
    let key: Vec<i64> = serde_json::from_str(&q.key)
        .map_err(|_| bad("key must be an array of numbers, as the table's key columns"))?;
    let rec = Rec { table, key };
    blocking(move || {
        let (r, words) = report(&s)?;
        let mut mine: Vec<&Finding> = r
            .findings
            .iter()
            .filter(|f| wiring::stays(f, &words) && f.concerns(&rec))
            .collect();
        mine.sort_by_key(|f| f.level);
        Ok(Json(json!({
            "findings": mine.iter().map(|f| finding_json(&r, f)).collect::<Vec<_>>(),
        })))
    })
    .await
}

/// A word to accept, or take back.
#[derive(Debug, Deserialize)]
pub struct WordBody {
    word: String,
    #[serde(default = "yes")]
    accept: bool,
}

fn yes() -> bool {
    true
}

/// `POST /api/checks/words`: kept beside the working copy, under the side files' lock (it is
/// no save to the game).
pub(crate) async fn words(
    State(s): State<Arc<AppState>>,
    Json(body): Json<WordBody>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let dir = s.realm_dir()?;
        let _lock = s
            .side
            .lock()
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let words = wiring::accept(&dir, &body.word, body.accept).map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidInput {
                bad(e.to_string())
            } else {
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;
        Ok(Json(json!({ "words": words })))
    })
    .await
}

/// `GET /api/checks/health`: kept until the realm's files change, as the wiring's report is.
pub(crate) async fn health(State(s): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let dir = s.realm_dir()?;
        let found = health::cached(&dir).map_err(failed)?;
        let checks: Vec<Value> = found
            .iter()
            .map(|f| {
                json!({
                    "id": f.check.id(),
                    "level": f.level.id(),
                    "records": f.records,
                    "values": f.values,
                    "list": f.list.iter().map(rec_json).collect::<Vec<_>>(),
                    "notes": f.notes,
                    "fix": f.check.fix_label(),
                })
            })
            .collect();
        Ok(Json(json!({ "checks": checks })))
    })
    .await
}

fn failed(e: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// A health check to put right.
#[derive(Debug, Deserialize)]
pub struct FixBody {
    check: String,
    #[serde(default)]
    dry_run: bool,
}

/// `POST /api/checks/health/fix`: every table the fix changes, in one step.
pub(crate) async fn health_fix(
    State(s): State<Arc<AppState>>,
    Json(body): Json<FixBody>,
) -> Result<Json<Value>, ApiError> {
    let check = health::Check::parse(&body.check).ok_or_else(|| {
        bad(format!(
            "no health check {}: {}",
            body.check,
            health::Check::ALL.map(health::Check::id).join(", ")
        ))
    })?;
    let label = check
        .fix_label()
        .ok_or_else(|| bad(format!("{} has no sure fix", check.id())))?;
    let dry_run = body.dry_run;
    blocking(move || {
        let (writing, dir) = s.lock_write(dry_run, label)?;
        let realm = Realm::open(&dir)?;
        let layout = daydream_nmr::layout_of_realm(&realm);
        let plans = health::fix(&realm, check, layout).map_err(failed)?;
        let mut reports = Vec::new();
        if !dry_run {
            for p in &plans {
                let r = daydream_nmr::modify::apply(&realm, p, layout).map_err(failed)?;
                reports.push(rows::report_json(&r));
            }
        }
        let step = writing.finish();
        Ok(Json(json!({
            "dry_run": dry_run,
            "check": check.id(),
            "records": plans.iter().map(|p| p.records()).sum::<usize>(),
            "values": plans.iter().map(|p| p.changes.len()).sum::<usize>(),
            "tables": plans
                .iter()
                .map(|p| json!({ "table": p.table, "records": p.records(), "values": p.changes.len() }))
                .collect::<Vec<_>>(),
            "notes": plans.iter().flat_map(|p| p.notes.iter()).collect::<Vec<_>>(),
            "reports": reports,
            "step": step,
        })))
    })
    .await
}
