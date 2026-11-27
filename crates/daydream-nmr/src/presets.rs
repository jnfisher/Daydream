//! NMR's one-click fixes from its Items and Monsters tool menus, each planned like a
//! Universal Modifier run (so it can be previewed) and written with [`modify::apply`].
//!
//! Each follows the NMR code it is named after, in `frmMain` and `frmMonItemDropPct`. Where
//! NMR logs a record it could not fix, the plan carries a note.
//!
//! "Multiply Boss Exp" is not here: in NMR it only fills in the Universal Modifier's form
//! (Exp Multiplier × 2 where Game Limit = 1), and the browser does the same.
//!
//! [`modify::apply`]: crate::modify::apply

use std::collections::HashMap;

use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::data::NmrRows;
use crate::export::export_table;
use crate::modify::{Change, Cmp, ModifyError, Plan, Step, cell, collect, num, slots};
use crate::schema::{self, NmrTable, Value};

/// The largest experience NMR writes on 1.11j and later.
pub const MAX_EXP: u64 = 2_147_483_646;

/// The items "No Limited Items" leaves limited: the portals, the vortex and the face that
/// quests depend on (NMR's general help lists them).
pub const KEPT_LIMITED: [i64; 7] = [1078, 1326, 1531, 1643, 1686, 1749, 839];

/// Which items "Modify Item Drop Percentages" looks at, by their Game Limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Limited {
    #[default]
    Include,
    Exclude,
    Only,
}

/// NMR's "Modify Item Drop Percentages" form.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DropPct {
    /// True: set the drop % to `value`; false: add `value` (which may be negative) and hold
    /// the result in 1 to 100.
    pub set: bool,
    pub value: i64,
    /// Only drops whose current % passes this.
    pub only_if: (Cmp, i64),
    #[serde(default)]
    pub limited: Limited,
    /// Leave out weapons and armour that are deleted at maintenance (ability 119).
    #[serde(default)]
    pub exclude_del_at_maint: bool,
    /// Only monsters whose Regen Time is more than this.
    #[serde(default)]
    pub min_regen: Option<i64>,
    pub weapons: bool,
    pub armour: bool,
    pub keys: bool,
    pub containers: bool,
    pub scrolls: bool,
}

/// One of NMR's fixes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "preset", rename_all = "snake_case")]
pub enum Preset {
    /// Items: Game Limit 0, except [`KEPT_LIMITED`] and gang items (abilities 181 to 184).
    NoLimitedItems,
    /// Items: empty every Level Restriction slot (abilities 135 and 136), value too.
    NoLevelRestrictions,
    /// Items: 0 uses becomes -1; an item kept after its uses gets its recharge (121) value
    /// set to its uses.
    FixItemUses,
    /// Monsters: each drop's uses set to the item's uses.
    FixDropUses,
    /// Monsters: experience times `by` (2 to 20), held at `cap` (0 or none: [`MAX_EXP`]).
    MultiplyExp {
        by: u32,
        #[serde(default)]
        cap: Option<u64>,
    },
    /// Monsters: experience divided by `by` (2 to 20), at least 1.
    DivideExp { by: u32 },
    /// Monsters: Experience × Exp Multiplier into Experience, multiplier 1.
    CombineExp,
    /// Monsters: Date and Time Killed 0.
    ResetKillTimes,
    /// Monsters: killed on this date at 01:01:02.
    SetKillTimes { year: i32, month: u32, day: u32 },
    /// Monsters: NMR's drop % form.
    ItemDropPct(DropPct),
    /// Shops: each item's stock to its max, where its regen number, regen % and regen
    /// time are all set; gang shops (type 11) are left alone.
    RestockShops,
    /// Text blocks: `text` taken off the end of every part that ends with it.
    StripTextblocks { text: String },
    /// Rooms: items of the same number and uses, on the floor or hidden, put in one slot.
    CombineRoomItems,
    /// Rooms: a "Buffer Room" for every room number missing below a map's last room, on
    /// `map`, or on maps 1 to 18 when none is given.
    InsertBufferRooms {
        #[serde(default)]
        map: Option<i64>,
    },
    /// Rooms: every room whose name begins with "Buffer Room" deleted, on `map`, or on every
    /// map when none is given.
    DeleteBufferRooms {
        #[serde(default)]
        map: Option<i64>,
    },
    /// NMR's Monster Group/Index Changer, over `table` ("monsters" or "rooms"): a monster
    /// whose group and index are a change's `from` takes its `to`; a room whose Mon Type is
    /// a change's group and whose Min-Max Index range holds its index (the first such
    /// change, in list order) moves its range to start at the new index, keeping its width.
    ChangeMonsterIndexes {
        table: IndexTable,
        changes: Vec<IndexChange>,
    },
    /// Users: NMR's Revive Character for the user `user` (their BBS Name). HP and Mana to
    /// their max; with `lives`, Lives to 9 when below; with `spells`, the spells cast on
    /// them (and the poison flag beside them) cleared; with `to`, moved.
    ReviveCharacter {
        user: String,
        #[serde(default)]
        lives: bool,
        #[serde(default)]
        spells: bool,
        #[serde(default)]
        to: Option<ReviveTo>,
    },
}

