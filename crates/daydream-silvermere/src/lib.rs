//! A realm's world in silvermere's own database.
//!
//! silvermere (a Rust MajorMUD 1.11p server by the same author) keeps its world in SQLite or
//! PostgreSQL, in a normalised schema. This crate moves a Daydream realm's world into that
//! database ([`export`]) and back ([`import`]), straight in the database and not through
//! silvermere's API, while silvermere is stopped:
//!
//! - **The writer lock.** silvermere's `serve` holds a single-writer guard for as long as it
//!   runs: an exclusive lock on `<database>.lock` beside a SQLite file, or a PostgreSQL
//!   advisory lock. Both directions take the same guard first and refuse while it is held, so
//!   Daydream never writes under a running server or reads a world it is still changing.
//! - **The schema.** silvermere 1.0.1's migrations are copied here ([`db`]); an empty
//!   database is given them, and a database with migrations Daydream does not know (a newer
//!   silvermere) is refused rather than guessed at.
//! - **The tables.** The world content: rooms, items, known monsters, spells, classes, races,
//!   shops, messages, text blocks and actions, written as `silvermere import` writes them,
//!   with their live state (room floors, shop stock, monster kills, items in play) kept or
//!   reset ([`State`]). Players, gangs, bank books, active monsters and help topics are not
//!   touched.
//! - **Lossless.** silvermere's rows do not hold every byte of a record, so an export also
//!   keeps the records' bytes in a table of Daydream's own ([`raw`]), and an import rebuilds
//!   each record over them ([`records`]): a realm that goes out and comes back unchanged
//!   comes back byte for byte, and a change made in silvermere changes only its own bytes.
//!
//! The functions are async (sqlx); [`block_on`] runs one from synchronous code.

#![forbid(unsafe_code)]

#[allow(dead_code)]
pub mod db;
pub mod raw;
pub mod records;
mod sync;

use std::path::Path;

use daydream_store::{Realm, StoreError};
use daydream_types::records::LayoutSet;
use daydream_wcc::tables::{Table, WORLDGROUP_FILES};
use jiff::tz::TimeZone;
use serde::Serialize;
use sqlx::Row;

pub use db::{Db, DbError, Dialect, redact_url};
pub use sync::{State, TableReport};

use db::{TryAcquire, WriterGuard};
use sync::{
    Actions, Classes, Ctx, Items, Messages, Monsters, Races, Rooms, Shops, Spells, TextBlocks,
};

/// What went wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// silvermere is running on the database (or something else holds its writer lock).
    #[error("silvermere is running on {url}: {holder}. Stop it first")]
    Running { url: String, holder: String },
    /// The database has no silvermere schema, or one Daydream does not know.
    #[error("{url}: {reason}")]
    Schema { url: String, reason: String },
    /// A SQLite database to read from is not there.
    #[error("{0}: no such database file")]
    NoDatabase(String),
    /// The realm has no file of a table it is to take in.
    #[error("the realm has no file for {0:?}")]
    NoFile(Table),
    /// A record or row that cannot be converted.
    #[error("{0}")]
    Record(String),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.into())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// How [`export`] writes.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// What happens to the live state of rows silvermere already has.
    pub state: State,
    /// Delete the rows the realm does not have.
    pub delete: bool,
    /// Keep the records' bytes in `daydream_records`, so they come back byte for byte.
    pub raw: bool,
    /// Report what would be written without writing.
    pub dry_run: bool,
    /// The zone the realm's dates are in (silvermere's `import --legacy-tz`).
    pub legacy_tz: TimeZone,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            state: State::Keep,
            delete: true,
            raw: true,
            dry_run: false,
            legacy_tz: TimeZone::UTC,
        }
    }
}

/// How [`import`] writes.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Delete the realm's records silvermere does not have.
    pub delete: bool,
    /// Report what would change without writing.
    pub dry_run: bool,
    /// The zone the realm's dates are in.
    pub legacy_tz: TimeZone,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            delete: true,
            dry_run: false,
            legacy_tz: TimeZone::UTC,
        }
    }
}

/// A connection URL from what a user gives: a `sqlite:`, `postgres:` or `postgresql:` URL
/// as it is, or a path to a SQLite file.
pub fn database_url(given: &str) -> String {
    let given = given.trim();
    if Dialect::from_url(given).is_ok() {
        given.to_string()
    } else {
        format!("sqlite:{given}")
    }
}

