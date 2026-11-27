//! Health checks across the whole world: what is wrong that has a sure fix, and counts worth
//! knowing. A check with a fix counts what its fix would change (it works the fix out to count
//! it), so the count and what one click does always agree, and the fix is one step of the
//! history. Home and the Workshop show them.
//!
//! - [`Check::Uses`]: NMR's Fix Number of Uses ([`Preset::FixItemUses`]).
//! - [`Check::BlankBlows`]: blows told by a blank message, as the wiring's
//!   [placeholders](wiring::Check::Placeholders) check finds them, each set to 0 (the game's
//!   own words).
//! - [`Check::Exits`]: exits to rooms the world does not have. No sure fix: the room may be
//!   still to dig, or the exit a slip.
//! - [`Check::DropUses`]: NMR's Fix Number of Uses on Item Drops ([`Preset::FixDropUses`]).
//! - [`Check::Leftovers`]: empty ability slots (no ability) that still hold a value, such as
//!   8224, two spaces an old tool left. The game skips them; the fix sets each to 0.
//! - [`Check::Spaces`]: text blocks whose text ends in spaces (before its last line break, if
//!   it has one), NMR's Strip characters off the end done for you. Spaces inside a text are
//!   left alone: on a coloured line they can be drawn. A text goes on from part to part, so a
//!   part that ends in a space is only trimmed when it is the end of the text.
//! - [`Check::Limited`]: items with a game limit, NMR's Limited Item List: a count to know,
//!   not a problem.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::data::NmrRows;
use crate::export::export_table;
use crate::modify::{ModifyError, Plan, Step, cell, collect, num, slots};
use crate::presets::{self, Preset, PresetError};
use crate::refs::MAP_CHANGE;
use crate::rows::RowError;
use crate::schema::{self, NmrTable, Value};
use crate::wiring::{self, Fingerprint, Rec};

/// The records a check lists at most.
pub const LISTED: usize = 60;

/// The tables whose records have ability slots.
const WITH_ABILITIES: [&str; 5] = ["Items", "Monsters", "Spells", "Classes", "Races"];

/// One health check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Check {
    Uses,
    BlankBlows,
    Exits,
    DropUses,
    Leftovers,
    Spaces,
    Limited,
}

impl Check {
    pub const ALL: [Check; 7] = [
        Check::Uses,
        Check::BlankBlows,
        Check::Exits,
        Check::DropUses,
        Check::Leftovers,
        Check::Spaces,
        Check::Limited,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Check::Uses => "uses",
            Check::BlankBlows => "blank_blows",
            Check::Exits => "exits",
            Check::DropUses => "drop_uses",
            Check::Leftovers => "leftovers",
            Check::Spaces => "spaces",
            Check::Limited => "limited",
        }
    }

    pub fn parse(id: &str) -> Option<Check> {
        Check::ALL.into_iter().find(|c| c.id() == id)
    }

    /// What its fix is called in the history; `None` when it has no sure fix.
    pub fn fix_label(self) -> Option<&'static str> {
        match self {
            Check::Uses => Some("Fix uses on items"),
            Check::BlankBlows => Some("Clear blank attack messages"),
            Check::DropUses => Some("Fix uses on item drops"),
            Check::Leftovers => Some("Clear leftover ability values"),
            Check::Spaces => Some("Trim text block endings"),
            Check::Exits | Check::Limited => None,
        }
    }

    /// How much what it finds matters.
    fn level(self) -> Level {
        match self {
            Check::Uses | Check::BlankBlows | Check::Exits => Level::Warn,
            Check::DropUses | Check::Leftovers | Check::Spaces => Level::Note,
            Check::Limited => Level::Info,
        }
    }
}

/// How much a check's findings matter: players meet them, worth a tidy, a count to know, or
/// nothing found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Warn,
    Note,
    Info,
    Ok,
}

impl Level {
    pub fn id(self) -> &'static str {
        match self {
            Level::Warn => "warn",
            Level::Note => "note",
            Level::Info => "info",
            Level::Ok => "ok",
        }
    }
}

/// What a check found.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub check: Check,
    pub level: Level,
    /// The records it is about: for a check with a fix, the records the fix changes.
    pub records: usize,
    /// The values it is about (uses, messages, slots, exits), at least one a record.
    pub values: usize,
    /// The first [`LISTED`] of those records.
    pub list: Vec<Rec>,
    /// What its fix could not put right, as NMR logs it (an item with uses but no recharge).
    pub notes: Vec<String>,
}

