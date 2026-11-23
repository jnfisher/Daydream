//! Realms for the tests.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use daydream_nmr::columns::TABLES;
use daydream_nmr::schema::{Source, Value, Vb, layout_of, scatter};
use daydream_nmr::{NmrData, Value as V};
use daydream_store::{Metadata, Realm, WbtrvDb};
use daydream_types::records::LayoutSet;
use daydream_wcc::tables::{FILES, Table};

/// A deterministic byte source.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    pub fn fill(&mut self, bytes: &mut [u8]) {
        for b in bytes {
            *b = self.next() as u8;
        }
    }
}

/// The stem NMR's records of `table` live in.
pub fn stem(table: Table) -> &'static str {
    FILES.iter().find(|f| f.table == table).unwrap().stem
}

/// An empty `.db` for every NMR table (no Btrieve keys: the tests match by NMR's index).
pub fn empty_realm(dir: &Path) {
    for t in TABLES.iter().chain(daydream_nmr::players::TABLES) {
        let Some(table) = t.table else { continue };
        let layout = layout_of(table, LayoutSet::Dos);
        let variable = table.is_variable_length();
        let fixed = if variable {
            layout.field("text").unwrap().offset
        } else {
            layout.len
        };
        let meta = Metadata {
            record_length: fixed,
            physical_record_length: fixed,
            page_length: 4096,
            variable_length_records: variable,
            version: 4,
        };
        WbtrvDb::create(&dir.join(format!("{}.db", stem(table))), meta, Vec::new()).unwrap();
    }
}

/// A realm of `n` random records per table, each with a distinct index key (a text key is
/// a short name, NUL-terminated).
pub fn random_realm(dir: &Path, n: usize, seed: u64) {
    empty_realm(dir);
    let mut rng = Lcg(seed);
    for t in TABLES {
        let Some(table) = t.table else { continue };
        let layout = layout_of(table, LayoutSet::Dos);
        let mut db = WbtrvDb::open(&dir.join(format!("{}.db", stem(table)))).unwrap();
        let index = t.index.as_ref().unwrap();
        for i in 0..n {
            let mut record = vec![0u8; layout.len];
            rng.fill(&mut record);
            for (k, name) in index.columns.iter().enumerate() {
                let c = t.column(name).unwrap();
                let Source::Element { vb, pieces } = &c.source else {
                    panic!("{name}: not an element")
                };
                let len = pieces.iter().map(|p| p.len).sum();
                let value = match vb {
                    Vb::Text => V::Text(format!("act{i}")),
                    _ => V::Long((i as i32 + 1) * if k == 0 { 1 } else { 7 }),
                };
                scatter(layout, pieces, &vb.write(&value, len).unwrap(), &mut record);
            }
            if table.is_variable_length() {
                // a stored text of random length, then the terminator
                let f = layout.field("text").unwrap();
                let used = (rng.next() as usize) % 300;
                record.truncate(f.offset + used + 1);
                record[f.offset + used] = 0;
            }
            db.insert(&record).unwrap();
        }
    }
}

/// One file's records: id and bytes.
pub type Records = Vec<(i64, Vec<u8>)>;

/// Every record of every `.db` in a realm, by stem.
pub fn records(dir: &Path) -> Vec<(String, Records)> {
    Realm::open(dir)
        .unwrap()
        .files()
        .iter()
        .map(|f| {
            let db = WbtrvDb::open_read_only(&f.path).unwrap();
            (f.stem.clone(), db.records().unwrap())
        })
        .collect()
}

/// Copies a realm's `.db` files.
pub fn copy_realm(from: &Path, to: &Path) {
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let name = e.file_name();
        if name.to_string_lossy().ends_with(".db") {
            std::fs::copy(e.path(), to.join(name)).unwrap();
        }
    }
}

/// A value's text, for messages.
pub fn show(v: &Value) -> String {
    format!("{v:?}")
}

/// Asserts two exports hold the same tables, columns and rows (in any row order).
pub fn assert_same_data(a: &NmrData, b: &NmrData) {
    assert_eq!(a.tables.len(), b.tables.len());
    for (x, y) in a.tables.iter().zip(&b.tables) {
        assert_eq!(x.name, y.name);
        assert_eq!(x.columns, y.columns, "{}", x.name);
        assert_eq!(x.rows.len(), y.rows.len(), "{}: row count", x.name);
        let key = |r: &Vec<Value>| format!("{r:?}");
        let mut xr: Vec<_> = x.rows.iter().map(key).collect();
        let mut yr: Vec<_> = y.rows.iter().map(key).collect();
        xr.sort();
        yr.sort();
        for (i, (p, q)) in xr.iter().zip(&yr).enumerate() {
            assert_eq!(p, q, "{}: sorted row {i}", x.name);
        }
    }
}

