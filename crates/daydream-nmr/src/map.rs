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
    /// Which monsters come (`Mon Type`), and which of the group (`Min Index` to `Max Index`) and
    /// how many at most (`Max Regen`).
    pub mon_type: i64,
    pub min_index: i64,
    pub max_index: i64,
    pub max_regen: i64,
    pub light: i64,
}

/// The map change exits from one map to another: the world's portals between two maps, one way.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MapLink {
    pub from: i64,
    pub to: i64,
    /// How many map change exits lead from `from` to `to`.
    pub exits: usize,
    /// The first of them, by room number: its room on `from`, and the room it leads to on `to`.
    pub room: i64,
    pub to_room: i64,
}

/// The name most of a map's rooms share, which says what the map mostly is (MajorMUD's maps have
/// only numbers).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MapName {
    pub map: i64,
    pub name: String,
    /// How many of the map's rooms have it.
    pub rooms: usize,
}

/// One map's rooms, and the realm's maps and how they join.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MapView {
    /// Every map number with rooms, and its room count.
    pub maps: Vec<(i64, usize)>,
    /// Each map's commonest room name, by map (a tie goes to the name of the lower room; a map
    /// whose rooms are all unnamed has none).
    pub names: Vec<MapName>,
    /// The rooms of the map asked for, in room order.
    pub rooms: Vec<MapRoom>,
    /// Every pair of maps a map change exit leads between, one way, by `from` and `to` (a map
    /// change onto its own map is left out).
    pub links: Vec<MapLink>,
    /// The links no exit draws that start or end on the map: where its rooms' scripts and
    /// spells send players, and the exits remote actions work ([`crate::scripted`]).
    pub scripts: crate::scripted::Scripts,
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
    let (min_index, max_index, max_regen) = (at("Min Index")?, at("Max Index")?, at("Max Regen")?);
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
            min_index: n(&r[min_index]),
            max_index: n(&r[max_index]),
            max_regen: n(&r[max_regen]),
            light: n(&r[light]),
        })
        .collect();
    out.sort_by_key(|r| r.room);
    let mut maps: std::collections::BTreeMap<i64, usize> = Default::default();
    let mut links: std::collections::BTreeMap<(i64, i64), MapLink> = Default::default();
    // by map, each room name's count and its lowest room
    let mut named: std::collections::BTreeMap<
        i64,
        std::collections::HashMap<String, (usize, i64)>,
    > = Default::default();
    for r in &rows.0.rows {
        let from = n(&r[mapn]);
        *maps.entry(from).or_default() += 1;
        let title = text(&r[name]);
        if !title.trim().is_empty() {
            let at = n(&r[room]);
            let seen = named
                .entry(from)
                .or_default()
                .entry(title)
                .or_insert((0, at));
            seen.0 += 1;
            seen.1 = seen.1.min(at);
        }
        for d in 0..10 {
            let (to_room, to) = (n(&r[exits[d]]), n(&r[p1[d]]));
            if to_room == 0 || n(&r[kinds[d]]) != 8 || to == from {
                continue;
            }
            let at = n(&r[room]);
            let link = links.entry((from, to)).or_insert(MapLink {
                from,
                to,
                exits: 0,
                room: at,
                to_room,
            });
            link.exits += 1;
            if at < link.room {
                (link.room, link.to_room) = (at, to_room);
            }
        }
    }
    let names = named
        .into_iter()
        .filter_map(|(map, counts)| {
            let (name, (rooms, _)) = counts
                .into_iter()
                .min_by_key(|(_, (count, first))| (std::cmp::Reverse(*count), *first))?;
            Some(MapName { map, name, rooms })
        })
        .collect();
    let scripts = crate::scripted::of_realm(realm, layout, &rows)?.of_map(map);
    Ok(MapView {
        maps: maps.into_iter().collect(),
        names,
        rooms: out,
        links: links.into_values().collect(),
        scripts,
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
    /// New rooms to dig in a line, each from the one before it the same way (one when left
    /// out, at most [`MAX_LINE`]): `to_room`, then the next numbers `to_map` does not use.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub count: u32,
}

/// The most rooms one dig makes in a line.
pub const MAX_LINE: u32 = 100;

fn one() -> u32 {
    1
}

fn is_one(n: &u32) -> bool {
    *n == 1
}

impl Dig {
    /// What the dig does, for the history (`Dig room 1/12 east of 1/4`).
    pub fn label(&self) -> String {
        let way = DIRECTION_WORDS.get(self.dir).copied().unwrap_or("away");
        let from = format!("{}/{}", self.map, self.room);
        if self.existing {
            format!("Join {from} {way} to {}/{}", self.to_map, self.to_room)
        } else if self.count > 1 {
            format!("Dig {} rooms {way} of {from}", self.count)
        } else {
            format!("Dig room {}/{} {way} of {from}", self.to_map, self.to_room)
        }
    }
}

/// [`DIRECTIONS`] in words.
pub const DIRECTION_WORDS: [&str; 10] = [
    "north",
    "south",
    "east",
    "west",
    "northeast",
    "northwest",
    "southeast",
    "southwest",
    "up",
    "down",
];

/// Why a dig cannot be done.
#[derive(Debug, thiserror::Error)]
pub enum DigError {
    #[error("{0}")]
    Bad(String),
    #[error(transparent)]
    Report(#[from] ReportError),
}

/// A room as a new room copies it, as NMR's Map Editor digs one: without its exits, items,
/// coins and permanent NPC.
fn blank_copy(
    rows: &Rows,
    row: &[crate::schema::Value],
) -> Result<Vec<crate::schema::Value>, ReportError> {
    let mut out = row.to_vec();
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
    clear.extend(rows.family("Exit", 10, "")?);
    clear.extend(rows.family("Type", 10, "")?);
    for p in 1..=4 {
        clear.extend(rows.family(&format!("Para{p}"), 10, "")?);
    }
    for c in clear {
        out[c] = crate::modify::cell(&out[c], 0.0);
    }
    Ok(out)
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
    if d.count == 0 || d.count > MAX_LINE {
        return Err(bad(format!("a line is 1 to {MAX_LINE} rooms")));
    }
    if d.existing && d.count > 1 {
        return Err(bad("a line of rooms is dug as new rooms".into()));
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
    // the rooms the dig makes or changes after the room it starts from, in order
    let mut made: Vec<Vec<Value>> = Vec::new();
    match there {
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
            made.push(row);
        }
        None => {
            // a new room is the room it is dug from without its exits, items, coins and
            // permanent NPC; each room of a line is the same
            let like_current = blank_copy(&rows, current)?;
            // the numbers the map already uses, for the line's rooms after the first
            let used: std::collections::HashSet<i64> = rows
                .0
                .rows
                .iter()
                .filter(|row| n(&row[mapn]) == d.to_map)
                .map(|row| n(&row[roomn]))
                .collect();
            let (mut from_map, mut from_room, mut number) = (d.map, d.room, d.to_room);
            for k in 0..d.count {
                if k > 0 {
                    number += 1;
                    while used.contains(&number) {
                        number += 1;
                    }
                    if number > i64::from(i32::MAX) {
                        return Err(bad(format!(
                            "map {} has no free room numbers left",
                            d.to_map
                        )));
                    }
                    let last = made.last_mut().expect("the line's room before");
                    set_exit(last, d.dir, d.to_map, d.to_map, number);
                }
                let mut row = like_current.clone();
                row[mapn] = like(&row[mapn], d.to_map);
                row[roomn] = like(&row[roomn], number);
                set_exit(&mut row, back, d.to_map, from_map, from_room);
                made.push(row);
                (from_map, from_room) = (d.to_map, number);
            }
        }
    }
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
    for row in &made {
        diff(there.map(|i| &rows.0.rows[i]), row);
    }
    let mut out = vec![new_current];
    out.extend(made);
    plan.rows = Some(crate::data::NmrRows {
        name: rows.0.name.clone(),
        columns: rows.0.columns.clone(),
        rows: out,
    });
    Ok(plan)
}

/// The most new rooms one stamp makes.
pub const MAX_STAMP_ROOMS: usize = 2_000;
/// The most exits one stamp asks for.
pub const MAX_STAMP_LINKS: usize = 20_000;
/// The notes a stamp lists before it counts the rest.
const STAMP_NOTES: usize = 12;

/// A stamp from the World map: new rooms and the exits between them and the map's rooms, put
/// down at once as one save. Each new room is a copy of the `brush` room, as a dig makes one.
/// An exit goes only where its slot is free (or already leads there): a link whose way there,
/// or (both ways) whose way back, is another exit's is left whole, with a note, unless
/// `overwrite`, so a stamp never makes a one-way exit by surprise. Nothing is deleted.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Stamp {
    pub map: i64,
    /// The room new rooms copy, without its exits, items, coins and permanent NPC.
    #[serde(default)]
    pub brush: i64,
    /// The new rooms, by number: none may be on the map yet.
    #[serde(default)]
    pub rooms: Vec<i64>,
    /// The exits to make, in order.
    #[serde(default)]
    pub links: Vec<Link>,
    /// Replace an exit in a link's way instead of leaving the link.
    #[serde(default)]
    pub overwrite: bool,
    /// What was stamped, for the history (`a ring`).
    #[serde(default)]
    pub what: String,
    /// Dug out of a room by hand (a corridor dragged out on the map): the history says "Dig",
    /// not "Stamp".
    #[serde(default, skip_serializing_if = "is_false")]
    pub dug: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// An exit a stamp makes: from room `from` in `dir` to room `to`, both on the stamp's map, and
/// the way back unless `one_way`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Link {
    pub from: i64,
    /// 0 to 9, as [`DIRECTIONS`].
    pub dir: usize,
    pub to: i64,
    #[serde(default)]
    pub one_way: bool,
}

/// What a stamp does.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Stamped {
    /// New rooms made.
    pub rooms: usize,
    /// Exits set, each way counted.
    pub exits: usize,
    /// Rooms already on the map that gain an exit.
    pub joined: usize,
    /// Links left whole because an exit was in their way.
    pub left: usize,
}

