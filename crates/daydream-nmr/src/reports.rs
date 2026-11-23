//! NMR's lists: the Limited Items List, Find an Item, the Control Room List, the Monster
//! Group / Index List and the NPC / Room List. Each reads the realm and writes nothing.
//!
//! Each follows its NMR form (`frmLimitedList`, `frmItemFind`, `BuildControlRoomList`,
//! `frmMonsterIndex`, `frmMonsterNPC_List`), down to what it counts and leaves out.

use std::collections::{BTreeMap, HashMap};

use daydream_store::{Realm, WbtrvDb};
use daydream_types::records::LayoutSet;
use daydream_wcc::tables::Table;

use crate::data::NmrRows;
use crate::export::{ExportError, export_table};
use crate::schema::{self, Value, layout_of};

/// Why a list cannot be built.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error(transparent)]
    Export(#[from] ExportError),
    #[error(transparent)]
    Store(#[from] daydream_store::StoreError),
    #[error("{table} has no column {column}")]
    NoColumn { table: String, column: String },
}

/// Where an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    /// On a room's floor.
    Room,
    /// Hidden in a room.
    Hidden,
    /// In a shop's stock.
    Shop,
    /// Carried by a user.
    User,
}

/// One place an item is, and how many are there.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Place {
    pub kind: PlaceKind,
    /// Map and room, a shop's number, or a user's ID.
    pub key: String,
    /// The room's, shop's or user's name.
    pub name: String,
    pub count: i64,
}

/// Which places to look in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Scan {
    pub rooms: bool,
    pub shops: bool,
    pub users: bool,
}

impl Default for Scan {
    fn default() -> Self {
        Scan {
            rooms: true,
            shops: true,
            users: true,
        }
    }
}

pub(crate) fn n(v: &Value) -> i64 {
    schema::number(v).unwrap_or(0.0) as i64
}

/// Text up to its first NUL, as NMR's `ClipNull` reads it.
pub(crate) fn text(v: &Value) -> String {
    match v {
        Value::Text(s) => s.split('\0').next().unwrap_or("").trim_end().to_string(),
        _ => String::new(),
    }
}

/// A table's rows and a lookup of its columns.
pub(crate) struct Rows(pub(crate) NmrRows);

impl Rows {
    pub(crate) fn load(realm: &Realm, name: &str, layout: LayoutSet) -> Result<Self, ReportError> {
        let t = schema::table(name).expect("an NMR table");
        Ok(Rows(export_table(realm, t, layout)?))
    }

    pub(crate) fn at(&self, column: &str) -> Result<usize, ReportError> {
        self.0
            .position(column)
            .ok_or_else(|| ReportError::NoColumn {
                table: self.0.name.clone(),
                column: column.into(),
            })
    }

    /// The positions of `stem 0`, `stem 1`, ... `stem {count - 1}`, with `suffix`.
    pub(crate) fn family(
        &self,
        stem: &str,
        count: usize,
        suffix: &str,
    ) -> Result<Vec<usize>, ReportError> {
        (0..count)
            .map(|x| self.at(&format!("{stem} {x}{suffix}")))
            .collect()
    }
}

