//! One wbtrv32 `.db` file: a Btrieve file's records in SQLite (schema version 4).
//!
//! ```text
//! metadata_t(record_length, physical_record_length, page_length, variable_length_records, version)
//! keys_t(id, number, segment, attributes, data_type, offset, length, null_value, acs_name, acs)
//! data_t(id INTEGER PRIMARY KEY, data BLOB NOT NULL, key_0 ..., key_1 ...)
//! source_t(size, modified, hash)      -- optional: the .DAT it was converted from
//! ```
//!
//! `data` is the record's bytes, verbatim, so the store is lossless; `id` is the record's
//! physical position, which MajorMUD remembers (`absbtv`) and so must never change for an
//! existing record. Every write recomputes the key columns exactly as wbtrv32 would
//! ([`crate::key`]).

use std::path::{Path, PathBuf};

use daydream_wcc::btrieve::{BtrieveFile, OpenOptions};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params, params_from_iter};

use crate::key::{self, ACS_LENGTH, Key, KeyError, KeyValue, Segment, attr};

/// The schema version Daydream reads and writes (wbtrv32's `CURRENT_VERSION`).
pub const SCHEMA_VERSION: u32 = 4;

/// What working with a `.db` can fail on.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{}: {source}", path.display())]
    Sqlite {
        path: PathBuf,
        source: rusqlite::Error,
    },
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Btrieve(#[from] daydream_wcc::btrieve::BtrieveError),
    #[error("{}: {source}", path.display())]
    Key { path: PathBuf, source: KeyError },
    #[error(
        "{}: schema version {found}; Daydream reads version {SCHEMA_VERSION} (open it once with a current wbtrv32 to upgrade it)",
        path.display()
    )]
    Version { path: PathBuf, found: u32 },
    #[error("{}: no metadata_t row", path.display())]
    NoMetadata { path: PathBuf },
    #[error("{}: a record of {found} bytes; this file's records are {expected}", path.display())]
    RecordLength {
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    #[error("{}: no record {id}", path.display())]
    NoRecord { path: PathBuf, id: i64 },
}

/// `metadata_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    pub record_length: usize,
    pub physical_record_length: usize,
    pub page_length: usize,
    pub variable_length_records: bool,
    pub version: u32,
}

/// What a conversion did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConvertReport {
    /// Records stored.
    pub records: u64,
    /// Records the Btrieve file held that broke a unique key, and that wbtrv32 (and so
    /// Daydream) leaves out.
    pub skipped: u64,
}

/// An open `.db`.
pub struct WbtrvDb {
    path: PathBuf,
    conn: Connection,
    meta: Metadata,
    keys: Vec<Key>,
}

impl std::fmt::Debug for WbtrvDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WbtrvDb")
            .field("path", &self.path)
            .field("meta", &self.meta)
            .field("keys", &self.keys.len())
            .finish_non_exhaustive()
    }
}

/// The connection settings wbtrv32 uses (`initializeConnection`).
const PRAGMAS: &str = "PRAGMA journal_mode = WAL;\
                       PRAGMA synchronous = NORMAL;\
                       PRAGMA cache_size = -32768;\
                       PRAGMA wal_autocheckpoint = 10000;";

