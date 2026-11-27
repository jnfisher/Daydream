//! What points at what: the columns that hold another record's number (`fields.json`'s `ref`
//! and `links`), an exit's parameters by the exit's type, and the rooms that spawn a monster
//! by its group and index. [`pointing_at`] lists the rows that name a record (the MCP
//! server's `find_references`, and where a record card says a record is used); [`missing`]
//! what a row names that the realm does not have (the MCP server's preview warnings);
//! [`spawn_ladder`] a monster group's monsters by Index and the rooms that spawn from it.
//!
//! A value of 0 or less names nothing. A room's exits, control room and death room name a
//! room on its own map, but a Map Change exit's room is on the map its Para1 names.

use std::collections::{HashMap, HashSet};

use daydream_store::Realm;

use crate::NmrRows;
use crate::fields::{self, ColumnInfo};
use crate::rows::{self, RowError};
use crate::schema::{NmrTable, Value};

/// The most rows [`pointing_at`] lists.
pub const MAX: usize = 500;

/// The exit type that leads onto another map, the one its Para1 names.
pub const MAP_CHANGE: i64 = 8;

/// A cell's number, if it holds one.
pub fn num(v: &Value) -> Option<i64> {
    match v {
        Value::Long(n) => Some(i64::from(*n)),
        Value::Double(d) => Some(*d as i64),
        _ => None,
    }
}

/// A record as a value names it: by its number, or a room by its map and number. A text
/// block's parts are all its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Named {
    Number(i64),
    Room(i64, i64),
}

impl Named {
    /// The record keyed `key` in `t`, as values name it (`None` for a table keyed by text).
    pub fn of_key(t: &NmrTable, key: &[Value]) -> Option<Named> {
        if t.name == "Rooms" {
            Some(Named::Room(num(key.first()?)?, num(key.get(1)?)?))
        } else {
            Some(Named::Number(num(key.first()?)?))
        }
    }
}

impl std::fmt::Display for Named {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Named::Number(n) => write!(f, "{n}"),
            Named::Room(m, r) => write!(f, "{m}/{r}"),
        }
    }
}

/// The realm's tables, each read the first time it is asked for, so one question or a batch
/// of them reads each table once.
pub struct Tables<'a> {
    realm: &'a Realm,
    rows: HashMap<&'static str, NmrRows>,
    /// How many rows name each record, by table.
    named: HashMap<&'static str, HashMap<Named, usize>>,
}

impl<'a> Tables<'a> {
    pub fn new(realm: &'a Realm) -> Self {
        Self {
            realm,
            rows: HashMap::new(),
            named: HashMap::new(),
        }
    }

    pub fn realm(&self) -> &'a Realm {
        self.realm
    }

    /// Every row of `t`.
    pub fn rows(&mut self, t: &'static NmrTable) -> Result<&NmrRows, RowError> {
        if !self.rows.contains_key(t.name) {
            let (_, rows) = rows::load(self.realm, t)?;
            self.rows.insert(t.name, rows);
        }
        Ok(&self.rows[t.name])
    }

    /// How many rows of the table `name` are the record `n` (a text block's parts).
    fn count(&mut self, name: &str, n: Named) -> Result<usize, RowError> {
        let t = rows::record_table(name)?;
        if !self.named.contains_key(t.name) {
            let index = rows::index_columns(t);
            let all = self.rows(t)?;
            let mut counts: HashMap<Named, usize> = HashMap::new();
            for r in &all.rows {
                if let Some(k) = Named::of_key(t, &rows::key_of_row(all, r, &index)) {
                    *counts.entry(k).or_default() += 1;
                }
            }
            self.named.insert(t.name, counts);
        }
        Ok(self.named[t.name].get(&n).copied().unwrap_or(0))
    }

    /// Whether the realm has the record `n` of the table `name`, with `over`'s rows added and
    /// removed.
    pub fn has(&mut self, over: &Overlay, name: &str, n: Named) -> Result<bool, RowError> {
        let t = rows::record_table(name)?;
        let at = |m: &HashMap<(&'static str, Named), usize>| m.get(&(t.name, n)).copied();
        let base = self.count(name, n)?;
        Ok(base + at(&over.added).unwrap_or(0) > at(&over.removed).unwrap_or(0))
    }
}

/// Rows a batch of edits adds and removes, over the realm's: what [`missing`] looks in
/// besides the realm.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    added: HashMap<(&'static str, Named), usize>,
    removed: HashMap<(&'static str, Named), usize>,
}

impl Overlay {
    pub fn add(&mut self, t: &'static NmrTable, key: &[Value]) {
        if let Some(n) = Named::of_key(t, key) {
            *self.added.entry((t.name, n)).or_default() += 1;
        }
    }

