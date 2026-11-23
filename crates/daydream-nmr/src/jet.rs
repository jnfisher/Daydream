//! Writing an NMR `.mdb` through the Jet engine built into Windows.
//!
//! NMR's import seeks through the file's primary keys and indexes, so the file needs real
//! Jet index trees, and no Rust library writes those. NMR only runs on Windows, and every
//! Windows has the 32-bit Jet 4.0 engine, so Daydream writes the file there the way NMR
//! does: `tools/nmr/write-mdb.ps1` (embedded here) creates the tables and keys through
//! ADOX, as `CreateAccessTables` does, and adds the rows through OLE DB.
//!
//! Daydream hands the script a *plan*: a UTF-8 text file of the tables, columns, indexes
//! and rows. On another OS, [`write_plan`] writes it so the script can run on a Windows
//! machine:
//!
//! ```text
//! powershell -ExecutionPolicy Bypass -File write-mdb.ps1 realm.plan realm.mdb
//! ```
//!
//! Plan lines are tab-separated: `T name` starts a table, `C name type size` adds a column
//! (`Long`, `Double`, `Text`, `Memo`), `K name primary|index column...` an index, `R value...`
//! a row. A value is `N` (Null), `L` and a Long, `D` and a Double, or `S` and text with
//! `\\`, `\t`, `\n`, `\r` and `\0` escaped.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::columns::TABLES;
use crate::data::NmrData;
use crate::schema::{ColumnType, Value};

/// The PowerShell script that turns a plan into a `.mdb`.
pub const SCRIPT: &str = include_str!("../../../tools/nmr/write-mdb.ps1");

/// Why writing a `.mdb` failed.
#[derive(Debug, thiserror::Error)]
pub enum JetError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error(
        "writing a .mdb needs the 32-bit Windows PowerShell and its Jet 4.0 engine, which \
         this machine does not have; write a plan with --plan and run write-mdb.ps1 on Windows"
    )]
    NoJet,
    #[error("write-mdb.ps1 failed ({status}): {stderr}")]
    Script { status: String, stderr: String },
    #[error("table {0} is not one of NMR's")]
    UnknownTable(String),
}

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            c => out.push(c),
        }
    }
}

/// A table as a plan creates it: its columns and its key (name, primary, columns).
#[derive(Debug, Clone)]
pub struct PlanTable<'a> {
    pub name: &'a str,
    pub columns: Vec<(&'a str, ColumnType)>,
    pub key: Option<(&'a str, bool, Vec<&'a str>)>,
}

/// The plan for `data`: NMR's tables in `CreateAccessTables` order, each with its schema
/// columns (an optional column only when the data has it) and its rows.
pub fn plan(data: &NmrData) -> Result<String, JetError> {
    for rows in &data.tables {
        if crate::schema::table(&rows.name).is_none() {
            return Err(JetError::UnknownTable(rows.name.clone()));
        }
    }
    let tables: Vec<PlanTable> = TABLES
        .iter()
        .map(|t| PlanTable {
            name: t.name,
            columns: t.columns.iter().map(|c| (c.name, c.ty)).collect(),
            key: t
                .index
                .as_ref()
                .map(|i| (i.name, i.primary, i.columns.to_vec())),
        })
        .collect();
    Ok(plan_for(&tables, data, true))
}

