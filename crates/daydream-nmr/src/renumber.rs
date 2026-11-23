//! NMR's Record Number Changer (`frmRecordChange`): records of one table move to new
//! numbers, and every place that names them follows: other tables' fields, ability values
//! and text block script commands. Its Change Map Number moves a map's rooms to another map.
//!
//! The places looked at are NMR's (`ScanRooms`, `ScanMonsters`, `ScanShops`, `ScanClasses`,
//! `ScanItems`, `ScanSpells`, `ScanTextblocks`), with these differences, each where NMR's
//! loop does something it plainly does not mean to:
//!
//! - A run is refused when a new number is already taken, or a number is moved twice or
//!   moved onto one that moves. NMR logs a taken number and goes on, which leaves the
//!   references changed and the record not.
//! - Each value moves once. NMR's loops go on through the change list after a match, so a
//!   value moved from 5 to 9 could move again on a later 9 → 12.
//! - A map change exit into the renumbered map moves on its room number. NMR's test there
//!   reads the exit's map in place of its room.
//! - A map change exit onto its own room's map keeps its type; NMR turns it into a normal
//!   exit as it goes by.
//! - Rooms' Command Text names a text block and follows text block moves; NMR leaves it.
//! - A text block that would grow past its record's room is left as it is and listed;
//!   NMR splits the overflow into a new part.
//! - A spell's ScatterItems room is listed, not changed: NMR asks each time, as it cannot
//!   tell which map the spell is for.
//!
//! And one addition: asked to (`users`), users' references follow too, which NMR's changer
//! never touches (`users_plan`, `users_map_plan`).

use std::collections::{HashMap, HashSet};

use daydream_store::Realm;
use daydream_types::records::LayoutSet;
use daydream_wcc::tables::Table;

use crate::data::NmrRows;
use crate::export::export_table;
use crate::import::TableReport;
use crate::modify::{self, ModifyError, Plan, Step, cell, collect, num};
use crate::schema::{self, Value, layout_of};

/// The tables whose records can be renumbered, as NMR's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Classes,
    Items,
    Messages,
    Monsters,
    Shops,
    Spells,
    Races,
    Rooms,
    Textblocks,
}

impl Kind {
    fn table(self) -> &'static str {
        match self {
            Kind::Classes => "Classes",
            Kind::Items => "Items",
            Kind::Messages => "Messages",
            Kind::Monsters => "Monsters",
            Kind::Shops => "Shops",
            Kind::Spells => "Spells",
            Kind::Races => "Races",
            Kind::Rooms => "Rooms",
            Kind::Textblocks => "Textblocks",
        }
    }
}

/// One record's move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Move {
    pub from: i64,
    pub to: i64,
}

fn yes() -> bool {
    true
}

/// A run of the Record Number Changer.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Renumber {
    /// Records of `table` move; rooms move within `map`.
    Records {
        table: Kind,
        #[serde(default)]
        map: Option<i64>,
        moves: Vec<Move>,
        /// Users' references follow too (see [`users_plan`]); NMR leaves users alone.
        #[serde(default)]
        users: bool,
    },
    /// Every room of map `from` moves to map `to`, with NMR's four boxes.
    Map {
        from: i64,
        to: i64,
        /// Map change exits into `from` lead to `to`.
        #[serde(default = "yes")]
        exits: bool,
        /// A map change exit that now leads to its own room's map becomes a normal exit.
        #[serde(default = "yes")]
        normal: bool,
        /// Text blocks' `teleport <room> <map>` commands follow.
        #[serde(default = "yes")]
        textblocks: bool,
        /// Spells' Teleport Map (141) abilities follow.
        #[serde(default = "yes")]
        spells: bool,
        /// Users on the map, and their trails, follow; NMR leaves users alone.
        #[serde(default)]
        users: bool,
    },
}

/// Why a run cannot be planned.
#[derive(Debug, thiserror::Error)]
pub enum RenumberError {
    #[error("{0}")]
    Bad(String),
    #[error(transparent)]
    Modify(#[from] ModifyError),
}

fn bad(msg: impl Into<String>) -> RenumberError {
    RenumberError::Bad(msg.into())
}

/// A table's rows, loaded once.
struct Loaded {
    t: &'static schema::NmrTable,
    rows: NmrRows,
}

impl Loaded {
    fn load(realm: &Realm, name: &str, layout: LayoutSet) -> Result<Self, RenumberError> {
        let t = schema::table(name).ok_or_else(|| ModifyError::NoTable(name.into()))?;
        let rows = export_table(realm, t, layout).map_err(ModifyError::from)?;
        Ok(Loaded { t, rows })
    }

    fn at(&self, column: &str) -> Result<usize, RenumberError> {
        self.rows.position(column).ok_or_else(|| {
            RenumberError::Modify(ModifyError::NoColumn {
                table: self.rows.name.clone(),
                column: column.into(),
            })
        })
    }

