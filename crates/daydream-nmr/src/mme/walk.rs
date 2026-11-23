//! NMR's walk of the world (`LocateRecords`, then `FillTextblockCommands`): what is in the
//! game and where it comes from.
//!
//! From the rooms (not the excluded ones) NMR follows everything a player can reach: the
//! items placed in rooms, their shops, NPCs, room commands and spells, and the monsters of
//! their groups; then, again and again while anything new turns up, what those monsters
//! drop, cast and summon, what their spells do, what text blocks they run and what the items
//! do. Each record it reaches is marked `In Game` and given a note of where it came from
//! (`Obtained From`, `Learned From`, `Summoned By`, ...), and the text blocks reached go into
//! `TBInfo`. Last it opens the containers and listens to the NPCs' greetings.
//!
//! This is a port, quirks and all, as MMUD Explorer reads what NMR writes. Positions in text
//! are VB's (from 1), so the scanning reads as NMR's does. Where NMR would stop on a run-time
//! error and ask whether to go on (a record number out of its arrays' range), the walk goes
//! on as NMR does when told to.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::data::{NmrData, NmrRows};
use crate::schema::Value;

use super::{Exclusions, Src, num};

// ---- VB's string functions, on Windows-1252 bytes, positions from 1 ----

/// `InStr(start, s, pat)`: where `pat` is at or after `start`, or 0.
fn instr(start: usize, s: &[u8], pat: &[u8]) -> usize {
    if start == 0 || start > s.len() {
        return 0;
    }
    if pat.is_empty() {
        return start;
    }
    s[start - 1..]
        .windows(pat.len())
        .position(|w| w == pat)
        .map_or(0, |p| p + start)
}

/// `Mid(s, start, len)`.
fn mid(s: &[u8], start: usize, len: usize) -> &[u8] {
    if start == 0 || start > s.len() {
        return &[];
    }
    &s[start - 1..(start - 1 + len).min(s.len())]
}

/// `Mid(s, start)`.
fn mid_from(s: &[u8], start: usize) -> &[u8] {
    mid(s, start, s.len())
}

/// How many digits run from `at`.
fn digits_at(s: &[u8], at: usize) -> usize {
    if at == 0 {
        return 0;
    }
    s.iter()
        .skip(at - 1)
        .take_while(|b| b.is_ascii_digit())
        .count()
}

/// VB's `Val`: a leading number, blanks, tabs and line feeds anywhere ignored.
fn val(s: &[u8]) -> i64 {
    let mut it = s
        .iter()
        .copied()
        .filter(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .peekable();
    let neg = match it.peek() {
        Some(b'-') => {
            it.next();
            true
        }
        Some(b'+') => {
            it.next();
            false
        }
        _ => false,
    };
    let mut n: i64 = 0;
    for b in it {
        if !b.is_ascii_digit() {
            break;
        }
        n = n.saturating_mul(10).saturating_add(i64::from(b - b'0'));
    }
    if neg { -n } else { n }
}

/// `LCase` (the letters the walk looks for are ASCII).
fn lcase(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

/// `ExtractValueFromString`: the number after `look`'s first match, leading blanks skipped,
/// up to the first blank or other character after it.
fn extract_value(s: &[u8], look: &[u8]) -> i64 {
    let mut x = instr(1, s, look);
    if x == 0 {
        return 0;
    }
    x += look.len();
    let mut y = x;
    while y <= s.len() {
        match s[y - 1] {
            b'0'..=b'9' => {}
            b' ' if y > x => break,
            b' ' => x += 1,
            _ => break,
        }
        y += 1;
    }
    if y > x { val(mid(s, x, y - x)) } else { 0 }
}

/// VB's `Currency`: four decimal places, each result rounded half to even.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Cur(i64);

impl Cur {
    const ONE: Cur = Cur(10_000);

    fn whole(n: i64) -> Cur {
        Cur(n * 10_000)
    }

    fn round_div(n: i128, d: i128) -> i64 {
        let q = n.div_euclid(d);
        let r = n.rem_euclid(d);
        let q = match (2 * r).cmp(&d) {
            std::cmp::Ordering::Less => q,
            std::cmp::Ordering::Greater => q + 1,
            std::cmp::Ordering::Equal => q + (q & 1),
        };
        q as i64
    }

    /// `(a - b) / 100` stored in a Currency.
    fn percent(n: i64) -> Cur {
        Cur(n * 100)
    }

    fn add(self, o: Cur) -> Cur {
        Cur(self.0 + o.0)
    }

    fn sub(self, o: Cur) -> Cur {
        Cur(self.0 - o.0)
    }

    fn mul(self, o: Cur) -> Cur {
        Cur(Cur::round_div(i128::from(self.0) * i128::from(o.0), 10_000))
    }

    /// `Round(c * 100, 1)` as VB prints it.
    fn shown_percent(self) -> String {
        let c = self.mul(Cur::whole(100));
        let tenths = Cur::round_div(i128::from(c.0), 1_000);
        let (sign, t) = if tenths < 0 {
            ("-", -tenths)
        } else {
            ("", tenths)
        };
        if t % 10 == 0 {
            format!("{sign}{}", t / 10)
        } else {
            format!("{sign}{}.{}", t / 10, t % 10)
        }
    }
}

/// `StringOfNumbersToArray`.
fn numbers_of(s: &str) -> Vec<String> {
    let one = |p: &str| val(p.replace(['(', ')'], "").as_bytes()).to_string();
    if !s.contains(',') {
        vec![one(s)]
    } else {
        s.split(',').map(one).collect()
    }
}

/// `MergeStringArrays`: the longer list (the second when as long), then the other's new
/// entries.
fn merge(a: &[String], b: &[String]) -> Vec<String> {
    let (first, then) = if a.len() > b.len() { (a, b) } else { (b, a) };
    let mut out = first.to_vec();
    for s in then {
        if !out.contains(s) {
            out.push(s.clone());
        }
    }
    out
}

/// `"(" & Join(merged, "),(") & ")"`.
fn merged_classes(a: &str, b: &str) -> String {
    format!("({})", merge(&numbers_of(a), &numbers_of(b)).join("),("))
}

/// NMR's notes: `from` after what is there (", " between), unless it is already in it; past
/// `max` characters, the first `keep` and a `+`. `None` when it is already in.
fn noted(old: &str, from: &str, max: usize, keep: usize) -> Option<String> {
    if old.contains(from) {
        return None;
    }
    let mut s = if old == "\0" {
        String::new()
    } else {
        format!("{old}, ")
    };
    s.push_str(from);
    Some(clip_note(s, max, keep))
}

fn clip_note(s: String, max: usize, keep: usize) -> String {
    if s.chars().count() > max {
        let mut t: String = s.chars().take(keep).collect();
        t.push_str("+\0");
        t
    } else {
        s
    }
}

// ---- the world as NMR's records hold it ----

struct Room {
    map: i64,
    room: i64,
    ty: i64,
    shop: i64,
    perm_npc: i64,
    cmd_text: i64,
    spell: i64,
    placed: Vec<i64>,
    hidden: Vec<i64>,
    exits: Vec<(i64, i64, [i64; 4])>,
    mon_type: i64,
    min_index: i64,
    max_index: i64,
    max_regen: i64,
}

struct Shop {
    number: i64,
    /// Item, Max, Regen Number, Regen %.
    stock: Vec<[i64; 4]>,
}

struct Monster {
    number: i64,
    name: String,
    group: i64,
    index: i64,
    drops: Vec<(i64, i64)>,
    greet: i64,
    create_spell: i64,
    death_spell: i64,
    abilities: Vec<(i64, i64)>,
    /// Type, Hit Spell, Accu/Spell.
    attacks: Vec<[i64; 3]>,
    spells: Vec<i64>,
}

struct Spell {
    number: i64,
    min: i64,
    max: i64,
    abilities: Vec<(i64, i64)>,
}

struct Item {
    number: i64,
    ty: i64,
    classes: Vec<i64>,
    abilities: Vec<(i64, i64)>,
}

struct Tb {
    number: i64,
    part: i64,
    link: i64,
    /// Decrypted.
    data: Vec<u8>,
}

/// The source rows, read through NMR's columns.
struct Reader<'a> {
    src: &'a Src,
    at: HashMap<String, usize>,
}

impl<'a> Reader<'a> {
    fn new(src: &'a Src) -> Self {
        Reader {
            src,
            at: src
                .rows
                .columns
                .iter()
                .enumerate()
                .map(|(i, c)| (c.clone(), i))
                .collect(),
        }
    }

    fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        self.src.rows.rows.iter().map(|r| Row { r, at: &self.at })
    }
}

