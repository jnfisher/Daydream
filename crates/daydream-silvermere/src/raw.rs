//! `daydream_records`: each exported record's bytes, kept in silvermere's database beside
//! its row.
//!
//! silvermere's tables hold what the game uses of a record, not every byte of it (see
//! [`crate::records`]). So that a realm comes back from silvermere byte for byte, an export
//! keeps each record it wrote, deflated, in this table of its own, keyed by silvermere's table
//! and the row's key; an import rebuilds each record over these bytes. silvermere never reads
//! or writes the table, so it is left alone by `serve`, `import`, backups and migrations; a
//! row silvermere added has no bytes here and is built over an empty record.

use std::collections::HashMap;

use miniz_oxide::deflate::compress_to_vec;
use miniz_oxide::inflate::decompress_to_vec;
use sqlx::Row;

use crate::Result;
use crate::db::{Db, DbError, Dialect};

/// The table's name in silvermere's database.
pub const TABLE: &str = "daydream_records";

/// Records per insert statement (three values each).
const PER_STATEMENT: usize = 150;

fn create_sql(dialect: Dialect) -> String {
    let blob = match dialect {
        Dialect::Sqlite => "BLOB",
        Dialect::Postgres => "BYTEA",
    };
    format!(
        "CREATE TABLE {TABLE} (\
         kind TEXT NOT NULL, \
         key TEXT NOT NULL, \
         record {blob} NOT NULL, \
         PRIMARY KEY (kind, key))"
    )
}

/// Whether the table is there.
pub async fn exists(db: &Db) -> Result<bool> {
    crate::table_exists(db, TABLE).await
}

/// Replaces the records of `kind` with `records` (key, bytes); with none, only removes.
pub async fn replace(db: &Db, kind: &str, records: &[(String, &[u8])]) -> Result<()> {
    let pool = db.pool();
    if records.is_empty() && !exists(db).await? {
        return Ok(());
    }
    let create = !exists(db).await?;
    let mut tx = pool.begin().await.map_err(DbError::from)?;
    if create {
        sqlx::query(sqlx::AssertSqlSafe(create_sql(db.dialect())))
            .execute(&mut *tx)
            .await
            .map_err(DbError::from)?;
    }
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM {TABLE} WHERE kind = $1"
    )))
    .bind(kind)
    .execute(&mut *tx)
    .await
    .map_err(DbError::from)?;
    for chunk in records.chunks(PER_STATEMENT) {
        let values: Vec<String> = (0..chunk.len())
            .map(|i| format!("(${}, ${}, ${})", 3 * i + 1, 3 * i + 2, 3 * i + 3))
            .collect();
        let sql = format!(
            "INSERT INTO {TABLE} (kind, key, record) VALUES {}",
            values.join(", ")
        );
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for (key, bytes) in chunk {
            q = q
                .bind(kind.to_string())
                .bind(key.clone())
                .bind(compress_to_vec(bytes, 6));
        }
        q.execute(&mut *tx).await.map_err(DbError::from)?;
    }
    tx.commit().await.map_err(DbError::from)?;
    Ok(())
}

/// The records of `kind`, by key.
pub async fn load(db: &Db, kind: &str) -> Result<HashMap<String, Vec<u8>>> {
    if !exists(db).await? {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT key, record FROM {TABLE} WHERE kind = $1"
    )))
    .bind(kind)
    .fetch_all(&db.pool())
    .await
    .map_err(DbError::from)?;
    let mut out = HashMap::with_capacity(rows.len());
    for row in rows {
        let key: String = row.try_get(0).map_err(DbError::from)?;
        let packed: Vec<u8> = row.try_get(1).map_err(DbError::from)?;
        let bytes = decompress_to_vec(&packed).map_err(|e| {
            crate::Error::Record(format!("{TABLE} {kind} {key}: not deflated ({e:?})"))
        })?;
        out.insert(key, bytes);
    }
    Ok(out)
}
