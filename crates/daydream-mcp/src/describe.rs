//! What columns mean and what points where: `describe_table`'s notes, lists and exit
//! parameters from [`daydream_nmr::fields`], and `find_references`.

use std::collections::BTreeMap;

use daydream_nmr::Value as Cell;
use daydream_nmr::fields::{self, ColumnInfo};
use daydream_nmr::rows;
use daydream_nmr::schema::NmrTable;
use daydream_store::Realm;
use serde_json::{Map, Value, json};

use crate::ToolError;

/// The most references `find_references` lists.
pub const MAX_REFS: usize = 500;

fn num(v: &Cell) -> Option<i64> {
    match v {
        Cell::Long(n) => Some(i64::from(*n)),
        Cell::Double(d) => Some(*d as i64),
        _ => None,
    }
}

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

/// One row that names the record asked about.
fn reference(t: &NmrTable, all: &daydream_nmr::NmrRows, row: &[Cell], why: String) -> Value {
    let index = rows::index_columns(t);
    let key: Vec<Value> = rows::key_of_row(all, row, &index)
        .iter()
        .map(rows::cell_json)
        .collect();
    let title = rows::TITLE_COLUMNS
        .iter()
        .find_map(|c| all.position(c))
        .map(|i| rows::title_json(&row[i]));
    json!({ "table": t.name, "key": key, "title": title, "by": why })
}

/// Every row of the tables `tables` that names `target`'s record `key`: columns that refer to
/// its table, exit parameters, exits into a room, and the rooms that spawn a monster.
pub fn find_references(
    realm: &Realm,
    tables: &[&'static NmrTable],
    target: &NmrTable,
    key: &[Cell],
) -> Result<Value, ToolError> {
    let (number, map) = if target.name == "Rooms" {
        (key.get(1).and_then(num), key.first().and_then(num))
    } else {
        (key.first().and_then(num), None)
    };
    let number = number.ok_or_else(|| crate::bad("the key's number is not a number"))?;
    let mut found = Vec::new();
    let mut more = false;
    let mut push = |v: Value| {
        if found.len() < MAX_REFS {
            found.push(v);
        } else {
            more = true;
        }
    };
    for t in tables {
        let (_, all) = rows::load(realm, t)?;
        let at = |c: &str| all.position(c);
        // columns that refer to the target's table
        let cols: Vec<(usize, &str)> = all
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                fields::column(t.name, c).and_then(ColumnInfo::target) == Some(target.name)
            })
            .map(|(i, c)| (i, c.as_str()))
            .collect();
        let row_map = at("Map Number");
        for row in &all.rows {
            for &(i, c) in &cols {
                if num(&row[i]) != Some(number) {
                    continue;
                }
                // a room is on the referring room's map (an exit of type Map Change: on
                // the map its Para1 names)
                if let Some(m) = map {
                    let mut on = row_map.and_then(|j| num(&row[j]));
                    if t.name == "Rooms"
                        && let Some(n) = c.strip_prefix("Exit ")
                        && let (Some(ty), Some(p1)) =
                            (at(&format!("Type {n}")), at(&format!("Para1 {n}")))
                        && num(&row[ty]) == Some(8)
                    {
                        on = num(&row[p1]);
                    }
                    if on != Some(m) {
                        continue;
                    }
                }
                push(reference(t, &all, row, c.to_string()));
            }
            // exit parameters, by each exit's type
            if t.name == "Rooms" {
                for n in 0..10 {
                    let Some(ty) = at(&format!("Type {n}")).and_then(|i| num(&row[i])) else {
                        continue;
                    };
                    let Some(params) = fields::exit_params(ty) else {
                        continue;
                    };
                    for (para, p) in params {
                        if p.links.as_deref() == Some(target.name)
                            && at(&format!("{para} {n}")).and_then(|i| num(&row[i])) == Some(number)
                        {
                            push(reference(
                                t,
                                &all,
                                row,
                                format!("{para} {n} ({}: {})", exit_name(ty), p.means),
                            ));
                        }
                    }
                }
            }
        }
        // the rooms that spawn a monster
        if target.name == "Monsters" && t.name == "Rooms" {
            spawning_rooms(realm, target, number, t, &all, &mut push)?;
        }
    }
    Ok(json!({
        "table": target.name,
        "key": key.iter().map(rows::cell_json).collect::<Vec<_>>(),
        "references": found,
        "more": more,
    }))
}

fn exit_name(ty: i64) -> &'static str {
    fields::label("EXIT_TYPE", ty).unwrap_or("exit")
}

fn spawning_rooms(
    realm: &Realm,
    monsters: &NmrTable,
    number: i64,
    rooms_t: &NmrTable,
    rooms: &daydream_nmr::NmrRows,
    push: &mut impl FnMut(Value),
) -> Result<(), ToolError> {
    let (_, mons) = rows::load(realm, monsters)?;
    let index = rows::index_columns(monsters);
    let Some(mon) = rows::find_row(&mons, &index, &[Cell::Long(number as i32)]) else {
        return Ok(());
    };
    let get = |c: &str| mons.position(c).and_then(|i| num(&mon[i]));
    let (Some(group), Some(gen_index)) = (get("Group"), get("Index")) else {
        return Ok(());
    };
    let at = |c: &str| rooms.position(c);
    let (Some(ty), Some(lo), Some(hi), Some(regen)) = (
        at("Mon Type"),
        at("Min Index"),
        at("Max Index"),
        at("Max Regen"),
    ) else {
        return Ok(());
    };
    for row in &rooms.rows {
        let v = |i: usize| num(&row[i]).unwrap_or(0);
        if v(ty) == group
            && v(regen) > 0
            && (v(lo) == 0 || gen_index >= v(lo))
            && (v(hi) == 0 || gen_index <= v(hi))
        {
            push(reference(
                rooms_t,
                rooms,
                row,
                format!("spawns it (Mon Type {group}, Index {gen_index} within Min/Max Index)"),
            ));
        }
    }
    Ok(())
}