impl WbtrvDb {
    /// Opens an existing `.db` for reading and writing.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::open_with(path, false)
    }

    /// Opens an existing `.db` read only.
    pub fn open_read_only(path: &Path) -> Result<Self, StoreError> {
        Self::open_with(path, true)
    }

    fn open_with(path: &Path, read_only: bool) -> Result<Self, StoreError> {
        let sql = sql_err(path);
        let flags = if read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        };
        let conn = Connection::open_with_flags(path, flags | OpenFlags::SQLITE_OPEN_FULL_MUTEX)
            .map_err(sql)?;
        if !read_only {
            conn.execute_batch(PRAGMAS).map_err(sql)?;
        }
        let meta = conn
            .query_row(
                "SELECT record_length, physical_record_length, page_length, \
                 variable_length_records, version FROM metadata_t",
                [],
                |r| {
                    Ok(Metadata {
                        record_length: r.get::<_, i64>(0)? as usize,
                        physical_record_length: r.get::<_, i64>(1)? as usize,
                        page_length: r.get::<_, i64>(2)? as usize,
                        variable_length_records: r.get::<_, i64>(3)? != 0,
                        version: r.get::<_, i64>(4)? as u32,
                    })
                },
            )
            .optional()
            .map_err(sql)?
            .ok_or_else(|| StoreError::NoMetadata {
                path: path.to_path_buf(),
            })?;
        if meta.version != SCHEMA_VERSION {
            return Err(StoreError::Version {
                path: path.to_path_buf(),
                found: meta.version,
            });
        }
        let keys = load_keys(&conn).map_err(sql)?;
        for k in &keys {
            k.validate().map_err(|source| StoreError::Key {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(Self {
            path: path.to_path_buf(),
            conn,
            meta,
            keys,
        })
    }

    /// Converts the Btrieve file `source` (`.DAT` or `.VIR`) into a new `.db` at `dest`,
    /// replacing any file there, as wbtrv32 does when it first opens a `.DAT`.
    pub fn convert(source: &Path, dest: &Path) -> Result<(Self, ConvertReport), StoreError> {
        let mut file = OpenOptions::new().variable_length(true).open(source)?;
        let fingerprint = Fingerprint::of(source).map_err(|e| StoreError::Io {
            path: source.to_path_buf(),
            source: e,
        })?;
        Self::create_from(&mut file, dest, Some(fingerprint))
    }

    /// Writes `file`'s records into a new `.db` at `dest`. `fingerprint` is recorded in
    /// `source_t`, which wbtrv32 checks to tell whether the `.DAT` beside the `.db` changed.
    pub fn create_from(
        file: &mut BtrieveFile,
        dest: &Path,
        fingerprint: Option<Fingerprint>,
    ) -> Result<(Self, ConvertReport), StoreError> {
        let keys = keys_of(file);
        for k in &keys {
            k.validate().map_err(|source| StoreError::Key {
                path: file.path().to_path_buf(),
                source,
            })?;
        }
        let h = &file.header;
        let meta = Metadata {
            record_length: usize::from(h.record_length),
            physical_record_length: usize::from(h.physical_record_length),
            page_length: usize::from(h.page_length),
            variable_length_records: h.is_variable_length(),
            version: SCHEMA_VERSION,
        };
        let Self { mut conn, .. } = Self::create(dest, meta, keys.clone())?;
        let sql = sql_err(dest);

        let mut report = ConvertReport::default();
        let tx = conn.transaction().map_err(sql)?;
        {
            let mut insert = tx.prepare(&insert_sql(&keys)).map_err(sql)?;
            let mut records = file.records();
            while let Some(record) = records.next_record() {
                let record = record?;
                let values = key_values(&keys, record.data).map_err(|source| StoreError::Key {
                    path: dest.to_path_buf(),
                    source,
                })?;
                let data = KeyValue::Blob(record.data.to_vec());
                match insert.execute(params_from_iter(std::iter::once(&data).chain(&values))) {
                    Ok(_) => report.records += 1,
                    // wbtrv32 silently skips records that break a unique key
                    Err(rusqlite::Error::SqliteFailure(e, _))
                        if e.code == rusqlite::ErrorCode::ConstraintViolation =>
                    {
                        report.skipped += 1;
                    }
                    Err(e) => return Err(sql(e)),
                }
            }
        }
        tx.commit().map_err(sql)?;
        if let Some(fp) = fingerprint {
            set_fingerprint(&conn, fp).map_err(sql)?;
        }
        Ok((
            Self {
                path: dest.to_path_buf(),
                conn,
                meta,
                keys,
            },
            report,
        ))
    }

    /// Creates an empty `.db` at `dest` with these record lengths and keys, replacing any
    /// file there.
    pub fn create(dest: &Path, meta: Metadata, keys: Vec<Key>) -> Result<Self, StoreError> {
        for k in &keys {
            k.validate().map_err(|source| StoreError::Key {
                path: dest.to_path_buf(),
                source,
            })?;
        }
        for suffix in ["", "-wal", "-shm"] {
            let mut p = dest.as_os_str().to_owned();
            p.push(suffix);
            match std::fs::remove_file(&p) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    return Err(StoreError::Io {
                        path: PathBuf::from(p),
                        source: e,
                    });
                }
                _ => {}
            }
        }
        let sql = sql_err(dest);
        let conn = Connection::open(dest).map_err(sql)?;
        conn.execute_batch(PRAGMAS).map_err(sql)?;
        let meta = Metadata {
            version: SCHEMA_VERSION,
            ..meta
        };
        create_schema(&conn, &meta, &keys).map_err(sql)?;
        Ok(Self {
            path: dest.to_path_buf(),
            conn,
            meta,
            keys,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn metadata(&self) -> Metadata {
        self.meta
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    /// The number of records.
    pub fn count(&self) -> Result<u64, StoreError> {
        self.conn
            .query_row("SELECT COUNT(*) FROM data_t", [], |r| r.get::<_, i64>(0))
            .map(|n| n as u64)
            .map_err(sql_err(&self.path))
    }

    /// Every record as `(id, data)`, in physical order.
    pub fn records(&self) -> Result<Vec<(i64, Vec<u8>)>, StoreError> {
        self.page(0, i64::MAX)
    }

    /// Up to `limit` records with an id above `after`, in physical order.
    pub fn page(&self, after: i64, limit: i64) -> Result<Vec<(i64, Vec<u8>)>, StoreError> {
        let sql = sql_err(&self.path);
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, data FROM data_t WHERE id > ?1 ORDER BY id LIMIT ?2")
            .map_err(sql)?;
        let rows = stmt
            .query_map(params![after, limit], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(sql)?;
        rows.collect::<Result<_, _>>().map_err(sql)
    }

    /// Record `id`'s bytes, if there is one.
    pub fn get(&self, id: i64) -> Result<Option<Vec<u8>>, StoreError> {
        self.conn
            .query_row("SELECT data FROM data_t WHERE id = ?1", [id], |r| r.get(0))
            .optional()
            .map_err(sql_err(&self.path))
    }

    /// Replaces record `id`'s bytes, recomputing its key columns. Fails, changing nothing,
    /// when a non-modifiable key would change or a unique key would clash.
    pub fn update(&mut self, id: i64, data: &[u8]) -> Result<(), StoreError> {
        self.check_length(data)?;
        let values = key_values(&self.keys, data).map_err(|source| StoreError::Key {
            path: self.path.clone(),
            source,
        })?;
        let mut sets = vec!["data = ?1".to_string()];
        let mut n = 2;
        for k in &self.keys {
            for c in k.column_names() {
                sets.push(format!("{c} = ?{n}"));
                n += 1;
            }
        }
        let sql = format!("UPDATE data_t SET {} WHERE id = ?{n}", sets.join(", "));
        let data = KeyValue::Blob(data.to_vec());
        let idv = KeyValue::Integer(id);
        let changed = self
            .conn
            .execute(
                &sql,
                params_from_iter(std::iter::once(&data).chain(&values).chain([&idv])),
            )
            .map_err(sql_err(&self.path))?;
        if changed == 0 {
            return Err(StoreError::NoRecord {
                path: self.path.clone(),
                id,
            });
        }
        Ok(())
    }

    /// Adds a record and returns its id. An AutoInc key whose bytes are all its null value
    /// is given the next value, written back into the record, as wbtrv32 does.
    pub fn insert(&mut self, data: &[u8]) -> Result<i64, StoreError> {
        self.check_length(data)?;
        let mut data = data.to_vec();
        let sql = sql_err(&self.path);
        let tx = self.conn.transaction().map_err(sql)?;
        for k in &self.keys {
            let s = k.primary();
            if k.is_composite() || s.data_type != key::ty::AUTO_INC {
                continue;
            }
            let (start, len) = (usize::from(s.offset), usize::from(s.length));
            if data[start..start + len].iter().all(|&b| b == s.null_value) {
                // as wbtrv32: MAX + 1, which is NULL (read as 0) on an empty file
                let next: i64 = tx
                    .query_row(
                        &format!("SELECT MAX({}) + 1 FROM data_t", k.name()),
                        [],
                        |r| r.get::<_, Option<i64>>(0),
                    )
                    .map_err(sql)?
                    .unwrap_or(0);
                data[start..start + len].copy_from_slice(&next.to_le_bytes()[..len.min(8)]);
            }
        }
        let values = key_values(&self.keys, &data).map_err(|source| StoreError::Key {
            path: self.path.clone(),
            source,
        })?;
        let blob = KeyValue::Blob(data);
        tx.execute(
            &insert_sql(&self.keys),
            params_from_iter(std::iter::once(&blob).chain(&values)),
        )
        .map_err(sql)?;
        let id = tx.last_insert_rowid();
        tx.commit().map_err(sql)?;
        Ok(id)
    }

    /// Removes record `id`.
    pub fn delete(&mut self, id: i64) -> Result<(), StoreError> {
        let changed = self
            .conn
            .execute("DELETE FROM data_t WHERE id = ?1", [id])
            .map_err(sql_err(&self.path))?;
        if changed == 0 {
            return Err(StoreError::NoRecord {
                path: self.path.clone(),
                id,
            });
        }
        Ok(())
    }

    fn check_length(&self, data: &[u8]) -> Result<(), StoreError> {
        let expected = self.meta.record_length;
        let ok = if self.meta.variable_length_records {
            data.len() >= expected
        } else {
            data.len() == expected
        };
        if ok {
            Ok(())
        } else {
            Err(StoreError::RecordLength {
                path: self.path.clone(),
                expected,
                found: data.len(),
            })
        }
    }
}

fn sql_err(path: &Path) -> impl Fn(rusqlite::Error) -> StoreError + Copy + '_ {
    move |source| StoreError::Sqlite {
        path: path.to_path_buf(),
        source,
    }
}

fn key_values(keys: &[Key], data: &[u8]) -> Result<Vec<KeyValue>, KeyError> {
    let mut out = Vec::new();
    for k in keys {
        out.extend(k.values(data)?);
    }
    Ok(out)
}

fn insert_sql(keys: &[Key]) -> String {
    let mut columns = vec!["data".to_string()];
    for k in keys {
        columns.extend(k.column_names());
    }
    let params: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
    format!(
        "INSERT INTO data_t({}) VALUES({})",
        columns.join(", "),
        params.join(", ")
    )
}

/// The keys of a Btrieve file as wbtrv32 models them. Every key records an ACS in `keys_t`:
/// its own for a MultipleACS key, else the file's (wbtrv32's `loadKeyDefinitions`).
fn keys_of(file: &BtrieveFile) -> Vec<Key> {
    file.keys
        .iter()
        .map(|k| {
            let segments: Vec<Segment> = k
                .segments
                .iter()
                .map(|s| Segment {
                    number: s.number,
                    index: s.segment,
                    attributes: s.attributes.0,
                    data_type: s.data_type.code(),
                    offset: s.offset,
                    length: s.length,
                    null_value: s.null_value,
                    acs: segment_acs(file, s),
                })
                .collect();
            let p = &k.segments[0];
            let multiple = p.attributes.0 & (attr::CASE_INSENSITIVE | attr::NUMBERED_ACS)
                == (attr::CASE_INSENSITIVE | attr::NUMBERED_ACS);
            let (acs_name, acs) = if multiple {
                (p.acs_name.clone(), p.acs.map(|a| a.to_vec()))
            } else {
                match &file.default_acs {
                    Some(a) => (Some(a.name.clone()), Some(a.table.to_vec())),
                    None => (None, None),
                }
            };
            Key {
                segments,
                acs_name,
                acs,
            }
        })
        .collect()
}

/// The ACS a segment sorts by, if it requires one: the reader's (its own or the file's).
fn segment_acs(file: &BtrieveFile, s: &daydream_wcc::btrieve::KeyDef) -> Option<Vec<u8>> {
    if s.attributes.0 & attr::NUMBERED_ACS == 0 {
        return None;
    }
    s.acs
        .map(|a| a.to_vec())
        .or_else(|| file.default_acs.as_ref().map(|a| a.table.to_vec()))
}

fn create_schema(conn: &Connection, meta: &Metadata, keys: &[Key]) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE metadata_t(record_length INTEGER NOT NULL, \
         physical_record_length INTEGER NOT NULL, page_length INTEGER NOT NULL, \
         variable_length_records INTEGER NOT NULL, version INTEGER NOT NULL)",
        [],
    )?;
    conn.execute(
        "INSERT INTO metadata_t(record_length, physical_record_length, page_length, \
         variable_length_records, version) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            meta.record_length as i64,
            meta.physical_record_length as i64,
            meta.page_length as i64,
            i64::from(meta.variable_length_records),
            meta.version
        ],
    )?;
    conn.execute(
        "CREATE TABLE keys_t(id INTEGER PRIMARY KEY, number INTEGER NOT NULL, \
         segment INTEGER NOT NULL, attributes INTEGER NOT NULL, data_type \
         INTEGER NOT NULL, offset INTEGER NOT NULL, length INTEGER NOT NULL, \
         null_value INTEGER NOT NULL, acs_name STRING, acs BLOB, UNIQUE(number, \
         segment))",
        [],
    )?;
    {
        let mut insert = conn.prepare(
            "INSERT INTO keys_t(number, segment, attributes, data_type, offset, \
             length, null_value, acs_name, acs) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for k in keys {
            for s in &k.segments {
                insert.execute(params![
                    s.number,
                    s.index,
                    s.attributes,
                    s.data_type,
                    s.offset,
                    s.length,
                    s.null_value,
                    k.acs_name.as_deref().filter(|n| !n.is_empty()),
                    k.acs.as_deref()
                ])?;
            }
        }
    }
    let mut columns = vec![
        "id INTEGER PRIMARY KEY".to_string(),
        "data BLOB NOT NULL".into(),
    ];
    for k in keys {
        columns.extend(k.column_definitions());
    }
    conn.execute(&format!("CREATE TABLE data_t({});", columns.join(", ")), [])?;
    for k in keys {
        for sql in k.index_sql() {
            conn.execute(&sql, [])?;
        }
    }
    if let Some(sql) = key::trigger_sql(keys) {
        conn.execute(&sql, [])?;
    }
    Ok(())
}