impl Preset {
    /// What the fix is called in the history of saves (and in the Workshop, which names it
    /// the same way).
    pub fn label(&self) -> String {
        match self {
            Preset::NoLimitedItems => "Lift item game limits".into(),
            Preset::NoLevelRestrictions => "Lift item level restrictions".into(),
            Preset::FixItemUses => "Fix uses on items".into(),
            Preset::FixDropUses => "Fix uses on item drops".into(),
            Preset::MultiplyExp { by, .. } => format!("Multiply experience by {by}"),
            Preset::DivideExp { by } => format!("Divide experience by {by}"),
            Preset::CombineExp => "Fold Exp Multiplier into Experience".into(),
            Preset::ResetKillTimes => "Clear the kill timers".into(),
            Preset::SetKillTimes { year, month, day } => {
                format!("Set kill times to {year}-{month:02}-{day:02}")
            }
            Preset::ItemDropPct(d) if d.set => format!("Set drop rates to {}%", d.value),
            Preset::ItemDropPct(d) => format!("Change drop rates by {:+}%", d.value),
            Preset::RestockShops => "Restock the shops".into(),
            Preset::StripTextblocks { .. } => "Strip text block endings".into(),
            Preset::CombineRoomItems => "Tidy the floors".into(),
            Preset::InsertBufferRooms { map: Some(m) } => format!("Insert buffer rooms on map {m}"),
            Preset::InsertBufferRooms { map: None } => "Insert buffer rooms".into(),
            Preset::DeleteBufferRooms { map: Some(m) } => format!("Delete buffer rooms on map {m}"),
            Preset::DeleteBufferRooms { map: None } => "Delete buffer rooms".into(),
            Preset::ChangeMonsterIndexes { table, .. } => match table {
                IndexTable::Monsters => "Change monster groups and indexes".into(),
                IndexTable::Rooms => "Change rooms' monster indexes".into(),
            },
            Preset::ReviveCharacter { user, .. } => format!("Revive {user}"),
        }
    }
}

/// Where Revive Character moves a user: NMR's "move them that many rooms back" (1 to 19,
/// from their trail), the Halls of the Dead (1/2189) or a map and room.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviveTo {
    Back(u32),
    Halls,
    Room { map: i64, room: i64 },
}

/// Which table the Monster Group/Index Changer runs over: NMR's runs over both, in turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexTable {
    Monsters,
    Rooms,
}

/// One line of NMR's change list: group/index -> group/index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct IndexChange {
    pub from_group: i64,
    pub from_index: i64,
    pub to_group: i64,
    pub to_index: i64,
}

/// Why a fix cannot run.
#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error("{0}")]
    Bad(String),
    #[error(transparent)]
    Modify(#[from] ModifyError),
}

fn bad(msg: impl Into<String>) -> PresetError {
    PresetError::Bad(msg.into())
}

/// The table and its rows.
fn load(
    realm: &Realm,
    name: &str,
    layout: LayoutSet,
) -> Result<(&'static NmrTable, NmrRows), PresetError> {
    let t = schema::table(name).ok_or_else(|| ModifyError::NoTable(name.into()))?;
    let rows = export_table(realm, t, layout).map_err(ModifyError::from)?;
    Ok((t, rows))
}

fn at(rows: &NmrRows, column: &str) -> Result<usize, PresetError> {
    rows.position(column).ok_or_else(|| {
        PresetError::Modify(ModifyError::NoColumn {
            table: rows.name.clone(),
            column: column.into(),
        })
    })
}

/// VB's `Round`: halves to even.
fn vb_round(x: f64) -> f64 {
    x.round_ties_even()
}

/// NMR's split of a 1.11j experience into Experience × Exp Multiplier: above 100,000 the
/// multiplier is the largest divisor from 3 up to the smallest `x` with `x × 65538 >= exp`
/// (at least 20); an experience with no such divisor loses 1 and is tried again.
pub fn split_exp(mut base: u64) -> (u64, u64) {
    loop {
        if base <= 100_000 {
            return (base.max(1), 1);
        }
        let max = (20..=32767u64).find(|x| x * 65538 >= base).unwrap_or(20);
        let multi = (3..=max)
            .filter(|x| base.is_multiple_of(*x))
            .max()
            .unwrap_or(1);
        if multi == 1 {
            base -= 1;
            continue;
        }
        return ((base / multi).max(1), multi);
    }
}