    fn family(&self, stem: &str, count: usize) -> Result<Vec<usize>, RenumberError> {
        (0..count)
            .map(|x| self.at(&format!("{stem} {x}")))
            .collect()
    }

    /// (Ability n, Ability Value n) positions.
    fn abilities(&self, count: usize) -> Result<Vec<(usize, usize)>, RenumberError> {
        (0..count)
            .map(|x| {
                Ok((
                    self.at(&format!("Ability {x}"))?,
                    self.at(&format!("Ability Value {x}"))?,
                ))
            })
            .collect()
    }
}

/// A room's exits: (Exit, Type, Para1..Para4) positions.
struct Exits(Vec<[usize; 6]>);

impl Exits {
    fn of(rooms: &Loaded) -> Result<Self, RenumberError> {
        (0..10)
            .map(|x| {
                Ok([
                    rooms.at(&format!("Exit {x}"))?,
                    rooms.at(&format!("Type {x}"))?,
                    rooms.at(&format!("Para1 {x}"))?,
                    rooms.at(&format!("Para2 {x}"))?,
                    rooms.at(&format!("Para3 {x}"))?,
                    rooms.at(&format!("Para4 {x}"))?,
                ])
            })
            .collect::<Result<_, _>>()
            .map(Exits)
    }
}

/// An edit of one row.
type Edit<'a> = Box<dyn Fn(&mut Vec<Value>) + 'a>;

/// The moves, by old number.
type Moves = HashMap<i64, i64>;

/// `row[i]` moved, if it is a positive number that moves.
fn remap(row: &mut [Value], i: usize, moves: &Moves) {
    let n = num(&row[i]) as i64;
    if n > 0
        && let Some(&to) = moves.get(&n)
    {
        row[i] = cell(&row[i], to as f64);
    }
}

/// `row[i]` moved where `row[i]` is not 0 (NMR's `Not x = 0` tests: negative numbers are
/// looked up too, and never match a move).
fn remap_nonzero(row: &mut [Value], i: usize, moves: &Moves) {
    let n = num(&row[i]) as i64;
    if n != 0
        && let Some(&to) = moves.get(&n)
    {
        row[i] = cell(&row[i], to as f64);
    }
}

/// Ability values of the listed abilities moved.
fn remap_abilities(row: &mut [Value], slots: &[(usize, usize)], which: &[i64], moves: &Moves) {
    for &(a, v) in slots {
        if which.contains(&(num(&row[a]) as i64)) {
            remap(row, v, moves);
        }
    }
}

/// A spell's ranged abilities (NMR's `ScanSpells`): an ability whose value is 0 takes its
/// value from Min, and Max keeps its distance from Min. NMR stops at the first ranged one
/// it moves.
fn remap_spell_ranged(
    row: &mut [Value],
    slots: &[(usize, usize)],
    which: &[i64],
    min: usize,
    max: usize,
    moves: &Moves,
) {
    for &(a, v) in slots {
        let ability = num(&row[a]) as i64;
        if ability <= 0 || !which.contains(&ability) {
            continue;
        }
        let ranged = num(&row[v]) == 0.0;
        let at = if ranged { min } else { v };
        let n = num(&row[at]) as i64;
        let Some(&to) = moves.get(&n) else {
            continue;
        };
        if ranged {
            let width = num(&row[max]) as i64 - num(&row[min]) as i64;
            row[min] = cell(&row[min], to as f64);
            row[max] = cell(&row[max], (to + width) as f64);
            return;
        }
        row[v] = cell(&row[v], to as f64);
    }
}

// ---- text block scripts ------------------------------------------------------------------

/// The ASCII-lowercased text, so byte positions match the original's.
fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

fn digits_from(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    i
}

/// The number at `start..end` replaced by its move, if it moves. Returns the position to
/// search on from.
fn replace_number(
    data: &mut String,
    start: usize,
    end: usize,
    moves: &dyn Fn(i64) -> Option<i64>,
) -> usize {
    let Ok(n) = data[start..end].parse::<i64>() else {
        return end;
    };
    match moves(n).filter(|_| n > 0) {
        Some(to) => {
            let new = to.to_string();
            data.replace_range(start..end, &new);
            start + new.len()
        }
        None => end,
    }
}

/// Every place `keyword` appears (case aside), NMR's way: its search runs only when the
/// keyword appears exactly as written somewhere.
fn keyword_hits(data: &str, keyword: &str, from: usize) -> Option<usize> {
    lower(data)[from..]
        .find(keyword)
        .map(|p| p + from + keyword.len())
}

/// NMR's `ChangeTextblockText`: the number right after `keyword` (one space may come
/// first) moves.
fn first_value(data: &mut String, keyword: &str, moves: &dyn Fn(i64) -> Option<i64>) {
    if !data.contains(keyword) {
        return;
    }
    let mut at = 0;
    while let Some(p) = keyword_hits(data, keyword, at) {
        let b = data.as_bytes();
        let start = if b.get(p) == Some(&b' ') { p + 1 } else { p };
        let end = digits_from(b, start);
        at = if end > start {
            replace_number(data, start, end, moves)
        } else {
            p
        };
    }
}

/// NMR's `ChangeTextblockText2ndVal` (and, with `word`, `ChangeTextblockTestSkill`): the
/// second value after `keyword` moves. The first is a number (`word`: letters and digits),
/// a space comes between, and one more space may come before the second.
fn second_value(data: &mut String, keyword: &str, word: bool, moves: &dyn Fn(i64) -> Option<i64>) {
    if !data.contains(keyword) {
        return;
    }
    let mut at = 0;
    while let Some(p) = keyword_hits(data, keyword, at) {
        let b = data.as_bytes();
        let mut i = p;
        if b.get(i) == Some(&b' ') {
            i += 1;
        }
        let first = i;
        while i < b.len() && (b[i].is_ascii_digit() || (word && b[i].is_ascii_alphabetic())) {
            i += 1;
        }
        if i == first || b.get(i) != Some(&b' ') {
            at = p;
            continue;
        }
        i += 1;
        if b.get(i) == Some(&b' ') {
            i += 1;
        }
        let end = digits_from(b, i);
        at = if end > i {
            replace_number(data, i, end, moves)
        } else {
            p
        };
    }
}

/// NMR's room renumbering in text blocks: `teleport <room> <map>` on the map, and the room
/// after `remoteaction` (on any map, as NMR's).
fn teleports(data: &mut String, map: i64, moves: &dyn Fn(i64) -> Option<i64>) {
    let mut at = 0;
    while let Some(p) = data[at..]
        .find("teleport ")
        .map(|p| p + at + "teleport ".len())
    {
        let b = data.as_bytes();
        let room_end = digits_from(b, p);
        at = p;
        if room_end == p || b.get(room_end) != Some(&b' ') {
            continue;
        }
        let map_end = digits_from(b, room_end + 1);
        if map_end == room_end + 1 || data[room_end + 1..map_end].parse::<i64>() != Ok(map) {
            continue;
        }
        at = replace_number(data, p, room_end, moves);
    }
    first_value(data, "remoteaction ", moves);
}

// ---- planning -------------------------------------------------------------------------------

/// Plans a run: one plan per table it changes, in the order NMR scans them, the
/// renumbered table last. Nothing is written.
pub fn plan(realm: &Realm, run: &Renumber, layout: LayoutSet) -> Result<Vec<Plan>, RenumberError> {
    match run {
        Renumber::Records {
            table,
            map,
            moves,
            users,
        } => {
            let mut plans = records(realm, *table, *map, moves, layout)?;
            if *users {
                let m: Moves = moves.iter().map(|x| (x.from, x.to)).collect();
                let p = users_plan(realm, layout, *table, *map, &m)?;
                // before the renumbering's own plan, which carries the notes
                if !p.changes.is_empty() {
                    plans.insert(plans.len().saturating_sub(1), p);
                }
            }
            Ok(plans)
        }
        Renumber::Map {
            from,
            to,
            exits,
            normal,
            textblocks,
            spells,
            users,
        } => {
            let mut plans = change_map(
                realm,
                *from,
                *to,
                [*exits, *normal, *textblocks, *spells],
                layout,
            )?;
            if *users {
                let p = users_map_plan(realm, layout, *from, *to)?;
                if !p.changes.is_empty() {
                    plans.insert(0, p);
                }
            }
            Ok(plans)
        }
    }
}

/// Users' references to moved records (Daydream's, not NMR's): items in the inventory,
/// key ring, worn slots and weapon hand; spells in the spellbook and cast on them; their
/// class and race; for rooms, where they stand and their trail on the rooms' map.
fn users_plan(
    realm: &Realm,
    layout: LayoutSet,
    kind: Kind,
    map: Option<i64>,
    m: &Moves,
) -> Result<Plan, RenumberError> {
    scan(realm, "Users", layout, |t| {
        let edit: Edit<'_> = match kind {
            Kind::Items => {
                let mut cols = t.family("Item", 100)?;
                cols.extend(t.family("Key", 50)?);
                cols.extend(t.family("Worn", 20)?);
                cols.push(t.at("Weapon")?);
                Box::new(move |row: &mut Vec<Value>| cols.iter().for_each(|&c| remap(row, c, m)))
            }
            Kind::Spells => {
                let mut cols = t.family("Spellbook", 100)?;
                cols.extend(t.family("Spell Cast", 10)?);
                Box::new(move |row: &mut Vec<Value>| cols.iter().for_each(|&c| remap(row, c, m)))
            }
            Kind::Classes | Kind::Races => {
                let c = t.at(if kind == Kind::Classes {
                    "Class"
                } else {
                    "Race"
                })?;
                Box::new(move |row: &mut Vec<Value>| remap(row, c, m))
            }
            Kind::Rooms => {
                let map = map.ok_or_else(|| bad("give the map whose rooms move"))?;
                let mut pairs = vec![(t.at("Map Number")?, t.at("Room Number")?)];
                for i in 0..20 {
                    pairs.push((
                        t.at(&format!("Trail {i} Map"))?,
                        t.at(&format!("Trail {i} Room"))?,
                    ));
                }
                Box::new(move |row: &mut Vec<Value>| {
                    for &(mc, rc) in &pairs {
                        if num(&row[mc]) as i64 == map {
                            remap(row, rc, m);
                        }
                    }
                })
            }
            Kind::Messages | Kind::Monsters | Kind::Shops | Kind::Textblocks => {
                Box::new(|_: &mut Vec<Value>| {})
            }
        };
        Ok(edit)
    })
}

