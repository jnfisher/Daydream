//! One map's rooms and exits, for a map view drawn as NMR's Map form draws it.

use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::reports::{ReportError, Rows, n, text};

/// The exit directions, in the order of a room's `Exit 0` to `Exit 9`.
pub const DIRECTIONS: [&str; 10] = ["N", "S", "E", "W", "NE", "NW", "SE", "SW", "U", "D"];

/// One exit of a room.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MapExit {
    /// 0 to 9, as [`DIRECTIONS`].
    pub dir: usize,
    /// The room it leads to (on map `para1` when `kind` is 8, a map change).
    pub to: i64,
    /// The exit type (`Type n`).
    pub kind: i64,
    pub para1: i64,
    pub para2: i64,
    pub para3: i64,
    pub para4: i64,
}

/// A room, with what NMR's map can mark on it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MapRoom {
    pub room: i64,
    pub name: String,
    pub exits: Vec<MapExit>,
    pub shop: i64,
    pub spell: i64,
    pub command: i64,
    pub npc: i64,
    pub control: i64,
    pub death: i64,
    pub exit_room: i64,
    /// The room type (`Type`: shop, arena, lair...).
    pub room_type: i64,
    /// Which monsters come (`Mon Type`).
    pub mon_type: i64,
    pub light: i64,
}

/// One map's rooms, and the realm's maps.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MapView {
    /// Every map number with rooms, and its room count.
    pub maps: Vec<(i64, usize)>,
    /// The rooms of the map asked for, in room order.
    pub rooms: Vec<MapRoom>,
}

/// Every room of `map`, and the realm's maps.
pub fn map_view(realm: &Realm, layout: LayoutSet, map: i64) -> Result<MapView, ReportError> {
    let rows = Rows::load(realm, "Rooms", layout)?;
    let at = |c: &str| rows.at(c);
    let (mapn, room, name) = (at("Map Number")?, at("Room Number")?, at("Name")?);
    let (shop, spell, command, npc, control, death, exit_room) = (
        at("Shop Number")?,
        at("Spell")?,
        at("Command Text")?,
        at("Perm NPC")?,
        at("Control Room")?,
        at("Death Room")?,
        at("Exit Room")?,
    );
    let (room_type, mon_type, light) = (at("Type")?, at("Mon Type")?, at("Light")?);
    let exits = rows.family("Exit", 10, "")?;
    let kinds = rows.family("Type", 10, "")?;
    let p1 = rows.family("Para1", 10, "")?;
    let p2 = rows.family("Para2", 10, "")?;
    let p3 = rows.family("Para3", 10, "")?;
    let p4 = rows.family("Para4", 10, "")?;
    let mut out: Vec<MapRoom> = rows
        .0
        .rows
        .iter()
        .filter(|r| n(&r[mapn]) == map)
        .map(|r| MapRoom {
            room: n(&r[room]),
            name: text(&r[name]),
            exits: (0..10)
                .filter(|&d| n(&r[exits[d]]) != 0)
                .map(|d| MapExit {
                    dir: d,
                    to: n(&r[exits[d]]),
                    kind: n(&r[kinds[d]]),
                    para1: n(&r[p1[d]]),
                    para2: n(&r[p2[d]]),
                    para3: n(&r[p3[d]]),
                    para4: n(&r[p4[d]]),
                })
                .collect(),
            shop: n(&r[shop]),
            spell: n(&r[spell]),
            command: n(&r[command]),
            npc: n(&r[npc]),
            control: n(&r[control]),
            death: n(&r[death]),
            exit_room: n(&r[exit_room]),
            room_type: n(&r[room_type]),
            mon_type: n(&r[mon_type]),
            light: n(&r[light]),
        })
        .collect();
    out.sort_by_key(|r| r.room);
    let mut maps: std::collections::BTreeMap<i64, usize> = Default::default();
    for r in &rows.0.rows {
        *maps.entry(n(&r[mapn])).or_default() += 1;
    }
    Ok(MapView {
        maps: maps.into_iter().collect(),
        rooms: out,
    })
}

/// The exit back: N for S, E for W, NE for SW, U for D, and so on.
pub const OPPOSITE: [usize; 10] = [1, 0, 3, 2, 7, 6, 5, 4, 9, 8];

/// NMR's Map Editor's move into a direction with no exit: a new room, or an existing one,
/// joined to the room by an exit each way (a map change exit when the maps differ).
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Dig {
    pub map: i64,
    pub room: i64,
    /// 0 to 9, as [`DIRECTIONS`].
    pub dir: usize,
    pub to_map: i64,
    pub to_room: i64,
    /// Join an existing room (else a new one is made).
    #[serde(default)]
    pub existing: bool,
    /// When joining an existing room whose exit back is in use: replace it (NMR asks).
    #[serde(default)]
    pub replace_back: bool,
}