/// A monster's experience after NMR's multiply or divide, as (Experience, Exp Multiplier).
fn scaled_exp(exp: u64, multi: u64, preset: &Preset) -> (u64, u64) {
    match *preset {
        Preset::MultiplyExp { by, cap } => {
            let cap = match cap {
                Some(c) if c > 0 => c.min(MAX_EXP),
                _ => MAX_EXP,
            };
            // NMR multiplies by a multiplier of 0 too, which leaves an experience of 1; a
            // multiplier of 0 is taken as 1 here, as NMR's divide and combine take it
            split_exp(
                exp.saturating_mul(multi.max(1))
                    .saturating_mul(u64::from(by))
                    .min(cap),
            )
        }
        Preset::DivideExp { by } => {
            let by = f64::from(by);
            if multi <= 1 {
                let base = vb_round(exp as f64 / by).clamp(1.0, MAX_EXP as f64);
                (base as u64, multi)
            } else {
                let base = vb_round(exp.saturating_mul(multi) as f64 / by).min(MAX_EXP as f64);
                split_exp(base as u64)
            }
        }
        _ => (exp, multi),
    }
}

/// A DOS date, as the record's 16-bit Integer holds it.
fn dos_date(year: i32, month: u32, day: u32) -> Result<i64, PresetError> {
    if !(1980..=2107).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(bad(format!(
            "{month:02}/{day:02}/{year} is not a date a DOS date holds"
        )));
    }
    let d = (((year - 1980) as u16) << 9) | ((month as u16) << 5) | day as u16;
    Ok(i64::from(d as i16))
}

/// 01:01:02 as a DOS time.
const KILL_TIME: i64 = (1 << 11) | (1 << 5) | 1;