/// Users on map `from`, and trail steps on it, move to map `to`.
fn users_map_plan(
    realm: &Realm,
    layout: LayoutSet,
    from: i64,
    to: i64,
) -> Result<Plan, RenumberError> {
    scan(realm, "Users", layout, |t| {
        let mut maps = vec![t.at("Map Number")?];
        for i in 0..20 {
            maps.push(t.at(&format!("Trail {i} Map"))?);
        }
        Ok(Box::new(move |row: &mut Vec<Value>| {
            for &c in &maps {
                if num(&row[c]) as i64 == from {
                    row[c] = cell(&row[c], to as f64);
                }
            }
        }))
    })
}

/// Writes the plans, in order.
pub fn apply(
    realm: &Realm,
    plans: &[Plan],
    layout: LayoutSet,
) -> Result<Vec<TableReport>, RenumberError> {
    plans
        .iter()
        .map(|p| modify::apply(realm, p, layout).map_err(RenumberError::from))
        .collect()
}

/// The text block plan: `edit` rewrites each part's script; a part that would outgrow its
/// record is listed and left.
fn textblock_plan(
    realm: &Realm,
    layout: LayoutSet,
    mut edit: impl FnMut(&mut String),
    mut also: impl FnMut(&mut [Value]),
    renumber: Option<(&Moves, usize)>,
) -> Result<Plan, RenumberError> {
    let tb = Loaded::load(realm, "Textblocks", layout)?;
    let (number, part, data) = (tb.at("Number")?, tb.at("Part #")?, tb.at("Data")?);
    let room = layout_of(Table::TextBlocks, layout)
        .field("text")
        .map_or(usize::MAX, |f| f.size);
    let mut notes = Vec::new();
    let mut plan = collect(tb.t, &tb.rows, |row| {
        let mut new = row.to_vec();
        also(&mut new);
        if let Value::Text(s) = &row[data] {
            let mut s2 = s.split('\0').next().unwrap_or("").to_string();
            let was = s2.clone();
            edit(&mut s2);
            if s2 != was {
                if s2.len() > room {
                    notes.push(format!(
                        "Text block {} part {}: would grow to {} characters, past its {}; left as it is.",
                        num(&row[number]),
                        num(&row[part]),
                        s2.len(),
                        room
                    ));
                } else {
                    new[data] = Value::Text(s2);
                }
            }
        }
        if let Some((moves, n)) = renumber {
            remap(&mut new, n, moves);
        }
        Step::Row(new)
    });
    plan.notes = notes;
    if let Some((moves, n)) = renumber {
        add_deletes(&mut plan, &tb, &[n, part], |row| {
            moves.contains_key(&(num(&row[n]) as i64))
        });
    }
    Ok(plan)
}

