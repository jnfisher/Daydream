//! What columns mean and what points where: `describe_table`'s notes, lists and exit
//! parameters from [`daydream_nmr::fields`], and `find_references` ([`daydream_nmr::refs`]).

use std::collections::BTreeMap;

use daydream_nmr::Value as Cell;
use daydream_nmr::fields::{self, ColumnInfo};
use daydream_nmr::schema::NmrTable;
use daydream_nmr::{refs, rows};
use daydream_store::Realm;
use serde_json::{Map, Value, json};

use crate::ToolError;

/// `describe_table`'s answer for `t`: its columns with what each picks from, the notes once
/// per column family, the lists used, and for Rooms what exit parameters mean.
pub fn describe(t: &NmrTable, with_ability_notes: bool) -> Value {
    let mut notes: BTreeMap<String, Value> = BTreeMap::new();
    let mut lists: BTreeMap<String, Value> = BTreeMap::new();
    let mut uses_abilities = false;
    let columns: Vec<Value> = t
        .columns
        .iter()
        .map(|c| {
            let (ty, size) = rows::column_type(c.ty);
            let mut v = json!({ "name": c.name, "type": ty, "size": size });
            if let Some(info) = fields::column(t.name, c.name) {
                add_info(&mut v, info);
                if let Some(list) = &info.options
                    && let Some(l) = fields::fields().lists.get(list)
                {
                    lists.insert(list.clone(), json!(l));
                }
                uses_abilities |= info.abilities;
                if info.note.is_some() {
                    let family = fields::template(c.name)
                        .filter(|tpl| fields::fields().tables[t.name].columns.contains_key(tpl))
                        .unwrap_or_else(|| c.name.to_string());
                    let mut n = json!({ "note": info.note });
                    if let Some(s) = &info.source {
                        n["source"] = json!(s);
                    }
                    notes.insert(family, n);
                }
            }
            v
        })
        .collect();
    let mut out = json!({
        "table": t.name,
        "key": rows::index_columns(t),
        "title": rows::title_column(t),
        "columns": columns,
    });
    if let Some(n) = fields::table_note(t.name) {
        out["note"] = json!(n);
    }
    if !notes.is_empty() {
        out["notes"] = json!(notes);
    }
    if !lists.is_empty() {
        out["lists"] = json!(lists);
    }
    if t.name == "Rooms" {
        let ep = &fields::fields().exit_params;
        let types: Map<String, Value> = ep
            .types
            .iter()
            .map(|(k, v)| {
                let name = k
                    .parse()
                    .ok()
                    .and_then(|n| fields::label("EXIT_TYPE", n))
                    .unwrap_or("?");
                (format!("{k} {name}"), json!(v))
            })
            .collect();
        out["exit_params"] = json!({
            "about": "What Para1 # to Para4 # mean, by the exit's Type #. Types not listed take no parameters.",
            "source": ep.source,
            "types": types,
        });
    }
    if uses_abilities {
        let abilities: Map<String, Value> = fields::fields()
            .abilities
            .iter()
            .map(|(n, a)| {
                let v = if with_ability_notes {
                    json!({ "name": a.name, "note": a.note })
                } else {
                    json!(a.name)
                };
                (n.clone(), v)
            })
            .collect();
        out["abilities"] = json!(abilities);
    }
    out
}

fn add_info(v: &mut Value, info: &ColumnInfo) {
    if let Some(l) = &info.options {
        v["options"] = json!(l);
    }
    if let Some(r) = info.target() {
        v["refers_to"] = json!(r);
    }
    if info.abilities {
        v["ability"] = json!(true);
    }
}

/// One row that names a record, as `find_references` lists it.
pub fn reference_json(r: &refs::Reference) -> Value {
    json!({
        "table": r.table,
        "key": r.key.iter().map(rows::cell_json).collect::<Vec<_>>(),
        "title": r.title.as_ref().map(rows::title_json),
        "by": r.by,
    })
}

/// Every row of the tables `tables` that names `target`'s record `key`
/// ([`refs::pointing_at`]): columns that refer to its table, exit parameters, exits into a
/// room, and the rooms that spawn a monster.
pub fn find_references(
    realm: &Realm,
    tables: &[&'static NmrTable],
    target: &'static NmrTable,
    key: &[Cell],
) -> Result<Value, ToolError> {
    let found = refs::pointing_at(&mut refs::Tables::new(realm), tables, target, key)?;
    Ok(json!({
        "table": target.name,
        "key": key.iter().map(rows::cell_json).collect::<Vec<_>>(),
        "references": found.list.iter().map(reference_json).collect::<Vec<_>>(),
        "more": found.more,
    }))
}