/// The SQLite file a URL names, if it names one.
pub fn sqlite_path(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))?;
    let rest = rest.split('?').next().unwrap_or(rest);
    (!rest.is_empty() && !rest.contains(":memory:")).then_some(rest)
}

/// silvermere's database, opened with its writer guard held.
struct Opened {
    db: Db,
    _guard: WriterGuard,
}

/// Takes the writer guard of `url` and opens the database, giving an empty one silvermere's
/// schema when `create`.
async fn open(url: &str, create: bool) -> Result<Opened> {
    let shown = redact_url(url);
    Dialect::from_url(url)?;
    if !create
        && let Some(p) = sqlite_path(url)
        && !Path::new(p).exists()
    {
        return Err(Error::NoDatabase(p.to_string()));
    }
    let guard = match WriterGuard::try_acquire(url).await? {
        TryAcquire::Taken(g) => g,
        TryAcquire::Held(holder) => return Err(Error::Running { url: shown, holder }),
    };
    let db = Db::connect(url).await?;
    check_schema(&db, &shown, create).await?;
    Ok(Opened { db, _guard: guard })
}

/// The migrations Daydream knows.
fn known_versions(dialect: Dialect) -> Vec<i64> {
    dialect.migrator().iter().map(|m| m.version).collect()
}

async fn check_schema(db: &Db, shown: &str, create: bool) -> Result<()> {
    let schema = |reason: String| Error::Schema {
        url: shown.to_string(),
        reason,
    };
    let known = known_versions(db.dialect());
    if !table_exists(db, "_sqlx_migrations").await? {
        return if table_exists(db, "rooms").await? {
            Err(schema(
                "has a rooms table but no migrations: not a silvermere database".into(),
            ))
        } else if create {
            db.migrate().await?;
            Ok(())
        } else {
            Err(schema("no silvermere world here (no schema)".into()))
        };
    }
    // the flag as a number: the Any driver cannot decode a SQLite BOOLEAN
    let rows = sqlx::query(
        "SELECT version, CAST(CASE WHEN success THEN 1 ELSE 0 END AS BIGINT) \
         FROM _sqlx_migrations",
    )
    .fetch_all(&db.pool())
    .await?;
    let mut versions = Vec::new();
    for r in rows {
        let v: i64 = r.try_get(0)?;
        let ok = r.try_get::<i64, _>(1)? != 0;
        if !ok {
            return Err(schema(format!(
                "silvermere's migration {v} did not finish; run `silvermere migrate`"
            )));
        }
        versions.push(v);
    }
    if let Some(v) = versions.iter().find(|v| !known.contains(v)) {
        return Err(schema(format!(
            "written by a newer silvermere (migration {v}); this Daydream knows \
             silvermere 1.0.1's schema (migrations {known:?})"
        )));
    }
    if versions.len() < known.len() {
        if !create {
            return Err(schema(
                "silvermere's schema is not up to date; run `silvermere migrate`".into(),
            ));
        }
        db.migrate().await?;
    }
    Ok(())
}

/// Whether the database (on PostgreSQL, the current schema) has a table `name`.
pub(crate) async fn table_exists(db: &Db, name: &str) -> Result<bool> {
    let sql = match db.dialect() {
        Dialect::Sqlite => "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = $1",
        Dialect::Postgres => {
            "SELECT COUNT(*) FROM information_schema.tables \
             WHERE table_schema = current_schema() AND table_name = $1"
        }
    };
    let n: i64 = sqlx::query_scalar(sql)
        .bind(name)
        .fetch_one(&db.pool())
        .await?;
    Ok(n > 0)
}

/// The layout set of a realm's records: a Worldgroup 3 realm (`WCCITEM2`, ...) holds the
/// aligned layouts.
pub fn layout_of_realm(realm: &Realm) -> LayoutSet {
    let wg = realm
        .files()
        .iter()
        .any(|f| WORLDGROUP_FILES.iter().any(|w| w.stem == f.stem));
    if wg {
        LayoutSet::Worldgroup
    } else {
        LayoutSet::Dos
    }
}

/// Writes `realm`'s world into silvermere's database at `url`, creating silvermere's schema
/// in an empty database. silvermere must be stopped. A preview (`dry_run`) into a SQLite file
/// not made yet says what a new one would take, which is everything, and makes no file.
pub async fn export(realm: &Realm, url: &str, opts: &ExportOptions) -> Result<Vec<TableReport>> {
    if opts.dry_run
        && let Some(p) = sqlite_path(url)
        && !Path::new(p).exists()
    {
        let scratch = Db::connect("sqlite::memory:").await?;
        scratch.migrate().await?;
        return export_tables(realm, &scratch, opts).await;
    }
    let opened = open(url, !opts.dry_run).await?;
    export_tables(realm, &opened.db, opts).await
}