impl Stamp {
    /// What the stamp does, for the history (`Stamp a ring: 8 rooms on map 1`, or `Dig a
    /// corridor east of 1/4: 3 rooms on map 1`).
    pub fn label(&self) -> String {
        let what = match self.what.trim() {
            "" => "a stamp",
            w => w,
        };
        let verb = if self.dug { "Dig" } else { "Stamp" };
        match self.rooms.as_slice() {
            [] => format!("{verb} {what} on map {}", self.map),
            [one] => format!("{verb} {what}: room {}/{one}", self.map),
            more => format!("{verb} {what}: {} rooms on map {}", more.len(), self.map),
        }
    }
}

/// Plans a stamp (nothing is written; [`crate::modify::apply`] writes it), and says what it
/// does.
pub fn stamp(
    realm: &Realm,
    layout: LayoutSet,
    s: &Stamp,
) -> Result<(crate::modify::Plan, Stamped), DigError> {
    use crate::modify::{Change, Plan};
    use crate::schema::Value;
    use std::collections::{BTreeMap, HashMap, HashSet};
    let bad = |m: String| DigError::Bad(m);
    let top = i64::from(i32::MAX);
    if !(1..=top).contains(&s.map) {
        return Err(bad(format!("no map {}: a map is numbered from 1", s.map)));
    }
    if s.rooms.is_empty() && s.links.is_empty() {
        return Err(bad("a stamp makes rooms or exits: it has neither".into()));
    }
    if s.rooms.len() > MAX_STAMP_ROOMS {
        return Err(bad(format!(
            "a stamp makes at most {MAX_STAMP_ROOMS} rooms"
        )));
    }
    if s.links.len() > MAX_STAMP_LINKS {
        return Err(bad(format!(
            "a stamp makes at most {MAX_STAMP_LINKS} exits"
        )));
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
    // the map's rooms, by number
    let on_map: HashMap<i64, usize> = rows
        .0
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| n(&r[mapn]) == s.map)
        .map(|(i, r)| (n(&r[roomn]), i))
        .collect();
    let mut seen = HashSet::new();
    for &r in &s.rooms {
        if !(1..=top).contains(&r) {
            return Err(bad(format!(
                "no room {}/{r}: a room is numbered from 1",
                s.map
            )));
        }
        if on_map.contains_key(&r) {
            return Err(bad(format!(
                "room {}/{r} is already on the map: a stamp's new rooms take free numbers",
                s.map
            )));
        }
        if !seen.insert(r) {
            return Err(bad(format!("room {}/{r} is made twice", s.map)));
        }
    }
    // the rows as the stamp leaves them, by room: new ones from the brush, the map's own as
    // they are until a link changes them
    let mut work: BTreeMap<i64, Vec<Value>> = BTreeMap::new();
    if !s.rooms.is_empty() {
        let brush = on_map.get(&s.brush).ok_or_else(|| {
            bad(format!(
                "room {}/{} is not on the map: new rooms copy a room of it",
                s.map, s.brush
            ))
        })?;
        let blank = blank_copy(&rows, &rows.0.rows[*brush])?;
        for &r in &s.rooms {
            let mut row = blank.clone();
            row[mapn] = crate::modify::cell(&row[mapn], s.map as f64);
            row[roomn] = crate::modify::cell(&row[roomn], r as f64);
            work.insert(r, row);
        }
    }
    let room_of = |r: i64| -> Result<(), DigError> {
        if seen.contains(&r) || on_map.contains_key(&r) {
            Ok(())
        } else {
            Err(DigError::Bad(format!(
                "room {}/{r} is neither on the map nor one the stamp makes",
                s.map
            )))
        }
    };
    for l in &s.links {
        if l.dir > 9 {
            return Err(bad("directions run from 0 to 9".into()));
        }
        room_of(l.from)?;
        room_of(l.to)?;
        if l.from == l.to {
            return Err(bad(format!(
                "room {}/{} cannot lead to itself here",
                s.map, l.from
            )));
        }
    }
    // a slot's exit: free, already the link's, or another exit's (where it leads)
    enum Slot {
        Free,
        Have,
        Taken(i64),
    }
    let slot = |row: &[Value], dir: usize, to: i64| {
        let at = n(&row[exits[dir]]);
        if at == 0 {
            Slot::Free
        } else if at == to && n(&row[kinds[dir]]) != 8 {
            Slot::Have
        } else {
            Slot::Taken(at)
        }
    };
    let set = |row: &mut Vec<Value>, dir: usize, to: i64| {
        row[exits[dir]] = crate::modify::cell(&row[exits[dir]], to as f64);
        row[kinds[dir]] = crate::modify::cell(&row[kinds[dir]], 0.0);
        for p in &paras {
            row[p[dir]] = crate::modify::cell(&row[p[dir]], 0.0);
        }
    };
    let mut done = Stamped {
        rooms: s.rooms.len(),
        ..Stamped::default()
    };
    let mut notes = Vec::new();
    for l in &s.links {
        let back = OPPOSITE[l.dir];
        for r in [l.from, l.to] {
            work.entry(r)
                .or_insert_with(|| rows.0.rows[on_map[&r]].clone());
        }
        let there = slot(&work[&l.from], l.dir, l.to);
        let home = if l.one_way {
            Slot::Have
        } else {
            slot(&work[&l.to], back, l.from)
        };
        let in_way: Vec<String> = [(l.from, l.dir, &there), (l.to, back, &home)]
            .into_iter()
            .filter_map(|(r, d, sl)| match sl {
                Slot::Taken(to) => Some(format!(
                    "room {}/{r}'s exit {} leads to {to}",
                    s.map, DIRECTION_WORDS[d]
                )),
                _ => None,
            })
            .collect();
        if !in_way.is_empty() && !s.overwrite {
            done.left += 1;
            if notes.len() < STAMP_NOTES {
                notes.push(format!(
                    "{}: the exit {} from {}/{} to {} was left.",
                    upper_first(&in_way.join(", and ")),
                    DIRECTION_WORDS[l.dir],
                    s.map,
                    l.from,
                    l.to
                ));
            }
            continue;
        }
        if !matches!(there, Slot::Have) {
            set(work.get_mut(&l.from).expect("kept above"), l.dir, l.to);
            done.exits += 1;
        }
        if !matches!(home, Slot::Have) {
            set(work.get_mut(&l.to).expect("kept above"), back, l.from);
            done.exits += 1;
        }
    }
    if done.left > STAMP_NOTES {
        notes.push(format!(
            "And {} more links were left the same way.",
            done.left - STAMP_NOTES
        ));
    }
    let mut plan = Plan {
        table: "Rooms".into(),
        notes,
        ..Plan::default()
    };
    let mut out = Vec::new();
    for (room, after) in work {
        let before = on_map.get(&room).map(|&i| &rows.0.rows[i]);
        let key = vec![after[mapn].clone(), after[roomn].clone()];
        let mut changed = false;
        for (i, b) in after.iter().enumerate() {
            let a = before.map(|r| &r[i]);
            if a == Some(b) || (a.is_none() && i != name) {
                continue;
            }
            changed = true;
            plan.changes.push(Change {
                key: key.clone(),
                column: if a.is_some() {
                    rows.0.columns[i].clone()
                } else {
                    "(new room)".into()
                },
                before: a.cloned().unwrap_or(Value::Null),
                after: b.clone(),
            });
        }
        if before.is_none() || changed {
            if before.is_some() {
                done.joined += 1;
            }
            out.push(after);
        }
    }
    plan.rows = Some(crate::data::NmrRows {
        name: rows.0.name.clone(),
        columns: rows.0.columns.clone(),
        rows: out,
    });
    Ok((plan, done))
}