    pub fn remove(&mut self, t: &'static NmrTable, key: &[Value]) {
        if let Some(n) = Named::of_key(t, key) {
            *self.removed.entry((t.name, n)).or_default() += 1;
        }
    }
}

/// One row that names a record.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub table: &'static str,
    pub key: Vec<Value>,
    /// The row's title (`Name`, `Line 1`, ...), as a row list shows it.
    pub title: Option<Value>,
    /// The column that names it (`Weapon Number`, `Item Number 3`, `Para1 2`); `None` for a
    /// room that spawns a monster.
    pub column: Option<String>,
    /// For a person: the column, with what an exit's parameter means
    /// (`Para1 2 (Key: the key item)`), or how a room spawns the monster.
    pub by: String,
}

/// The rows that name a record: at most [`MAX`], and whether there were more.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct References {
    pub list: Vec<Reference>,
    pub more: bool,
}

impl References {
    fn push(&mut self, r: Reference) {
        if self.list.len() < MAX {
            self.list.push(r);
        } else {
            self.more = true;
        }
    }
}

fn reference(
    t: &'static NmrTable,
    all: &NmrRows,
    row: &[Value],
    column: Option<String>,
    by: String,
) -> Reference {
    let index = rows::index_columns(t);
    let title = rows::TITLE_COLUMNS
        .iter()
        .find_map(|c| all.position(c))
        .map(|i| row[i].clone());
    Reference {
        table: t.name,
        key: rows::key_of_row(all, row, &index),
        title,
        column,
        by,
    }
}

/// The name of exit type `ty`.
pub fn exit_name(ty: i64) -> &'static str {
    fields::label("EXIT_TYPE", ty).unwrap_or("exit")
}

