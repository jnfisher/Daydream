//! NMR's database tools in the browser: the Database Importer, the Database Deleter, Merge
//! Users and the Universal Modifier, over the same library calls the command line uses.
//!
//! | Route | Answer |
//! |---|---|
//! | `POST /api/tools/import?options={...}` | body: a `.mdb` or NMR JSON export; the import's reports |
//! | `POST /api/tools/delete` | `{table, range?, dry_run}`; the deleter's report |
//! | `POST /api/tools/merge-users?dry_run=` | body: a user file (`.DAT` or `.db`); what was copied |
//! | `POST /api/tools/modify` | a [`Modify`](daydream_nmr::modify::Modify) and `dry_run`; every change, then the write's report |
//! | `POST /api/tools/preset` | one of NMR's fixes ([`Preset`](daydream_nmr::presets::Preset)) and `dry_run`; as `modify` |
//! | `POST /api/map/dig` | a [`Dig`](daydream_nmr::map::Dig) and `dry_run`; as `modify` |
//! | `POST /api/tools/renumber` | a [`Renumber`](daydream_nmr::renumber::Renumber) and `dry_run`; `{dry_run, tables: [as modify, per table]}` |
//! | `POST /api/tools/write-dat` | `{wcc, out?, backup, dry_run, force}`; `{files: [{stem, dest, action, records}], backup?}` |
//!
//! `options` is JSON: `tables` (names, or absent for all), `ranges` (`[{table, map?, from,
//! to}]`), `skip_existing`, `room_parts` (`all`, `only_items`, `not_items`), `extra_parts`
//! and `dry_run`. Like the editors, the tools write the realm's files: stop the realm first.
//! `write-dat` writes the realm back into its Btrieve files (`daydream write-dat`): `wcc`
//! is the folder the realm was converted from, on the machine the server runs on.

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use daydream_nmr::{ImportOptions, NmrData, Range, RoomParts};
use daydream_store::{Realm, WbtrvDb};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState, blocking};

/// The largest file the tools take: a whole realm's `.mdb` is about 64 MB.
pub const MAX_UPLOAD: usize = 512 * 1024 * 1024;

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// One table's range, as the UI sends it.
#[derive(Debug, Deserialize)]
pub struct RangeJson {
    table: String,
    #[serde(default)]
    map: Option<i64>,
    from: i64,
    to: i64,
}

impl From<RangeJson> for Range {
    fn from(r: RangeJson) -> Self {
        Range {
            table: r.table,
            map: r.map,
            from: r.from,
            to: r.to,
        }
    }
}

/// The importer's options, as the UI sends them.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ImportJson {
    tables: Option<Vec<String>>,
    ranges: Vec<RangeJson>,
    skip_existing: bool,
    room_parts: Option<String>,
    extra_parts: bool,
    dry_run: bool,
}

/// `?options=`.
#[derive(Debug, Deserialize)]
pub struct ImportQuery {
    #[serde(default)]
    options: Option<String>,
}

/// The file's tables: NMR's JSON export when it parses as JSON, else a `.mdb`.
fn read_export(body: &[u8]) -> Result<NmrData, ApiError> {
    if body.first() == Some(&b'{') {
        return serde_json::from_slice(body)
            .map_err(|e| bad(format!("not an NMR JSON export: {e}")));
    }
    let dir = tempfile::tempdir().map_err(internal)?;
    let path = dir.path().join("import.mdb");
    std::fs::write(&path, body).map_err(internal)?;
    daydream_nmr::mdb::read(&path).map_err(|e| bad(format!("not an NMR .mdb export: {e}")))
}

pub(crate) async fn import(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ImportQuery>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let o: ImportJson = match q.options.as_deref() {
            Some(text) => serde_json::from_str(text).map_err(|e| bad(format!("options: {e}")))?,
            None => ImportJson::default(),
        };
        if body.is_empty() {
            return Err(bad("send the .mdb (or JSON export) as the request body"));
        }
        let data = read_export(&body)?;
        let _writing = s.write.lock().map_err(internal)?;
        let realm = Realm::open(&s.realm_dir)?;
        let opts = ImportOptions {
            skip_existing: o.skip_existing,
            dry_run: o.dry_run,
            layout: daydream_nmr::layout_of_realm(&realm),
            like_nmr: false,
            tables: o.tables,
            ranges: o.ranges.into_iter().map(Range::from).collect(),
            room_parts: match o.room_parts.as_deref() {
                None | Some("all") => RoomParts::All,
                Some("only_items") => RoomParts::OnlyItems,
                Some("not_items") => RoomParts::NotItems,
                Some(other) => return Err(bad(format!("room_parts: {other}?"))),
            },
            extra_parts: o.extra_parts,
        };
        let reports = daydream_nmr::import(&realm, &data, &opts).map_err(|e| bad(e.to_string()))?;
        let reports: Vec<Value> = reports.iter().map(crate::tables::report_json).collect();
        Ok(Json(json!({ "dry_run": opts.dry_run, "reports": reports })))
    })
    .await
}