/// The most rooms one set from the World map reaches.
pub const MAX_BULK_ROOMS: usize = 50_000;
/// The most columns one set changes.
pub const MAX_BULK_COLUMNS: usize = 64;

/// Fields set on rooms picked on the World map, together, as one step: its bulk bar. Planned
/// as NMR's Mass Room Editor sets them, on those rooms alone.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Bulk {
    pub map: i64,
    pub rooms: Vec<i64>,
    /// Each column and its value (text or a number).
    pub values: Vec<(String, crate::schema::Value)>,
    /// What is set, for the history ("Light", "Monsters"); left out, the columns.
    #[serde(default)]
    pub what: String,
}

impl Bulk {
    /// The history's label: "Set Light on 14 rooms of map 1".
    pub fn label(&self) -> String {
        let what: String = match self.what.trim() {
            "" => self
                .values
                .iter()
                .map(|(c, _)| c.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            w => w.to_string(),
        };
        let what: String = what.chars().take(60).collect();
        match self.rooms.as_slice() {
            [r] => format!("Set {what} on room {}/{r}", self.map),
            rooms => format!("Set {what} on {} rooms of map {}", rooms.len(), self.map),
        }
    }
}

/// Plans a [`Bulk`] set: every changed value, on the rooms named alone.
pub fn bulk(realm: &Realm, layout: LayoutSet, b: &Bulk) -> Result<crate::modify::Plan, DigError> {
    if !(1..=i64::from(u16::MAX)).contains(&b.map) {
        return Err(DigError::Bad(format!("no map {}", b.map)));
    }
    if b.rooms.is_empty() {
        return Err(DigError::Bad("pick the rooms to set".into()));
    }
    if b.rooms.len() > MAX_BULK_ROOMS {
        return Err(DigError::Bad(format!(
            "a set reaches at most {MAX_BULK_ROOMS} rooms"
        )));
    }
    if b.values.is_empty() {
        return Err(DigError::Bad("name the fields to set".into()));
    }
    if b.values.len() > MAX_BULK_COLUMNS {
        return Err(DigError::Bad(format!(
            "a set changes at most {MAX_BULK_COLUMNS} fields"
        )));
    }
    let on_map: std::collections::BTreeSet<i64> = map_view(realm, layout, b.map)?
        .rooms
        .iter()
        .map(|r| r.room)
        .collect();
    if let Some(r) = b.rooms.iter().find(|r| !on_map.contains(r)) {
        return Err(DigError::Bad(format!("map {} has no room {r}", b.map)));
    }
    let (from, to) = (
        *b.rooms.iter().min().expect("not empty"),
        *b.rooms.iter().max().expect("not empty"),
    );
    let m = crate::modify::Modify {
        table: "Rooms".into(),
        action: crate::modify::Action::Set {
            values: b.values.clone(),
        },
        range: Some(crate::import::Range {
            table: "Rooms".into(),
            map: Some(b.map),
            from,
            to,
        }),
        only: b.rooms.clone(),
        only_if: Vec::new(),
    };
    crate::modify::plan(realm, &m, layout).map_err(|e| DigError::Bad(e.to_string()))
}

/// `text` with its first letter a capital.
fn upper_first(text: &str) -> String {
    let mut c = text.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}
