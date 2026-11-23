//! Reading an NMR `.mdb`, on any OS, with the pure-Rust Jet 4 reader of `datacraft-jet`.
//! (Daydream builds a patched copy of it that reads Jet's compressed text correctly; see
//! `vendor/datacraft-jet/PATCHES.md`.)

use std::path::Path;

use datacraft_jet::StorageValue;
use datacraft_jet::jet::Jet4TableReader;

use crate::columns::TABLES;
use crate::data::{NmrData, NmrRows};
use crate::schema::Value;

/// Why a `.mdb` could not be read.
#[derive(Debug, thiserror::Error)]
pub enum MdbError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Jet { path: String, message: String },
    #[error("{path}: table {table} column {column}: unexpected value {value}")]
    Value {
        path: String,
        table: String,
        column: String,
        value: String,
    },
}

/// Every NMR table the file has (Rooms, Items, ..., Info), rows in file order. Tables the
/// file lacks are left out; other tables in it are ignored.
pub fn read(path: &Path) -> Result<NmrData, MdbError> {
    let p = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|source| MdbError::Io {
        path: p.clone(),
        source,
    })?;
    let jet = |e: datacraft_jet::StorageError| MdbError::Jet {
        path: p.clone(),
        message: e.to_string(),
    };
    let reader = Jet4TableReader::new(bytes).map_err(jet)?;
    let names: Vec<String> = reader
        .catalog()
        .map_err(jet)?
        .into_iter()
        .map(|e| e.name)
        .collect();
    let mut data = NmrData::default();
    for t in TABLES {
        let Some(name) = names.iter().find(|n| n.eq_ignore_ascii_case(t.name)) else {
            continue;
        };
        let table = reader.open_table(name).map_err(jet)?;
        let columns: Vec<String> = table
            .schema
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();
        let mut rows = Vec::with_capacity(table.rows.len());
        for row in table.rows {
            let mut out = Vec::with_capacity(row.len());
            for (v, column) in row.into_iter().zip(&columns) {
                out.push(match v {
                    StorageValue::Null => Value::Null,
                    StorageValue::Int(n) => match i32::try_from(n) {
                        Ok(n) => Value::Long(n),
                        Err(_) => Value::Double(n as f64),
                    },
                    StorageValue::Float(d) => Value::Double(d),
                    StorageValue::Text(s) => Value::Text(s),
                    StorageValue::Bit(b) => Value::Long(i32::from(b)),
                    other => {
                        return Err(MdbError::Value {
                            path: p.clone(),
                            table: t.name.to_string(),
                            column: column.clone(),
                            value: format!("{other:?}"),
                        });
                    }
                });
            }
            rows.push(out);
        }
        data.tables.push(NmrRows {
            name: t.name.to_string(),
            columns,
            rows,
        });
    }
    Ok(data)
}