/// The plan for `data` over `tables` (another `.mdb`'s, such as MME's): each table `data`
/// has, in `tables`' order, with the columns `data` has. With `empty_as_null`, an empty
/// text goes in as Null (NMR's export: its text columns are required, so Jet refuses
/// both); without, as a zero-length text (MME's file, which holds empty room names).
pub fn plan_for(tables: &[PlanTable], data: &NmrData, empty_as_null: bool) -> String {
    let mut out = String::new();
    for t in tables {
        let Some(rows) = data.table(t.name) else {
            continue;
        };
        let cols: Vec<_> = t
            .columns
            .iter()
            .filter_map(|c| rows.position(c.0).map(|i| (c, i)))
            .collect();
        writeln!(out, "T\t{}", t.name).unwrap();
        for ((name, ty), _) in &cols {
            let (ty, size) = match ty {
                ColumnType::Long => ("Long", 0),
                ColumnType::Double => ("Double", 0),
                ColumnType::Text(n) => ("Text", *n),
                ColumnType::Memo => ("Memo", 2000),
            };
            writeln!(out, "C\t{name}\t{ty}\t{size}").unwrap();
        }
        if let Some((name, primary, columns)) = &t.key {
            let kind = if *primary { "primary" } else { "index" };
            writeln!(out, "K\t{name}\t{kind}\t{}", columns.join("\t")).unwrap();
        }
        for row in &rows.rows {
            out.push('R');
            for (_, i) in &cols {
                out.push('\t');
                match &row[*i] {
                    Value::Null => out.push('N'),
                    Value::Long(n) => write!(out, "L{n}").unwrap(),
                    Value::Double(d) => write!(out, "D{d:?}").unwrap(),
                    Value::Text(s) if s.is_empty() && empty_as_null => out.push('N'),
                    Value::Text(s) => {
                        out.push('S');
                        escape(s, &mut out);
                    }
                }
            }
            out.push('\n');
        }
    }
    out
}

/// Writes the plan for `data` to `path`.
pub fn write_plan(data: &NmrData, path: &Path) -> Result<(), JetError> {
    std::fs::write(path, plan(data)?)?;
    Ok(())
}

/// The 32-bit Windows PowerShell, where this machine has one.
pub fn powershell32() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("SystemRoot")?);
    // SysWOW64 holds the 32-bit binaries on 64-bit Windows; System32 is 32-bit on 32-bit Windows
    ["SysWOW64", "System32"]
        .iter()
        .map(|d| root.join(d).join(r"WindowsPowerShell\v1.0\powershell.exe"))
        .find(|p| p.is_file())
}

/// Writes `data` to the `.mdb` at `out` (replacing it) through Windows' Jet engine.
pub fn write_mdb(data: &NmrData, out: &Path) -> Result<(), JetError> {
    write_plan_mdb(&plan(data)?, out)
}

/// Writes a plan's `.mdb` at `out` (replacing it) through Windows' Jet engine.
pub fn write_plan_mdb(plan: &str, out: &Path) -> Result<(), JetError> {
    let ps = powershell32().ok_or(JetError::NoJet)?;
    let dir = tempfile::tempdir()?;
    let script = dir.path().join("write-mdb.ps1");
    let plan_path = dir.path().join("realm.plan");
    // Windows PowerShell 5.1 reads a script without a BOM as the ANSI code page
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice(SCRIPT.as_bytes());
    std::fs::write(&script, bom)?;
    std::fs::write(&plan_path, plan)?;
    if out.exists() {
        std::fs::remove_file(out)?;
    }
    let out = std::path::absolute(out)?;
    let result = Command::new(ps)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg(&plan_path)
        .arg(&out)
        .output()?;
    if !result.status.success() {
        return Err(JetError::Script {
            status: result.status.to_string(),
            stderr: format!(
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::NmrRows;

    #[test]
    fn plans_escape_text_and_mark_nulls() {
        let data = NmrData {
            tables: vec![NmrRows {
                name: "Messages".into(),
                columns: vec![
                    "Number".into(),
                    "Line 1".into(),
                    "Line 2".into(),
                    "Line 3".into(),
                ],
                rows: vec![vec![
                    Value::Long(-3),
                    Value::Text("a\tb\\c\0".into()),
                    Value::Text(String::new()),
                    Value::Null,
                ]],
            }],
        };
        assert_eq!(
            plan(&data).unwrap(),
            "T\tMessages\nC\tNumber\tLong\t0\nC\tLine 1\tText\t74\nC\tLine 2\tText\t74\n\
             C\tLine 3\tText\t74\nK\tpkMessages\tprimary\tNumber\nR\tL-3\tSa\\tb\\\\c\\0\tN\tN\n"
        );
    }
}