/// The stock 1.11p realm, converted once per test run.
pub fn stock_realm() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let vir = daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| {
            panic!(
                "the tests need the stock distribution and could not get it: {e}\n\
                 (cache directory {}; set {} to use another)",
                daydream_wcc::stock_files::cache_dir().display(),
                daydream_wcc::stock_files::CACHE_ENV
            )
        });
        let dir = std::env::temp_dir().join(format!("daydream-nmr-stock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        daydream_store::realm::convert_dir(&vir, &dir).unwrap();
        dir
    })
    .clone()
}

/// What [`add_cases`] changed in one table.
#[derive(Debug, Default)]
pub struct Cases {
    pub table: String,
    pub edited: usize,
    pub added: usize,
}

/// Turns an export into a batch of test cases, all in one file: in every world table,
/// every fourth row takes each of its element columns (all but the index) from another
/// row, so every column changes somewhere to a value real data holds; and two new rows,
/// copies of the first with new index keys, are added after the last.
pub fn add_cases(data: &mut NmrData) -> Vec<Cases> {
    let mut out = Vec::new();
    for rows in &mut data.tables {
        let Some(t) = daydream_nmr::schema::table(&rows.name) else {
            continue;
        };
        if t.table.is_none() || rows.rows.is_empty() {
            continue;
        }
        let index = t.index.as_ref().unwrap();
        let at = |name: &str| rows.position(name);
        let key_cols: Vec<usize> = index.columns.iter().filter_map(|c| at(c)).collect();
        let editable: Vec<usize> = t
            .columns
            .iter()
            .filter(|c| {
                matches!(
                    c.source,
                    Source::Element { .. } | Source::Unsigned(_) | Source::TextblockData
                )
            })
            .filter_map(|c| at(c.name))
            .filter(|i| !key_cols.contains(i))
            .collect();
        let n = rows.rows.len();
        let mut cases = Cases {
            table: rows.name.clone(),
            ..Cases::default()
        };
        let donors: Vec<Vec<Value>> = rows.rows.clone();
        for (i, row) in rows.rows.iter_mut().enumerate() {
            if i % 4 != 1 {
                continue;
            }
            let donor = &donors[(i * 7 + 3) % n];
            for &c in &editable {
                row[c] = donor[c].clone();
            }
            cases.edited += 1;
        }
        let first = key_cols[0];
        let max = rows
            .rows
            .iter()
            .filter_map(|r| match r[first] {
                V::Long(x) => Some(x),
                _ => None,
            })
            .max();
        for k in 1..=2 {
            let mut row = rows.rows[0].clone();
            row[first] = match (&row[first], max) {
                (V::Long(_), Some(m)) => V::Long(m + k),
                // as exported: the name, then NULs to the column's width
                (V::Text(t), _) => {
                    let name = format!("daydream{k}");
                    let width = t.chars().count().max(name.len());
                    V::Text(format!("{name:\0<width$}"))
                }
                (v, _) => panic!("{}: index value {v:?}", rows.name),
            };
            rows.rows.push(row);
            cases.added += 1;
        }
        out.push(cases);
    }
    out
}

/// Rows of `name`, every column zero or empty but those given.
pub fn put(realm: &Realm, name: &str, rows: &[&[(&str, daydream_nmr::Value)]]) {
    let t = daydream_nmr::schema::table(name).unwrap();
    let rows = rows
        .iter()
        .map(|given| {
            t.columns
                .iter()
                .map(|c| match given.iter().find(|(n, _)| *n == c.name) {
                    Some((_, v)) => v.clone(),
                    None => match c.ty {
                        daydream_nmr::schema::ColumnType::Long => daydream_nmr::Value::Long(0),
                        daydream_nmr::schema::ColumnType::Double => {
                            daydream_nmr::Value::Double(0.0)
                        }
                        _ => daydream_nmr::Value::Text(String::new()),
                    },
                })
                .collect()
        })
        .collect();
    let data = NmrData {
        tables: vec![daydream_nmr::NmrRows {
            name: t.name.into(),
            columns: t.columns.iter().map(|c| c.name.to_string()).collect(),
            rows,
        }],
    };
    let report =
        daydream_nmr::import(realm, &data, &daydream_nmr::ImportOptions::default()).unwrap();
    assert!(report.iter().all(|r| r.errors.is_empty()), "{report:?}");
}

pub fn l(n: i32) -> daydream_nmr::Value {
    daydream_nmr::Value::Long(n)
}

pub fn t(s: &str) -> daydream_nmr::Value {
    daydream_nmr::Value::Text(s.into())
}