/// Works out every change `preset` makes, writing nothing.
pub fn plan(realm: &Realm, preset: &Preset, layout: LayoutSet) -> Result<Plan, PresetError> {
    match preset {
        Preset::NoLimitedItems => {
            let (t, rows) = load(realm, "Items", layout)?;
            let (number, limit, slots) =
                (at(&rows, "Number")?, at(&rows, "Game Limit")?, slots(&rows));
            Ok(collect(t, &rows, |row| {
                let gang = slots
                    .iter()
                    .any(|&(a, _)| (181..=184).contains(&(num(&row[a]) as i64)));
                if KEPT_LIMITED.contains(&(num(&row[number]) as i64)) || gang {
                    return Step::Skip;
                }
                let mut new = row.to_vec();
                new[limit] = cell(&row[limit], 0.0);
                Step::Row(new)
            }))
        }
        Preset::NoLevelRestrictions => {
            let (t, rows) = load(realm, "Items", layout)?;
            let slots = slots(&rows);
            Ok(collect(t, &rows, |row| {
                let mut new = row.to_vec();
                for &(a, v) in &slots {
                    if matches!(num(&row[a]) as i64, 135 | 136) {
                        new[a] = cell(&row[a], 0.0);
                        new[v] = cell(&row[v], 0.0);
                    }
                }
                Step::Row(new)
            }))
        }
        Preset::FixItemUses => {
            let (t, rows) = load(realm, "Items", layout)?;
            let (number, name) = (at(&rows, "Number")?, at(&rows, "Name")?);
            let (uses, retain, slots) = (
                at(&rows, "Uses")?,
                at(&rows, "Retain After Uses")?,
                slots(&rows),
            );
            let mut notes = Vec::new();
            let mut plan = collect(t, &rows, |row| {
                let mut new = row.to_vec();
                let n = num(&row[uses]);
                if n == 0.0 {
                    new[uses] = cell(&row[uses], -1.0);
                } else if n > 0.0 && num(&row[retain]) == 1.0 {
                    match slots.iter().find(|&&(a, _)| num(&row[a]) == 121.0) {
                        Some(&(_, v)) => new[v] = cell(&row[v], n),
                        None => notes.push(format!(
                            "Item #{} [{}] has {n} uses, but no recharge ability",
                            num(&row[number]),
                            text(&row[name]),
                        )),
                    }
                }
                Step::Row(new)
            });
            plan.notes = notes;
            Ok(plan)
        }
        Preset::FixDropUses => {
            let (_, items) = load(realm, "Items", layout)?;
            let (inum, iuses) = (at(&items, "Number")?, at(&items, "Uses")?);
            let uses_of: HashMap<i64, Value> = items
                .rows
                .iter()
                .map(|r| (num(&r[inum]) as i64, r[iuses].clone()))
                .collect();
            let (t, rows) = load(realm, "Monsters", layout)?;
            let (mnum, mname) = (at(&rows, "Number")?, at(&rows, "Name")?);
            let drops = drop_slots(&rows)?;
            let mut notes = Vec::new();
            let mut plan = collect(t, &rows, |row| {
                let mut new = row.to_vec();
                for d in &drops {
                    let item = num(&row[d.number]) as i64;
                    if item <= 0 {
                        continue;
                    }
                    match uses_of.get(&item) {
                        Some(u) => new[d.uses] = cell(&row[d.uses], num(u)),
                        None => notes.push(format!(
                            "Monster #{} [{}]: item #{item} not found",
                            num(&row[mnum]),
                            text(&row[mname]),
                        )),
                    }
                }
                Step::Row(new)
            });
            plan.notes = notes;
            Ok(plan)
        }
        Preset::MultiplyExp { by, .. } | Preset::DivideExp { by } => {
            if !(2..=20).contains(by) {
                return Err(bad("multiply or divide by 2 to 20"));
            }
            let (t, rows) = load(realm, "Monsters", layout)?;
            let (exp, multi) = (at(&rows, "Experience")?, at(&rows, "Exp Multiplier")?);
            Ok(collect(t, &rows, |row| {
                let (e, m) = scaled_exp(num(&row[exp]) as u64, num(&row[multi]) as u64, preset);
                let mut new = row.to_vec();
                new[exp] = cell(&row[exp], e as f64);
                new[multi] = cell(&row[multi], m as f64);
                Step::Row(new)
            }))
        }
        Preset::CombineExp => {
            let (t, rows) = load(realm, "Monsters", layout)?;
            let (exp, multi) = (at(&rows, "Experience")?, at(&rows, "Exp Multiplier")?);
            Ok(collect(t, &rows, |row| {
                let m = num(&row[multi]);
                if m == 1.0 {
                    return Step::Row(row.to_vec());
                }
                let mut new = row.to_vec();
                let e = num(&row[exp]) * m.max(1.0);
                new[exp] = cell(&row[exp], e.min(f64::from(u32::MAX)));
                new[multi] = cell(&row[multi], 1.0);
                Step::Row(new)
            }))
        }
        Preset::ResetKillTimes | Preset::SetKillTimes { .. } => {
            let (date, time) = match *preset {
                Preset::SetKillTimes { year, month, day } => {
                    (dos_date(year, month, day)?, KILL_TIME)
                }
                _ => (0, 0),
            };
            let (t, rows) = load(realm, "Monsters", layout)?;
            let (d, tm) = (at(&rows, "Date Killed")?, at(&rows, "Time Killed")?);
            Ok(collect(t, &rows, |row| {
                let mut new = row.to_vec();
                new[d] = cell(&row[d], date as f64);
                new[tm] = cell(&row[tm], time as f64);
                Step::Row(new)
            }))
        }
        Preset::ItemDropPct(o) => drop_pct(realm, o, layout),
        Preset::RestockShops => {
            let (t, rows) = load(realm, "Shops", layout)?;
            let ty = at(&rows, "Type")?;
            let slots = (0..20)
                .map(|x| {
                    Ok([
                        at(&rows, &format!("Item {x}"))?,
                        at(&rows, &format!("Regen Number{x}"))?,
                        at(&rows, &format!("Regen %{x}"))?,
                        at(&rows, &format!("Regen Time {x}"))?,
                        at(&rows, &format!("Normal {x}"))?,
                        at(&rows, &format!("Max {x}"))?,
                    ])
                })
                .collect::<Result<Vec<_>, PresetError>>()?;
            Ok(collect(t, &rows, |row| {
                if num(&row[ty]) == 11.0 {
                    return Step::Skip;
                }
                let mut new = row.to_vec();
                for &[item, rnum, rpct, rtime, now, max] in &slots {
                    if [item, rnum, rpct, rtime]
                        .iter()
                        .all(|&c| num(&row[c]) > 0.0)
                    {
                        new[now] = cell(&row[now], num(&row[max]));
                    }
                }
                Step::Row(new)
            }))
        }
        Preset::StripTextblocks { text } => {
            if text.is_empty() {
                return Err(bad("give the text to take off the end"));
            }
            let (t, rows) = load(realm, "Textblocks", layout)?;
            let data = at(&rows, "Data")?;
            Ok(collect(t, &rows, |row| {
                let mut new = row.to_vec();
                if let Value::Text(d) = &row[data]
                    && let Some(kept) = d.strip_suffix(text.as_str())
                {
                    new[data] = Value::Text(kept.to_string());
                }
                Step::Row(new)
            }))
        }
        Preset::InsertBufferRooms { map } => insert_buffer_rooms(realm, *map, layout),
        Preset::ChangeMonsterIndexes { table, changes } => {
            change_indexes(realm, *table, changes, layout)
        }
        Preset::ReviveCharacter {
            user,
            lives,
            spells,
            to,
        } => revive(realm, user, *lives, *spells, to.as_ref(), layout),
        Preset::DeleteBufferRooms { map } => {
            let (t, rows) = load(realm, "Rooms", layout)?;
            let (m, r, name) = (
                at(&rows, "Map Number")?,
                at(&rows, "Room Number")?,
                at(&rows, "Name")?,
            );
            let mut plan = Plan {
                table: t.name.to_string(),
                ..Plan::default()
            };
            for row in &rows.rows {
                if map.is_some_and(|want| num(&row[m]) as i64 != want) {
                    continue;
                }
                if text(&row[name]).to_uppercase().starts_with("BUFFER ROOM") {
                    let key = vec![row[m].clone(), row[r].clone()];
                    plan.changes.push(Change {
                        key: key.clone(),
                        column: "(deleted)".into(),
                        before: row[name].clone(),
                        after: Value::Null,
                    });
                    plan.deletes.push(key);
                } else {
                    plan.unchanged += 1;
                }
            }
            Ok(plan)
        }
        Preset::CombineRoomItems => {
            let (t, rows) = load(realm, "Rooms", layout)?;
            let family = |stem: &str, n: usize| -> Result<Vec<[usize; 3]>, PresetError> {
                (0..n)
                    .map(|x| {
                        Ok([
                            at(&rows, &format!("{stem} {x}"))?,
                            at(&rows, &format!("{stem} {x} USES"))?,
                            at(&rows, &format!("{stem} {x} QTY"))?,
                        ])
                    })
                    .collect()
            };
            let (floor, hidden) = (family("Room Item", 17)?, family("Hidden Item", 15)?);
            Ok(collect(t, &rows, |row| {
                let mut new = row.to_vec();
                for slots in [&floor, &hidden] {
                    combine(&mut new, slots);
                }
                Step::Row(new)
            }))
        }
    }
}

