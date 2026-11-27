//! What a batch of edits would leave pointing at nothing, warned of in its preview: values
//! the edits set that name a record the realm does not have (the batch's own inserts and
//! deletes counted), and rows that would still name a record the batch deletes. Warnings, not
//! refusals: the game takes such values (it shows nothing, or falls back), and the agent may
//! mean to add the record next.

use std::collections::{BTreeMap, HashMap, HashSet};

use daydream_nmr::Value as Cell;
use daydream_nmr::refs::{self, Named, Overlay, Tables};
use daydream_nmr::rows::{self, Edit, Preview};
use daydream_nmr::schema::NmrTable;
use daydream_store::Realm;

use crate::ToolError;

/// The most warnings a preview lists.
pub const MAX_WARNINGS: usize = 50;

/// The most rows a deleted record's warning names.
const NAMED: usize = 5;

/// A row after its edit: the realm's row (a blank one for an insert) with the preview's
/// changes over it.
fn row_after(
    tables: &mut Tables,
    t: &'static NmrTable,
    p: &Preview,
    insert: bool,
) -> Result<Vec<Cell>, ToolError> {
    let mut row: Vec<Cell> = if insert {
        t.columns.iter().map(|c| rows::blank(c.ty)).collect()
    } else {
        let all = tables.rows(t)?;
        rows::find_row(all, &rows::index_columns(t), &p.key)
            .cloned()
            .ok_or_else(|| {
                crate::bad(format!(
                    "{} {} not found",
                    t.name,
                    rows::key_display(&p.key)
                ))
            })?
    };
    for c in &p.changes {
        if let Some(i) = t.columns.iter().position(|col| col.name == c.column) {
            row[i] = c.after.clone();
        }
    }
    Ok(row)
}

/// The warnings for `edits`, previewed as `previews` (one each, in order), looking for rows
/// that name a deleted record in the tables `search`.
pub fn warnings(
    realm: &Realm,
    edits: &[Edit],
    previews: &[Preview],
    search: &[&'static NmrTable],
) -> Result<Vec<String>, ToolError> {
    let mut tables = Tables::new(realm);
    let mut over = Overlay::default();
    // each edited row after its edit, by table and key (`None`: deleted)
    let mut after: HashMap<(&'static str, String), Option<Vec<Cell>>> = HashMap::new();
    let mut planned = Vec::with_capacity(edits.len());
    for (e, p) in edits.iter().zip(previews) {
        let t = rows::record_table(&p.table)?;
        let row = match e {
            Edit::Insert { .. } => {
                over.add(t, &p.key);
                Some(row_after(&mut tables, t, p, true)?)
            }
            Edit::Update { .. } => Some(row_after(&mut tables, t, p, false)?),
            Edit::Delete { .. } => {
                over.remove(t, &p.key);
                None
            }
        };
        after.insert((t.name, rows::key_display(&p.key)), row.clone());
        planned.push((t, p, row));
    }

    let mut out = Vec::new();
    // values set that name nothing
    let mut deleted: HashMap<&'static str, HashSet<Named>> = HashMap::new();
    let mut deleted_by: HashMap<(&'static str, Named), String> = HashMap::new();
    for (t, p, row) in &planned {
        let what = format!("{} {}", t.name, rows::key_display(&p.key));
        match row {
            Some(row) => {
                let changed: HashSet<String> = p.changes.iter().map(|c| c.column.clone()).collect();
                for m in refs::missing(&mut tables, &over, t, row, Some(&changed))? {
                    out.push(format!("{what}: {m}"));
                }
            }
            None => {
                if let Some(n) = Named::of_key(t, &p.key)
                    && !tables.has(&over, t.name, n)?
                {
                    deleted.entry(t.name).or_default().insert(n);
                    deleted_by.insert((t.name, n), what);
                }
            }
        }
    }

    // rows that would still name a deleted record: those the batch leaves alone, or edits
    // and leaves naming it
    if !deleted.is_empty() {
        let mut still: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
        refs::scan(&mut tables, search, &deleted, &mut |table, named, r| {
            let Some(column) = &r.column else {
                return;
            };
            let names_it = match after.get(&(r.table, rows::key_display(&r.key))) {
                None => true,
                Some(None) => false,
                Some(Some(row)) => {
                    let value = match named {
                        Named::Number(n) | Named::Room(_, n) => n,
                    };
                    rows::record_table(r.table)
                        .ok()
                        .and_then(|rt| rt.columns.iter().position(|c| c.name == column.as_str()))
                        .is_some_and(|i| refs::num(&row[i]) == Some(value))
                }
            };
            if names_it && let Some(what) = deleted_by.get(&(table, named)) {
                let e = still.entry(what.clone()).or_default();
                e.0 += 1;
                if e.1.len() < NAMED {
                    e.1.push(format!(
                        "{} {} {column}",
                        r.table,
                        rows::key_display(&r.key)
                    ));
                }
            }
        })?;
        for (what, (count, named)) in still {
            let rest = count - named.len();
            let more = if rest > 0 {
                format!(", and {rest} more")
            } else {
                String::new()
            };
            let rows_word = if count == 1 { "row" } else { "rows" };
            out.push(format!(
                "deleting {what} leaves {count} {rows_word} naming it: {}{more}",
                named.join(", ")
            ));
        }
    }

    if out.len() > MAX_WARNINGS {
        let rest = out.len() - MAX_WARNINGS;
        out.truncate(MAX_WARNINGS);
        out.push(format!("and {rest} more warnings"));
    }
    Ok(out)
}