/// Every row of the tables `search` that names `target`'s record `key`: columns that refer to
/// its table, exit parameters, exits into a room, and the rooms that spawn a monster.
pub fn pointing_at(
    tables: &mut Tables,
    search: &[&'static NmrTable],
    target: &'static NmrTable,
    key: &[Value],
) -> Result<References, RowError> {
    let wanted = Named::of_key(target, key)
        .ok_or_else(|| RowError::Bad("the key's number is not a number".into()))?;
    let mut found = References::default();
    let targets = HashMap::from([(target.name, HashSet::from([wanted]))]);
    scan(tables, search, &targets, &mut |_, _, r| found.push(r))?;
    // the rooms that spawn a monster
    if target.name == "Monsters"
        && let Named::Number(number) = wanted
        && let Some(&rooms) = search.iter().find(|t| t.name == "Rooms")
    {
        spawning_rooms(tables, target, number, rooms, &mut found)?;
    }
    Ok(found)
}

/// Each row of the tables `search` that names one of `targets` (records by table name), with
/// the table and record it names: in one pass over each table, however many records are
/// looked for.
pub fn scan(
    tables: &mut Tables,
    search: &[&'static NmrTable],
    targets: &HashMap<&'static str, HashSet<Named>>,
    found: &mut dyn FnMut(&'static str, Named, Reference),
) -> Result<(), RowError> {
    for &t in search {
        let all = tables.rows(t)?;
        let at = |c: &str| all.position(c);
        // columns that refer to a table looked in
        let cols: Vec<(usize, &str, &'static str, &HashSet<Named>)> = all
            .columns
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let target = fields::column(t.name, c).and_then(ColumnInfo::target)?;
                let (&name, set) = targets.get_key_value(target)?;
                Some((i, c.as_str(), name, set))
            })
            .collect();
        let row_map = at("Map Number");
        for row in &all.rows {
            for &(i, c, target, set) in &cols {
                let Some(v) = num(&row[i]).filter(|v| *v > 0) else {
                    continue;
                };
                let named = if target == "Rooms" {
                    // a room is on the referring room's map (an exit of type Map Change: on
                    // the map its Para1 names)
                    let mut on = row_map.and_then(|j| num(&row[j]));
                    if t.name == "Rooms"
                        && let Some(n) = c.strip_prefix("Exit ")
                        && let (Some(ty), Some(p1)) =
                            (at(&format!("Type {n}")), at(&format!("Para1 {n}")))
                        && num(&row[ty]) == Some(MAP_CHANGE)
                    {
                        on = num(&row[p1]);
                    }
                    let Some(m) = on else {
                        continue;
                    };
                    Named::Room(m, v)
                } else {
                    Named::Number(v)
                };
                if set.contains(&named) {
                    found(
                        target,
                        named,
                        reference(t, all, row, Some(c.to_string()), c.to_string()),
                    );
                }
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
                        let Some((&target, set)) =
                            p.links.as_deref().and_then(|l| targets.get_key_value(l))
                        else {
                            continue;
                        };
                        let col = format!("{para} {n}");
                        let Some(v) = at(&col).and_then(|i| num(&row[i])) else {
                            continue;
                        };
                        if set.contains(&Named::Number(v)) {
                            let by = format!("{col} ({}: {})", exit_name(ty), p.means);
                            found(
                                target,
                                Named::Number(v),
                                reference(t, all, row, Some(col), by),
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// A room's spawn fields, as silvermere's spawn passes read them (silvermere-engine
/// `monsters/generate.rs`, `monsters/spawn_pass.rs`, `monsters/mod.rs`; silvermere-rules
/// `spawn.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spawning {
    /// `Mon Type`: the monster Group the room spawns from.
    pub group: i64,
    /// `Min Index`, `Max Index`: the Index range (a Min Index of 0 is no lower limit).
    pub min: i64,
    pub max: i64,
    /// `Max Regen`: the most monsters it holds before it stops.
    pub regen: i64,
    /// `Type`.
    pub room_type: i64,
}

impl Spawning {
    /// The spawn fields of `row`, when the table has them.
    pub fn of(rooms: &NmrRows, row: &[Value]) -> Option<Spawning> {
        let v = |c: &str| rooms.position(c).map(|i| num(&row[i]).unwrap_or(0));
        Some(Spawning {
            group: v("Mon Type")?,
            min: v("Min Index")?,
            max: v("Max Index")?,
            regen: v("Max Regen")?,
            room_type: v("Type")?,
        })
    }

    /// Whether the room spawns from its group at all: a type the passes fill (Normal, Shop,
    /// Arena and Lair rooms; Shops only at start-up), room for a monster, and a Max Index
    /// (a pick from the group needs one: `generate_monster`'s "nothing to choose from").
    pub fn spawns(&self) -> bool {
        self.regen > 0 && self.max != 0 && (0..=3).contains(&self.room_type)
    }

    /// Whether it spawns its group's monsters of `index`.
    pub fn takes(&self, index: i64) -> bool {
        self.spawns() && (self.min == 0 || index >= self.min) && index <= self.max
    }
}

/// The rooms whose monster type is the monster's group, with its index in their range.
fn spawning_rooms(
    tables: &mut Tables,
    monsters: &'static NmrTable,
    number: i64,
    rooms_t: &'static NmrTable,
    found: &mut References,
) -> Result<(), RowError> {
    let (group, gen_index) = {
        let mons = tables.rows(monsters)?;
        let index = rows::index_columns(monsters);
        let Some(mon) = rows::find_row(mons, &index, &[Value::Long(number as i32)]) else {
            return Ok(());
        };
        let get = |c: &str| mons.position(c).and_then(|i| num(&mon[i]));
        let (Some(group), Some(gen_index)) = (get("Group"), get("Index")) else {
            return Ok(());
        };
        (group, gen_index)
    };
    let rooms = tables.rows(rooms_t)?;
    for row in &rooms.rows {
        let Some(s) = Spawning::of(rooms, row) else {
            return Ok(());
        };
        if s.group == group && s.takes(gen_index) {
            let by = format!(
                "spawns it (Mon Type {group}, Index {gen_index} within {} to {})",
                s.min, s.max
            );
            found.push(reference(rooms_t, rooms, row, None, by));
        }
    }
    Ok(())
}

/// Where a monster group spawns: its monsters by Index, and its rooms by the Index range
/// they spawn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ladder {
    /// The group's monsters: number, name and Index, by Index then number.
    pub monsters: Vec<(i64, String, i64)>,
    /// The rooms that spawn from the group, by their range (lowest Min Index first).
    pub ranges: Vec<SpawnRange>,
    /// The group's rooms that spawn nothing: no Max Index, no Max Regen, or a room type the
    /// spawn passes leave alone.
    pub idle: usize,
}

/// The rooms of a group with one Index range.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnRange {
    pub min: i64,
    pub max: i64,
    pub rooms: usize,
    /// How many are on each map, by map number.
    pub maps: Vec<(i64, usize)>,
    /// How many have each name, most first.
    pub names: Vec<(String, usize)>,
}

/// A text cell up to its first NUL, as the editors show it.
fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.split('\0').next().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// The group `group`'s monsters and spawning rooms.
pub fn spawn_ladder(tables: &mut Tables, group: i64) -> Result<Ladder, RowError> {
    let mut ladder = Ladder::default();
    {
        let t = rows::record_table("Monsters")?;
        let mons = tables.rows(t)?;
        let at = |c: &str| mons.position(c);
        if let (Some(n), Some(g), Some(ix), Some(name)) =
            (at("Number"), at("Group"), at("Index"), at("Name"))
        {
            for row in &mons.rows {
                if num(&row[g]) == Some(group) {
                    let nm = text_of(&row[name]);
                    ladder.monsters.push((
                        num(&row[n]).unwrap_or(0),
                        nm,
                        num(&row[ix]).unwrap_or(0),
                    ));
                }
            }
        }
        ladder.monsters.sort_by_key(|m| (m.2, m.0));
    }
    let t = rows::record_table("Rooms")?;
    let rooms = tables.rows(t)?;
    let (map, name) = (rooms.position("Map Number"), rooms.position("Name"));
    type Tally = (HashMap<i64, usize>, HashMap<String, usize>, usize);
    let mut by: HashMap<(i64, i64), Tally> = HashMap::new();
    for row in &rooms.rows {
        let Some(s) = Spawning::of(rooms, row) else {
            break;
        };
        if s.group != group {
            continue;
        }
        if !s.spawns() {
            ladder.idle += 1;
            continue;
        }
        let e = by.entry((s.min, s.max)).or_default();
        *e.0.entry(map.and_then(|i| num(&row[i])).unwrap_or(0))
            .or_default() += 1;
        let nm = name.map_or_else(String::new, |i| text_of(&row[i]).trim().to_string());
        *e.1.entry(nm).or_default() += 1;
        e.2 += 1;
    }
    ladder.ranges = by
        .into_iter()
        .map(|((min, max), (maps, names, rooms))| {
            let mut maps: Vec<_> = maps.into_iter().collect();
            maps.sort();
            let mut names: Vec<_> = names.into_iter().collect();
            names.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            SpawnRange {
                min,
                max,
                rooms,
                maps,
                names,
            }
        })
        .collect();
    ladder.ranges.sort_by_key(|r| (r.min, r.max));
    Ok(ladder)
}

/// A value of a row that names a record the realm does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    /// The column (`Hit Msg`, `Exit 3`, `Para1 2`).
    pub column: String,
    /// The record it names.
    pub named: Named,
    /// The table of that record.
    pub table: &'static str,
    /// What an exit's parameter means (`Key: the key item`).
    pub means: Option<String>,
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.column)?;
        if let Some(m) = &self.means {
            write!(f, " ({m})")?;
        }
        match self.named {
            Named::Room(..) => write!(f, " names room {}", self.named)?,
            Named::Number(n) => write!(f, " names {} {n}", self.table)?,
        }
        write!(f, ", which the realm does not have")
    }
}

/// What `row` of `t` (its cells in `t`'s column order) names that the realm, with `over`
/// over it, does not have. With `only`, just the values of those columns; a room's exit is
/// looked at when its room, its type or one of its parameters is among them.
pub fn missing(
    tables: &mut Tables,
    over: &Overlay,
    t: &NmrTable,
    row: &[Value],
    only: Option<&HashSet<String>>,
) -> Result<Vec<Missing>, RowError> {
    let at = |c: &str| {
        t.columns
            .iter()
            .position(|col| col.name.eq_ignore_ascii_case(c))
    };
    let cell = |c: &str| at(c).and_then(|i| row.get(i)).and_then(num);
    let asked = |c: &str| only.is_none_or(|s| s.contains(c));
    let own_map = cell("Map Number");
    let mut out = Vec::new();
    for (i, c) in t.columns.iter().enumerate() {
        let Some(target) = fields::column(t.name, c.name).and_then(ColumnInfo::target) else {
            continue;
        };
        let Some(v) = row.get(i).and_then(num).filter(|v| *v > 0) else {
            continue;
        };
        let mut look = asked(c.name);
        let named = if target == "Rooms" {
            let mut map = own_map;
            if let Some(n) = c.name.strip_prefix("Exit ")
                && n.parse::<u8>().is_ok()
            {
                if cell(&format!("Type {n}")) == Some(MAP_CHANGE) {
                    map = cell(&format!("Para1 {n}"));
                }
                look |= asked(&format!("Type {n}")) || asked(&format!("Para1 {n}"));
            }
            let Some(m) = map else {
                continue;
            };
            Named::Room(m, v)
        } else {
            Named::Number(v)
        };
        if look && !tables.has(over, target, named)? {
            out.push(Missing {
                column: c.name.to_string(),
                named,
                table: rows::record_table(target)?.name,
                means: None,
            });
        }
    }
    // exit parameters, by each exit's type
    if t.name == "Rooms" {
        for n in 0..10 {
            let Some(ty) = cell(&format!("Type {n}")) else {
                continue;
            };
            let Some(params) = fields::exit_params(ty) else {
                continue;
            };
            let retyped = asked(&format!("Type {n}"));
            for (para, p) in params {
                let Some(target) = p.links.as_deref() else {
                    continue;
                };
                let col = format!("{para} {n}");
                let Some(v) = cell(&col).filter(|v| *v > 0) else {
                    continue;
                };
                if (retyped || asked(&col)) && !tables.has(over, target, Named::Number(v))? {
                    out.push(Missing {
                        column: col,
                        named: Named::Number(v),
                        table: rows::record_table(target)?.name,
                        means: Some(format!("{}: {}", exit_name(ty), p.means)),
                    });
                }
            }
        }
    }
    Ok(out)
}