/// NMR's `mnuRoomPad_Click`: a room for each number missing from 1 to the map's last
/// room, all zero but its name, ANSI map, first description line and one exit (to room 1,
/// type 8, parameter 1), as NMR makes them.
fn insert_buffer_rooms(
    realm: &Realm,
    map: Option<i64>,
    layout: LayoutSet,
) -> Result<Plan, PresetError> {
    let (t, rows) = load(realm, "Rooms", layout)?;
    let (m, r) = (at(&rows, "Map Number")?, at(&rows, "Room Number")?);
    let maps: Vec<i64> = match map {
        Some(m) if m > 0 => vec![m],
        Some(m) => return Err(bad(format!("no map {m}"))),
        None => (1..=18).collect(),
    };
    let blank: Vec<Value> = t
        .columns
        .iter()
        .map(|c| match c.ty {
            schema::ColumnType::Long => Value::Long(0),
            schema::ColumnType::Double => Value::Double(0.0),
            _ => Value::Text(String::new()),
        })
        .collect();
    let set = |row: &mut Vec<Value>, column: &str, v: Value| -> Result<(), PresetError> {
        let i = at(&rows, column)?;
        row[i] = v;
        Ok(())
    };
    let mut plan = Plan {
        table: t.name.to_string(),
        ..Plan::default()
    };
    let mut new_rows = Vec::new();
    for map in maps {
        let have: std::collections::BTreeSet<i64> = rows
            .rows
            .iter()
            .filter(|row| num(&row[m]) as i64 == map)
            .map(|row| num(&row[r]) as i64)
            .collect();
        let Some(&last) = have.last() else {
            plan.notes.push(format!("Map {map} has no rooms"));
            continue;
        };
        for n in (1..last).filter(|n| !have.contains(n)) {
            let mut row = blank.clone();
            let name = format!("Buffer Room {n}");
            set(&mut row, "Map Number", Value::Long(map as i32))?;
            set(&mut row, "Room Number", Value::Long(n as i32))?;
            set(&mut row, "Name", Value::Text(name.clone()))?;
            set(&mut row, "AnsiMap", Value::Text("WCCMAP01.ANS".into()))?;
            set(
                &mut row,
                "Desc 0",
                Value::Text(
                    "This is just a buffer room to keep the rooms scanning on load.".into(),
                ),
            )?;
            set(&mut row, "Exit 0", Value::Long(1))?;
            set(&mut row, "Type 0", Value::Long(8))?;
            set(&mut row, "Para1 0", Value::Long(1))?;
            plan.changes.push(Change {
                key: vec![Value::Long(map as i32), Value::Long(n as i32)],
                column: "(new)".into(),
                before: Value::Null,
                after: Value::Text(name),
            });
            new_rows.push(row);
        }
    }
    plan.rows = Some(NmrRows {
        name: rows.name.clone(),
        columns: rows.columns.clone(),
        rows: new_rows,
    });
    Ok(plan)
}

