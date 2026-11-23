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
    let exits = rows.family("Exit", 10, "")?;
    let kinds = rows.family("Type", 10, "")?;
    let p1 = rows.family("Para1", 10, "")?;
    let p2 = rows.family("Para2", 10, "")?;
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
                })
                .collect(),
            shop: n(&r[shop]),
            spell: n(&r[spell]),
            command: n(&r[command]),
            npc: n(&r[npc]),
            control: n(&r[control]),
            death: n(&r[death]),
            exit_room: n(&r[exit_room]),
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
