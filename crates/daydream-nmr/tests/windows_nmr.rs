//! The M1 gate: real Nightmare Redux imports what Daydream exports, and writes the realm
//! exactly as Daydream's own import does.
//!
//! On Windows, with `DAYDREAM_NMR_DIR` naming a folder holding `nightmare_redux.exe` with its
//! controls registered and the Btrieve engine beside it, and `DAYDREAM_NMR_REALM` a folder of
//! Worldgroup 3 1.11p `.dat` files (the `windows` workflow sets both up from a pinned
//! Nightmare Redux checkout): Daydream exports the realm, turns the export into one batch of
//! cases ([`add_cases`]: edits in every column of every table, and new records), and writes
//! it as a `.mdb` through Jet. NMR's Database Importer (driven by `tools/nmr/nmr-import.ps1`)
//! imports it into one copy of the realm, Daydream's import into another, and every record
//! of the two must be the same, byte for byte. Every difference is reported, by table, row
//! and column. Elsewhere the test does nothing; in CI on Windows NMR is required.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use daydream_nmr::{ExportOptions, ImportOptions, NmrData, export, import, jet, mdb};
use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::support::{add_cases, copy_realm, records};

fn nmr() -> Option<(PathBuf, PathBuf)> {
    if !cfg!(windows) {
        return None;
    }
    let var = |name| {
        std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    match (var("DAYDREAM_NMR_DIR"), var("DAYDREAM_NMR_REALM")) {
        (Some(app), Some(realm)) => Some((app, realm)),
        _ if std::env::var_os("CI").is_some() => {
            panic!("CI on Windows needs DAYDREAM_NMR_DIR and DAYDREAM_NMR_REALM")
        }
        _ => {
            eprintln!("skipping the NMR test: DAYDREAM_NMR_DIR or DAYDREAM_NMR_REALM is not set");
            None
        }
    }
}

fn opts() -> ExportOptions {
    ExportOptions {
        zero_runtime: false,
        custom: "Daydream test".into(),
        date: "11/23/2026".into(),
        time: "3:30:00 AM".into(),
        layout: LayoutSet::Worldgroup,
    }
}

/// Copies the `.dat` files of a realm.
fn copy_dats(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        if e.file_name()
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".dat")
        {
            std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}

/// Runs NMR's Database Importer on `realm` with `file`; its output goes to `out`.
fn nmr_import(app: &Path, realm: &Path, file: &Path, out: &Path) -> Vec<String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/nmr/nmr-import.ps1");
    let result = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .args([app, realm, file, out])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&result.stdout);
    println!("{}", text.trim());
    let mut problems = Vec::new();
    if !result.status.success() {
        problems.push(format!(
            "nmr-import.ps1 exited with {}: {}",
            result.status,
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    problems
}

/// Every row of every table, keyed by its index values.
fn rows_by_key(data: &NmrData) -> BTreeMap<String, BTreeMap<String, Vec<daydream_nmr::Value>>> {
    let mut out = BTreeMap::new();
    for rows in &data.tables {
        let Some(t) = daydream_nmr::schema::table(&rows.name) else {
            continue;
        };
        let Some(index) = &t.index else { continue };
        let at: Vec<usize> = index
            .columns
            .iter()
            .filter_map(|c| rows.position(c))
            .collect();
        let keyed = rows
            .rows
            .iter()
            .map(|r| {
                let key: Vec<String> = at.iter().map(|&i| format!("{:?}", r[i])).collect();
                (key.join(" "), r.clone())
            })
            .collect();
        out.insert(rows.name.clone(), keyed);
    }
    out
}

/// The differences between NMR's realm and Daydream's, column by column.
fn compare_columns(ours: &NmrData, theirs: &NmrData) -> Vec<String> {
    let (a, b) = (rows_by_key(ours), rows_by_key(theirs));
    let mut problems = Vec::new();
    for (table, our_rows) in &a {
        let columns = &ours.table(table).unwrap().columns;
        let their_rows = &b[table];
        let mut lines = Vec::new();
        for (key, row) in our_rows {
            let Some(other) = their_rows.get(key) else {
                lines.push(format!("{table} {key}: only Daydream has it"));
                continue;
            };
            for (i, (x, y)) in row.iter().zip(other).enumerate() {
                if x != y {
                    lines.push(format!(
                        "{table} {key} {}: Daydream {x:?}, NMR {y:?}",
                        columns[i]
                    ));
                }
            }
        }
        for key in their_rows.keys().filter(|k| !our_rows.contains_key(*k)) {
            lines.push(format!("{table} {key}: only NMR has it"));
        }
        if lines.len() > 20 {
            let n = lines.len();
            lines.truncate(20);
            lines.push(format!("{table}: {n} differences in all"));
        }
        problems.extend(lines);
    }
    problems
}

/// Records of NMR's realm and Daydream's that are not in the other, whatever their order.
fn compare_bytes(ours: &Path, theirs: &Path) -> Vec<String> {
    let theirs: HashMap<String, Vec<Vec<u8>>> = records(theirs)
        .into_iter()
        .map(|(stem, r)| (stem, r.into_iter().map(|(_, b)| b).collect()))
        .collect();
    let mut problems = Vec::new();
    for (stem, recs) in records(ours) {
        let mut a: Vec<Vec<u8>> = recs.into_iter().map(|(_, b)| b).collect();
        let mut b = theirs.get(&stem).cloned().unwrap_or_default();
        a.sort();
        b.sort();
        if a == b {
            println!("{stem}: {} records the same", a.len());
            continue;
        }
        let only_ours = a.iter().filter(|r| b.binary_search(r).is_err()).count();
        let only_theirs = b.iter().filter(|r| a.binary_search(r).is_err()).count();
        problems.push(format!(
            "{stem}: {} records from Daydream, {} from NMR; {only_ours} of Daydream's are not \
             NMR's, {only_theirs} of NMR's are not Daydream's",
            a.len(),
            b.len()
        ));
    }
    problems
}

#[test]
fn nmr_imports_daydreams_cases_as_daydream_does() {
    let Some((app, dats)) = nmr() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("base");
    daydream_store::realm::convert_dir(&dats, &base).unwrap();

    let mut data = export(&Realm::open(&base).unwrap(), &opts()).unwrap();
    for c in add_cases(&mut data) {
        println!("{}: {} rows edited, {} added", c.table, c.edited, c.added);
    }
    let import_dir = tmp.path().join("import");
    std::fs::create_dir_all(&import_dir).unwrap();
    let file = import_dir.join("cases.mdb");
    jet::write_mdb(&data, &file).unwrap();
    let written = mdb::read(&file).unwrap();

    // Daydream's import
    let ours = tmp.path().join("ours");
    std::fs::create_dir_all(&ours).unwrap();
    copy_realm(&base, &ours);
    let import_opts = ImportOptions {
        layout: LayoutSet::Worldgroup,
        ..ImportOptions::default()
    };
    for r in import(&Realm::open(&ours).unwrap(), &written, &import_opts).unwrap() {
        assert_eq!(r.errors, Vec::<String>::new(), "{}", r.table);
    }

    // NMR's
    let nmr_dats = tmp.path().join("nmr");
    copy_dats(&dats, &nmr_dats);
    // NMR's log, the message boxes it showed and any screenshots go to DAYDREAM_NMR_OUT
    let out = std::env::var_os("DAYDREAM_NMR_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| tmp.path().join("nmr-out"));
    let mut problems = nmr_import(&app, &nmr_dats, &file, &out);
    let theirs = tmp.path().join("theirs");
    daydream_store::realm::convert_dir(&nmr_dats, &theirs).unwrap();

    let our_data = export(&Realm::open(&ours).unwrap(), &opts()).unwrap();
    let their_data = export(&Realm::open(&theirs).unwrap(), &opts()).unwrap();
    problems.extend(compare_columns(&our_data, &their_data));
    problems.extend(compare_bytes(&ours, &theirs));
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