/// The old keys of the rows that move, to delete once their copies are written.
fn add_deletes(plan: &mut Plan, t: &Loaded, key: &[usize], moves: impl Fn(&[Value]) -> bool) {
    for row in &t.rows.rows {
        if moves(row) {
            plan.deletes
                .push(key.iter().map(|&k| row[k].clone()).collect());
            plan.moved += 1;
        }
    }
}

/// Checks the moves, NMR's Add checks and more: numbers above 0, none moved twice, no
/// number both moved and moved onto, and every new number free.
fn check(moves: &[Move], taken: &HashSet<i64>, what: &str) -> Result<Moves, RenumberError> {
    if moves.is_empty() {
        return Err(bad("the change list is empty"));
    }
    let mut map = Moves::new();
    let tos: HashSet<i64> = moves.iter().map(|m| m.to).collect();
    if tos.len() != moves.len() {
        return Err(bad("two records would take the same new number"));
    }
    for m in moves {
        if m.from <= 0 || m.to <= 0 {
            return Err(bad("record numbers start at 1"));
        }
        if map.insert(m.from, m.to).is_some() {
            return Err(bad(format!("{what} {} is moved twice", m.from)));
        }
    }
    for m in moves {
        if map.contains_key(&m.to) {
            return Err(bad(format!(
                "{what} {} is moved, and is also a new number: move to free numbers",
                m.to
            )));
        }
        if taken.contains(&m.to) {
            return Err(bad(format!("{what} {} is already taken", m.to)));
        }
    }
    Ok(map)
}