/// Why the checks cannot be made.
#[derive(Debug, thiserror::Error)]
pub enum HealthError {
    #[error(transparent)]
    Preset(#[from] PresetError),
    #[error(transparent)]
    Modify(#[from] ModifyError),
    #[error(transparent)]
    Row(#[from] RowError),
}

/// Every check on the realm, in [`Check::ALL`]'s order.
pub fn report(realm: &Realm, layout: LayoutSet) -> Result<Vec<Found>, HealthError> {
    let mut out = Vec::new();
    for check in Check::ALL {
        let ((records, values, list), notes) = match check {
            Check::Exits => (exits(realm, layout)?, Vec::new()),
            Check::Limited => (limited(realm, layout)?, Vec::new()),
            _ => {
                let plans = fix(realm, check, layout)?;
                let notes = plans.iter().flat_map(|p| p.notes.clone()).collect();
                (of_plans(&plans), notes)
            }
        };
        let level = if records == 0 {
            Level::Ok
        } else {
            check.level()
        };
        out.push(Found {
            check,
            level,
            records,
            values,
            list,
            notes,
        });
    }
    Ok(out)
}

/// The realms whose report is kept.
const KEPT: usize = 4;

/// The report on the realm in `dir`: kept, and made again only when the realm's files change,
/// as the wiring's is ([`wiring::fingerprint`]).
pub fn cached(dir: &Path) -> Result<Arc<Vec<Found>>, HealthError> {
    type Kept = Vec<(PathBuf, Fingerprint, Arc<Vec<Found>>)>;
    static CACHE: Mutex<Kept> = Mutex::new(Vec::new());
    let print = wiring::fingerprint(dir);
    // held while a report is made, so two asks make it once
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, _, r)) = cache.iter().find(|(d, p, _)| d == dir && *p == print) {
        return Ok(r.clone());
    }
    let realm = Realm::open(dir).map_err(RowError::from)?;
    let r = Arc::new(report(&realm, crate::layout_of_realm(&realm))?);
    cache.retain(|(d, _, _)| d != dir);
    if cache.len() >= KEPT {
        cache.remove(0);
    }
    cache.push((dir.to_path_buf(), print, r.clone()));
    Ok(r)
}

/// The plans that put `check` right, one for each table it changes; none when it has no sure
/// fix or nothing to fix.
pub fn fix(realm: &Realm, check: Check, layout: LayoutSet) -> Result<Vec<Plan>, HealthError> {
    let plans = match check {
        Check::Uses => vec![presets::plan(realm, &Preset::FixItemUses, layout)?],
        Check::DropUses => vec![presets::plan(realm, &Preset::FixDropUses, layout)?],
        Check::BlankBlows => blank_blows(realm, layout)?,
        Check::Leftovers => WITH_ABILITIES
            .iter()
            .map(|name| leftovers(realm, name, layout))
            .collect::<Result<_, _>>()?,
        Check::Spaces => vec![spaces(realm, layout)?],
        Check::Exits | Check::Limited => Vec::new(),
    };
    Ok(plans.into_iter().filter(|p| p.records() > 0).collect())
}

/// Records, values and the first records listed, from what plans change.
fn of_plans(plans: &[Plan]) -> (usize, usize, Vec<Rec>) {
    let records = plans.iter().map(Plan::records).sum();
    let values = plans.iter().map(|p| p.changes.len()).sum();
    let mut list: Vec<Rec> = Vec::new();
    for p in plans {
        let Some(t) = schema::table(&p.table) else {
            continue;
        };
        for c in &p.changes {
            if list.len() >= LISTED {
                break;
            }
            let key: Vec<i64> = c.key.iter().map(|v| num(v) as i64).collect();
            if !list.iter().any(|r| r.table == t.name && r.key == key) {
                list.push(Rec { table: t.name, key });
            }
        }
    }
    (records, values, list)
}

/// The table and its rows.
fn load(
    realm: &Realm,
    name: &str,
    layout: LayoutSet,
) -> Result<(&'static NmrTable, NmrRows), HealthError> {
    let t = schema::table(name).ok_or_else(|| ModifyError::NoTable(name.into()))?;
    let rows = export_table(realm, t, layout).map_err(ModifyError::from)?;
    Ok((t, rows))
}

fn at(rows: &NmrRows, column: &str) -> Result<usize, HealthError> {
    rows.position(column).ok_or_else(|| {
        HealthError::Modify(ModifyError::NoColumn {
            table: rows.name.clone(),
            column: column.into(),
        })
    })
}

/// A record's key, as whole numbers.
fn key_of(t: &NmrTable, rows: &NmrRows, row: &[Value]) -> Vec<i64> {
    t.index
        .as_ref()
        .map(|i| {
            i.columns
                .iter()
                .filter_map(|c| rows.position(c))
                .map(|p| num(&row[p]) as i64)
                .collect()
        })
        .unwrap_or_default()
}

/// The values to set on a table's records, `(column, value)`, by each record's key.
type Sets = HashMap<Vec<i64>, Vec<(String, i64)>>;

/// The placeholders check's fixes (blows told by a blank message), as plans.
fn blank_blows(realm: &Realm, layout: LayoutSet) -> Result<Vec<Plan>, HealthError> {
    let report = wiring::report(realm.dir())?;
    let mut sets: BTreeMap<&'static str, Sets> = BTreeMap::new();
    for f in &report.findings {
        if f.check == wiring::Check::Placeholders && !f.fix.is_empty() {
            sets.entry(f.about.table)
                .or_default()
                .entry(f.about.key.clone())
                .or_default()
                .extend(f.fix.iter().cloned());
        }
    }
    sets.into_iter()
        .map(|(name, sets)| {
            let (t, rows) = load(realm, name, layout)?;
            Ok(collect(t, &rows, |row| {
                let Some(set) = sets.get(&key_of(t, &rows, row)) else {
                    return Step::Out;
                };
                let mut new = row.to_vec();
                for (column, v) in set {
                    if let Some(i) = rows.position(column) {
                        new[i] = cell(&row[i], *v as f64);
                    }
                }
                Step::Row(new)
            }))
        })
        .collect()
}

/// The values of a table's empty ability slots, each set to 0.
fn leftovers(realm: &Realm, name: &str, layout: LayoutSet) -> Result<Plan, HealthError> {
    let (t, rows) = load(realm, name, layout)?;
    let slots = slots(&rows);
    Ok(collect(t, &rows, |row| {
        let mut new = row.to_vec();
        for &(a, v) in &slots {
            if num(&row[a]) == 0.0 && num(&row[v]) != 0.0 {
                new[v] = cell(&row[v], 0.0);
            }
        }
        Step::Row(new)
    }))
}

/// Each text block's text with the spaces at its end taken off, part by part.
fn spaces(realm: &Realm, layout: LayoutSet) -> Result<Plan, HealthError> {
    let (t, rows) = load(realm, "Textblocks", layout)?;
    let (number, part, data) = (
        at(&rows, "Number")?,
        at(&rows, "Part #")?,
        at(&rows, "Data")?,
    );
    let id = |row: &[Value]| (num(&row[number]) as i64, num(&row[part]) as i64);
    let mut blocks: BTreeMap<i64, Vec<(i64, &str)>> = BTreeMap::new();
    for row in &rows.rows {
        if let Value::Text(d) = &row[data] {
            let (n, p) = id(row);
            blocks.entry(n).or_default().push((p, d));
        }
    }
    let mut trimmed: HashMap<(i64, i64), String> = HashMap::new();
    for (n, mut parts) in blocks {
        parts.sort_by_key(|p| p.0);
        let texts: Vec<&str> = parts.iter().map(|p| p.1).collect();
        let Some(new) = trim_end(&texts) else {
            continue;
        };
        for ((p, old), new) in parts.into_iter().zip(new) {
            if old != new {
                trimmed.insert((n, p), new);
            }
        }
    }
    Ok(collect(t, &rows, |row| match trimmed.get(&id(row)) {
        Some(d) => {
            let mut new = row.to_vec();
            new[data] = Value::Text(d.clone());
            Step::Row(new)
        }
        None => Step::Out,
    }))
}

/// A text's parts with the spaces at the end of the text (before its last line break, if it
/// has one) taken off, or `None` when it ends in none. Only the parts that hold them change.
pub fn trim_end(parts: &[&str]) -> Option<Vec<String>> {
    let text = parts.concat();
    let body = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(&text);
    let kept = body.trim_end_matches([' ', '\t']);
    if kept.len() == body.len() {
        return None;
    }
    // the run to take off, in bytes of the whole text: spaces, so each end is a character's
    let (from, to) = (kept.len(), body.len());
    let mut start = 0;
    Some(
        parts
            .iter()
            .map(|p| {
                let end = start + p.len();
                let cut = (from.clamp(start, end) - start, to.clamp(start, end) - start);
                start = end;
                format!("{}{}", &p[..cut.0], &p[cut.1..])
            })
            .collect(),
    )
}

/// Rooms with exits to rooms the world does not have: a map change's exit leads to a room of
/// the map its first parameter names, any other to a room of its own map.
fn exits(realm: &Realm, layout: LayoutSet) -> Result<(usize, usize, Vec<Rec>), HealthError> {
    let (t, rows) = load(realm, "Rooms", layout)?;
    let (map, room) = (at(&rows, "Map Number")?, at(&rows, "Room Number")?);
    let family = |stem: &str| -> Result<Vec<usize>, HealthError> {
        (0..10).map(|d| at(&rows, &format!("{stem} {d}"))).collect()
    };
    let (to, kind, para1) = (family("Exit")?, family("Type")?, family("Para1")?);
    let have: HashSet<(i64, i64)> = rows
        .rows
        .iter()
        .map(|r| (num(&r[map]) as i64, num(&r[room]) as i64))
        .collect();
    let (mut records, mut values, mut list) = (0, 0, Vec::new());
    for r in &rows.rows {
        let own = num(&r[map]) as i64;
        let lost = (0..10)
            .filter(|&d| {
                let n = num(&r[to[d]]) as i64;
                let m = if num(&r[kind[d]]) as i64 == MAP_CHANGE {
                    num(&r[para1[d]]) as i64
                } else {
                    own
                };
                n > 0 && !have.contains(&(m, n))
            })
            .count();
        if lost > 0 {
            records += 1;
            values += lost;
            if list.len() < LISTED {
                list.push(Rec {
                    table: t.name,
                    key: key_of(t, &rows, r),
                });
            }
        }
    }
    Ok((records, values, list))
}

/// Items with a game limit and a name, as NMR's Limited Item List has them.
fn limited(realm: &Realm, layout: LayoutSet) -> Result<(usize, usize, Vec<Rec>), HealthError> {
    let (t, rows) = load(realm, "Items", layout)?;
    let (name, limit) = (at(&rows, "Name")?, at(&rows, "Game Limit")?);
    let named = |v: &Value| matches!(v, Value::Text(s) if !s.trim().is_empty());
    let all: Vec<&Vec<Value>> = rows
        .rows
        .iter()
        .filter(|r| num(&r[limit]) != 0.0 && named(&r[name]))
        .collect();
    let list = all
        .iter()
        .take(LISTED)
        .map(|r| Rec {
            table: t.name,
            key: key_of(t, &rows, r),
        })
        .collect();
    Ok((all.len(), all.len(), list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_of_a_text_is_trimmed_across_its_parts() {
        let t = |parts: &[&str]| trim_end(parts);
        assert_eq!(t(&["Hello  \n"]), Some(vec!["Hello\n".into()]));
        assert_eq!(t(&["Hello\t \r\n"]), Some(vec!["Hello\r\n".into()]));
        assert_eq!(t(&["Hello  "]), Some(vec!["Hello".into()]));
        // a part that goes on in the next keeps its space
        assert_eq!(t(&["giveitem ", "8:giveitem 9\n"]), None);
        // the run split over two parts
        assert_eq!(
            t(&["a line ", "  \n"]),
            Some(vec!["a line".into(), "\n".into()])
        );
        // spaces inside are left alone, and so is a blank last line
        assert_eq!(t(&["one  \ntwo\n"]), None);
        assert_eq!(t(&["one\n\n"]), None);
        assert_eq!(t(&[""]), None);
        assert_eq!(t(&["é  \n"]), Some(vec!["é\n".into()]));
    }

    #[test]
    fn checks_have_ids() {
        for c in Check::ALL {
            assert_eq!(Check::parse(c.id()), Some(c));
        }
        assert_eq!(Check::parse("nope"), None);
    }
}
