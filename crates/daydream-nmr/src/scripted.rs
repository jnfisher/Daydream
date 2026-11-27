//! The links between rooms that no exit draws, for the World map's layer of scripted links:
//! where a room's scripts and spells send a player, and the exits its remote actions work.
//! Read as silvermere runs them (silvermere-engine; see UPSTREAM.md):
//!
//! - A room's command block (`Command Text`) runs the lines whose words a player types
//!   (`scripts/special.rs`), each line's clauses in turn. A `random` block runs a line by
//!   chance and a `testskill` or `checkspell` block runs whole (`scripts/actions.rs`); `text`
//!   and `roomtext` only show a block.
//! - `teleport room map` moves the player there (both above 0). A named one
//!   (`teleport_slums`) rolls a room from its area's ranges, so it names no room and is left
//!   out.
//! - `remoteaction room msg action exit`, and a Remote Action exit, work an exit of a room on
//!   the same map.
//! - A spell teleports when it has a Teleport Room and a Teleport Map (`spells/room.rs`; a
//!   value of 0 is the cast's roll, from the spell's Min to its Max), and runs its Trigger
//!   Text Block. A room's spell is cast on whoever is in the room, a Cast exit's two on
//!   whoever walks it, a Spell Trap's on whoever springs it (`movement/exits.rs`), and a
//!   `cast` clause's on the player.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::reports::{ReportError, Rows, n, text};
use crate::script::{self, Clause, Reading};
use crate::wiring::{Fingerprint, fingerprint};

/// How a room sets a source off.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "by", rename_all = "lowercase")]
pub enum When {
    /// A line of the room's commands: the words a player types (`|` between alternatives, and
    /// a `*` lets the rest be cut short).
    Typed { words: String },
    /// The room's spell, cast on whoever is in the room.
    Here,
    /// Walking an exit: a Cast exit's spells, a Spell Trap's.
    Walking { dir: usize },
    /// A Remote Action exit, by the lines of its message: the words a player types.
    Words { dir: usize, words: Vec<String> },
}

/// Where a source leads.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScriptTarget {
    /// `teleport` (a script moves the player), `spell` (a spell does) or `remote` (a remote
    /// action works an exit of the room it leads to).
    pub kind: &'static str,
    /// The room it leads to: on the map the source runs on when `to_map` is 0 (a remote
    /// action's). A spell whose Teleport Room is its roll lands in a room from `to_room` to
    /// `to_last`.
    pub to_map: i64,
    pub to_room: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_last: Option<i64>,
    /// The exit a remote action works, 0 to 9.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<usize>,
    /// The record that does it: a text block (`Textblocks`), a spell (`Spells`) or the room's
    /// own exit (`Rooms`); and what in it: the clause as written, or the spell's name.
    pub table: &'static str,
    pub record: i64,
    pub what: String,
}

/// What a room can set off, which others may share: a line of a command block, a spell, a
/// Remote Action exit. Only those that lead somewhere are listed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScriptSource {
    /// A command line's text block, a spell, or the room of a Remote Action exit (`Rooms`,
    /// its map and room).
    pub table: &'static str,
    pub key: Vec<i64>,
    /// A command line's place: its block's part, and its number among the part's lines, from 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<(i64, usize)>,
    /// A spell's name.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Where it leads, each once, in the order the game reaches them.
    pub targets: Vec<ScriptTarget>,
}

/// A room's use of a source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScriptUse {
    pub map: i64,
    pub room: i64,
    pub when: When,
    /// Its source, by its place in [`Scripts::sources`].
    pub source: usize,
}

/// The links between rooms that no exit draws: each room's uses of what leads somewhere.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Scripts {
    pub sources: Vec<ScriptSource>,
    pub uses: Vec<ScriptUse>,
}

impl Scripts {
    /// The uses that start on `map`, or lead onto it from another, and their sources.
    pub fn of_map(&self, map: i64) -> Scripts {
        self.within(map, i64::MIN, i64::MAX)
    }

    /// The uses that start on `map` at a room numbered `from` to `to`, or lead onto one of
    /// them, and their sources.
    pub fn within(&self, map: i64, from: i64, to: i64) -> Scripts {
        let mut out = Scripts::default();
        let mut placed: HashMap<usize, usize> = HashMap::new();
        let here = |m: i64, first: i64, last: i64| m == map && first <= to && last >= from;
        for u in &self.uses {
            let s = &self.sources[u.source];
            // a remote action works an exit on the map it runs on
            let onto = || {
                s.targets.iter().any(|t| {
                    let m = if t.to_map == 0 { u.map } else { t.to_map };
                    here(m, t.to_room, t.to_last.unwrap_or(t.to_room).max(t.to_room))
                })
            };
            if !here(u.map, u.room, u.room) && !onto() {
                continue;
            }
            let at = *placed.entry(u.source).or_insert_with(|| {
                out.sources.push(s.clone());
                out.sources.len() - 1
            });
            out.uses.push(ScriptUse {
                source: at,
                ..u.clone()
            });
        }
        out
    }
}