/// Why a dig cannot be done.
#[derive(Debug, thiserror::Error)]
pub enum DigError {
    #[error("{0}")]
    Bad(String),
    #[error(transparent)]
    Report(#[from] ReportError),
}

/// Plans a dig (nothing is written; [`crate::modify::apply`] writes it). A new room is a copy
/// of the room it is dug from, as NMR's, without its exits, items, coins and permanent NPC.
pub fn dig(realm: &Realm, layout: LayoutSet, d: &Dig) -> Result<crate::modify::Plan, DigError> {
    use crate::modify::{Change, Plan};
    use crate::schema::Value;
    let bad = |m: String| DigError::Bad(m);
    if d.dir > 9 {
        return Err(bad("directions run from 0 to 9".into()));
    }
    if d.to_map <= 0 || d.to_room <= 0 {
        return Err(bad("give the room to join, map and room above 0".into()));
    }
    if (d.map, d.room) == (d.to_map, d.to_room) {
        return Err(bad("a room cannot lead to itself here".into()));
    }
    let rows = Rows::load(realm, "Rooms", layout)?;
    let (mapn, roomn, name) = (
        rows.at("Map Number")?,
        rows.at("Room Number")?,
        rows.at("Name")?,
    );
    let exits = rows.family("Exit", 10, "")?;
    let kinds = rows.family("Type", 10, "")?;
    let paras: Vec<Vec<usize>> = (1..=4)
        .map(|p| rows.family(&format!("Para{p}"), 10, ""))
        .collect::<Result<_, _>>()?;
    let find = |m: i64, r: i64| {
        rows.0
            .rows
            .iter()
            .position(|row| n(&row[mapn]) == m && n(&row[roomn]) == r)
    };
    let here = find(d.map, d.room)
        .ok_or_else(|| bad(format!("room {}/{} is not in the realm", d.map, d.room)))?;
    let there = find(d.to_map, d.to_room);
    let current = &rows.0.rows[here];
    if n(&current[exits[d.dir]]) != 0 {
        let to = n(&current[exits[d.dir]]);
        let kind = n(&current[kinds[d.dir]]);
        let map = if kind == 8 {
            n(&current[paras[0][d.dir]])
        } else {
            d.map
        };
        if find(map, to).is_some() {
            return Err(bad(format!(
                "room {}/{} already has an exit {} (to {map}/{to})",
                d.map, d.room, DIRECTIONS[d.dir]
            )));
        }
    }
    match (d.existing, there) {
        (false, Some(_)) => {
            return Err(bad(format!(
                "room {}/{} already exists: join it as an existing room",
                d.to_map, d.to_room
            )));
        }
        (true, None) => {
            return Err(bad(format!(
                "room {}/{} is not in the realm",
                d.to_map, d.to_room
            )));
        }
        _ => {}
    }
    let like = |v: &Value, x: i64| crate::modify::cell(v, x as f64);
    // an exit from `row` in `dir` to `to` on `to_map`
    let set_exit = |row: &mut Vec<Value>, dir: usize, from_map: i64, to_map: i64, to: i64| {
        row[exits[dir]] = like(&row[exits[dir]], to);
        if from_map == to_map {
            row[kinds[dir]] = like(&row[kinds[dir]], 0);
        } else {
            row[kinds[dir]] = like(&row[kinds[dir]], 8);
            row[paras[0][dir]] = like(&row[paras[0][dir]], to_map);
        }
    };
    let back = OPPOSITE[d.dir];
    let mut notes = Vec::new();
    let mut new_current = current.clone();
    set_exit(&mut new_current, d.dir, d.map, d.to_map, d.to_room);
    let target = match there {
        Some(i) => {
            let mut row = rows.0.rows[i].clone();
            if n(&row[exits[back]]) != 0 && !d.replace_back {
                notes.push(format!(
                    "Room {}/{}'s exit {} is in use (to {}): it is left, so there is no way back.",
                    d.to_map,
                    d.to_room,
                    DIRECTIONS[back],
                    n(&row[exits[back]])
                ));
            } else {
                set_exit(&mut row, back, d.to_map, d.map, d.room);
            }
            row
        }
        None => {
            let mut row = current.clone();
            row[mapn] = like(&row[mapn], d.to_map);
            row[roomn] = like(&row[roomn], d.to_room);
            let mut clear: Vec<usize> = Vec::new();
            for c in [
                "Perm NPC",
                "Runic",
                "Platinum",
                "Gold",
                "Silver",
                "Copper",
                "InvisRunic",
                "InvisPlatinum",
                "InvisGold",
                "InvisSilver",
                "InvisCopper",
            ] {
                clear.push(rows.at(c)?);
            }
            for (stem, count) in [("Room Item", 17), ("Hidden Item", 15)] {
                for x in 0..count {
                    for suffix in ["", " QTY", " USES"] {
                        clear.push(rows.at(&format!("{stem} {x}{suffix}"))?);
                    }
                }
            }
            clear.extend(rows.family("Placed Item", 10, "")?);
            clear.extend(&exits);
            clear.extend(&kinds);
            paras.iter().for_each(|p| clear.extend(p));
            for c in clear {
                row[c] = like(&row[c], 0);
            }
            set_exit(&mut row, back, d.to_map, d.map, d.room);
            row
        }
    };
    let mut plan = Plan {
        table: "Rooms".into(),
        notes,
        ..Plan::default()
    };
    let key = |row: &[Value]| vec![row[mapn].clone(), row[roomn].clone()];
    let mut diff = |before: Option<&Vec<Value>>, after: &Vec<Value>| {
        for (i, b) in after.iter().enumerate() {
            let a = before.map(|r| &r[i]);
            if a != Some(b) && (a.is_some() || i == name) {
                plan.changes.push(Change {
                    key: key(after),
                    column: if a.is_some() {
                        rows.0.columns[i].clone()
                    } else {
                        "(new room)".into()
                    },
                    before: a.cloned().unwrap_or(Value::Null),
                    after: b.clone(),
                });
            }
        }
    };
    diff(Some(current), &new_current);
    diff(there.map(|i| &rows.0.rows[i]), &target);
    plan.rows = Some(crate::data::NmrRows {
        name: rows.0.name.clone(),
        columns: rows.0.columns.clone(),
        rows: vec![new_current, target],
    });
    Ok(plan)
}