struct Row<'a> {
    r: &'a [Value],
    at: &'a HashMap<String, usize>,
}

impl Row<'_> {
    fn n(&self, c: &str) -> i64 {
        num(&self.r[*self.at.get(c).unwrap_or_else(|| panic!("no column {c}"))])
    }

    fn s(&self, c: &str) -> String {
        super::clip(&self.r[self.at[c]])
    }

    fn bytes(&self, c: &str) -> Vec<u8> {
        match &self.r[self.at[c]] {
            Value::Text(s) => crate::text::to_1252(s).unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn list(&self, n: usize, f: impl Fn(usize) -> String) -> Vec<i64> {
        (0..n).map(|x| self.n(&f(x))).collect()
    }

    fn abilities(&self, n: usize) -> Vec<(i64, i64)> {
        (0..n)
            .map(|x| {
                (
                    self.n(&format!("Ability {x}")),
                    self.n(&format!("Ability Value {x}")),
                )
            })
            .collect()
    }
}

/// The world's records, in their files' key order.
pub(super) struct World {
    rooms: Vec<Room>,
    shops: Vec<Shop>,
    monsters: Vec<Monster>,
    spells: Vec<Spell>,
    items: Vec<Item>,
    tbs: Vec<Tb>,
}

impl World {
    pub(super) fn read(
        rooms: &Src,
        shops: &Src,
        monsters: &Src,
        spells: &Src,
        items: &Src,
        tbs: &Src,
    ) -> World {
        let mut rooms: Vec<Room> = Reader::new(rooms)
            .rows()
            .map(|r| Room {
                map: r.n("Map Number"),
                room: r.n("Room Number"),
                ty: r.n("Type"),
                shop: r.n("Shop Number"),
                perm_npc: r.n("Perm NPC"),
                cmd_text: r.n("Command Text"),
                spell: r.n("Spell"),
                placed: r.list(10, |x| format!("Placed Item {x}")),
                hidden: r.list(15, |x| format!("Hidden Item {x}")),
                exits: (0..10)
                    .map(|x| {
                        (
                            r.n(&format!("Exit {x}")),
                            r.n(&format!("Type {x}")),
                            [1, 2, 3, 4].map(|p| r.n(&format!("Para{p} {x}"))),
                        )
                    })
                    .collect(),
                mon_type: r.n("Mon Type"),
                min_index: r.n("Min Index"),
                max_index: r.n("Max Index"),
                max_regen: r.n("Max Regen"),
            })
            .collect();
        // NMR reads the rooms in their file's key order: by room number, then map
        rooms.sort_by_key(|r| (r.room, r.map));
        let shops = Reader::new(shops)
            .rows()
            .map(|r| Shop {
                number: r.n("Number"),
                stock: (0..20)
                    .map(|x| {
                        [
                            r.n(&format!("Item {x}")),
                            r.n(&format!("Max {x}")),
                            r.n(&format!("Regen Number{x}")),
                            r.n(&format!("Regen %{x}")),
                        ]
                    })
                    .collect(),
            })
            .collect();
        let monsters = Reader::new(monsters)
            .rows()
            .map(|r| Monster {
                number: r.n("Number"),
                name: r.s("Name"),
                group: r.n("Group"),
                index: r.n("Index"),
                drops: (0..10)
                    .map(|x| {
                        (
                            r.n(&format!("Item Number {x}")),
                            r.n(&format!("Item Drop % {x}")),
                        )
                    })
                    .collect(),
                greet: r.n("Greet Txt"),
                create_spell: r.n("Create Spell"),
                death_spell: r.n("Death Spell"),
                abilities: r.abilities(10),
                attacks: (0..5)
                    .map(|x| {
                        [
                            r.n(&format!("Attack Type {x}")),
                            r.n(&format!("Attack Hit Spell {x}")),
                            r.n(&format!("Attack Accu/Spell {x}")),
                        ]
                    })
                    .collect(),
                spells: r.list(5, |x| format!("Spell Number {x}")),
            })
            .collect();
        let spells = Reader::new(spells)
            .rows()
            .map(|r| Spell {
                number: r.n("Number"),
                min: r.n("Min"),
                max: r.n("Max"),
                abilities: r.abilities(10),
            })
            .collect();
        let items = Reader::new(items)
            .rows()
            .map(|r| Item {
                number: r.n("Number"),
                ty: r.n("Type"),
                classes: r.list(10, |x| format!("Class {x}")),
                abilities: r.abilities(20),
            })
            .collect();
        let tbs = Reader::new(tbs)
            .rows()
            .map(|r| Tb {
                number: r.n("Number"),
                part: r.n("Part #"),
                link: r.n("Link To"),
                data: r.bytes("Data"),
            })
            .collect();
        World {
            rooms,
            shops,
            monsters,
            spells,
            items,
            tbs,
        }
    }
}

// ---- the MME tables the walk writes ----

/// One MME table's rows, found by `Number`.
struct Marked {
    rows: Vec<Vec<Value>>,
    columns: Vec<String>,
    by_number: HashMap<i64, usize>,
}

impl Marked {
    fn take(t: &mut NmrRows) -> Marked {
        let rows = std::mem::take(&mut t.rows);
        let n = t.position("Number").expect("a Number column");
        let by_number = rows
            .iter()
            .enumerate()
            .map(|(i, r)| (num(&r[n]), i))
            .rev()
            .collect();
        Marked {
            rows,
            columns: t.columns.clone(),
            by_number,
        }
    }

    fn put(self, t: &mut NmrRows) {
        t.rows = self.rows;
    }

    fn col(&self, c: &str) -> usize {
        self.columns
            .iter()
            .position(|x| x == c)
            .unwrap_or_else(|| panic!("no column {c}"))
    }

    fn row(&self, n: i64) -> Option<usize> {
        self.by_number.get(&n).copied()
    }

    fn text(&self, r: usize, c: &str) -> String {
        match &self.rows[r][self.col(c)] {
            Value::Text(s) => s.clone(),
            Value::Null => String::new(),
            Value::Long(n) => n.to_string(),
            Value::Double(d) => d.to_string(),
        }
    }

    fn long(&self, r: usize, c: &str) -> i64 {
        num(&self.rows[r][self.col(c)])
    }

    fn set(&mut self, r: usize, c: &str, v: Value) {
        let i = self.col(c);
        self.rows[r][i] = v;
    }
}

/// A `TBInfo` row.
struct TbInfo {
    link: i64,
    action: String,
    called_from: String,
}

/// The record kinds `CheckTBString` marks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Spell,
    Item,
    Monster,
    TextBlock,
}