/// What a block or spell does wherever it runs: a remote action's map is the one it runs on
/// (`to_map` 0).
type Found = ScriptTarget;

/// A spell's part in a link: where it teleports (room, map, and the last room of a roll), and
/// the text block it runs.
#[derive(Debug, Clone, Default)]
struct Spell {
    name: String,
    teleport: Option<(i64, i64, Option<i64>)>,
    block: i64,
}

/// The most text blocks and spells one room's links are followed through.
const MAX_FOLLOWED: usize = 200;
/// The parts of a block a run reads at most.
const MAX_PARTS: i64 = 50;
/// The abilities: Teleport Room, Teleport Map, Trigger Text Block.
const TELEPORT_ROOM: i64 = 140;
const TELEPORT_MAP: i64 = 141;
const TEXT_BLOCK: i64 = 148;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
    Block(i64, Reading),
    Spell(i64),
}

struct Lookup {
    /// Each text block's parts, in order from part 0 until one is missing.
    blocks: HashMap<i64, Vec<String>>,
    spells: HashMap<i64, Spell>,
    /// Each message's lines.
    messages: HashMap<i64, [String; 3]>,
    /// What each block and spell does, once followed.
    done: HashMap<Node, Vec<Found>>,
}

impl Lookup {
    /// What `node` does, and everything it runs (`seen` stops a loop, and caps a long chain).
    fn follow(&mut self, node: Node, seen: &mut HashSet<Node>) -> Vec<Found> {
        if let Some(f) = self.done.get(&node) {
            return f.clone();
        }
        if seen.len() >= MAX_FOLLOWED || !seen.insert(node) {
            return Vec::new();
        }
        let mut out = Vec::new();
        match node {
            Node::Block(b, reading) => {
                let parts = self.blocks.get(&b).cloned().unwrap_or_default();
                for part in &parts {
                    for line in script::lines_of(part, reading) {
                        if !line.never {
                            out.extend(self.clauses(b, &line.clauses, seen));
                        }
                    }
                }
            }
            Node::Spell(s) => {
                let Some(spell) = self.spells.get(&s).cloned() else {
                    return out;
                };
                if let Some((room, map, last)) = spell.teleport {
                    out.push(Found {
                        kind: "spell",
                        to_map: map,
                        to_room: room,
                        to_last: last,
                        exit: None,
                        table: "Spells",
                        record: s,
                        what: spell.name.clone(),
                    });
                }
                if spell.block > 0 {
                    out.extend(self.follow(Node::Block(spell.block, Reading::Script), seen));
                }
            }
        }
        seen.remove(&node);
        self.done.insert(node, out.clone());
        out
    }

    /// What a line's clauses do, in block `b`.
    fn clauses(&mut self, b: i64, clauses: &[Clause], seen: &mut HashSet<Node>) -> Vec<Found> {
        let mut out = Vec::new();
        for c in clauses {
            let Some(kw) = c.kw else {
                continue;
            };
            let g = |name: &str| i64::from(c.get(name));
            let found = |kind, to_map, to_room, exit| Found {
                kind,
                to_map,
                to_room,
                to_last: None,
                exit,
                table: "Textblocks",
                record: b,
                what: c.text.trim().to_string(),
            };
            match kw {
                // a named teleport rolls its room
                "teleport" if c.word.is_none() && g("room") > 0 && g("map") > 0 => {
                    out.push(found("teleport", g("map"), g("room"), None));
                }
                "remoteaction" if g("room") > 0 && (0..10).contains(&g("exit")) => {
                    out.push(found("remote", 0, g("room"), Some(g("exit") as usize)));
                }
                "cast" if g("spell") > 0 => out.extend(self.follow(Node::Spell(g("spell")), seen)),
                "random" if g("block") > 0 => {
                    out.extend(self.follow(Node::Block(g("block"), Reading::Chances), seen));
                }
                "testskill" | "checkspell" if g("block") > 0 => {
                    out.extend(self.follow(Node::Block(g("block"), Reading::Script), seen));
                }
                _ => {}
            }
        }
        out
    }
}