fn load_keys(conn: &Connection) -> rusqlite::Result<Vec<Key>> {
    let mut stmt = conn.prepare(
        "SELECT number, segment, attributes, data_type, offset, length, null_value, \
         acs_name, acs FROM keys_t ORDER BY number, segment",
    )?;
    let mut keys: Vec<Key> = Vec::new();
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)? as u16,
            r.get::<_, i64>(1)? as u16,
            r.get::<_, i64>(2)? as u16,
            r.get::<_, i64>(3)? as u8,
            r.get::<_, i64>(4)? as u16,
            r.get::<_, i64>(5)? as u16,
            r.get::<_, i64>(6)? as u8,
            r.get::<_, Option<String>>(7)?,
            r.get::<_, Option<Vec<u8>>>(8)?,
        ))
    })?;
    for row in rows {
        let (number, index, attributes, data_type, offset, length, null_value, acs_name, acs) =
            row?;
        let mut segment = Segment {
            number,
            index,
            attributes,
            data_type,
            offset,
            length,
            null_value,
            acs: acs.clone(),
        };
        // every segment of a key carries the key's ACS row; only one that requires it uses it
        if !segment.requires_acs() {
            segment.acs = None;
        }
        match keys.last_mut() {
            Some(k) if k.number() == number => k.segments.push(segment),
            _ => keys.push(Key {
                segments: vec![segment],
                acs_name,
                acs: acs.filter(|a| a.len() == ACS_LENGTH),
            }),
        }
    }
    Ok(keys)
}

/// What wbtrv32 records about the `.DAT` a `.db` was converted from (`source_t`). It
/// compares size and modification time first and, when they differ, size and hash, so
/// Daydream records the time as 0 (its clock's epoch is platform-specific) and lets
/// wbtrv32 settle it with the hash on first open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    pub size: i64,
    pub modified: i64,
    /// FNV-1a 64 of the file's bytes.
    pub hash: i64,
}

impl Fingerprint {
    pub fn of(path: &Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(Self::of_bytes(&bytes))
    }

    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self {
            size: bytes.len() as i64,
            modified: 0,
            hash: hash as i64,
        }
    }
}

fn set_fingerprint(conn: &Connection, fp: Fingerprint) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS source_t(size INTEGER NOT NULL, modified INTEGER NOT NULL, hash INTEGER NOT NULL)",
        [],
    )?;
    conn.execute("DELETE FROM source_t", [])?;
    conn.execute(
        "INSERT INTO source_t(size, modified, hash) VALUES(?1, ?2, ?3)",
        params![fp.size, fp.modified, fp.hash],
    )?;
    Ok(())
}