/// The body of a delete.
#[derive(Debug, Deserialize)]
pub struct DeleteJson {
    table: String,
    #[serde(default)]
    range: Option<RangeJson>,
    /// Must be true to delete a whole table, so a missing range never empties one.
    #[serde(default)]
    all: bool,
    #[serde(default)]
    dry_run: bool,
}

pub(crate) async fn delete(
    State(s): State<Arc<AppState>>,
    Json(body): Json<DeleteJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        if body.range.is_none() && !body.all {
            return Err(bad("give a range, or all: true to delete every record"));
        }
        let _writing = s.write.lock().map_err(internal)?;
        let realm = Realm::open(&s.realm_dir)?;
        let range = body.range.map(|mut r| {
            r.table = body.table.clone();
            Range::from(r)
        });
        let report = daydream_nmr::delete_range(
            &realm,
            &body.table,
            range.as_ref(),
            daydream_nmr::layout_of_realm(&realm),
            body.dry_run,
        )
        .map_err(|e| bad(e.to_string()))?;
        Ok(Json(json!({
            "dry_run": body.dry_run,
            "report": crate::tables::report_json(&report),
        })))
    })
    .await
}

/// `?dry_run=`.
#[derive(Debug, Deserialize)]
pub struct DryRun {
    #[serde(default)]
    dry_run: bool,
}

pub(crate) async fn merge_users(
    State(s): State<Arc<AppState>>,
    Query(q): Query<DryRun>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        if body.is_empty() {
            return Err(bad("send the user file as the request body"));
        }
        let dir = tempfile::tempdir().map_err(internal)?;
        // a .db is SQLite; anything else is taken for a Btrieve file and converted
        let source_path = if body.starts_with(b"SQLite format 3\0") {
            let p = dir.path().join("users.db");
            std::fs::write(&p, &body).map_err(internal)?;
            p
        } else {
            let dat = dir.path().join("users.dat");
            std::fs::write(&dat, &body).map_err(internal)?;
            let p = dir.path().join("users.db");
            WbtrvDb::convert(&dat, &p).map_err(|e| bad(format!("not a user file: {e}")))?;
            p
        };
        let _writing = s.write.lock().map_err(internal)?;
        let realm = Realm::open(&s.realm_dir)?;
        let into_path = realm
            .files_of(daydream_wcc::tables::Table::Users)
            .next()
            .map(|f| f.path.clone())
            .ok_or_else(|| bad("the realm has no user file"))?;
        let source = WbtrvDb::open_read_only(&source_path)?;
        let mut into = WbtrvDb::open(&into_path)?;
        let r = daydream_store::merge::merge(&source, &mut into, q.dry_run)
            .map_err(|e| bad(e.to_string()))?;
        let skipped: Vec<String> = r
            .skipped
            .iter()
            .map(|k| {
                String::from_utf8_lossy(k)
                    .trim_end_matches('\0')
                    .to_string()
            })
            .collect();
        Ok(Json(
            json!({ "dry_run": q.dry_run, "copied": r.copied, "skipped": skipped }),
        ))
    })
    .await
}

/// The body of a Universal Modifier run: the run itself, and whether to only preview it.
#[derive(Debug, Deserialize)]
pub struct ModifyJson {
    #[serde(flatten)]
    modify: daydream_nmr::modify::Modify,
    #[serde(default)]
    dry_run: bool,
}

/// The most changes an answer lists; the count is always whole.
const MAX_LISTED: usize = 2000;

pub(crate) async fn modify(
    State(s): State<Arc<AppState>>,
    Json(body): Json<ModifyJson>,
) -> Result<Json<Value>, ApiError> {
    run_plan(s, body.dry_run, move |realm, layout| {
        daydream_nmr::modify::plan(realm, &body.modify, layout).map_err(|e| bad(e.to_string()))
    })
    .await
}

/// The body of one of NMR's fixes: the fix, and whether to only preview it.
#[derive(Debug, Deserialize)]
pub struct PresetJson {
    #[serde(flatten)]
    preset: daydream_nmr::presets::Preset,
    #[serde(default)]
    dry_run: bool,
}

pub(crate) async fn preset(
    State(s): State<Arc<AppState>>,
    Json(body): Json<PresetJson>,
) -> Result<Json<Value>, ApiError> {
    run_plan(s, body.dry_run, move |realm, layout| {
        daydream_nmr::presets::plan(realm, &body.preset, layout).map_err(|e| bad(e.to_string()))
    })
    .await
}

/// The body of a Map Editor dig.
#[derive(Debug, Deserialize)]
pub struct DigJson {
    #[serde(flatten)]
    dig: daydream_nmr::map::Dig,
    #[serde(default)]
    dry_run: bool,
}

pub(crate) async fn dig(
    State(s): State<Arc<AppState>>,
    Json(body): Json<DigJson>,
) -> Result<Json<Value>, ApiError> {
    run_plan(s, body.dry_run, move |realm, layout| {
        daydream_nmr::map::dig(realm, layout, &body.dig).map_err(|e| bad(e.to_string()))
    })
    .await
}