/// NMR's `mnuRoomsCombineItems_Click`: while two slots hold the same item with the same
/// uses, the later-found one's quantity (less one, as slots hold it) moves to the first.
fn combine(row: &mut [Value], slots: &[[usize; 3]]) {
    'again: loop {
        for (x, &[ix, ux, qx]) in slots.iter().enumerate() {
            if num(&row[ix]) <= 0.0 {
                continue;
            }
            for (y, &[iy, uy, qy]) in slots.iter().enumerate() {
                if y != x && num(&row[iy]) == num(&row[ix]) && num(&row[uy]) == num(&row[ux]) {
                    let q = num(&row[qx]) + num(&row[qy]) + 1.0;
                    row[qx] = cell(&row[qx], q);
                    for c in [iy, uy, qy] {
                        row[c] = cell(&row[c], 0.0);
                    }
                    continue 'again;
                }
            }
        }
        return;
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::Text(s) => s.split('\0').next().unwrap_or("").trim_end().to_string(),
        _ => String::new(),
    }
}

/// A monster's drop: the positions of its item, uses and drop %.
struct Drop {
    number: usize,
    uses: usize,
    pct: usize,
}

fn drop_slots(rows: &NmrRows) -> Result<Vec<Drop>, PresetError> {
    (0..10)
        .map(|x| {
            Ok(Drop {
                number: at(rows, &format!("Item Number {x}"))?,
                uses: at(rows, &format!("Item Uses {x}"))?,
                pct: at(rows, &format!("Item Drop % {x}"))?,
            })
        })
        .collect()
}

/// NMR's `frmMonItemDropPct.cmdGo_Click`.
fn drop_pct(realm: &Realm, o: &DropPct, layout: LayoutSet) -> Result<Plan, PresetError> {
    let (cmp, limit) = o.only_if;
    if !(0..=100).contains(&limit) {
        return Err(bad("the drop % to compare with must be 0 to 100"));
    }
    if o.value > 100 || (o.set && o.value < 0) || (!o.set && o.value == 0) {
        return Err(bad(
            "set to 0 to 100, or add a value other than 0 (up to 100)",
        ));
    }
    let (_, items) = load(realm, "Items", layout)?;
    let (inum, itype, worn, glimit) = (
        at(&items, "Number")?,
        at(&items, "Type")?,
        at(&items, "Worn On")?,
        at(&items, "Game Limit")?,
    );
    let islots = slots(&items);
    let by_number: HashMap<i64, &Vec<Value>> = items
        .rows
        .iter()
        .map(|r| (num(&r[inum]) as i64, r))
        .collect();

    let (t, rows) = load(realm, "Monsters", layout)?;
    let (mnum, mname, regen) = (
        at(&rows, "Number")?,
        at(&rows, "Name")?,
        at(&rows, "Regen Time")?,
    );
    let drops = drop_slots(&rows)?;
    let mut notes = Vec::new();
    let mut plan = collect(t, &rows, |row| {
        if o.min_regen.is_some_and(|n| num(&row[regen]) as i64 <= n) {
            return Step::Skip;
        }
        let mut new = row.to_vec();
        for d in &drops {
            let item = num(&row[d.number]) as i64;
            let pct = num(&row[d.pct]) as i64;
            let mut matched = item > 0
                && pct > 0
                && match cmp {
                    Cmp::Eq => pct == limit,
                    Cmp::Le => pct <= limit,
                    Cmp::Ge => pct >= limit,
                    Cmp::Ne => pct != limit,
                };
            // NMR leaves a drop already at the value alone, adding or setting
            if pct == o.value {
                matched = false;
            }
            if !matched {
                continue;
            }
            let Some(it) = by_number.get(&item) else {
                notes.push(format!(
                    "Monster #{} - {}: item #{item} not found",
                    num(&row[mnum]),
                    text(&row[mname]),
                ));
                continue;
            };
            // 0 armour (worn somewhere), 1 weapon, 7 key, 8 container, 9 scroll
            let kind = num(&it[itype]) as i64;
            let wanted = match kind {
                0 if num(&it[worn]) != 0.0 => o.armour,
                1 => o.weapons,
                7 => o.keys,
                8 => o.containers,
                9 => o.scrolls,
                _ => false,
            };
            let limited = num(&it[glimit]) > 0.0;
            let by_limit = match o.limited {
                Limited::Include => true,
                Limited::Exclude => !limited,
                Limited::Only => limited,
            };
            let del_at_maint = o.exclude_del_at_maint
                && matches!(kind, 0 | 1)
                && islots.iter().any(|&(a, _)| num(&it[a]) == 119.0);
            if !wanted || !by_limit || del_at_maint {
                continue;
            }
            let to = if o.set {
                o.value
            } else {
                (pct + o.value).clamp(1, 100)
            };
            new[d.pct] = cell(&row[d.pct], to as f64);
        }
        Step::Row(new)
    });
    plan.notes = notes;
    Ok(plan)
}