/// The export into an open database.
async fn export_tables(realm: &Realm, db: &Db, opts: &ExportOptions) -> Result<Vec<TableReport>> {
    let ctx = Ctx::new(layout_of_realm(realm), opts.legacy_tz.clone());
    if !opts.dry_run {
        db::world_import::begin(db).await?;
    }
    let mut out = Vec::new();
    // rooms first, as silvermere's import: a cut-off export leaves rooms and an unfinished mark
    if realm.files_of(Table::Rooms).next().is_some() {
        out.push(sync::export_table::<Rooms>(realm, db, &ctx, opts).await?);
    }
    macro_rules! table {
        ($k:ty) => {
            if realm.files_of(<$k as sync::Kind>::TABLE).next().is_some() {
                out.push(sync::export_table::<$k>(realm, db, &ctx, opts).await?);
            }
        };
    }
    table!(Items);
    table!(Monsters);
    table!(Spells);
    table!(Classes);
    table!(Races);
    table!(Shops);
    table!(Messages);
    table!(TextBlocks);
    table!(Actions);
    if !opts.dry_run {
        db::world_import::finish(db).await?;
    }
    db.close().await;
    Ok(out)
}

/// Rewrites `realm`'s world from silvermere's database at `url`: each realm record becomes
/// silvermere's row, rebuilt over the bytes it was exported from (see the crate docs).
/// silvermere must be stopped. Stop the realm too.
pub async fn import(realm: &Realm, url: &str, opts: &ImportOptions) -> Result<Vec<TableReport>> {
    let opened = open(url, false).await?;
    let db = &opened.db;
    let scratch = Db::connect("sqlite::memory:").await?;
    scratch.migrate().await?;
    let ctx = Ctx::new(layout_of_realm(realm), opts.legacy_tz.clone());
    let mut out = Vec::new();
    macro_rules! table {
        ($k:ty) => {
            if realm.files_of(<$k as sync::Kind>::TABLE).next().is_some() {
                out.push(sync::import_table::<$k>(realm, db, &scratch, &ctx, opts).await?);
            }
        };
    }
    table!(Rooms);
    table!(Items);
    table!(Monsters);
    table!(Spells);
    table!(Classes);
    table!(Races);
    table!(Shops);
    table!(Messages);
    table!(TextBlocks);
    table!(Actions);
    scratch.close().await;
    db.close().await;
    Ok(out)
}

/// What a silvermere database holds, for a status line.
#[derive(Debug, Clone, Serialize)]
pub struct Info {
    /// The URL, password masked.
    pub url: String,
    /// The writer guard's holder when silvermere is running.
    pub running: Option<String>,
    /// Rows per world table.
    pub tables: Vec<(&'static str, i64)>,
    /// Whether it keeps the bytes of an export from Daydream.
    pub raw: bool,
}

/// Looks at silvermere's database at `url` without changing it. A running silvermere is
/// reported, and the rows are counted anyway (a count needs no lock).
pub async fn info(url: &str) -> Result<Info> {
    let shown = redact_url(url);
    if let Some(p) = sqlite_path(url)
        && !Path::new(p).exists()
    {
        return Err(Error::NoDatabase(p.to_string()));
    }
    let running = match WriterGuard::try_acquire(url).await? {
        TryAcquire::Taken(g) => {
            g.release().await;
            None
        }
        TryAcquire::Held(holder) => Some(holder),
    };
    let db = Db::connect(url).await?;
    check_schema(&db, &shown, false).await?;
    let mut tables = Vec::new();
    for t in [
        "rooms",
        "items",
        "monsters",
        "spells",
        "classes",
        "races",
        "shops",
        "messages",
        "text_blocks",
        "actions",
    ] {
        let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {t}")))
            .fetch_one(&db.pool())
            .await?;
        tables.push((t, n));
    }
    let raw = raw::exists(&db).await?;
    db.close().await;
    Ok(Info {
        url: shown,
        running,
        tables,
        raw,
    })
}

/// Runs `f` to completion from synchronous code: on the current Tokio runtime when called
/// from one of its blocking threads (`spawn_blocking`), else on a runtime of its own.
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h.block_on(f),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a Tokio runtime")
            .block_on(f),
    }
}