/// How many times the cross-referencing passes run at least (`nPasses`), and at most less
/// one.
const PASSES: usize = 10;
const MAX_PASSES: usize = 20;

struct Walk<'w> {
    world: &'w World,
    items: Marked,
    spells: Marked,
    monsters: Marked,
    shops: Marked,
    tb_info: BTreeMap<i64, TbInfo>,

    item_in_game: HashSet<i64>,
    spell_in_game: HashSet<i64>,
    monster_in_game: HashSet<i64>,
    something_marked: bool,
    mon_group: HashMap<(i64, i64), String>,
    tb_scanned: HashSet<(i64, i64)>,
    tb_random: HashSet<i64>,
    tb_commands: HashSet<i64>,
    tb_bad_source: HashSet<i64>,

    /// The text block file's position (`TextblockPosBlock`) and the record last read
    /// (`TextblockRec`), as indexes into `world.tbs`.
    tb_pos: usize,
    tb_rec: usize,
    tb_at: HashMap<(i64, i64), usize>,
    item_at: HashMap<i64, usize>,
    spell_at: HashMap<i64, usize>,
}

/// Walks the world and fills in `data`'s Items, Spells, Monsters and Shops and its TBInfo.
pub(super) fn locate(data: &mut NmrData, world: &World, excl: &Exclusions, hide_excluded: bool) {
    let mut take = |name: &str| {
        Marked::take(
            data.tables
                .iter_mut()
                .find(|t| t.name == name)
                .expect("an MME table"),
        )
    };
    let mut w = Walk {
        world,
        items: take("Items"),
        spells: take("Spells"),
        monsters: take("Monsters"),
        shops: take("Shops"),
        tb_info: BTreeMap::new(),
        item_in_game: HashSet::new(),
        spell_in_game: HashSet::new(),
        monster_in_game: HashSet::new(),
        something_marked: false,
        mon_group: HashMap::new(),
        tb_scanned: HashSet::new(),
        tb_random: HashSet::new(),
        tb_commands: HashSet::new(),
        tb_bad_source: HashSet::new(),
        tb_pos: 0,
        tb_rec: 0,
        tb_at: world
            .tbs
            .iter()
            .enumerate()
            .map(|(i, t)| ((t.number, t.part), i))
            .rev()
            .collect(),
        item_at: world
            .items
            .iter()
            .enumerate()
            .map(|(i, t)| (t.number, i))
            .rev()
            .collect(),
        spell_at: world
            .spells
            .iter()
            .enumerate()
            .map(|(i, t)| (t.number, i))
            .rev()
            .collect(),
    };

    // LocateRecords
    w.scan_rooms(excl, hide_excluded);
    w.scan_shops();
    let mut x = 1;
    while (x <= PASSES || w.something_marked) && x < MAX_PASSES {
        w.something_marked = false;
        w.scan_monsters();
        w.scan_spells();
        w.scan_textblocks();
        w.scan_items();
        x += 1;
    }
    w.scan_containers();
    w.scan_greets();
    w.fill_textblock_commands();

    for (name, m) in [
        ("Items", w.items),
        ("Spells", w.spells),
        ("Monsters", w.monsters),
        ("Shops", w.shops),
    ] {
        m.put(data.tables.iter_mut().find(|t| t.name == name).unwrap());
    }
    let t = data
        .tables
        .iter_mut()
        .find(|t| t.name == "TBInfo")
        .expect("TBInfo");
    t.rows = w
        .tb_info
        .into_iter()
        .map(|(n, i)| {
            vec![
                Value::Long(n as i32),
                Value::Long(i.link as i32),
                Value::Text(i.action),
                Value::Text(i.called_from),
            ]
        })
        .collect();
}

fn room_note(r: &Room) -> String {
    format!("Room {}/{}", r.map, r.room)
}