/// The body of a Record Number Changer run.
#[derive(Debug, Deserialize)]
pub struct RenumberJson {
    #[serde(flatten)]
    run: daydream_nmr::renumber::Renumber,
    #[serde(default)]
    dry_run: bool,
}

/// One plan's answer, as `modify` gives it.
fn plan_json(plan: &daydream_nmr::modify::Plan, dry_run: bool, report: Option<Value>) -> Value {
    let listed = &plan.changes[..plan.changes.len().min(MAX_LISTED)];
    json!({
        "dry_run": dry_run,
        "table": plan.table,
        "changed": plan.changes.len(),
        "records": plan.records(),
        "skipped": plan.skipped,
        "unchanged": plan.unchanged,
        "notes": plan.notes,
        "changes": listed,
        "report": report,
    })
}

pub(crate) async fn renumber(
    State(s): State<Arc<AppState>>,
    Json(body): Json<RenumberJson>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let _writing = s.write.lock().map_err(internal)?;
        let realm = Realm::open(&s.realm_dir)?;
        let layout = daydream_nmr::layout_of_realm(&realm);
        let plans = daydream_nmr::renumber::plan(&realm, &body.run, layout)
            .map_err(|e| bad(e.to_string()))?;
        let reports: Vec<Option<Value>> = if body.dry_run {
            vec![None; plans.len()]
        } else {
            daydream_nmr::renumber::apply(&realm, &plans, layout)
                .map_err(|e| bad(e.to_string()))?
                .iter()
                .map(|r| Some(crate::tables::report_json(r)))
                .collect()
        };
        let tables: Vec<Value> = plans
            .iter()
            .zip(reports)
            .map(|(p, r)| plan_json(p, body.dry_run, r))
            .collect();
        Ok(Json(json!({ "dry_run": body.dry_run, "tables": tables })))
    })
    .await
}

/// Plans a run, writes it unless `dry_run`, and answers with what changes.
async fn run_plan(
    s: Arc<AppState>,
    dry_run: bool,
    plan: impl FnOnce(
        &Realm,
        daydream_types::records::LayoutSet,
    ) -> Result<daydream_nmr::modify::Plan, ApiError>
    + Send
    + 'static,
) -> Result<Json<Value>, ApiError> {
    blocking(move || {
        let _writing = s.write.lock().map_err(internal)?;
        let realm = Realm::open(&s.realm_dir)?;
        let layout = daydream_nmr::layout_of_realm(&realm);
        let plan = plan(&realm, layout)?;
        let report = if dry_run {
            None
        } else {
            let r = daydream_nmr::modify::apply(&realm, &plan, layout)
                .map_err(|e| bad(e.to_string()))?;
            Some(crate::tables::report_json(&r))
        };
        Ok(Json(plan_json(&plan, dry_run, report)))
    })
    .await
}

/// The body of `write-dat`.
#[derive(Debug, Deserialize)]
pub struct WriteDatJson {
    wcc: String,
    #[serde(default)]
    out: Option<String>,
    #[serde(default = "yes")]
    backup: bool,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    force: bool,
}

fn yes() -> bool {
    true
}

pub(crate) async fn write_dat(
    State(s): State<Arc<AppState>>,
    Json(body): Json<WriteDatJson>,
) -> Result<Json<Value>, ApiError> {
    use daydream_store::dat::{Action, DatError, WriteOptions};
    blocking(move || {
        let wcc = std::path::PathBuf::from(body.wcc.trim());
        if body.wcc.trim().is_empty() || !wcc.is_dir() {
            return Err(bad(format!("{}: not a folder", wcc.display())));
        }
        let out = body
            .out
            .map(|o| o.trim().to_string())
            .filter(|o| !o.is_empty())
            .map(std::path::PathBuf::from);
        let backup = (out.is_none() && body.backup).then(|| {
            let at = jiff::Zoned::now().strftime("%Y%m%d-%H%M%S").to_string();
            wcc.join(format!("daydream-backup-{at}"))
        });
        let options = WriteOptions {
            out,
            backup: backup.clone(),
            dry_run: body.dry_run,
            force: body.force,
        };
        let _writing = s.write.lock().map_err(internal)?;
        let realm = Realm::open(&s.realm_dir)?;
        let written =
            daydream_store::dat::write_dat(&realm, &wcc, &options).map_err(|e| match e {
                DatError::NoSource { .. }
                | DatError::SourceChanged { .. }
                | DatError::Build { .. } => ApiError(StatusCode::CONFLICT, e.to_string()),
                e => internal(e),
            })?;
        let files: Vec<Value> = written
            .iter()
            .map(|w| {
                let (action, records) = match w.action {
                    Action::Unchanged => ("unchanged", None),
                    Action::Updated(n) => ("updated", Some(n)),
                    Action::Rebuilt(n) => ("rebuilt", Some(n)),
                };
                json!({
                    "stem": w.stem,
                    "dest": w.dest.display().to_string(),
                    "action": action,
                    "records": records,
                })
            })
            .collect();
        let backup = backup
            .filter(|b| b.exists())
            .map(|b| b.display().to_string());
        Ok(Json(
            json!({ "dry_run": body.dry_run, "files": files, "backup": backup }),
        ))
    })
    .await
}