/// A plan over `name` that edits each row with `edit`, made from its loaded rows.
fn scan<'m>(
    realm: &Realm,
    name: &str,
    layout: LayoutSet,
    make: impl FnOnce(&Loaded) -> Result<Box<dyn Fn(&mut Vec<Value>) + 'm>, RenumberError>,
) -> Result<Plan, RenumberError> {
    let t = Loaded::load(realm, name, layout)?;
    let edit = make(&t)?;
    Ok(collect(t.t, &t.rows, |row| {
        let mut new = row.to_vec();
        edit(&mut new);
        Step::Row(new)
    }))
}

/// Room exits whose type is one of `types` (and whose Exit is above 0, or not 0 where
/// `nonzero`) have the listed parameters (1 to 4) moved.
fn remap_exits(
    row: &mut [Value],
    exits: &Exits,
    rules: &[(&[i64], &[usize])],
    nonzero: bool,
    moves: &Moves,
) {
    for e in &exits.0 {
        let exit = num(&row[e[0]]) as i64;
        if (nonzero && exit == 0) || (!nonzero && exit <= 0) {
            continue;
        }
        let ty = num(&row[e[1]]) as i64;
        for (types, paras) in rules {
            if types.contains(&ty) {
                for &p in *paras {
                    if nonzero {
                        remap_nonzero(row, e[1 + p], moves);
                    } else {
                        remap(row, e[1 + p], moves);
                    }
                }
            }
        }
    }
}