impl Walk<'_> {
    // ---- the Mark* subs ----

    fn mark_shop(&mut self, n: i64, from: &str) {
        if n == 0 {
            return;
        }
        let Some(r) = self.shops.row(n) else { return };
        let Some(s) = noted(&self.shops.text(r, "Assigned To"), from, 250, 248) else {
            return;
        };
        self.shops.set(r, "In Game", Value::Long(1));
        self.shops.set(r, "Assigned To", Value::Text(s));
    }

    fn mark_monster(&mut self, n: i64, from: &str) {
        if n <= 0 {
            return;
        }
        if self.monster_in_game.insert(n) {
            self.something_marked = true;
        }
        let Some(r) = self.monsters.row(n) else {
            return;
        };
        let Some(s) = noted(&self.monsters.text(r, "Summoned By"), from, 20000, 19999) else {
            return;
        };
        self.monsters.set(r, "In Game", Value::Long(1));
        self.monsters.set(r, "Summoned By", Value::Text(s));
    }

    fn mark_item(&mut self, n: i64, from: &str, dont_mark_in_game: bool, reference: bool) {
        if n <= 0 {
            return;
        }
        let field = if reference {
            "References"
        } else {
            "Obtained From"
        };
        if self.item_in_game.insert(n) {
            self.something_marked = true;
        }
        let Some(r) = self.items.row(n) else { return };
        let old = self.items.text(r, field);
        let s = if from.is_empty() {
            old
        } else {
            match noted(&old, from, 2000, 1995) {
                Some(s) => s,
                None => return,
            }
        };
        if !dont_mark_in_game {
            self.items.set(r, "In Game", Value::Long(1));
        }
        self.items.set(r, field, Value::Text(s));
    }

    fn mark_spell(&mut self, n: i64, from: &str, learned: bool, classes: &str) {
        if n <= 0 {
            return;
        }
        if self.spell_in_game.insert(n) {
            self.something_marked = true;
        }
        let Some(r) = self.spells.row(n) else { return };
        if learned && !from.is_empty() {
            let Some(s) = noted(&self.spells.text(r, "Learned From"), from, 250, 248) else {
                return;
            };
            self.spells.set(r, "Learnable", Value::Long(1));
            self.spells.set(r, "Learned From", Value::Text(s));
            let have = self.spells.text(r, "Classes");
            if !classes.is_empty() && have != "(*)" {
                let new = if classes == "(*)" || have.is_empty() || have == "\0" {
                    classes.to_string()
                } else {
                    merged_classes(&have, classes)
                };
                self.spells.set(r, "Classes", Value::Text(new));
            }
        } else {
            let old = self.spells.text(r, "Casted By");
            // NMR looks for the note before it sees there is none: an empty note is
            // always found
            if old.contains(from) && !(old.is_empty() && from.is_empty()) {
                return;
            }
            if from.is_empty() {
                return;
            }
            let Some(s) = noted(&old, from, 250, 248) else {
                return;
            };
            self.spells.set(r, "Casted By", Value::Text(s));
        }
    }

    fn mark_tb(&mut self, n: i64, from: &str, random: bool) {
        if n == 0 {
            return;
        }
        let s = match self.tb_info.get(&n) {
            None => {
                self.something_marked = true;
                self.tb_info.insert(
                    n,
                    TbInfo {
                        link: 0,
                        action: "\0".into(),
                        called_from: String::new(),
                    },
                );
                clip_note(from.to_string(), 250, 248)
            }
            Some(i) => match noted(&i.called_from, from, 250, 248) {
                Some(s) => s,
                None => return,
            },
        };
        self.tb_info.get_mut(&n).unwrap().called_from = s;
        if random {
            self.tb_random.insert(n);
        }
    }

    // ---- the text block file ----

    /// `BGETEQUAL` on (number, part): the record, when there is one.
    fn tb_get(&mut self, number: i64, part: i64) -> Option<usize> {
        let i = *self.tb_at.get(&(number, part))?;
        self.tb_pos = i;
        self.tb_rec = i;
        Some(i)
    }

    /// `BGETNEXT`.
    fn tb_next(&mut self) -> Option<usize> {
        let i = self.tb_pos + 1;
        if i >= self.world.tbs.len() {
            return None;
        }
        self.tb_pos = i;
        self.tb_rec = i;
        Some(i)
    }

    /// `GetTextblockLink`.
    fn textblock_link(&mut self, n: i64) -> i64 {
        if n == 0 {
            return 0;
        }
        match self.tb_get(n, 0) {
            Some(i) => self.world.tbs[i].link,
            None => 0,
        }
    }

    /// A text block's part 0 and the parts after it, decrypted; `None` when it has no part
    /// 0, or part 0's text is under 5 characters (`min` true).
    fn tb_text(&mut self, n: i64, min: bool) -> Option<Vec<u8>> {
        let i = self.tb_get(n, 0)?;
        let mut data = self.world.tbs[i].data.clone();
        if min && data.len() < 5 {
            return None;
        }
        while let Some(j) = self.tb_next() {
            let t = &self.world.tbs[j];
            if t.number == n && t.part > 0 {
                data.extend_from_slice(&t.data);
            } else {
                break;
            }
        }
        Some(data)
    }

    // ---- the scans ----

    fn scan_rooms(&mut self, excl: &Exclusions, hide_excluded: bool) {
        let world = self.world;
        for r in &world.rooms {
            if excl.excludes(r.map, r.room, hide_excluded) {
                continue;
            }
            let note = room_note(r);
            for &n in &r.placed {
                if n > 0 {
                    self.mark_item(n, &note, false, false);
                }
            }
            for &n in &r.hidden {
                // signs
                if n > 0 && self.item_type(n) == 3 {
                    self.mark_item(n, &note, false, false);
                }
            }
            if r.ty == 1 && r.shop > 0 {
                self.mark_shop(r.shop, &note);
            }
            if r.perm_npc > 0 {
                self.mark_monster(r.perm_npc, &note);
            }
            if r.cmd_text > 0 {
                self.tb_commands.insert(r.cmd_text);
                self.mark_tb(r.cmd_text, &note, false);
            }
            if r.spell > 0 {
                self.mark_spell(r.spell, &note, false, "");
            }
            for &(to, ty, p) in &r.exits {
                if to == 0 {
                    continue;
                }
                match ty {
                    // ticket, item, key
                    2 | 3 | 17 => self.mark_item(p[0], &note, true, true),
                    // door, gate, remote action
                    7 | 11 | 12 => self.mark_item(p[3], &note, true, true),
                    // cast
                    22 => {
                        self.mark_spell(p[0], &note, false, "");
                        self.mark_spell(p[1], &note, false, "");
                    }
                    // spell trap
                    24 => self.mark_spell(p[0], &note, false, ""),
                    _ => {}
                }
            }
            if r.min_index <= r.max_index && r.max_index > 0 {
                let lair = r.ty == 3 && r.max_regen > 0;
                for x in r.min_index..=r.max_index {
                    let g = self.mon_group.entry((r.mon_type, x)).or_default();
                    if !g.is_empty() {
                        g.push(',');
                    }
                    if lair {
                        g.push_str(&format!(
                            "[{}-{}-{}][{}]Group(lair): {}/{}",
                            r.mon_type, r.min_index, r.max_index, r.max_regen, r.map, r.room
                        ));
                    } else {
                        g.push_str(&format!("Group: {}/{}", r.map, r.room));
                    }
                }
            }
        }
    }

    /// `GetItemTypeNumber`.
    fn item_type(&self, n: i64) -> i64 {
        self.item_at.get(&n).map_or(-1, |&i| self.world.items[i].ty)
    }

    fn scan_shops(&mut self) {
        let world = self.world;
        for s in &world.shops {
            let Some(r) = self.shops.row(s.number) else {
                continue;
            };
            if self.shops.long(r, "In Game") == 0 {
                continue;
            }
            for &[item, max, regen_number, regen_pct] in &s.stock {
                if item == 0 {
                    continue;
                }
                if max == 0 {
                    self.mark_item(item, &format!("Shop(sell) #{}", s.number), true, false);
                } else if regen_pct == 0 || regen_number == 0 {
                    self.mark_item(item, &format!("Shop(nogen) #{}", s.number), true, false);
                } else {
                    self.mark_item(item, &format!("Shop #{}", s.number), false, false);
                }
            }
        }
    }

    fn scan_monsters(&mut self) {
        let world = self.world;
        for m in &world.monsters {
            if m.name.to_ascii_lowercase().starts_with("sdf") {
                continue;
            }
            if let Some(g) = self.mon_group.get(&(m.group, m.index)).cloned()
                && !g.is_empty()
            {
                self.mark_monster(m.number, &g);
            }
            if !self.monster_in_game.contains(&m.number) {
                continue;
            }
            let from = format!("Monster #{}", m.number);
            for &(item, pct) in &m.drops {
                if item != 0 {
                    self.mark_item(item, &format!("{from}({pct}%)"), false, false);
                }
            }
            if m.greet != 0 {
                self.tb_bad_source.insert(m.greet);
                self.tb_commands.insert(m.greet);
                self.mark_tb(m.greet, &from, false);
            }
            if m.create_spell != 0 {
                self.mark_spell(m.create_spell, &from, false, "");
            }
            if m.death_spell != 0 {
                self.mark_spell(m.death_spell, &from, false, "");
            }
            for &(a, b) in &m.abilities {
                match a {
                    12 => self.mark_monster(b, &from),
                    42 => self.mark_spell(b, &from, true, ""),
                    43 | 151 | 153 => self.mark_spell(b, &from, false, ""),
                    148 | 155 => {
                        self.tb_commands.insert(b);
                        self.mark_tb(b, &from, false);
                    }
                    _ => {}
                }
            }
            for (&[ty, hit_spell, accu_spell], &spell) in m.attacks.iter().zip(&m.spells) {
                if ty != 0 {
                    self.mark_spell(hit_spell, &from, false, "");
                }
                if ty == 2 {
                    self.mark_spell(accu_spell, &from, false, "");
                }
                if spell != 0 {
                    self.mark_spell(spell, &from, false, "");
                }
            }
        }
    }

    fn scan_items(&mut self) {
        let world = self.world;
        for it in &world.items {
            if !self.item_in_game.contains(&it.number) {
                continue;
            }
            let mut classes = String::new();
            for &c in &it.classes {
                if c > 0 {
                    let one = format!("({c})");
                    if !classes.contains(&one) {
                        if !classes.is_empty() {
                            classes.push(',');
                        }
                        classes.push_str(&one);
                    }
                }
            }
            if classes.is_empty() {
                classes = "(*)".into();
            }
            let from = format!("Item #{}", it.number);
            for &(a, b) in &it.abilities {
                if a <= 0 || b <= 0 {
                    continue;
                }
                match a {
                    12 => self.mark_monster(b, &from),
                    42 => self.mark_spell(b, &from, true, &classes),
                    43 | 151 | 153 => self.mark_spell(b, &from, false, ""),
                    148 | 155 => {
                        self.tb_commands.insert(b);
                        self.mark_tb(b, &from, false);
                    }
                    _ => {}
                }
            }
        }
    }

    fn scan_spells(&mut self) {
        let world = self.world;
        for sp in &world.spells {
            if !self.spell_in_game.contains(&sp.number) {
                continue;
            }
            let from = format!("Spell #{}", sp.number);
            for &(a, b) in &sp.abilities {
                if a <= 0 {
                    continue;
                }
                // some spells hold the number in Min..Max with 0 for the value
                let (y1, y2) = if b == 0 {
                    (sp.min, sp.max.max(sp.min))
                } else {
                    (b, b)
                };
                if y1 <= 0 || y2 <= 0 {
                    continue;
                }
                for n in y1..=y2 {
                    match a {
                        12 => self.mark_monster(n, &from),
                        42 => self.mark_spell(n, &from, true, ""),
                        43 | 151 | 153 => self.mark_spell(n, &from, false, ""),
                        148 | 155 => {
                            self.tb_commands.insert(n);
                            self.mark_tb(n, &from, false);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn scan_textblocks(&mut self) {
        const LOOKS: [(&[u8], Kind); 15] = [
            (b"cast ", Kind::Spell),
            (b"learnspell ", Kind::Spell),
            (b"giveitem ", Kind::Item),
            (b"takeitem ", Kind::Item),
            (b"checkitem ", Kind::Item),
            (b"failitem ", Kind::Item),
            (b"failroomitem ", Kind::Item),
            (b"roomitem ", Kind::Item),
            (b"clearitem ", Kind::Item),
            (b"summon ", Kind::Monster),
            (b"random ", Kind::TextBlock),
            (b"testskill ", Kind::TextBlock),
            (b"text ", Kind::TextBlock),
            (b"checkspell ", Kind::TextBlock),
            (b":", Kind::TextBlock),
        ];
        let world = self.world;
        if world.tbs.is_empty() {
            return;
        }
        self.tb_pos = 0;
        self.tb_rec = 0;
        // NMR would go round for ever on a block that links to its own part 0 from a later
        // part; the walk stops there
        let mut steps = 0usize;
        loop {
            steps += 1;
            if steps > world.tbs.len() * 4 + 16 {
                break;
            }
            let cur = self.tb_rec;
            let tb = &world.tbs[cur];
            let (number, part) = (tb.number, tb.part);
            if !self.tb_scanned.contains(&(number, part)) && self.tb_info.contains_key(&number) {
                if tb.link > 0 {
                    let info = self.tb_info.get_mut(&number).unwrap();
                    if info.link < 1 {
                        info.link = tb.link;
                    }
                    if self.tb_bad_source.contains(&number) {
                        self.tb_bad_source.insert(tb.link);
                    }
                    self.mark_tb(tb.link, &format!("Textblock #{number}"), false);
                }
                let decrypted = tb.data.clone();
                let mut cleared = true;
                for (look, kind) in LOOKS {
                    if !self.check_tb_string(number, &decrypted, look, kind) && look != b":" {
                        cleared = false;
                    }
                }
                if world.tbs[self.tb_rec].number != number {
                    self.tb_get(number, part);
                }
                let rec = &world.tbs[self.tb_rec];
                if cleared {
                    self.tb_scanned.insert((rec.number, rec.part));
                }
            }
            if self.tb_next().is_none() {
                break;
            }
        }
    }

    /// `CheckTBString`: marks what `look`'s commands in `whole` name; false when one of
    /// them was held back (its items or spell not in the game yet).
    fn check_tb_string(&mut self, tb: i64, whole: &[u8], look: &[u8], kind: Kind) -> bool {
        let whole = lcase(whole);
        let starts = |p: &[u8]| look.starts_with(p);
        let learn = starts(b"learn");
        let random = starts(b"random");
        let test_skill = starts(b"testskill");
        let check_spell = starts(b"checkspell");
        let reference_only = starts(b"takeitem")
            || starts(b"checkitem")
            || starts(b"failitem")
            || starts(b"failroomitem")
            || starts(b"roomitem")
            || starts(b"clearitem");
        let give_item = starts(b"giveitem") && !reference_only;
        let mut ok = true;
        let mut x = 1;
        loop {
            let at = instr(x, &whole, look);
            if at == 0 {
                break;
            }
            x = at;
            let mut y1 = x + look.len();
            if test_skill {
                // past the skill and the amount, with no command between
                let a = instr(y1, &whole, b" ") + 1;
                if a == 1 {
                    x = y1;
                    continue;
                }
                let b = instr(a, &whole, b" ") + 1;
                if b == 1 {
                    x = y1;
                    continue;
                }
                let c = instr(y1, &whole, b":");
                if c > 0 && c < b {
                    x = y1;
                    continue;
                }
                let c = instr(y1, &whole, b"\n");
                if c > 0 && c < b {
                    x = y1;
                    continue;
                }
                x = b;
                y1 = b;
            }
            let y2 = digits_at(&whole, y1);
            if y2 == 0 {
                x = y1;
                continue;
            }
            let mut n = val(mid(&whole, y1, y2));
            if check_spell {
                if n == 0 {
                    x = y1;
                    continue;
                }
                y1 += y2 + 1;
                let y2 = digits_at(&whole, y1);
                if y2 == 0 {
                    x = y1;
                    continue;
                }
                let n2 = val(mid(&whole, y1, y2));
                if n2 == 0 {
                    x = y1;
                    continue;
                }
                if !self.spell_in_game.contains(&n) {
                    ok = false;
                    x = y1;
                    continue;
                }
                n = n2;
            }
            if self.item_fails(&whole, x) {
                ok = false;
            } else {
                let suffix = if self.tb_random.contains(&tb) {
                    tb_percent(&whole, x)
                } else {
                    String::new()
                };
                self.tb_commands.insert(tb);
                let from = format!("Textblock #{tb}{suffix}");
                match kind {
                    Kind::Spell => {
                        if learn {
                            if self.tb_bad_source.contains(&tb) {
                                self.mark_spell(n, "", false, "");
                            } else {
                                let classes = class_restriction(&whole, x);
                                self.mark_spell(n, &from, true, &classes);
                            }
                        } else {
                            self.mark_spell(n, &from, false, "");
                        }
                    }
                    Kind::Item => {
                        if give_item {
                            if self.tb_bad_source.contains(&tb) {
                                self.mark_item(n, "", false, false);
                            } else {
                                self.mark_item(n, &from, false, false);
                            }
                        } else {
                            self.mark_item(n, &from, reference_only, reference_only);
                        }
                    }
                    Kind::Monster => self.mark_monster(n, &from),
                    Kind::TextBlock => {
                        if self.tb_bad_source.contains(&tb) {
                            self.tb_bad_source.insert(n);
                        }
                        if random {
                            self.mark_tb(n, &format!("Textblock(rndm) #{tb}{suffix}"), true);
                            self.tb_commands.insert(n);
                        } else {
                            self.mark_tb(n, &from, false);
                            let link = self.textblock_link(n);
                            self.tb_commands.insert(link);
                        }
                    }
                }
            }
            x = y1;
        }
        ok
    }

    /// `TestItemFail`: whether a `checkitem`, `roomitem` (not `failroomitem`) or `takeitem`
    /// before `at` on its line names an item not in the game.
    fn item_fails(&self, whole: &[u8], at: usize) -> bool {
        let line = line_before(whole, at);
        for look in [&b"checkitem "[..], b"roomitem ", b"takeitem "] {
            let mut x = 1;
            loop {
                let p = instr(x, line, look);
                if p == 0 {
                    break;
                }
                x = p;
                if look == b"roomitem " && x > 4 && mid(line, x - 4, 4) == b"fail" {
                    x += 1;
                    continue;
                }
                let y1 = x + look.len();
                let y2 = digits_at(line, y1);
                if y2 == 0 {
                    x = y1;
                    continue;
                }
                let n = val(mid(line, y1, y2));
                if self.item_in_game.contains(&n) {
                    x = y1;
                    continue;
                }
                return true;
            }
        }
        false
    }

    fn fill_textblock_commands(&mut self) {
        let world = self.world;
        for t in &world.tbs {
            let Some(info) = self.tb_info.get_mut(&t.number) else {
                continue;
            };
            if !self.tb_commands.contains(&t.number) {
                continue;
            }
            let text = crate::text::from_1252(&t.data);
            if t.part == 0 {
                info.action = text;
            } else {
                info.action.push_str(&text);
            }
        }
    }

    fn scan_containers(&mut self) {
        let world = self.world;
        let mut nest = 0;
        for it in &world.items {
            if !self.item_in_game.contains(&it.number) || it.ty != 8 {
                continue;
            }
            for &(a, b) in &it.abilities {
                if a != 43 || b <= 0 {
                    continue;
                }
                let Some(&si) = self.spell_at.get(&b) else {
                    continue;
                };
                let sp = &world.spells[si];
                for &(sa, sb) in &sp.abilities {
                    if sa != 148 {
                        continue;
                    }
                    let tbn = if sb == 0 {
                        if sp.min > 0 { sp.min } else { sp.max }
                    } else {
                        sb
                    };
                    // part 0 only
                    let Some(i) = self.tb_get(tbn, 0) else {
                        continue;
                    };
                    let data = world.tbs[i].data.clone();
                    let mut chest = Chest::default();
                    let mut spells = Learned::default();
                    let mut pos = 1;
                    loop {
                        let p = instr(pos, &data, b"random ");
                        if p == 0 {
                            break;
                        }
                        pos = p + b"random ".len();
                        let z = digits_at(&data, pos);
                        if z > 0 {
                            let n = val(mid(&data, pos, z));
                            self.random_tb_items(
                                &mut chest,
                                n,
                                &mut nest,
                                Cur::ONE,
                                false,
                                &mut spells,
                            );
                        }
                    }
                    for (n, pct, _) in &chest.0 {
                        if *n > 0 {
                            let from = format!("Item #{}({}%)", it.number, pct.shown_percent());
                            self.mark_item(*n, &from, false, false);
                        }
                    }
                }
            }
        }
    }

    fn scan_greets(&mut self) {
        let world = self.world;
        let mut nest = 0;
        for m in &world.monsters {
            if !self.monster_in_game.contains(&m.number) || m.greet <= 0 {
                continue;
            }
            // the record's text is always 2,000 characters, so never too short
            let Some(data) = self.tb_text(m.greet, false) else {
                continue;
            };
            let mut items = Chest::default();
            let mut spells = Learned::default();
            let mut pos = 1;
            loop {
                let p = instr(pos, &data, b":");
                if p == 0 {
                    break;
                }
                pos = p + 1;
                let z = digits_at(&data, pos);
                if z > 0 {
                    let n = val(mid(&data, pos, z));
                    if let Some(i) = self.tb_get(n, 0) {
                        let link = world.tbs[i].link;
                        if link > 0 {
                            self.tb_items(&mut items, link, &mut nest, true, &mut spells);
                        }
                    }
                }
            }
            let from = format!("NPC #{}", m.number);
            for (n, _, _) in &items.0 {
                if *n > 0 {
                    self.mark_item(*n, &from, false, false);
                }
            }
            for (n, classes) in &spells.0 {
                if *n > 0 {
                    self.mark_spell(*n, &from, true, classes);
                }
            }
        }
    }

    /// `GetTBItems`: what a text block's lines give (not lines held back by an item not in
    /// the game), with the random blocks they call.
    fn tb_items(
        &mut self,
        out: &mut Chest,
        n: i64,
        nest: &mut i32,
        check_spells: bool,
        spells: &mut Learned,
    ) {
        if *nest > 5 {
            return;
        }
        let Some(data) = self.tb_text(n, true) else {
            return;
        };
        *nest += 1;
        // number, times given
        let mut given: Vec<(i64, Cur)> = Vec::new();
        let mut pos = 1;
        while pos < data.len() {
            let mut x = instr(pos, &data, b"\n");
            if x == 0 {
                x = data.len();
            }
            let line = lcase(mid(&data, pos, x - pos));
            let line_start = pos;
            pos = x + 1;
            if self.item_fails(&data, pos - 1) {
                continue;
            }
            let mut y = 1;
            loop {
                y = instr(y, &line, b"giveitem ");
                if y == 0 {
                    break;
                }
                let v = extract_value(mid_from(&line, y), b"giveitem ");
                match given.iter_mut().find(|(g, _)| *g == v) {
                    Some(g) => g.1 = g.1.add(Cur::ONE),
                    None => given.push((v, Cur::ONE)),
                }
                y += 1;
            }
            if check_spells {
                learn_spells(&data, &line, line_start, spells);
            }
            let mut y = 1;
            loop {
                y = instr(y, &line, b"random ");
                if y == 0 {
                    break;
                }
                let v = extract_value(mid_from(&line, y), b"random ");
                if v > 0 {
                    self.random_tb_items(out, v, nest, Cur::ONE, check_spells, spells);
                }
                y += 1;
            }
        }
        for (v, pct) in given {
            if v > 0 {
                out.add(v, pct, Cur::ONE);
            }
        }
        *nest -= 1;
    }

    /// `GetRandomTBItems`: what a random block's `NN:` lines give, at each line's chance
    /// (its number less the one before) times `modifier`.
    fn random_tb_items(
        &mut self,
        out: &mut Chest,
        n: i64,
        nest: &mut i32,
        modifier: Cur,
        check_spells: bool,
        spells: &mut Learned,
    ) {
        if *nest > 5 {
            return;
        }
        let Some(data) = self.tb_text(n, true) else {
            return;
        };
        *nest += 1;
        let modifier = if modifier <= Cur(0) {
            Cur::ONE
        } else {
            modifier
        };
        let mut given: Vec<(i64, Cur)> = Vec::new();
        let mut pos = 1;
        let mut last = 0;
        while pos < data.len() {
            let x = instr(pos, &data, b":");
            if x > pos {
                let per = val(mid(&data, pos, x - pos));
                let pct = Cur::percent(per - last);
                last = per;
                pos = x + 1;
                let mut x = instr(pos, &data, b"\n");
                if x == 0 {
                    x = data.len();
                }
                let line = lcase(mid(&data, pos, x.saturating_sub(pos)));
                let line_start = pos;
                pos = x;
                let mut y = 1;
                loop {
                    y = instr(y, &line, b"giveitem ");
                    if y == 0 {
                        break;
                    }
                    let v = extract_value(mid_from(&line, y), b"giveitem ");
                    match given.iter_mut().find(|(g, _)| *g == v) {
                        Some(g) => g.1 = g.1.add(pct),
                        None => given.push((v, pct)),
                    }
                    y += 1;
                }
                if check_spells {
                    learn_spells(&data, &line, line_start, spells);
                }
                let mut y = 1;
                loop {
                    y = instr(y, &line, b"random ");
                    if y == 0 {
                        break;
                    }
                    let v = extract_value(mid_from(&line, y), b"random ");
                    if v > 0 {
                        self.random_tb_items(out, v, nest, pct.mul(modifier), check_spells, spells);
                    }
                    y += 1;
                }
            } else if x == 0 {
                break;
            } else {
                pos += 1;
            }
        }
        for (v, pct) in given {
            if v > 0 {
                out.add(v, pct, modifier);
            }
        }
        *nest -= 1;
    }
}

/// The items a block gives: number, chance, chance of not getting it (NMR's index 0, the
/// number 0, is left out).
#[derive(Default)]
struct Chest(Vec<(i64, Cur, Cur)>);

impl Chest {
    /// NMR's merge of one block's items into the chest.
    fn add(&mut self, n: i64, pct: Cur, modifier: Cur) {
        match self.0.iter_mut().find(|(m, _, _)| *m == n) {
            Some((_, p, fail)) => {
                *p = p.add(fail.mul(pct).mul(modifier));
                *fail = fail.mul(Cur::ONE.sub(pct));
            }
            None => {
                let p = pct.mul(modifier);
                self.0.push((n, p, Cur::ONE.sub(p)));
            }
        }
    }
}

/// The spells a block teaches, with the classes it teaches each to.
#[derive(Default)]
struct Learned(Vec<(i64, String)>);

/// The `learnspell` commands of a line, as `GetTBItems` reads them.
fn learn_spells(data: &[u8], line: &[u8], line_start: usize, spells: &mut Learned) {
    let mut y = 1;
    loop {
        y = instr(y, line, b"learnspell ");
        if y == 0 {
            break;
        }
        let v = extract_value(mid_from(line, y), b"learnspell ");
        let test = class_restriction(data, line_start + y);
        // NMR's index 0 is the spell 0, and a learnspell of 0 lands on it
        if v == 0 {
            y += 1;
            continue;
        }
        match spells.0.iter_mut().find(|(s, _)| *s == v) {
            Some((_, classes)) => {
                *classes = if test != "(*)" {
                    merged_classes(&test, classes)
                } else {
                    test
                };
            }
            None => spells.0.push((v, test)),
        }
        y += 1;
    }
}

/// The text from the start of `at`'s line up to `at` (not including it).
fn line_before(whole: &[u8], at: usize) -> &[u8] {
    let before = mid(whole, 1, at.saturating_sub(1));
    let mut x = 1;
    loop {
        let p = instr(x, before, b"\n");
        if p == 0 {
            break;
        }
        x = p + 1;
    }
    mid(whole, x, at.saturating_sub(x))
}

/// `CheckForClassRestriction`: `(n)` for the first `class n` before `at` on its line (not
/// part of a longer word), or `(*)`.
fn class_restriction(whole: &[u8], at: usize) -> String {
    let line = line_before(whole, at);
    let look = b"class ";
    let mut x = 1;
    loop {
        let p = instr(x, line, look);
        if p == 0 {
            return "(*)".into();
        }
        x = p;
        if x > 1 && line[x - 2].is_ascii_lowercase() {
            // NMR looks at the same place again here, for ever; this moves on
            x += 1;
            continue;
        }
        let y1 = x + look.len();
        let y2 = digits_at(line, y1);
        if y2 == 0 {
            x = y1;
            continue;
        }
        return format!("({})", String::from_utf8_lossy(mid(line, y1, y2)));
    }
}

/// `GetTBPercent`: a random block line's chance, `(n%)`, its number less the line before's,
/// or `(?%)`.
fn tb_percent(whole: &[u8], at: usize) -> String {
    let before = mid(whole, 1, at.saturating_sub(1));
    let back_over_digits = |s: &[u8], y2: usize| {
        let mut y1 = y2 - 1;
        while y1 != 0 && s[y1 - 1].is_ascii_digit() {
            y1 -= 1;
        }
        y1
    };
    let mut last = 0;
    let mut x = 1;
    while instr(x, before, b"\n") != 0 {
        let y2 = instr(x, before, b":");
        if y2 == 0 {
            return "(?%)".into();
        }
        let y1 = back_over_digits(before, y2);
        last = val(mid(before, y1 + 1, y2 - y1 + 1));
        x = instr(x, before, b"\n") + 1;
    }
    let line = mid(whole, x, at.saturating_sub(x));
    let y2 = instr(1, line, b":");
    if y2 == 0 {
        return "(?%)".into();
    }
    let y1 = back_over_digits(line, y2);
    let pct = val(mid(line, y1 + 1, y2 - y1 + 1)) - last;
    if pct == 0 {
        "(?%)".into()
    } else {
        format!("({pct}%)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vb_strings() {
        assert_eq!(instr(1, b"abcabc", b"c"), 3);
        assert_eq!(instr(4, b"abcabc", b"c"), 6);
        assert_eq!(instr(7, b"abcabc", b"c"), 0);
        assert_eq!(mid(b"abcdef", 2, 3), b"bcd");
        assert_eq!(val(b"\n50:x"), 50);
        assert_eq!(val(b"1 2"), 12);
        assert_eq!(extract_value(b"giveitem  12 3", b"giveitem "), 12);
    }

    #[test]
    fn currency_rounds_half_even() {
        let third = Cur::percent(33);
        assert_eq!(third.mul(third), Cur(1089));
        assert_eq!(Cur(1).mul(Cur(5000)), Cur(0));
        assert_eq!(Cur(3).mul(Cur(5000)), Cur(2));
        assert_eq!(Cur::percent(25).shown_percent(), "25");
        assert_eq!(Cur(1234).shown_percent(), "12.3");
        assert_eq!(Cur(1250).shown_percent(), "12.5");
    }

    #[test]
    fn notes_append_and_clip() {
        assert_eq!(
            noted("\0", "Room 1/2", 250, 248).as_deref(),
            Some("Room 1/2")
        );
        assert_eq!(
            noted("Room 1/2", "Shop #3", 250, 248).as_deref(),
            Some("Room 1/2, Shop #3")
        );
        // NMR's InStr: a note inside a longer one counts as there
        assert_eq!(noted("Room 1/23", "Room 1/2", 250, 248), None);
        let long = "x".repeat(249);
        assert_eq!(noted(&long, "yy", 250, 248).unwrap().chars().count(), 250);
    }

    #[test]
    fn classes_merge_as_nmr_merges_them() {
        assert_eq!(merged_classes("(1),(2)", "(2),(3)"), "(2),(3),(1)");
        assert_eq!(merged_classes("(1),(2),(4)", "(3)"), "(1),(2),(4),(3)");
        assert_eq!(merged_classes("(*)", "(3)"), "(3),(0)");
    }

    #[test]
    fn percent_and_class_of_a_line() {
        let t = b"50:giveitem 3\n80:class 4 learnspell 9\n";
        assert_eq!(tb_percent(t, 4), "(50%)");
        let at = instr(1, t, b"learnspell");
        assert_eq!(tb_percent(t, at), "(30%)");
        assert_eq!(class_restriction(t, at), "(4)");
        assert_eq!(class_restriction(t, 4), "(*)");
    }
}
