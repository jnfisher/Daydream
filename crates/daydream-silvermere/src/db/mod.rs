//! silvermere's database layer, copied from `silvermere-db` 0.1.18 (see `UPSTREAM.md`): the
//! dialects, the single-writer guard, the pool and the world-content repositories.
//!
//! Queries are runtime-checked (`sqlx::query`, bound by position) on the `Any` driver
//! so one query text serves both backends. The portable subset is: `$1` placeholders,
//! `INSERT ... ON CONFLICT ... DO UPDATE SET col = excluded.col`, `RETURNING id`,
//! `lower()`. The dialect branches (case-insensitive keys, booleans) live in [`dialect`].

pub mod dialect;
pub mod guard;
pub mod pool;
pub mod repo;
pub mod world_import;

pub use dialect::{Dialect, redact_url};
pub use guard::{TryAcquire, WriterGuard};
pub use pool::Db;
pub use world_import::WorldImport;

/// The errors of silvermere's database layer.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// A unique key already holds this value (Btrieve's duplicate-key error in the original).
    #[error("duplicate key: {0}")]
    Duplicate(String),
    /// The connection URL names no supported backend.
    #[error("unsupported database URL: {0}")]
    UnsupportedUrl(String),
    /// A stored value does not fit the record field it is read into.
    #[error("{table}.{column}: {reason}")]
    Decode {
        /// The table read.
        table: &'static str,
        /// The column read.
        column: String,
        /// What was wrong.
        reason: String,
    },
    /// Applying the migrations failed.
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    /// Any other database error.
    #[error("database: {0}")]
    Sqlx(sqlx::Error),
}

impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                Self::Duplicate(db.message().to_string())
            }
            other => Self::Sqlx(other),
        }
    }
}

/// `Result` with [`DbError`].
pub type Result<T, E = DbError> = std::result::Result<T, E>;