/// Every place the items `wanted` picks are, by item. `skip_shop_109` is the Limited
/// Items List's: NMR leaves shop 109 out there, as it is not in the game.
fn places(
    realm: &Realm,
    layout: LayoutSet,
    scan: Scan,
    skip_shop_109: bool,
    wanted: impl Fn(i64) -> bool,
) -> Result<HashMap<i64, Vec<Place>>, ReportError> {
    let mut out: HashMap<i64, Vec<Place>> = HashMap::new();
    if scan.users {
        for (key, name, items) in users(realm, layout)? {
            let mut counts: BTreeMap<i64, i64> = BTreeMap::new();
            for i in items.into_iter().filter(|&i| i != 0 && wanted(i)) {
                *counts.entry(i).or_default() += 1;
            }
            for (item, count) in counts {
                out.entry(item).or_default().push(Place {
                    kind: PlaceKind::User,
                    key: key.clone(),
                    name: name.clone(),
                    count,
                });
            }
        }
    }
    if scan.shops {
        let shops = Rows::load(realm, "Shops", layout)?;
        let (number, name) = (shops.at("Number")?, shops.at("Name")?);
        let (items, now) = (
            shops.family("Item", 20, "")?,
            shops.family("Normal", 20, "")?,
        );
        for row in &shops.0.rows {
            if skip_shop_109 && n(&row[number]) == 109 {
                continue;
            }
            let mut counts: BTreeMap<i64, i64> = BTreeMap::new();
            for (&i, &q) in items.iter().zip(&now) {
                let (item, qty) = (n(&row[i]), n(&row[q]));
                if item != 0 && qty != 0 && wanted(item) {
                    *counts.entry(item).or_default() += qty;
                }
            }
            for (item, count) in counts {
                out.entry(item).or_default().push(Place {
                    kind: PlaceKind::Shop,
                    key: n(&row[number]).to_string(),
                    name: text(&row[name]),
                    count,
                });
            }
        }
    }
    if scan.rooms {
        let rooms = Rows::load(realm, "Rooms", layout)?;
        let (map, room, name) = (
            rooms.at("Map Number")?,
            rooms.at("Room Number")?,
            rooms.at("Name")?,
        );
        let floor = (
            rooms.family("Room Item", 17, "")?,
            rooms.family("Room Item", 17, " QTY")?,
        );
        let hidden = (
            rooms.family("Hidden Item", 15, "")?,
            rooms.family("Hidden Item", 15, " QTY")?,
        );
        for row in &rooms.0.rows {
            for (kind, (items, qtys)) in [(PlaceKind::Room, &floor), (PlaceKind::Hidden, &hidden)] {
                // a slot holds its quantity less one
                let mut counts: BTreeMap<i64, i64> = BTreeMap::new();
                for (&i, &q) in items.iter().zip(qtys) {
                    let item = n(&row[i]);
                    if item != 0 && wanted(item) {
                        *counts.entry(item).or_default() += n(&row[q]) + 1;
                    }
                }
                for (item, count) in counts {
                    out.entry(item).or_default().push(Place {
                        kind,
                        key: format!("{}/{}", n(&row[map]), n(&row[room])),
                        name: text(&row[name]),
                        count,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Each user's ID, "first name/ID", and the 100 item numbers they carry.
fn users(realm: &Realm, layout: LayoutSet) -> Result<Vec<(String, String, Vec<i64>)>, ReportError> {
    let l = layout_of(Table::Users, layout);
    let (Some(id), Some(first), Some(items)) =
        (l.field("userid"), l.field("firstname"), l.field("items"))
    else {
        return Ok(Vec::new());
    };
    let cstr = |b: &[u8]| {
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        String::from_utf8_lossy(&b[..end]).trim_end().to_string()
    };
    let mut out = Vec::new();
    for file in realm.files_of(Table::Users) {
        let db = WbtrvDb::open_read_only(&file.path)?;
        for (_, raw) in db.records()? {
            if raw.len() < items.offset + items.size {
                continue;
            }
            let userid = cstr(&raw[id.offset..id.offset + id.size]);
            let firstname = cstr(&raw[first.offset..first.offset + first.size]);
            let list = raw[items.offset..items.offset + items.size]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| i64::from(i32::from_le_bytes(*c)))
                .collect();
            out.push((userid.clone(), format!("{firstname}/{userid}"), list));
        }
    }
    Ok(out)
}

/// A limited item and where its copies are.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LimitedItem {
    pub number: i64,
    pub name: String,
    /// Its Game Limit.
    pub limit: i64,
    pub places: Vec<Place>,
    /// Copies found.
    pub in_use: i64,
    /// More copies than the limit (NMR shows these in red).
    pub over: bool,
}

/// NMR's Limited Items List: every named item with a Game Limit, and where its copies are.
pub fn limited_items(
    realm: &Realm,
    layout: LayoutSet,
    scan: Scan,
) -> Result<Vec<LimitedItem>, ReportError> {
    let items = Rows::load(realm, "Items", layout)?;
    let (number, name, limit) = (
        items.at("Number")?,
        items.at("Name")?,
        items.at("Game Limit")?,
    );
    let mut list: Vec<LimitedItem> = items
        .0
        .rows
        .iter()
        .filter(|r| n(&r[limit]) != 0 && !text(&r[name]).is_empty())
        .map(|r| LimitedItem {
            number: n(&r[number]),
            name: text(&r[name]),
            limit: n(&r[limit]),
            places: Vec::new(),
            in_use: 0,
            over: false,
        })
        .collect();
    let index: HashMap<i64, usize> = list
        .iter()
        .enumerate()
        .map(|(i, l)| (l.number, i))
        .collect();
    for (item, places) in places(realm, layout, scan, true, |i| index.contains_key(&i))? {
        let l = &mut list[index[&item]];
        l.places = places;
    }
    for l in &mut list {
        l.in_use = l.places.iter().map(|p| p.count).sum();
        // NMR reads the limit back from "(n)" after the name, shown only above 1
        l.over = l.in_use > l.limit.max(1);
    }
    Ok(list)
}

/// NMR's Find an Item: every place `item` is.
pub fn find_item(
    realm: &Realm,
    layout: LayoutSet,
    item: i64,
    scan: Scan,
) -> Result<Vec<Place>, ReportError> {
    Ok(places(realm, layout, scan, false, |i| i == item)?
        .remove(&item)
        .unwrap_or_default())
}

/// A control room and the rooms that name it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ControlRoom {
    pub map: i64,
    pub room: i64,
    /// Its name, or empty when the map has no such room.
    pub name: String,
    /// The room numbers, on the same map, whose Control Room is this one.
    pub rooms: Vec<i64>,
}

/// NMR's Control Room List.
pub fn control_rooms(realm: &Realm, layout: LayoutSet) -> Result<Vec<ControlRoom>, ReportError> {
    let rooms = Rows::load(realm, "Rooms", layout)?;
    let (map, room, name, control) = (
        rooms.at("Map Number")?,
        rooms.at("Room Number")?,
        rooms.at("Name")?,
        rooms.at("Control Room")?,
    );
    let names: HashMap<(i64, i64), String> = rooms
        .0
        .rows
        .iter()
        .map(|r| ((n(&r[map]), n(&r[room])), text(&r[name])))
        .collect();
    let mut by: BTreeMap<(i64, i64), Vec<i64>> = BTreeMap::new();
    for r in &rooms.0.rows {
        if n(&r[control]) > 0 {
            by.entry((n(&r[map]), n(&r[control])))
                .or_default()
                .push(n(&r[room]));
        }
    }
    Ok(by
        .into_iter()
        .map(|((map, room), rooms)| ControlRoom {
            map,
            room,
            name: names.get(&(map, room)).cloned().unwrap_or_default(),
            rooms,
        })
        .collect())
}

/// A monster group and index, and the monsters in it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GroupIndex {
    pub group: i64,
    pub index: i64,
    /// (number, name), at most 15, as NMR keeps.
    pub monsters: Vec<(i64, String)>,
}

/// NMR's Monster Group / Index List. Monsters named `sdf...` (unused records) and those with
/// a negative index are left out.
pub fn monster_groups(realm: &Realm, layout: LayoutSet) -> Result<Vec<GroupIndex>, ReportError> {
    let mons = Rows::load(realm, "Monsters", layout)?;
    let (number, name, group, index) = (
        mons.at("Number")?,
        mons.at("Name")?,
        mons.at("Group")?,
        mons.at("Index")?,
    );
    let mut by: BTreeMap<(i64, i64), Vec<(i64, String)>> = BTreeMap::new();
    for r in &mons.0.rows {
        let nm = text(&r[name]);
        if nm.starts_with("sdf") || n(&r[index]) < 0 {
            continue;
        }
        let list = by.entry((n(&r[group]), n(&r[index]))).or_default();
        if list.len() < 15 {
            list.push((n(&r[number]), nm));
        }
    }
    Ok(by
        .into_iter()
        .map(|((group, index), monsters)| GroupIndex {
            group,
            index,
            monsters,
        })
        .collect())
}

/// A room with a permanent NPC.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct NpcRoom {
    pub map: i64,
    pub room: i64,
    pub room_name: String,
    pub monster: i64,
    pub monster_name: String,
}

/// NMR's NPC / Room List: each room's permanent NPC, sorted by the monster's name.
pub fn npc_rooms(realm: &Realm, layout: LayoutSet) -> Result<Vec<NpcRoom>, ReportError> {
    let mons = Rows::load(realm, "Monsters", layout)?;
    let (mnum, mname) = (mons.at("Number")?, mons.at("Name")?);
    let names: HashMap<i64, String> = mons
        .0
        .rows
        .iter()
        .map(|r| (n(&r[mnum]), text(&r[mname])))
        .collect();
    let rooms = Rows::load(realm, "Rooms", layout)?;
    let (map, room, name, npc) = (
        rooms.at("Map Number")?,
        rooms.at("Room Number")?,
        rooms.at("Name")?,
        rooms.at("Perm NPC")?,
    );
    let mut list: Vec<NpcRoom> = rooms
        .0
        .rows
        .iter()
        .filter(|r| n(&r[npc]) > 0)
        .map(|r| NpcRoom {
            map: n(&r[map]),
            room: n(&r[room]),
            room_name: text(&r[name]),
            monster: n(&r[npc]),
            monster_name: names.get(&n(&r[npc])).cloned().unwrap_or_default(),
        })
        .collect();
    list.sort_by(|a, b| {
        a.monster_name
            .cmp(&b.monster_name)
            .then((a.map, a.room).cmp(&(b.map, b.room)))
    });
    Ok(list)
}