fn records(
    realm: &Realm,
    kind: Kind,
    map: Option<i64>,
    moves: &[Move],
    layout: LayoutSet,
) -> Result<Vec<Plan>, RenumberError> {
    let own = Loaded::load(realm, kind.table(), layout)?;
    let key: Vec<usize> = own
        .t
        .index
        .as_ref()
        .map(|i| {
            i.columns
                .iter()
                .map(|c| own.at(c))
                .collect::<Result<_, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    // the number that moves, and the rows it can move in
    let (number, on_map): (usize, Option<(usize, i64)>) = match kind {
        Kind::Rooms => {
            let map = map.ok_or_else(|| bad("give the map whose rooms move"))?;
            (own.at("Room Number")?, Some((own.at("Map Number")?, map)))
        }
        _ => (
            own.at(match kind {
                Kind::Messages
                | Kind::Spells
                | Kind::Items
                | Kind::Monsters
                | Kind::Shops
                | Kind::Classes
                | Kind::Races
                | Kind::Textblocks => "Number",
                Kind::Rooms => unreachable!(),
            })?,
            None,
        ),
    };
    let in_scope = |row: &[Value]| on_map.is_none_or(|(m, want)| num(&row[m]) as i64 == want);
    let taken: HashSet<i64> = own
        .rows
        .rows
        .iter()
        .filter(|r| in_scope(r))
        .map(|r| num(&r[number]) as i64)
        .collect();
    let what = match kind {
        Kind::Rooms => "room",
        Kind::Textblocks => "text block",
        _ => "record",
    };
    let owned = check(moves, &taken, what)?;
    let m = &owned;
    let mut notes: Vec<String> = moves
        .iter()
        .filter(|x| !taken.contains(&x.from))
        .map(|x| {
            format!(
                "{} {} is not in the realm; references to it still move.",
                kind.table(),
                x.from
            )
        })
        .collect();
    let mv = |n: i64| m.get(&n).copied();
    let mut plans = Vec::new();

    let rooms = |plans: &mut Vec<Plan>,
                 rules: &[(&[i64], &[usize])],
                 nonzero: bool,
                 cols: &[&str]|
     -> Result<(), RenumberError> {
        plans.push(scan(realm, "Rooms", layout, |t| {
            let exits = Exits::of(t)?;
            let cols: Vec<usize> = cols.iter().map(|c| t.at(c)).collect::<Result<_, _>>()?;
            let rules: Vec<(Vec<i64>, Vec<usize>)> = rules
                .iter()
                .map(|(a, b)| (a.to_vec(), b.to_vec()))
                .collect();
            Ok(Box::new(move |row: &mut Vec<Value>| {
                let r: Vec<(&[i64], &[usize])> = rules
                    .iter()
                    .map(|(a, b)| (a.as_slice(), b.as_slice()))
                    .collect();
                remap_exits(row, &exits, &r, nonzero, m);
                for &c in &cols {
                    remap(row, c, m);
                }
            }))
        })?);
        Ok(())
    };
    let textblocks =
        |plans: &mut Vec<Plan>, edit: &dyn Fn(&mut String)| -> Result<(), RenumberError> {
            plans.push(textblock_plan(realm, layout, edit, |_| {}, None)?);
            Ok(())
        };

    match kind {
        Kind::Items => {
            plans.push(scan(realm, "Shops", layout, |t| {
                let items = t.family("Item", 20)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    items.iter().for_each(|&i| remap(row, i, m))
                }))
            })?);
            textblocks(&mut plans, &|d| first_value(d, "item ", &mv))?;
            plans.push(scan(realm, "Rooms", layout, |t| {
                let exits = Exits::of(t)?;
                let mut cols = t.family("Placed Item", 10)?;
                cols.extend(t.family("Room Item", 17)?);
                cols.extend(t.family("Hidden Item", 15)?);
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    remap_exits(
                        row,
                        &exits,
                        &[(&[2, 3, 17], &[1]), (&[7, 11, 12], &[4])],
                        false,
                        m,
                    );
                    cols.iter().for_each(|&c| remap(row, c, m));
                }))
            })?);
            plans.push(scan(realm, "Monsters", layout, |t| {
                let mut cols = t.family("Item Number", 10)?;
                cols.push(t.at("Weapon Number")?);
                let abil = t.abilities(10)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m));
                    remap_abilities(row, &abil, &[185], m);
                }))
            })?);
            plans.push(scan(realm, "Spells", layout, |t| {
                let abil = t.abilities(10)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    remap_abilities(row, &abil, &[143, 56], m)
                }))
            })?);
        }
        Kind::Spells => {
            const SPELL_ABILITIES: &[i64] = &[42, 43, 73, 122, 151, 153, 160];
            plans.push(scan(realm, "Items", layout, |t| {
                let negate = t.family("Negate", 10)?;
                let abil = t.abilities(20)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    negate.iter().for_each(|&c| remap(row, c, m));
                    remap_abilities(row, &abil, SPELL_ABILITIES, m);
                }))
            })?);
            plans.push(scan(realm, "Monsters", layout, |t| {
                let mut cols = vec![t.at("Create Spell")?, t.at("Death Spell")?];
                cols.extend(t.family("Spell Number", 5)?);
                cols.extend(t.family("Attack Hit Spell", 5)?);
                let attacks: Vec<(usize, usize)> = (0..5)
                    .map(|x| {
                        Ok((
                            t.at(&format!("Attack Type {x}"))?,
                            t.at(&format!("Attack Accu/Spell {x}"))?,
                        ))
                    })
                    .collect::<Result<_, RenumberError>>()?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m));
                    for &(ty, spell) in &attacks {
                        if num(&row[ty]) as i64 == 2 {
                            remap(row, spell, m);
                        }
                    }
                }))
            })?);
            plans.push(scan(realm, "Rooms", layout, |t| {
                let exits = Exits::of(t)?;
                let spell = t.at("Spell")?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    remap_exits(
                        row,
                        &exits,
                        &[(&[1], &[1, 4]), (&[22], &[1, 2]), (&[24], &[1])],
                        true,
                        m,
                    );
                    remap_nonzero(row, spell, m);
                }))
            })?);
            // the spells' own abilities, with the renumbering below
            textblocks(&mut plans, &|d| {
                first_value(d, "cast ", &mv);
                first_value(d, "learnspell ", &mv);
                first_value(d, "checkspell ", &mv);
            })?;
        }
        Kind::Rooms => {
            let (_, map) = on_map.expect("rooms have a map");
            // the rooms' own references are planned with their renumbering below
            plans.push(scan(realm, "Spells", layout, |t| {
                let abil = t.abilities(10)?;
                let (min, max) = (t.at("Min")?, t.at("Max")?);
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    let teleports_here = abil
                        .iter()
                        .any(|&(a, v)| num(&row[a]) as i64 == 141 && num(&row[v]) as i64 == map);
                    if teleports_here {
                        remap_spell_ranged(row, &abil, &[140], min, max, m);
                    }
                }))
            })?);
            // NMR asks about each ScatterItems spell in range; here they are listed
            let spells = Loaded::load(realm, "Spells", layout)?;
            let (abil, min, number, name) = (
                spells.abilities(10)?,
                spells.at("Min")?,
                spells.at("Number")?,
                spells.at("Name")?,
            );
            for row in &spells.rows.rows {
                for &(a, v) in &abil {
                    if num(&row[a]) as i64 != 157 {
                        continue;
                    }
                    let room = if num(&row[v]) == 0.0 {
                        num(&row[min])
                    } else {
                        num(&row[v])
                    } as i64;
                    if m.contains_key(&room) {
                        notes.push(format!(
                            "Spell {} ({}) scatters items to room {room}, which moves on map {map}; it is not changed, as the spell does not say which map it means.",
                            num(&row[number]),
                            crate::reports::text(&row[name])
                        ));
                    }
                }
            }
            textblocks(&mut plans, &|d| teleports(d, map, &mv))?;
        }
        Kind::Shops => rooms(&mut plans, &[], false, &["Shop Number"])?,
        Kind::Monsters => {
            plans.push(scan(realm, "Spells", layout, |t| {
                let abil = t.abilities(10)?;
                let (min, max) = (t.at("Min")?, t.at("Max")?);
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    remap_spell_ranged(row, &abil, &[12], min, max, m)
                }))
            })?);
            rooms(&mut plans, &[], false, &["Perm NPC"])?;
            textblocks(&mut plans, &|d| {
                first_value(d, "summon ", &mv);
                first_value(d, "needmonster ", &mv);
            })?;
        }
        Kind::Messages => {
            const MSG_ABILITIES: &[i64] = &[101, 115, 120, 137, 178];
            plans.push(scan(realm, "Items", layout, |t| {
                let cols = [t.at("Hit Msg")?, t.at("Miss Msg")?, t.at("Distruct Msg")?];
                let abil = t.abilities(20)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m));
                    remap_abilities(row, &abil, MSG_ABILITIES, m);
                }))
            })?);
            plans.push(scan(realm, "Monsters", layout, |t| {
                let mut cols = vec![t.at("Move Msg")?, t.at("Death Msg")?];
                cols.extend(t.family("Attack Hit Msg", 5)?);
                cols.extend(t.family("Attack Dodge Msg", 5)?);
                cols.extend(t.family("Attack Miss Msg", 5)?);
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m))
                }))
            })?);
            rooms(
                &mut plans,
                &[
                    (&[5, 16], &[1]),
                    (&[3, 17], &[2, 3]),
                    (&[12], &[1, 3]),
                    (&[10], &[1, 2, 3]),
                    (&[1, 13, 14, 15], &[3]),
                    (&[6, 9, 22, 24], &[3, 4]),
                    (&[23], &[4]),
                ],
                false,
                &[],
            )?;
            plans.push(scan(realm, "Spells", layout, |t| {
                let cols = [t.at("Cast MSG A")?, t.at("Cast MSG B")?];
                let abil = t.abilities(10)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m));
                    remap_abilities(row, &abil, MSG_ABILITIES, m);
                }))
            })?);
            textblocks(&mut plans, &|d| {
                first_value(d, "message ", &mv);
                first_value(d, "nomonsters ", &mv);
                for k in [
                    "checkitem ",
                    "failitem ",
                    "failroomitem ",
                    "takeitem ",
                    "roomitem ",
                    "maxlevel ",
                    "minlevel ",
                    "needmonster ",
                    "price ",
                    "evilaligned ",
                    "goodaligned ",
                    "failability ",
                    "remoteaction ",
                ] {
                    second_value(d, k, false, &mv);
                }
            })?;
        }
        Kind::Textblocks => {
            plans.push(scan(realm, "Classes", layout, |t| {
                let c = t.at("Title Text")?;
                Ok(Box::new(move |row: &mut Vec<Value>| remap(row, c, m)))
            })?);
            plans.push(scan(realm, "Items", layout, |t| {
                let c = t.at("Read Msg")?;
                Ok(Box::new(move |row: &mut Vec<Value>| remap(row, c, m)))
            })?);
            rooms(&mut plans, &[], false, &["Command Text"])?;
            plans.push(scan(realm, "Spells", layout, |t| {
                let abil = t.abilities(10)?;
                let (min, max) = (t.at("Min")?, t.at("Max")?);
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    remap_spell_ranged(row, &abil, &[148], min, max, m);
                    remap_abilities(row, &abil, &[155], m);
                }))
            })?);
            plans.push(scan(realm, "Monsters", layout, |t| {
                let cols = [t.at("Greet Txt")?, t.at("Desc Txt")?, t.at("Talk Txt")?];
                let abil = t.abilities(10)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m));
                    remap_abilities(row, &abil, &[155], m);
                }))
            })?);
            // the text blocks' own scripts and links, with their renumbering below
        }
        Kind::Classes => {
            textblocks(&mut plans, &|d| first_value(d, "class ", &mv))?;
            plans.push(scan(realm, "Items", layout, |t| {
                let cols = t.family("Class", 10)?;
                let abil = t.abilities(20)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m));
                    remap_abilities(row, &abil, &[59], m);
                }))
            })?);
            rooms(&mut plans, &[(&[13], &[1, 2])], true, &[])?;
            plans.push(scan(realm, "Shops", layout, |t| {
                let c = t.at("Class Limit")?;
                Ok(Box::new(move |row: &mut Vec<Value>| remap(row, c, m)))
            })?);
        }
        Kind::Races => {
            textblocks(&mut plans, &|d| first_value(d, "race ", &mv))?;
            plans.push(scan(realm, "Items", layout, |t| {
                let cols = t.family("Race", 10)?;
                Ok(Box::new(move |row: &mut Vec<Value>| {
                    cols.iter().for_each(|&c| remap(row, c, m))
                }))
            })?);
            rooms(&mut plans, &[(&[14], &[1, 2])], true, &[])?;
        }
    }

    // the table itself: its records move, with the references it holds to its own kind
    let own_plan = match kind {
        Kind::Textblocks => textblock_plan(
            realm,
            layout,
            |d| {
                first_value(d, "text ", &mv);
                first_value(d, "random ", &mv);
                first_value(d, ":", &mv);
                second_value(d, "checkspell ", false, &mv);
                second_value(d, "testskill ", true, &mv);
            },
            {
                let link = own.at("Link To")?;
                move |row: &mut [Value]| remap(row, link, m)
            },
            Some((m, number)),
        )?,
        _ => {
            let edit: Edit = match kind {
                Kind::Rooms => {
                    let (_, map) = on_map.expect("rooms have a map");
                    let exits = Exits::of(&own)?;
                    let mapn = own.at("Map Number")?;
                    let cols = [
                        own.at("Death Room")?,
                        own.at("Exit Room")?,
                        own.at("Control Room")?,
                    ];
                    Box::new(move |row: &mut Vec<Value>| {
                        let here = num(&row[mapn]) as i64 == map;
                        for e in &exits.0 {
                            if num(&row[e[0]]) as i64 <= 0 {
                                continue;
                            }
                            let ty = num(&row[e[1]]) as i64;
                            let leads_to = if ty == 8 {
                                num(&row[e[2]]) as i64
                            } else {
                                num(&row[mapn]) as i64
                            };
                            if leads_to == map {
                                remap(row, e[0], m);
                            }
                        }
                        if here {
                            cols.iter().for_each(|&c| remap(row, c, m));
                        }
                    })
                }
                Kind::Spells => {
                    let abil = own.abilities(10)?;
                    Box::new(move |row: &mut Vec<Value>| {
                        remap_abilities(row, &abil, &[42, 43, 73, 122, 151, 153, 160], m)
                    })
                }
                Kind::Monsters => {
                    let abil = own.abilities(10)?;
                    Box::new(move |row: &mut Vec<Value>| remap_abilities(row, &abil, &[146], m))
                }
                _ => Box::new(|_: &mut Vec<Value>| {}),
            };
            let mut plan = collect(own.t, &own.rows, |row| {
                let mut new = row.to_vec();
                edit(&mut new);
                if in_scope(row) {
                    remap(&mut new, number, m);
                }
                Step::Row(new)
            });
            add_deletes(&mut plan, &own, &key, |row| {
                in_scope(row) && m.contains_key(&(num(&row[number]) as i64))
            });
            plan
        }
    };
    plans.push(own_plan);
    // the notes go with the last plan, the renumbering's
    if let Some(last) = plans.last_mut() {
        notes.append(&mut last.notes);
        last.notes = notes;
    }
    Ok(plans
        .into_iter()
        .filter(|p| !p.changes.is_empty() || !p.notes.is_empty())
        .collect())
}