/// NMR's Monster Group/Index Changer (`frmMonsterIndexChanger.ScanMonsters` and
/// `ScanRooms`). Its Add refuses a `from` already in the list; so does this.
fn change_indexes(
    realm: &Realm,
    table: IndexTable,
    changes: &[IndexChange],
    layout: LayoutSet,
) -> Result<Plan, PresetError> {
    if changes.is_empty() {
        return Err(bad("the change list is empty"));
    }
    let mut seen = std::collections::HashSet::new();
    for c in changes {
        if c.from_index < 0 || c.to_index < 0 {
            return Err(bad("indexes cannot be negative"));
        }
        if !(0..=39).contains(&c.from_group) || !(0..=39).contains(&c.to_group) {
            return Err(bad("groups run from 0 to 39"));
        }
        if !seen.insert((c.from_group, c.from_index)) {
            return Err(bad(format!(
                "group {} index {} is changed twice",
                c.from_group, c.from_index
            )));
        }
    }
    match table {
        IndexTable::Monsters => {
            let (t, rows) = load(realm, "Monsters", layout)?;
            let (group, index) = (at(&rows, "Group")?, at(&rows, "Index")?);
            Ok(collect(t, &rows, |row| {
                let (g, i) = (num(&row[group]) as i64, num(&row[index]) as i64);
                let Some(c) = changes
                    .iter()
                    .find(|c| c.from_group == g && c.from_index == i)
                else {
                    return Step::Out;
                };
                let mut new = row.to_vec();
                new[group] = cell(&row[group], c.to_group as f64);
                new[index] = cell(&row[index], c.to_index as f64);
                Step::Row(new)
            }))
        }
        IndexTable::Rooms => {
            let (t, rows) = load(realm, "Rooms", layout)?;
            let (group, min, max) = (
                at(&rows, "Mon Type")?,
                at(&rows, "Min Index")?,
                at(&rows, "Max Index")?,
            );
            Ok(collect(t, &rows, |row| {
                let g = num(&row[group]) as i64;
                let (lo, hi) = (num(&row[min]) as i64, num(&row[max]) as i64);
                let Some(c) = changes
                    .iter()
                    .find(|c| c.from_group == g && (lo..=hi).contains(&c.from_index))
                else {
                    return Step::Out;
                };
                let mut new = row.to_vec();
                new[group] = cell(&row[group], c.to_group as f64);
                new[min] = cell(&row[min], c.to_index as f64);
                new[max] = cell(&row[max], (c.to_index + hi - lo) as f64);
                Step::Row(new)
            }))
        }
    }
}