/// A source's key: its table, its record's key, and a command line's place.
type SourceKey = (&'static str, Vec<i64>, Option<(i64, usize)>);

/// The sources being gathered, each once by its key (`None`: it leads nowhere).
#[derive(Default)]
struct Gather {
    out: Scripts,
    at: HashMap<SourceKey, Option<usize>>,
}

impl Gather {
    /// Room `room` of map `map` sets off the source keyed so, when it leads somewhere
    /// (`targets` made only the first time it is seen).
    fn add(
        &mut self,
        (map, room): (i64, i64),
        when: When,
        (table, key, line): SourceKey,
        made: impl FnOnce() -> (String, Vec<ScriptTarget>),
    ) {
        let id = (table, key, line);
        let source = match self.at.get(&id) {
            Some(&s) => s,
            None => {
                let (name, found) = made();
                // each once, in the order reached
                let mut targets: Vec<ScriptTarget> = Vec::new();
                for f in found {
                    let same = |t: &ScriptTarget| {
                        (t.kind, t.to_map, t.to_room, t.to_last, t.exit)
                            == (f.kind, f.to_map, f.to_room, f.to_last, f.exit)
                    };
                    if !targets.iter().any(same) {
                        targets.push(f);
                    }
                }
                let s = (!targets.is_empty()).then(|| {
                    self.out.sources.push(ScriptSource {
                        table: id.0,
                        key: id.1.clone(),
                        line: id.2,
                        name,
                        targets,
                    });
                    self.out.sources.len() - 1
                });
                self.at.insert(id, s);
                s
            }
        };
        if let Some(source) = source {
            self.out.uses.push(ScriptUse {
                map,
                room,
                when,
                source,
            });
        }
    }
}

/// Every room's scripted links (`rooms`: the Rooms table, loaded already).
fn of_rooms(realm: &Realm, layout: LayoutSet, rooms: &Rows) -> Result<Scripts, ReportError> {
    let mut look = Lookup {
        blocks: HashMap::new(),
        spells: HashMap::new(),
        messages: HashMap::new(),
        done: HashMap::new(),
    };
    let tb = Rows::load(realm, "Textblocks", layout)?;
    let (number, part, data) = (tb.at("Number")?, tb.at("Part #")?, tb.at("Data")?);
    let mut parts: HashMap<i64, Vec<(i64, String)>> = HashMap::new();
    for r in &tb.0.rows {
        parts
            .entry(n(&r[number]))
            .or_default()
            .push((n(&r[part]), text(&r[data])));
    }
    for (b, mut ps) in parts {
        ps.sort_by_key(|p| p.0);
        // parts 0, 1, 2 ... until one is missing
        let run: Vec<String> = ps
            .into_iter()
            .enumerate()
            .take_while(|(i, (p, _))| *p == *i as i64 && *p < MAX_PARTS)
            .map(|(_, (_, d))| d)
            .collect();
        look.blocks.insert(b, run);
    }
    let sp = Rows::load(realm, "Spells", layout)?;
    let (number, name, min, max) = (
        sp.at("Number")?,
        sp.at("Name")?,
        sp.at("Min")?,
        sp.at("Max")?,
    );
    let abil = sp.family("Ability", 10, "")?;
    let vals = sp.family("Ability Value", 10, "")?;
    for r in &sp.0.rows {
        let value = |a: i64| {
            (0..10)
                .find(|&i| n(&r[abil[i]]) == a)
                .map(|i| n(&r[vals[i]]))
        };
        let teleport = match (value(TELEPORT_ROOM), value(TELEPORT_MAP)) {
            // a value of 0 is the cast's roll (one room when Min and Max agree)
            (Some(0), Some(map)) if map > 0 && n(&r[min]) > 0 => {
                let (first, last) = (n(&r[min]), n(&r[max]));
                Some((first, map, (last > first).then_some(last)))
            }
            (Some(room), Some(map)) if room > 0 && map > 0 => Some((room, map, None)),
            _ => None,
        };
        let spell = Spell {
            name: text(&r[name]),
            teleport,
            block: value(TEXT_BLOCK).unwrap_or(0),
        };
        if spell.teleport.is_some() || spell.block > 0 {
            look.spells.insert(n(&r[number]), spell);
        }
    }
    let ms = Rows::load(realm, "Messages", layout)?;
    let (number, l1, l2, l3) = (
        ms.at("Number")?,
        ms.at("Line 1")?,
        ms.at("Line 2")?,
        ms.at("Line 3")?,
    );
    for r in &ms.0.rows {
        look.messages
            .insert(n(&r[number]), [text(&r[l1]), text(&r[l2]), text(&r[l3])]);
    }

    let at = |c: &str| rooms.at(c);
    let (mapn, room, spell, command) = (
        at("Map Number")?,
        at("Room Number")?,
        at("Spell")?,
        at("Command Text")?,
    );
    let exits = rooms.family("Exit", 10, "")?;
    let kinds = rooms.family("Type", 10, "")?;
    let p1 = rooms.family("Para1", 10, "")?;
    let p2 = rooms.family("Para2", 10, "")?;
    let mut g = Gather::default();
    let spell_name = |look: &Lookup, s: i64| {
        look.spells
            .get(&s)
            .map(|x| x.name.clone())
            .unwrap_or_default()
    };
    for r in &rooms.0.rows {
        let here = (n(&r[mapn]), n(&r[room]));
        // the room's commands, a source per line, by its words
        let cmd = n(&r[command]);
        if cmd > 0 {
            let parts = look.blocks.get(&cmd).cloned().unwrap_or_default();
            for (pt, part) in parts.iter().enumerate() {
                for line in script::lines_of(part, Reading::Commands) {
                    let words = line.pattern.clone().unwrap_or_default().trim().to_string();
                    let key = ("Textblocks", vec![cmd], Some((pt as i64, line.no)));
                    g.add(here, When::Typed { words }, key, || {
                        (
                            String::new(),
                            look.clauses(cmd, &line.clauses, &mut HashSet::new()),
                        )
                    });
                }
            }
        }
        // the room's spell, on whoever is here
        let s = n(&r[spell]);
        if s > 0 {
            g.add(here, When::Here, ("Spells", vec![s], None), || {
                (
                    spell_name(&look, s),
                    look.follow(Node::Spell(s), &mut HashSet::new()),
                )
            });
        }
        for d in 0..10 {
            let to = n(&r[exits[d]]);
            if to == 0 {
                continue;
            }
            let (kind, a, b) = (n(&r[kinds[d]]), n(&r[p1[d]]), n(&r[p2[d]]));
            match kind {
                // Cast: a spell before and one after; Spell Trap: its spell
                22 | 24 => {
                    let spells = if kind == 22 { vec![a, b] } else { vec![a] };
                    for s in spells.into_iter().filter(|&s| s > 0) {
                        g.add(
                            here,
                            When::Walking { dir: d },
                            ("Spells", vec![s], None),
                            || {
                                (
                                    spell_name(&look, s),
                                    look.follow(Node::Spell(s), &mut HashSet::new()),
                                )
                            },
                        );
                    }
                }
                // Remote Action: Para2's units are the exit it works, below 0 none
                12 => {
                    let code = i64::from(b as i16);
                    if code < 0 {
                        continue;
                    }
                    let words = look
                        .messages
                        .get(&a)
                        .map(|l| l.iter().filter(|t| !t.trim().is_empty()).cloned().collect())
                        .unwrap_or_default();
                    let target = ScriptTarget {
                        kind: "remote",
                        to_map: 0,
                        to_room: to,
                        to_last: None,
                        exit: Some((code % 10) as usize),
                        table: "Rooms",
                        record: here.1,
                        what: format!("Remote Action exit {}", crate::map::DIRECTIONS[d]),
                    };
                    let key = ("Rooms", vec![here.0, here.1, d as i64], None);
                    g.add(here, When::Words { dir: d, words }, key, || {
                        (String::new(), vec![target])
                    });
                }
                _ => {}
            }
        }
    }
    Ok(g.out)
}

/// The realms whose links are kept.
const KEPT: usize = 4;

/// Every room's scripted links in `realm` (`rooms`: its Rooms table, loaded already): kept,
/// and read again only when the realm's files change.
pub(crate) fn of_realm(
    realm: &Realm,
    layout: LayoutSet,
    rooms: &Rows,
) -> Result<Arc<Scripts>, ReportError> {
    type Kept = (PathBuf, LayoutSet, Fingerprint, Arc<Scripts>);
    static CACHE: Mutex<Vec<Kept>> = Mutex::new(Vec::new());
    let dir = realm.dir().to_path_buf();
    let print = fingerprint(&dir);
    // held while they are read, so two asks read them once
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, _, _, s)) = cache
        .iter()
        .find(|(d, l, p, _)| *d == dir && *l == layout && *p == print)
    {
        return Ok(s.clone());
    }
    let s = Arc::new(of_rooms(realm, layout, rooms)?);
    cache.retain(|(d, _, _, _)| *d != dir);
    if cache.len() >= KEPT {
        cache.remove(0);
    }
    cache.push((dir, layout, print, s.clone()));
    Ok(s)
}