fn change_map(
    realm: &Realm,
    from: i64,
    to: i64,
    [exits, normal, textblocks, spells]: [bool; 4],
    layout: LayoutSet,
) -> Result<Vec<Plan>, RenumberError> {
    if from <= 0 || to <= 0 || from == to {
        return Err(bad("give two different map numbers above 0"));
    }
    let own = Loaded::load(realm, "Rooms", layout)?;
    let (mapn, room) = (own.at("Map Number")?, own.at("Room Number")?);
    let on = |m: i64| -> HashSet<i64> {
        own.rows
            .rows
            .iter()
            .filter(|r| num(&r[mapn]) as i64 == m)
            .map(|r| num(&r[room]) as i64)
            .collect()
    };
    let (old, new) = (on(from), on(to));
    if old.is_empty() {
        return Err(bad(format!("map {from} has no rooms")));
    }
    if let Some(clash) = old.intersection(&new).min() {
        return Err(bad(format!("map {to} already has a room {clash}")));
    }
    let mut plans = Vec::new();
    if spells {
        plans.push(scan(realm, "Spells", layout, |t| {
            let abil = t.abilities(10)?;
            Ok(Box::new(move |row: &mut Vec<Value>| {
                for &(a, v) in &abil {
                    if num(&row[a]) as i64 == 141 && num(&row[v]) as i64 == from {
                        row[v] = cell(&row[v], to as f64);
                    }
                }
            }))
        })?);
    }
    if textblocks {
        let mv = |n: i64| (n == from).then_some(to);
        plans.push(textblock_plan(
            realm,
            layout,
            |d| second_value(d, "teleport ", false, &mv),
            |_| {},
            None,
        )?);
    }
    let exits_at = Exits::of(&own)?;
    let mut plan = collect(own.t, &own.rows, |row| {
        let mut new = row.to_vec();
        if num(&row[mapn]) as i64 == from {
            new[mapn] = cell(&row[mapn], to as f64);
        }
        if exits {
            let map_after = num(&new[mapn]) as i64;
            for e in &exits_at.0 {
                if num(&row[e[0]]) as i64 > 0
                    && num(&row[e[1]]) as i64 == 8
                    && num(&row[e[2]]) as i64 == from
                {
                    if normal && map_after == to {
                        new[e[1]] = cell(&row[e[1]], 0.0);
                    } else {
                        new[e[2]] = cell(&row[e[2]], to as f64);
                    }
                }
            }
        }
        Step::Row(new)
    });
    add_deletes(&mut plan, &own, &[mapn, room], |row| {
        num(&row[mapn]) as i64 == from
    });
    plans.push(plan);
    Ok(plans
        .into_iter()
        .filter(|p| !p.changes.is_empty() || !p.notes.is_empty())
        .collect())
}