/// NMR's Revive Character (`cmdReviveChar_Click`), its questions answered up front.
fn revive(
    realm: &Realm,
    user: &str,
    lives: bool,
    spells: bool,
    to: Option<&ReviveTo>,
    layout: LayoutSet,
) -> Result<Plan, PresetError> {
    if let Some(ReviveTo::Back(n)) = to
        && !(1..=19).contains(n)
    {
        return Err(bad("a user moves 1 to 19 rooms back"));
    }
    let (t, rows) = load(realm, "Users", layout)?;
    let name = at(&rows, "BBS Name")?;
    let want = crate::export::key_text(user);
    if !rows.rows.iter().any(|r| match &r[name] {
        Value::Text(s) => crate::export::key_text(s) == want,
        _ => false,
    }) {
        return Err(bad(format!("no user {user}")));
    }
    let hp = (at(&rows, "Max HP")?, at(&rows, "HP")?);
    let mana = (at(&rows, "Max Mana")?, at(&rows, "Mana")?);
    let life = at(&rows, "Lives")?;
    let poison = at(&rows, "Poison")?;
    let cast: Vec<usize> = (0..10)
        .flat_map(|i| {
            [
                format!("Spell Cast {i}"),
                format!("Spell Cast {i} Value"),
                format!("Spell Cast {i} Rounds"),
            ]
        })
        .map(|c| at(&rows, &c))
        .collect::<Result<_, _>>()?;
    let (map, room) = (at(&rows, "Map Number")?, at(&rows, "Room Number")?);
    let trail = match to {
        Some(ReviveTo::Back(n)) => Some((
            at(&rows, &format!("Trail {n} Map"))?,
            at(&rows, &format!("Trail {n} Room"))?,
        )),
        _ => None,
    };
    Ok(collect(t, &rows, |row| {
        match &row[name] {
            Value::Text(s) if crate::export::key_text(s) == want => {}
            _ => return Step::Out,
        }
        let mut new = row.to_vec();
        for (max, cur) in [hp, mana] {
            new[cur] = row[max].clone();
        }
        if lives && num(&row[life]) < 9.0 {
            new[life] = cell(&row[life], 9.0);
        }
        if spells {
            for &c in cast.iter().chain([&poison]) {
                new[c] = cell(&row[c], 0.0);
            }
        }
        // as NMR's: a user already in the Halls is not asked to move
        let in_halls = num(&row[map]) == 1.0 && num(&row[room]) == 2189.0;
        let place = match to {
            _ if in_halls => None,
            None => None,
            Some(ReviveTo::Halls) => Some((1.0, 2189.0)),
            Some(ReviveTo::Room { map, room }) => Some((*map as f64, *room as f64)),
            Some(ReviveTo::Back(_)) => trail.map(|(m, r)| (num(&row[m]), num(&row[r]))),
        };
        if let Some((m, r)) = place {
            new[map] = cell(&row[map], m);
            new[room] = cell(&row[room], r);
        }
        Step::Row(new)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fix_is_named_for_the_history() {
        let fix = |v: serde_json::Value| serde_json::from_value::<Preset>(v).unwrap().label();
        assert_eq!(
            fix(
                serde_json::json!({"preset": "set_kill_times", "year": 2026, "month": 3, "day": 7})
            ),
            "Set kill times to 2026-03-07"
        );
        assert_eq!(
            fix(serde_json::json!({"preset": "insert_buffer_rooms", "map": 2})),
            "Insert buffer rooms on map 2"
        );
        assert_eq!(
            fix(serde_json::json!({"preset": "delete_buffer_rooms", "map": null})),
            "Delete buffer rooms"
        );
        let drops = |set: bool, value: i64| {
            fix(serde_json::json!({
                "preset": "item_drop_pct", "set": set, "value": value, "only_if": ["ge", 0],
                "weapons": true, "armour": true, "keys": true, "containers": true, "scrolls": true,
            }))
        };
        assert_eq!(drops(true, 20), "Set drop rates to 20%");
        assert_eq!(drops(false, -5), "Change drop rates by -5%");
        assert_eq!(drops(false, 5), "Change drop rates by +5%");
    }

    #[test]
    fn exp_splits_as_nmrs() {
        assert_eq!(split_exp(0), (1, 1));
        assert_eq!(split_exp(90_000), (90_000, 1));
        // 200,000 = 20 × 10,000; the largest divisor up to 20
        assert_eq!(split_exp(200_000), (10_000, 20));
        // a prime above 100,000 loses 1 until it divides
        // (100,002 = 2 × 3 × 7 × 2381)
        assert_eq!(split_exp(100_003), (7143, 14));
        let (e, m) = split_exp(2_000_000_000);
        assert!(e * m == 2_000_000_000 && e <= 65538 * m, "{e} × {m}");
    }

    #[test]
    fn exp_scales_as_nmrs() {
        let mul = |by, cap| Preset::MultiplyExp { by, cap };
        assert_eq!(scaled_exp(1000, 1, &mul(2, None)), (2000, 1));
        assert_eq!(scaled_exp(1000, 0, &mul(2, None)), (2000, 1));
        assert_eq!(scaled_exp(1000, 1, &mul(2, Some(1500))), (1500, 1));
        assert_eq!(scaled_exp(60_000, 2, &mul(2, None)), (12_000, 20));
        let div = |by| Preset::DivideExp { by };
        // VB's Round: 5 / 2 = 2.5 rounds to 2
        assert_eq!(scaled_exp(5, 1, &div(2)), (2, 1));
        assert_eq!(scaled_exp(1, 0, &div(3)), (1, 0));
        assert_eq!(scaled_exp(10_000, 30, &div(2)), (7500, 20));
    }

    #[test]
    fn dates_are_dos_dates() {
        assert_eq!(dos_date(1990, 1, 1).unwrap(), (10 << 9) | (1 << 5) | 1);
        assert_eq!(KILL_TIME, 2081);
        assert!(dos_date(1979, 1, 1).is_err());
        assert!(dos_date(2000, 13, 1).is_err());
    }
}
