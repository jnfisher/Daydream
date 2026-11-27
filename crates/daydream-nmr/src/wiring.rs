//! Checks that read the wiring: what records say to each other, read across the whole world
//! (a card's own checks read one record). Each finding says what is wrong, or what was
//! checked and holds, about one record, naming the records it rests on.
//!
//! - [`Check::Prices`]: what a script line charges (its `price` clauses added up, as the game
//!   takes them one after another) against the amounts its words quote: first the line's own
//!   (the message it says when the player cannot pay or after, the text it shows, the answer
//!   that runs it), then anything else the same conversation says. With an item given, the
//!   price is put against the item's Cost.
//! - [`Check::Spelling`]: a word in the world's text that the dictionary does not have, used
//!   once or twice in the world, one letter from a word the dictionary and the world both have.
//!   Dialect (`askin`), old forms (`couldst`), words joined (`batlike`), plurals of known words
//!   and names (always capitalised) are left alone, and so are the world's accepted words.
//! - [`Check::Placeholders`]: a blank message where the game would otherwise tell a blow in its
//!   own words (a monster's attack, a weapon's verbs), and names or text left as placeholders.
//! - [`Check::Spawns`]: rooms whose spawn range has no monster of their group, so they spawn
//!   nothing, and monsters no room spawns, with what does bring them: a room that keeps one as
//!   its NPC, a spell that summons it (and what casts the spell), a script, or nothing at all.
//!
//! The game's rules are silvermere's (silvermere-engine `combat/attack.rs` for the attack
//! slots, `combat/mod.rs` and `combat/messages.rs` for the lines a blank message replaces,
//! silvermere-rules `spawn.rs` through [`crate::refs::Spawning`]).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use daydream_store::Realm;

use crate::NmrRows;
use crate::refs::{Spawning, num};
use crate::rows::{self, RowError};
use crate::schema::Value;
use crate::script::{self, Clause, Reading};

/// The tables the checks read.
pub const TABLES: [&str; 9] = [
    "Items",
    "Monsters",
    "Spells",
    "Rooms",
    "Shops",
    "Messages",
    "Textblocks",
    "Classes",
    "Races",
];

/// One of the checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Check {
    Prices,
    Spelling,
    Placeholders,
    Spawns,
}

impl Check {
    pub const ALL: [Check; 4] = [
        Check::Prices,
        Check::Spelling,
        Check::Placeholders,
        Check::Spawns,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Check::Prices => "prices",
            Check::Spelling => "spelling",
            Check::Placeholders => "placeholders",
            Check::Spawns => "spawns",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Check::Prices => "Prices",
            Check::Spelling => "Spelling",
            Check::Placeholders => "Placeholders",
            Check::Spawns => "Spawns",
        }
    }

    /// What it reads, in a sentence.
    pub fn about(self) -> &'static str {
        match self {
            Check::Prices => {
                "What each script charges, against the prices its words quote and the Cost of what it gives."
            }
            Check::Spelling => {
                "Words the dictionary does not have, used once or twice, one letter from a word the world uses."
            }
            Check::Placeholders => {
                "Blank messages where the game would tell a blow in its own words, and names left as placeholders."
            }
            Check::Spawns => {
                "Rooms that spawn nothing, and monsters no room spawns, with what else brings them into play."
            }
        }
    }

    pub fn parse(s: &str) -> Option<Check> {
        Check::ALL.into_iter().find(|c| c.id() == s)
    }
}

/// How much a finding matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    /// Players meet it: a price that is not what was said, a blow told by an empty line, a
    /// room that spawns nothing.
    Warn,
    /// Worth a look: a likely typo, a placeholder, a price nothing quotes, a monster nothing
    /// brings (often one kept back on purpose).
    Note,
    /// Checked and fine, with what it rests on: a price that matches its words, a monster
    /// brought another way.
    Ok,
}

impl Level {
    pub fn id(self) -> &'static str {
        match self {
            Level::Warn => "warn",
            Level::Note => "note",
            Level::Ok => "ok",
        }
    }
}

/// A record, as a finding names it: a room by its map and number, a text block by its number
/// and part, anything else by its number.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rec {
    pub table: &'static str,
    pub key: Vec<i64>,
}

impl Rec {
    pub fn of(table: &'static str, n: i64) -> Rec {
        Rec {
            table,
            key: vec![n],
        }
    }

    pub fn room(map: i64, n: i64) -> Rec {
        Rec {
            table: "Rooms",
            key: vec![map, n],
        }
    }

    pub fn block(n: i64, part: i64) -> Rec {
        Rec {
            table: "Textblocks",
            key: vec![n, part],
        }
    }

    /// The record's number (a room's, a text block's).
    pub fn number(&self) -> i64 {
        match self.table {
            "Rooms" => self.key.get(1).copied().unwrap_or(0),
            _ => self.key.first().copied().unwrap_or(0),
        }
    }

    /// `message 1711`, `text block 909`, `room 1/224`.
    pub fn label(&self) -> String {
        let kind = match self.table {
            "Items" => "item",
            "Monsters" => "monster",
            "Spells" => "spell",
            "Rooms" => "room",
            "Shops" => "shop",
            "Messages" => "message",
            "Textblocks" => "text block",
            "Classes" => "class",
            "Races" => "race",
            other => other,
        };
        match self.table {
            "Rooms" => format!("{kind} {}/{}", self.key[0], self.key[1]),
            _ => format!("{kind} {}", self.number()),
        }
    }

    /// Whether it is `other`, a text block's parts all one block.
    pub fn is(&self, other: &Rec) -> bool {
        self.table == other.table
            && match self.table {
                "Textblocks" => self.number() == other.number(),
                _ => self.key == other.key,
            }
    }
}

/// A piece of what a finding says: words, or a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bit {
    Text(String),
    Rec(Rec),
}

fn tx(s: impl Into<String>) -> Bit {
    Bit::Text(s.into())
}

fn rc(rec: Rec) -> Bit {
    Bit::Rec(rec)
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub check: Check,
    pub level: Level,
    /// The record it is about.
    pub about: Rec,
    /// The column of `about` it is about.
    pub column: Option<String>,
    /// What it says, records as records.
    pub bits: Vec<Bit>,
    /// The other records it rests on.
    pub refs: Vec<Rec>,
    /// Its fix: values to set on `about`.
    pub fix: Vec<(String, i64)>,
    /// Spelling: the word, and what it may be (likeliest first).
    pub word: Option<String>,
    pub suggest: Vec<String>,
}

impl Finding {
    fn new(check: Check, level: Level, about: Rec, bits: Vec<Bit>) -> Finding {
        let refs = bits
            .iter()
            .filter_map(|b| match b {
                Bit::Rec(x) if !x.is(&about) => Some(x.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Finding {
            check,
            level,
            about,
            column: None,
            bits,
            refs,
            fix: Vec::new(),
            word: None,
            suggest: Vec::new(),
        }
    }

    fn column(mut self, c: impl Into<String>) -> Finding {
        self.column = Some(c.into());
        self
    }

    /// Whether it is about `rec` or rests on it.
    pub fn concerns(&self, rec: &Rec) -> bool {
        self.about.is(rec) || self.refs.iter().any(|x| x.is(rec))
    }

    /// What it says as text, each record by its label and the name `name` gives it.
    pub fn text(&self, name: impl Fn(&Rec) -> Option<String>) -> String {
        let mut out = String::new();
        for b in &self.bits {
            match b {
                Bit::Text(s) => out.push_str(s),
                Bit::Rec(x) => {
                    let mut label = x.label();
                    // a sentence that starts with a record
                    if out.is_empty() || out.ends_with(". ") {
                        label[..1].make_ascii_uppercase();
                    }
                    out.push_str(&label);
                    if let Some(n) = name(x).filter(|n| !n.is_empty()) {
                        out.push_str(&format!(" ({n})"));
                    }
                }
            }
        }
        out
    }
}

/// What the checks are told besides the world.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Words the world uses on purpose, lower case: spelling leaves them alone.
    pub words: HashSet<String>,
}

// ---- the world's tables -------------------------------------------------------------------

/// A table's rows, with where each column is.
pub struct Table {
    pub rows: NmrRows,
    at: HashMap<String, usize>,
}

impl Table {
    pub fn new(rows: NmrRows) -> Table {
        let at = rows
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.clone(), i))
            .collect();
        Table { rows, at }
    }

    fn has(&self, c: &str) -> bool {
        self.at.contains_key(c)
    }

    /// A number cell (0 when the column is not there or holds none).
    fn n(&self, row: &[Value], c: &str) -> i64 {
        self.at.get(c).and_then(|&i| num(&row[i])).unwrap_or(0)
    }

    /// A text cell, up to its first NUL.
    fn s<'r>(&self, row: &'r [Value], c: &str) -> &'r str {
        match self.at.get(c).map(|&i| &row[i]) {
            Some(Value::Text(s)) => s.split('\0').next().unwrap_or(""),
            _ => "",
        }
    }
}

/// The tables the checks read, each read once.
pub struct World {
    tables: HashMap<String, Table>,
    /// Each message's lines, by number.
    messages: HashMap<i64, [String; 3]>,
    /// Each text block's parts in order: part, data and Link To.
    blocks: HashMap<i64, Vec<(i64, String, i64)>>,
}

impl World {
    /// The realm's tables [`TABLES`].
    pub fn load(realm: &Realm) -> Result<World, RowError> {
        let mut rows = Vec::new();
        for name in TABLES {
            let t = rows::record_table(name)?;
            rows.push(rows::load(realm, t)?.1);
        }
        Ok(World::from_rows(rows))
    }

    /// A world of these tables (any the checks read; those left out are empty).
    pub fn from_rows(tables: Vec<NmrRows>) -> World {
        let tables: HashMap<String, Table> = tables
            .into_iter()
            .map(|r| (r.name.clone(), Table::new(r)))
            .collect();
        let mut messages = HashMap::new();
        if let Some(m) = tables.get("Messages") {
            for row in &m.rows.rows {
                let line = |i: usize| m.s(row, &format!("Line {i}")).to_string();
                messages.insert(m.n(row, "Number"), [line(1), line(2), line(3)]);
            }
        }
        let mut blocks: HashMap<i64, Vec<(i64, String, i64)>> = HashMap::new();
        if let Some(b) = tables.get("Textblocks") {
            for row in &b.rows.rows {
                blocks.entry(b.n(row, "Number")).or_default().push((
                    b.n(row, "Part #"),
                    b.s(row, "Data").to_string(),
                    b.n(row, "Link To"),
                ));
            }
            for parts in blocks.values_mut() {
                parts.sort_by_key(|p| p.0);
            }
        }
        World {
            tables,
            messages,
            blocks,
        }
    }

    fn table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    /// Each row of a table, with the table (none when the world lacks it).
    fn each(&self, name: &str) -> impl Iterator<Item = (&Table, &Vec<Value>)> {
        self.table(name)
            .into_iter()
            .flat_map(|t| t.rows.rows.iter().map(move |row| (t, row)))
    }

    /// A record's name, as a list titles it.
    pub fn name(&self, rec: &Rec) -> Option<String> {
        let t = self.table(rec.table)?;
        let row = t.rows.rows.iter().find(|row| match rec.table {
            "Rooms" => {
                t.n(row, "Map Number") == rec.key[0] && t.n(row, "Room Number") == rec.key[1]
            }
            "Textblocks" => false,
            _ => t.n(row, "Number") == rec.number(),
        })?;
        let col = match rec.table {
            "Messages" => "Line 1",
            _ => "Name",
        };
        if !t.has(col) {
            return None;
        }
        let name = ansi_free(t.s(row, col));
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        Some(match name.char_indices().nth(48) {
            // a long line cut at a word
            Some((at, _)) => {
                let cut = name[..at].rfind(' ').unwrap_or(at);
                format!("{}…", &name[..cut])
            }
            None => name,
        })
    }

    /// Whether message `n` is in the world with nothing on any of its lines.
    fn blank(&self, n: i64) -> bool {
        self.messages
            .get(&n)
            .is_some_and(|m| m.iter().all(|l| l.trim().is_empty()))
    }

    /// A text block's whole text, its parts in order.
    fn block_text(&self, n: i64) -> Option<String> {
        self.blocks
            .get(&n)
            .map(|p| p.iter().map(|(_, d, _)| d.as_str()).collect())
    }

    /// A text block's Link To (its first part's).
    fn link(&self, n: i64) -> i64 {
        self.blocks
            .get(&n)
            .and_then(|p| p.first())
            .map_or(0, |p| p.2)
    }
}

/// Text with its ANSI colour codes taken out.
fn ansi_free(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for d in chars.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn thousands(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// The coins, largest first, and their worth in copper.
const COINS: [(&str, i64); 5] = [
    ("runic", 1_000_000),
    ("platinum", 10_000),
    ("gold", 100),
    ("silver", 10),
    ("copper", 1),
];

/// An amount of copper in the coins it makes, largest first (`20 platinum`, `12 gold, 3
/// copper`), or in `coin` when it comes to a whole number of them.
fn coins(copper: i64, coin: Option<&str>) -> String {
    if let Some(c) = coin
        && let Some(&(_, worth)) = COINS.iter().find(|(n, _)| *n == c)
        && copper % worth == 0
    {
        return format!("{} {c}", thousands(copper / worth));
    }
    let mut left = copper;
    let mut out = Vec::new();
    for (name, worth) in COINS {
        let k = left / worth;
        if k > 0 {
            out.push(format!("{} {name}", thousands(k)));
        }
        left -= k * worth;
    }
    if out.is_empty() {
        "nothing".into()
    } else {
        out.join(", ")
    }
}

/// Runs the checks `which` over the world.
pub fn check(world: &World, opts: &Options, which: &[Check]) -> Vec<Finding> {
    let mut out = Vec::new();
    for c in which {
        match c {
            Check::Prices => prices(world, &mut out),
            Check::Spelling => spelling(world, opts, &mut out),
            Check::Placeholders => placeholders(world, &mut out),
            Check::Spawns => spawns(world, &mut out),
        }
    }
    out
}

// ---- a realm's report ---------------------------------------------------------------------

/// Every check over a realm, and the names of the records its findings name.
pub struct Report {
    pub findings: Vec<Finding>,
    /// Each record a finding names, by its name (a message by its first line).
    pub names: HashMap<Rec, String>,
}

impl Report {
    /// The checks over `world`.
    pub fn of(world: &World) -> Report {
        let findings = check(world, &Options::default(), &Check::ALL);
        let mut names = HashMap::new();
        for f in &findings {
            for x in std::iter::once(&f.about)
                .chain(&f.refs)
                .chain(f.bits.iter().filter_map(|b| match b {
                    Bit::Rec(x) => Some(x),
                    Bit::Text(_) => None,
                }))
            {
                if !names.contains_key(x)
                    && let Some(n) = world.name(x).filter(|n| !n.is_empty())
                {
                    names.insert(x.clone(), n);
                }
            }
        }
        Report { findings, names }
    }

    /// A finding as text, records by label and name.
    pub fn text(&self, f: &Finding) -> String {
        f.text(|x| self.names.get(x).cloned())
    }
}

/// What says a realm's files changed: each world file's name, length and time written.
type Fingerprint = Vec<(String, u64, Option<SystemTime>)>;

fn fingerprint(dir: &Path) -> Fingerprint {
    let mut out: Fingerprint = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let upper = name.to_ascii_uppercase();
            // the database and its write-ahead log; the shared-memory file changes on reads
            let world =
                upper.starts_with("WCC") && (upper.ends_with(".DB") || upper.ends_with(".DB-WAL"));
            let meta = e.metadata().ok().filter(|_| world)?;
            Some((name, meta.len(), meta.modified().ok()))
        })
        .collect();
    out.sort();
    out
}

/// The realms whose report is kept.
const KEPT: usize = 4;

/// The report on the realm in `dir`: kept, and made again only when the realm's files change
/// (a save, a publish, an import, the game's own writes).
pub fn report(dir: &Path) -> Result<Arc<Report>, RowError> {
    static CACHE: Mutex<Vec<(PathBuf, Fingerprint, Arc<Report>)>> = Mutex::new(Vec::new());
    let print = fingerprint(dir);
    // held while a report is made, so two asks make it once
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, _, r)) = cache.iter().find(|(d, p, _)| d == dir && *p == print) {
        return Ok(r.clone());
    }
    let realm = Realm::open(dir)?;
    let r = Arc::new(Report::of(&World::load(&realm)?));
    cache.retain(|(d, _, _)| d != dir);
    if cache.len() >= KEPT {
        cache.remove(0);
    }
    cache.push((dir.to_path_buf(), print, r.clone()));
    Ok(r)
}

/// The side file of a realm's accepted words.
fn words_file(dir: &Path) -> PathBuf {
    dir.join(".daydream").join("checks.json")
}

/// The words the realm in `dir` uses on purpose, which spelling leaves alone: kept beside its
/// working copy in `.daydream/checks.json`.
pub fn accepted(dir: &Path) -> HashSet<String> {
    std::fs::read(words_file(dir))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| {
            v.get("words").and_then(|w| w.as_array()).map(|ws| {
                ws.iter()
                    .filter_map(|w| w.as_str())
                    .map(str::to_ascii_lowercase)
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Accepts `word` for the realm in `dir` (or, with `yes` false, takes it back), and gives the
/// accepted words. The caller keeps two writers apart.
pub fn accept(dir: &Path, word: &str, yes: bool) -> std::io::Result<BTreeSet<String>> {
    let w = word.trim().to_ascii_lowercase();
    if w.is_empty() || w.len() > 64 || !w.bytes().all(|b| b.is_ascii_alphabetic() || b == b'\'') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "a word is letters (and apostrophes), at most 64",
        ));
    }
    let mut words: BTreeSet<String> = accepted(dir).into_iter().collect();
    if yes {
        words.insert(w);
    } else {
        words.remove(&w);
    }
    let file = words_file(dir);
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d)?;
    }
    let body = serde_json::json!({ "daydream": "checks", "version": 1, "words": words });
    let tmp = file.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&body).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(&tmp, &file)?;
    Ok(words)
}

/// Whether a finding stays once the realm's accepted words are left alone.
pub fn stays(f: &Finding, words: &HashSet<String>) -> bool {
    f.word.as_ref().is_none_or(|w| !words.contains(w))
}

// ---- prices -------------------------------------------------------------------------------

/// An amount a text quotes: its words' place, and the amount in copper and the coin it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    pub copper: i64,
    pub coin: &'static str,
}

const UNITS: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 8] = [
    "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

/// A number word's worth: (value, scale) where a scale multiplies what came before.
fn number_word(w: &str) -> Option<(i64, bool)> {
    if let Some(i) = UNITS.iter().position(|u| *u == w) {
        return Some((i as i64, false));
    }
    if let Some(i) = TENS.iter().position(|u| *u == w) {
        return Some((20 + 10 * i as i64, false));
    }
    match w {
        "hundred" => Some((100, true)),
        "thousand" => Some((1_000, true)),
        "million" => Some((1_000_000, true)),
        _ => None,
    }
}

/// Number words in order (`a thousand`, `two thousand five hundred`) as a number.
fn words_number(ws: &[String]) -> Option<i64> {
    let (mut total, mut cur, mut any) = (0i64, 0i64, false);
    for (i, w) in ws.iter().enumerate() {
        if (w == "a" || w == "an") && i == 0 {
            cur = 1;
            continue;
        }
        if w == "and" {
            continue;
        }
        let (v, scale) = number_word(w)?;
        any = true;
        if !scale {
            cur += v;
        } else if v == 100 {
            cur = cur.max(1) * 100;
        } else {
            total += cur.max(1) * v;
            cur = 0;
        }
    }
    any.then_some(total + cur)
}

/// The amounts a text quotes: a number, in figures (`2,000`) or words (`a thousand`), just
/// before a coin's name (`gold`, `platinum`...).
pub fn quotes(text: &str) -> Vec<Quote> {
    let clean = ansi_free(text).to_ascii_lowercase();
    let tokens: Vec<String> = clean
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == ','))
        .map(|w| w.trim_matches(',').to_string())
        .filter(|w| !w.is_empty())
        .collect();
    let mut out = Vec::new();
    for (i, w) in tokens.iter().enumerate() {
        let Some(&(coin, worth)) = COINS.iter().find(|(n, _)| *n == w.as_str()) else {
            continue;
        };
        let Some(prev) = i.checked_sub(1).map(|j| &tokens[j]) else {
            continue;
        };
        let amount = if prev.bytes().all(|b| b.is_ascii_digit() || b == b',')
            && prev.bytes().any(|b| b.is_ascii_digit())
        {
            prev.replace(',', "").parse::<i64>().ok()
        } else {
            // the number words before it
            let mut from = i;
            while from > 0 {
                let w = tokens[from - 1].as_str();
                if number_word(w).is_some() || w == "and" || w == "a" || w == "an" {
                    from -= 1;
                } else {
                    break;
                }
            }
            // an "and" or "a" alone is no number
            words_number(&tokens[from..i])
        };
        if let Some(a) = amount.filter(|a| *a > 0) {
            out.push(Quote {
                copper: a.saturating_mul(worth),
                coin,
            });
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum How {
    Topics,
    Shown,
    Answer,
    Run(Reading),
}

/// A script line that charges.
#[derive(Debug, Clone)]
struct PriceLine {
    block: i64,
    part: i64,
    /// Its line among the part's lines, from 1, and how many lines the part has.
    no: usize,
    of: usize,
    total: i64,
    /// The messages it says: the price's own (said when the player cannot pay), then the rest.
    cannot: Vec<i64>,
    said: Vec<i64>,
    shows: Vec<i64>,
    gives: Vec<i64>,
    /// What it does besides charging, clause by clause, as written.
    does: Vec<String>,
}

/// Everything one conversation reaches: the blocks shown, the messages said and the lines
/// that charge, and the answers that run each block.
#[derive(Default)]
struct Walk {
    shown: Vec<i64>,
    said: Vec<i64>,
    lines: Vec<PriceLine>,
    led_by: HashMap<i64, Vec<i64>>,
}

/// The most blocks one conversation is followed through.
const MAX_WALK: usize = 400;

fn walk(world: &World, root: i64, how: How) -> Walk {
    let mut w = Walk::default();
    let mut seen: HashSet<(i64, How)> = HashSet::new();
    let mut queue = vec![(root, how)];
    while let Some((n, how)) = queue.pop() {
        if n <= 0 || seen.len() >= MAX_WALK || !seen.insert((n, how)) {
            continue;
        }
        let Some(parts) = world.blocks.get(&n) else {
            continue;
        };
        match how {
            How::Topics => {
                let link = world.link(n);
                if link > 0 {
                    queue.push((link, How::Shown));
                }
                for (_, data, _) in parts {
                    for l in script::lines_of(data, Reading::Topics) {
                        if let Some(b) = l.block.filter(|b| *b > 0) {
                            queue.push((i64::from(b), How::Answer));
                        }
                    }
                }
            }
            How::Shown => w.shown.push(n),
            How::Answer => {
                w.shown.push(n);
                let link = world.link(n);
                if link > 0 {
                    w.led_by.entry(link).or_default().push(n);
                    queue.push((link, How::Run(Reading::Script)));
                }
            }
            How::Run(reading) => {
                for (part, data, _) in parts {
                    let lines = script::lines_of(data, reading);
                    let of = lines.len();
                    for l in lines {
                        if l.never {
                            continue;
                        }
                        let mut p = PriceLine {
                            block: n,
                            part: *part,
                            no: l.no,
                            of,
                            total: 0,
                            cannot: Vec::new(),
                            said: Vec::new(),
                            shows: Vec::new(),
                            gives: Vec::new(),
                            does: Vec::new(),
                        };
                        for c in &l.clauses {
                            run_clause(c, &mut p, &mut queue);
                        }
                        w.said.extend(p.cannot.iter().chain(&p.said));
                        if p.total > 0 {
                            w.lines.push(p);
                        }
                    }
                }
            }
        }
    }
    w
}

/// What one clause of a line adds to it: a price, the messages it says, the blocks it shows
/// or runs (queued), the items it gives.
fn run_clause(c: &Clause, p: &mut PriceLine, queue: &mut Vec<(i64, How)>) {
    let Some(kw) = c.kw else {
        return;
    };
    let g = |name: &str| i64::from(c.get(name));
    let msg = g("msg");
    if kw == "price" {
        if g("copper") > 0 {
            p.total += g("copper");
            if msg > 0 {
                p.cannot.push(msg);
            }
        }
        return;
    }
    p.does.push(c.text.trim().to_ascii_lowercase());
    match kw {
        "text" | "roomtext" => {
            if g("block") > 0 {
                p.shows.push(g("block"));
                queue.push((g("block"), How::Shown));
            }
        }
        "random" => queue.push((g("block"), How::Run(Reading::Chances))),
        "testskill" | "checkspell" => queue.push((g("block"), How::Run(Reading::Script))),
        "giveitem" => p.gives.push(g("item")),
        _ => {}
    }
    if msg > 0 {
        p.said.push(msg);
    }
}

/// A ratio in words: `7.3`, `533`.
fn times(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{}", ratio.round() as i64)
    } else {
        let s = format!("{ratio:.1}");
        s.strip_suffix(".0").map_or(s.clone(), str::to_string)
    }
}

/// A price against the Cost of the item it gives: `That is twice the 1,000 gold item 868
/// costs.`
fn against_cost(world: &World, total: i64, item: i64) -> Option<Vec<Bit>> {
    let t = world.table("Items")?;
    let row = t.rows.rows.iter().find(|row| t.n(row, "Number") == item)?;
    let cost = t.n(row, "Cost");
    let ty = t.n(row, "Cost Type").clamp(0, 4) as usize;
    let (coin, worth) = COINS[4 - ty];
    let copper = cost * worth;
    if copper <= 0 {
        return None;
    }
    let item = rc(Rec::of("Items", item));
    if total == copper {
        return Some(vec![tx(" That is what "), item, tx(" costs.")]);
    }
    let ratio = total as f64 / copper as f64;
    let how = if total == copper * 2 {
        "twice".to_string()
    } else if total * 2 == copper {
        "half".to_string()
    } else if total * 10 == copper {
        "a tenth of".to_string()
    } else if ratio < 1.0 {
        format!("{}% of", (ratio * 100.0).round() as i64)
    } else {
        format!("{} times", times(ratio))
    };
    Some(vec![
        tx(format!(" That is {how} the {} {coin} ", thousands(cost))),
        item,
        tx(" costs."),
    ])
}

/// The quotes of a message's lines, or of a block's text.
fn quotes_of(world: &World, rec: &Rec) -> Vec<Quote> {
    match rec.table {
        "Messages" => world
            .messages
            .get(&rec.number())
            .map(|m| quotes(&m.join("\n")))
            .unwrap_or_default(),
        "Textblocks" => world
            .block_text(rec.number())
            .map(|s| quotes(&s))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// `2,000 gold`, `2,000 and 8,000 gold`: quotes in words, the same coin's together.
fn quoted(qs: &[Quote]) -> String {
    let mut by: BTreeMap<i64, (&str, BTreeSet<i64>)> = BTreeMap::new();
    for q in qs {
        let worth = COINS.iter().find(|(n, _)| *n == q.coin).map_or(1, |c| c.1);
        by.entry(-worth)
            .or_insert((q.coin, BTreeSet::new()))
            .1
            .insert(q.copper / worth);
    }
    by.values()
        .map(|(coin, amounts)| {
            let a: Vec<String> = amounts.iter().map(|a| thousands(*a)).collect();
            format!("{} {coin}", and_list(&a))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `a`, `a and b`, `a, b and c`.
fn and_list(a: &[String]) -> String {
    match a.len() {
        0 => String::new(),
        1 => a[0].clone(),
        n => format!("{} and {}", a[..n - 1].join(", "), a[n - 1]),
    }
}

/// Records as bits joined by commas and "and".
fn listed(recs: &[Rec]) -> Vec<Bit> {
    let mut out = Vec::new();
    for (i, x) in recs.iter().enumerate() {
        if i > 0 {
            out.push(tx(if i + 1 == recs.len() { " and " } else { ", " }));
        }
        out.push(rc(x.clone()));
    }
    out
}

/// The first `most` records as bits, and how many more: `a, b, c and 4 more`.
fn listed_some(recs: &[Rec], most: usize) -> Vec<Bit> {
    if recs.len() <= most {
        return listed(recs);
    }
    let mut out = Vec::new();
    for (i, x) in recs.iter().take(most).enumerate() {
        if i > 0 {
            out.push(tx(", "));
        }
        out.push(rc(x.clone()));
    }
    out.push(tx(format!(" and {} more", recs.len() - most)));
    out
}

/// Sources with the quotes that match a price.
fn matching(qs: &[(Rec, Vec<Quote>)], total: i64) -> Vec<Rec> {
    qs.iter()
        .filter(|(_, q)| q.iter().any(|q| q.copper == total))
        .map(|(s, _)| s.clone())
        .collect()
}

fn prices(world: &World, out: &mut Vec<Finding>) {
    // the conversations: a monster's greeting and topics, a room's commands, once each
    let mut roots: Vec<(Rec, i64, How)> = Vec::new();
    let mut started: HashSet<(i64, How)> = HashSet::new();
    for (t, row) in world.each("Monsters") {
        let greet = t.n(row, "Greet Txt");
        if greet > 0 && started.insert((greet, How::Topics)) {
            roots.push((Rec::of("Monsters", t.n(row, "Number")), greet, How::Topics));
        }
    }
    for (t, row) in world.each("Rooms") {
        let cmd = t.n(row, "Command Text");
        let how = How::Run(Reading::Commands);
        if cmd > 0 && started.insert((cmd, how)) {
            roots.push((rec_of("Rooms", t, row), cmd, how));
        }
    }
    let mut done: HashSet<(i64, i64, usize)> = HashSet::new();
    for (who, root, how) in roots {
        let mut w = walk(world, root, how);
        w.lines.sort_by_key(|p| (p.block, p.part, p.no));
        let lines: Vec<&PriceLine> = w
            .lines
            .iter()
            .filter(|p| done.insert((p.block, p.part, p.no)))
            .collect();
        if lines.is_empty() {
            continue;
        }
        // everything the conversation says, what it shows first
        let mut sources: Vec<Rec> = w.shown.iter().map(|&b| Rec::block(b, 0)).collect();
        sources.extend(w.said.iter().map(|&m| Rec::of("Messages", m)));
        sources.sort_by_key(|s| (s.table != "Textblocks", s.clone()));
        sources.dedup();
        let all: Vec<(Rec, Vec<Quote>)> = sources
            .into_iter()
            .map(|s| {
                let q = quotes_of(world, &s);
                (s, q)
            })
            .filter(|(_, q)| !q.is_empty())
            .collect();
        // each line's finding, the words after its price
        type Said = (i64, i64, Level, i64, Option<&'static str>, Vec<Bit>);
        let mut said: Vec<(usize, Said)> = Vec::new();
        for p in &lines {
            let coin =
                |qs: &[(Rec, Vec<Quote>)]| qs.first().and_then(|(_, q)| q.first()).map(|q| q.coin);
            // the line's own words: the messages it says, the text it shows, the answer that
            // runs its block
            let mut near: Vec<Rec> = p.cannot.iter().map(|&m| Rec::of("Messages", m)).collect();
            near.extend(p.said.iter().map(|&m| Rec::of("Messages", m)));
            near.extend(p.shows.iter().map(|&b| Rec::block(b, 0)));
            near.extend(
                w.led_by
                    .get(&p.block)
                    .into_iter()
                    .flatten()
                    .map(|&b| Rec::block(b, 0)),
            );
            near.dedup();
            let near_q: Vec<(Rec, Vec<Quote>)> = near
                .iter()
                .map(|s| (s.clone(), quotes_of(world, s)))
                .filter(|(_, q)| !q.is_empty())
                .collect();
            // the block's other lines that do the same for another price
            let same: Vec<&&PriceLine> = lines
                .iter()
                .filter(|o| {
                    o.block == p.block && o.part == p.part && o.no != p.no && o.does == p.does
                })
                .collect();
            let others: Vec<&&PriceLine> = same
                .iter()
                .copied()
                .filter(|o| o.total != p.total)
                .collect();
            let alike = same.len() - others.len();
            let majority = others
                .iter()
                .map(|o| o.total)
                .max_by_key(|t| (others.iter().filter(|o| o.total == *t).count(), -t));
            let quote_bits = |hit: &[Rec]| {
                let mut b = vec![tx(", as ")];
                b.extend(listed(hit));
                b.push(tx(if hit.len() == 1 {
                    " quotes."
                } else {
                    " quote."
                }));
                b
            };
            let (level, hint, mut bits) = if let Some(m) = majority.filter(|m| {
                !p.does.is_empty() && others.iter().filter(|o| o.total == *m).count() > alike + 1
            }) {
                let nos: Vec<String> = others
                    .iter()
                    .filter(|o| o.total == m)
                    .map(|o| o.no.to_string())
                    .collect();
                (
                    Level::Warn,
                    None,
                    vec![tx(format!(
                        ", but line{} {} charge{} {} to do the same.",
                        if nos.len() == 1 { "" } else { "s" },
                        and_list(&nos),
                        if nos.len() == 1 { "s" } else { "" },
                        coins(m, None)
                    ))],
                )
            } else if !near_q.is_empty() {
                let hit = matching(&near_q, p.total);
                if hit.is_empty() {
                    let qs: Vec<Quote> = near_q.iter().flat_map(|(_, q)| q.clone()).collect();
                    let srcs: Vec<Rec> = near_q.iter().map(|(s, _)| s.clone()).collect();
                    let mut b = vec![tx(format!(", but its own words quote {}: ", quoted(&qs)))];
                    b.extend(listed(&srcs));
                    b.push(tx("."));
                    (Level::Warn, coin(&near_q), b)
                } else {
                    (Level::Ok, coin(&near_q), quote_bits(&hit))
                }
            } else if !all.is_empty() {
                // what the conversation shows first, then what its other lines say
                let shown: Vec<(Rec, Vec<Quote>)> = all
                    .iter()
                    .filter(|(s, _)| s.table == "Textblocks")
                    .cloned()
                    .collect();
                let hit = match matching(&shown, p.total) {
                    h if h.is_empty() => matching(&all, p.total),
                    h => h,
                };
                if hit.is_empty() {
                    let qs = quoted(&all.iter().flat_map(|(_, q)| q.clone()).collect::<Vec<_>>());
                    let b = if who.table == "Rooms" {
                        vec![
                            tx(", but the commands of "),
                            rc(who.clone()),
                            tx(format!(" never quote that price; they quote {qs}.")),
                        ]
                    } else {
                        vec![
                            tx(", but the talk with "),
                            rc(who.clone()),
                            tx(format!(" never quotes that price; it quotes {qs}.")),
                        ]
                    };
                    (Level::Note, coin(&all), b)
                } else {
                    (Level::Ok, coin(&all), quote_bits(&hit))
                }
            } else {
                continue;
            };
            if p.gives.len() == 1
                && let Some(c) = against_cost(world, p.total, p.gives[0])
            {
                bits.extend(c);
            }
            said.push((p.no, (p.block, p.part, level, p.total, hint, bits)));
        }
        // lines of one part that say the same are one finding
        let mut groups: Vec<(Vec<usize>, Said)> = Vec::new();
        for (no, s) in said {
            match groups.iter_mut().find(|(_, g)| *g == s) {
                Some((nos, _)) => nos.push(no),
                None => groups.push((vec![no], s)),
            }
        }
        for (nos, (block, part, level, total, hint, bits)) in groups {
            let of = lines
                .iter()
                .find(|p| p.block == block && p.part == part)
                .map_or(1, |p| p.of);
            let parts = world.blocks.get(&block).map_or(0, Vec::len);
            let which = and_list(&nos.iter().map(usize::to_string).collect::<Vec<_>>());
            let lines_word = if nos.len() == 1 { "line" } else { "lines" };
            let at = match (parts > 1, of > 1) {
                (true, _) => format!(" (part {part}, {lines_word} {which})"),
                (false, true) => format!(" ({lines_word} {which})"),
                _ => String::new(),
            };
            let mut all_bits = vec![tx(format!("Charges {}{at}", coins(total, hint)))];
            all_bits.extend(bits);
            let mut f = Finding::new(Check::Prices, level, Rec::block(block, part), all_bits)
                .column("Data");
            if !f.refs.iter().any(|x| x.is(&who)) {
                f.refs.push(who.clone());
            }
            out.push(f);
        }
    }
}

// ---- spelling -----------------------------------------------------------------------------

/// The dictionary: SCOWL's words to size 60, every dialect (`words/`, built by
/// `tools/words/build.mjs`).
fn dictionary() -> &'static HashSet<&'static str> {
    static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        let raw = include_bytes!("words/words.deflate");
        let text = miniz_oxide::inflate::decompress_to_vec(raw).unwrap_or_default();
        let text: &'static str =
            Box::leak(String::from_utf8(text).unwrap_or_default().into_boxed_str());
        text.split('\n').filter(|w| !w.is_empty()).collect()
    })
}

/// Whether the dictionary has `w` (lower case).
pub fn known(w: &str) -> bool {
    dictionary().contains(w)
}

/// The text columns players read, by table.
const TEXTS: [(&str, &[&str]); 9] = [
    ("Messages", &["Line 1", "Line 2", "Line 3"]),
    ("Textblocks", &["Data"]),
    (
        "Items",
        &["Name", "Desc1", "Desc2", "Desc3", "Desc4", "Desc5", "Desc6"],
    ),
    (
        "Monsters",
        &["Name", "Desc 1", "Desc 2", "Desc 3", "Desc 4"],
    ),
    (
        "Rooms",
        &[
            "Name", "Desc 0", "Desc 1", "Desc 2", "Desc 3", "Desc 4", "Desc 5", "Desc 6",
        ],
    ),
    ("Spells", &["Name", "Desc 1", "Desc 2"]),
    ("Shops", &["Name", "Desc A", "Desc B", "Desc C"]),
    ("Classes", &["Name"]),
    ("Races", &["Name"]),
];

/// A row's record.
fn rec_of(table: &'static str, t: &Table, row: &[Value]) -> Rec {
    match table {
        "Rooms" => Rec::room(t.n(row, "Map Number"), t.n(row, "Room Number")),
        "Textblocks" => Rec::block(t.n(row, "Number"), t.n(row, "Part #")),
        _ => Rec::of(table, t.n(row, "Number")),
    }
}

/// Text as words: colour codes and `%s`-style blanks out, letters with an apostrophe inside
/// (`ye're`) or in front (`'cept`), a possessive `'s` cut off. Each word as written.
fn words_of(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // colour codes
        if b[i] == 0x1b {
            i += 1;
            if b.get(i) == Some(&b'[') {
                while i < b.len() && !b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                i += 1;
            }
            continue;
        }
        // printf blanks: %s, %d, %-12s
        if b[i] == b'%' {
            i += 1;
            while i < b.len() && (b[i].is_ascii_digit() || b"-+ #.l".contains(&b[i])) {
                i += 1;
            }
            if i < b.len() && b[i].is_ascii_alphabetic() {
                i += 1;
            }
            continue;
        }
        if !b[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        // a letter left out in front (`'cept`) keeps its apostrophe
        let start = if i > 0 && b[i - 1] == b'\'' && (i == 1 || !b[i - 2].is_ascii_alphabetic()) {
            i - 1
        } else {
            i
        };
        while i < b.len()
            && (b[i].is_ascii_alphabetic()
                || b[i] == b'\'' && b.get(i + 1).is_some_and(u8::is_ascii_alphabetic))
        {
            i += 1;
        }
        let mut w = &text[start..i];
        if w.len() > 2 && (w.ends_with("'s") || w.ends_with("'S")) {
            w = &w[..w.len() - 2];
        }
        out.push(w);
    }
    out
}

/// Every word one edit from `w`: a letter dropped, swapped with the next, changed or added.
fn edits(w: &str) -> Vec<String> {
    let b = w.as_bytes();
    let mut out = Vec::new();
    for i in 0..=b.len() {
        if i < b.len() {
            out.push([&b[..i], &b[i + 1..]].concat());
            if i + 1 < b.len() {
                let mut s = b.to_vec();
                s.swap(i, i + 1);
                out.push(s);
            }
            for c in b'a'..=b'z' {
                if c != b[i] {
                    let mut s = b.to_vec();
                    s[i] = c;
                    out.push(s);
                }
            }
        }
        for c in b'a'..=b'z' {
            out.push([&b[..i], &[c], &b[i..]].concat());
        }
    }
    out.into_iter()
        .filter_map(|s| String::from_utf8(s).ok())
        .collect()
}

/// Why a word the dictionary does not have is left alone: dialect (`askin'`), a plural or old
/// form of a known word, two known words joined, or a name.
fn excused(w: &str, always_capital: bool) -> bool {
    if w.contains('\'') || always_capital {
        return true;
    }
    let base = |cut: usize| w.len() > cut && known(&w[..w.len() - cut]);
    if w.ends_with("in") && known(&format!("{w}g")) {
        return true;
    }
    if w.ends_with('s') && base(1) || w.ends_with("es") && base(2) {
        return true;
    }
    if w.ends_with("st") && base(2)
        || w.ends_with("eth") && (base(3) || known(&format!("{}e", &w[..w.len() - 3])))
    {
        return true;
    }
    (3..w.len().saturating_sub(2)).any(|i| known(&w[..i]) && known(&w[i..]))
}

/// The most findings spelling makes.
const MAX_SPELLING: usize = 2000;

fn spelling(world: &World, opts: &Options, out: &mut Vec<Finding>) {
    // each word: in how many different texts, whether always capitalised, and where
    #[derive(Default)]
    struct Seen {
        texts: usize,
        lower: bool,
        at: Vec<(Rec, &'static str)>,
    }
    let mut seen: HashMap<String, Seen> = HashMap::new();
    let mut texts: HashSet<&str> = HashSet::new();
    for (table, cols) in TEXTS {
        let Some(t) = world.table(table) else {
            continue;
        };
        for row in &t.rows.rows {
            for &col in cols {
                if !t.has(col) {
                    continue;
                }
                let s = t.s(row, col);
                if s.is_empty() {
                    continue;
                }
                let fresh = texts.insert(s);
                let mut in_this: HashSet<String> = HashSet::new();
                for w in words_of(s) {
                    let lw = w.to_ascii_lowercase();
                    if lw.len() < 4 || !in_this.insert(lw.clone()) {
                        continue;
                    }
                    let e = seen.entry(lw).or_default();
                    if fresh {
                        e.texts += 1;
                    }
                    e.lower |= w.starts_with(|c: char| c.is_ascii_lowercase());
                    if e.at.len() < 4 {
                        e.at.push((rec_of(table, t, row), col));
                    }
                }
            }
        }
    }
    let mut found: Vec<Finding> = Vec::new();
    let mut words: Vec<(&String, &Seen)> = seen.iter().collect();
    words.sort_by(|a, b| a.0.cmp(b.0));
    for (w, s) in words {
        if s.texts > 2 || known(w) || opts.words.contains(w.as_str()) || excused(w, !s.lower) {
            continue;
        }
        let mut cands: Vec<(usize, String)> = edits(w)
            .into_iter()
            .filter(|c| known(c))
            .filter_map(|c| seen.get(&c).map(|x| (x.texts, c)))
            .collect();
        if cands.is_empty() {
            continue;
        }
        cands.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        cands.dedup_by(|a, b| a.1 == b.1);
        let suggest: Vec<String> = cands.into_iter().take(3).map(|c| c.1).collect();
        for (rec, col) in &s.at {
            let mut bits = vec![tx(format!("“{w}” may be “{}”", suggest[0]))];
            for (i, other) in suggest.iter().enumerate().skip(1) {
                bits.push(tx(format!(
                    "{}“{other}”",
                    if i + 1 == suggest.len() { " or " } else { ", " }
                )));
            }
            bits.push(tx("."));
            let mut f = Finding::new(Check::Spelling, Level::Note, rec.clone(), bits).column(*col);
            f.word = Some(w.clone());
            f.suggest = suggest.clone();
            found.push(f);
        }
        if found.len() >= MAX_SPELLING {
            break;
        }
    }
    out.extend(found);
}

// ---- placeholders -------------------------------------------------------------------------

/// The attack type of a plain blow.
const NORMAL: i64 = 1;
/// The item type of a weapon.
const WEAPON: i64 = 1;

/// Words that mark a name or text as a placeholder.
const PLACEHOLDER_WORDS: [&str; 6] = ["placeholder", "todo", "tbd", "fixme", "lorem", "ipsum"];

/// Why a name reads as a placeholder: a placeholder word, `test room`-style, `new item`, or
/// keys mashed (a word of five letters or more with no vowel).
fn placeholder(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let ws: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if let Some(w) = ws.iter().find(|w| PLACEHOLDER_WORDS.contains(w)) {
        return Some(format!("says “{w}”"));
    }
    for pair in ws.windows(2) {
        let thing = [
            "item", "monster", "room", "spell", "shop", "message", "text",
        ];
        if (pair[0] == "test" || pair[0] == "new" || pair[0] == "dummy") && thing.contains(&pair[1])
        {
            return Some(format!("says “{} {}”", pair[0], pair[1]));
        }
    }
    if let Some(w) = ws.iter().find(|w| {
        w.len() >= 5
            && w.bytes().all(|b| b.is_ascii_alphabetic())
            && !w.bytes().any(|b| b"aeiouy".contains(&b))
    }) {
        return Some(format!("“{w}” reads like keys mashed"));
    }
    None
}

fn placeholders(world: &World, out: &mut Vec<Finding>) {
    // a monster's plain blows told by a blank message
    for (t, row) in world.each("Monsters") {
        let me = Rec::of("Monsters", t.n(row, "Number"));
        let mut top = -1;
        for i in 0..5 {
            let ty = t.n(row, &format!("Attack Type {i}"));
            let pct = t.n(row, &format!("Attack % {i}"));
            // the slot is picked by the cumulative percentages: one under an earlier, larger
            // one never is; the first is the blow when none is picked
            let picked = i == 0 || pct > top;
            top = top.max(pct);
            if ty != NORMAL || !picked {
                continue;
            }
            let mut blank: Vec<(String, &str, i64)> = Vec::new();
            for (what, col) in [("hit", "Hit"), ("dodge", "Dodge"), ("miss", "Miss")] {
                let c = format!("Attack {col} Msg {i}");
                let m = t.n(row, &c);
                if m > 0 && world.blank(m) {
                    blank.push((c, what, m));
                }
            }
            if blank.is_empty() {
                continue;
            }
            let msgs: BTreeSet<i64> = blank.iter().map(|b| b.2).collect();
            let whats: Vec<&str> = blank.iter().map(|b| b.1).collect();
            let what = match whats.len() {
                1 => format!("{} message is", whats[0]),
                2 => format!("{} and {} messages are", whats[0], whats[1]),
                _ => "hit, dodge and miss messages are".into(),
            };
            let mut bits = vec![tx(format!("Attack {}'s {what} ", i + 1))];
            bits.extend(listed(
                &msgs
                    .iter()
                    .map(|&m| Rec::of("Messages", m))
                    .collect::<Vec<_>>(),
            ));
            bits.push(tx(format!(
                ", which {} blank, so players read an empty line for its blows. 0 tells them in the game's own words.",
                if msgs.len() == 1 { "is" } else { "are" }
            )));
            let mut f = Finding::new(Check::Placeholders, Level::Warn, me.clone(), bits)
                .column(blank[0].0.clone());
            f.fix = blank.iter().map(|b| (b.0.clone(), 0)).collect();
            out.push(f);
        }
    }
    // a weapon's verbs from a blank message
    for (t, row) in world.each("Items") {
        if t.n(row, "Type") != WEAPON {
            continue;
        }
        let me = Rec::of("Items", t.n(row, "Number"));
        for (col, what) in [("Hit Msg", "hit"), ("Miss Msg", "miss")] {
            let m = t.n(row, col);
            if m > 0 && world.blank(m) {
                let mut f = Finding::new(
                    Check::Placeholders,
                    Level::Warn,
                    me.clone(),
                    vec![
                        tx(format!("Its {what} verbs are ")),
                        rc(Rec::of("Messages", m)),
                        tx(format!(
                            ", which is blank, so its {what}s read with no verb (“You  the orc”). 0 uses the game's own."
                        )),
                    ],
                )
                .column(col);
                f.fix = vec![(col.to_string(), 0)];
                out.push(f);
            }
        }
    }
    // names and text left as placeholders
    for (table, col) in [
        ("Items", "Name"),
        ("Monsters", "Name"),
        ("Spells", "Name"),
        ("Rooms", "Name"),
        ("Shops", "Name"),
        ("Classes", "Name"),
        ("Races", "Name"),
    ] {
        for (t, row) in world.each(table) {
            if let Some(why) = placeholder(t.s(row, col)) {
                out.push(
                    Finding::new(
                        Check::Placeholders,
                        Level::Note,
                        rec_of(table, t, row),
                        vec![tx(format!("Its name {why}: a placeholder left in?"))],
                    )
                    .column(col),
                );
            }
        }
    }
    for (table, cols) in [
        ("Messages", &["Line 1", "Line 2", "Line 3"][..]),
        ("Textblocks", &["Data"][..]),
    ] {
        for (t, row) in world.each(table) {
            for &col in cols {
                let lower = t.s(row, col).to_ascii_lowercase();
                let hit = lower
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .find(|w| PLACEHOLDER_WORDS.contains(w));
                if let Some(w) = hit {
                    out.push(
                        Finding::new(
                            Check::Placeholders,
                            Level::Note,
                            rec_of(table, t, row),
                            vec![tx(format!("{col} says “{w}”: a placeholder left in?"))],
                        )
                        .column(col),
                    );
                    break;
                }
            }
        }
    }
}

// ---- spawns -------------------------------------------------------------------------------

/// The ability that summons the monster its value names.
const SUMMON: i64 = 12;
/// An item's ability that casts the spell its value names.
const CASTS_SPELL: i64 = 43;
/// A spell's ability that casts another after it.
const END_CAST: i64 = 151;
/// The attack type that casts a spell.
const SPELL_ATTACK: i64 = 2;

/// What casts each spell, as records with how.
fn casters(world: &World) -> HashMap<i64, Vec<(Rec, String)>> {
    let mut by: HashMap<i64, Vec<(Rec, String)>> = HashMap::new();
    let mut add = |s: i64, rec: Rec, how: String| {
        if s > 0 {
            by.entry(s).or_default().push((rec, how));
        }
    };
    for (t, row) in world.each("Monsters") {
        let me = Rec::of("Monsters", t.n(row, "Number"));
        add(
            t.n(row, "Create Spell"),
            me.clone(),
            "'s Create Spell".into(),
        );
        add(t.n(row, "Death Spell"), me.clone(), "'s Death Spell".into());
        for i in 0..5 {
            add(
                t.n(row, &format!("Spell Number {i}")),
                me.clone(),
                format!("'s spell {}", i + 1),
            );
            add(
                t.n(row, &format!("Attack Hit Spell {i}")),
                me.clone(),
                format!("'s attack {} on a hit", i + 1),
            );
            if t.n(row, &format!("Attack Type {i}")) == SPELL_ATTACK {
                add(
                    t.n(row, &format!("Attack Accu/Spell {i}")),
                    me.clone(),
                    format!("'s attack {}", i + 1),
                );
            }
        }
    }
    for (t, row) in world.each("Items") {
        let me = Rec::of("Items", t.n(row, "Number"));
        for i in 0..20 {
            if t.n(row, &format!("Ability {i}")) == CASTS_SPELL {
                add(
                    t.n(row, &format!("Ability Value {i}")),
                    me.clone(),
                    " when used".into(),
                );
            }
        }
    }
    for (t, row) in world.each("Rooms") {
        add(
            t.n(row, "Spell"),
            rec_of("Rooms", t, row),
            "'s room spell".into(),
        );
    }
    for (t, row) in world.each("Spells") {
        let me = Rec::of("Spells", t.n(row, "Number"));
        for i in 0..10 {
            if t.n(row, &format!("Ability {i}")) == END_CAST {
                add(
                    t.n(row, &format!("Ability Value {i}")),
                    me.clone(),
                    " as it ends".into(),
                );
            }
        }
    }
    for (rec, c) in script_clauses(world) {
        if c.kw == Some("cast") {
            add(i64::from(c.get("spell")), rec, " in a script".into());
        }
    }
    by
}

/// Each clause of the world's scripts, with its block: blocks read as text are left out, so
/// words in a story (`Summon the guards!`) are not taken for a keyword.
fn script_clauses(world: &World) -> impl Iterator<Item = (Rec, Clause)> + '_ {
    world.each("Textblocks").flat_map(|(t, row)| {
        let data = t.s(row, "Data");
        let rec = rec_of("Textblocks", t, row);
        let lines: Vec<&str> = match script::reading_of(data) {
            Reading::Text => Vec::new(),
            _ => data.split('\n').collect(),
        };
        lines
            .into_iter()
            .flat_map(script::clauses)
            .map(move |c| (rec.clone(), c))
    })
}

/// Spawn ranges in words, overlapping ones as one: `indexes 1 to 5`, `index 3`, `indexes up
/// to 4 and 9 to 12`.
fn ranges(list: impl Iterator<Item = (i64, i64)>) -> String {
    let mut rs: Vec<(i64, i64)> = list
        .map(|(lo, hi)| (if lo == 0 { i64::MIN } else { lo }, hi))
        .collect();
    rs.sort();
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (lo, hi) in rs {
        match merged.last_mut() {
            Some(last) if lo <= last.1.saturating_add(1) => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    let one = |&(lo, hi): &(i64, i64)| match (lo, hi) {
        (i64::MIN, hi) => format!("up to {hi}"),
        (lo, hi) if lo == hi => format!("{lo}"),
        (lo, hi) => format!("{lo} to {hi}"),
    };
    let single = merged.len() == 1 && merged[0].0 == merged[0].1;
    if merged.len() > 4 {
        let (lo, hi) = (merged[0].0, merged[merged.len() - 1].1);
        return format!("indexes {}, in {} ranges", one(&(lo, hi)), merged.len());
    }
    let words: Vec<String> = merged.iter().map(one).collect();
    format!(
        "{} {}",
        if single { "index" } else { "indexes" },
        and_list(&words)
    )
}

fn spawns(world: &World, out: &mut Vec<Finding>) {
    let Some(rooms) = world.table("Rooms") else {
        return;
    };
    let Some(mons) = world.table("Monsters") else {
        return;
    };
    // the group's monsters' indexes
    let mut indexes: HashMap<i64, BTreeSet<i64>> = HashMap::new();
    for row in &mons.rows.rows {
        if !mons.s(row, "Name").trim().is_empty() {
            indexes
                .entry(mons.n(row, "Group"))
                .or_default()
                .insert(mons.n(row, "Index"));
        }
    }
    // the rooms that spawn, by group
    let mut spawning: HashMap<i64, Vec<(Spawning, Rec)>> = HashMap::new();
    for row in &rooms.rows.rows {
        let Some(s) = Spawning::of(&rooms.rows, row) else {
            continue;
        };
        if !s.spawns() {
            continue;
        }
        let room = rec_of("Rooms", rooms, row);
        spawning.entry(s.group).or_default().push((s, room.clone()));
        // a room whose range no monster of the group is in
        let group = s.group;
        let have = indexes.get(&group);
        if have.is_some_and(|ix| ix.iter().any(|&i| s.takes(i))) {
            continue;
        }
        let has = match have {
            None => format!("no monster is in group {group}"),
            Some(ix) => {
                let below = (s.min > 0).then(|| ix.range(..s.min).next_back()).flatten();
                let above = ix.range(s.max + 1..).next();
                let near: Vec<String> = below
                    .into_iter()
                    .chain(above)
                    .map(|i| i.to_string())
                    .collect();
                match near.len() {
                    0 => format!("group {group} has no monster there"),
                    1 => format!("group {group} has none there (its nearest is {})", near[0]),
                    _ => format!(
                        "group {group} has none there (its nearest are {})",
                        and_list(&near)
                    ),
                }
            }
        };
        out.push(
            Finding::new(
                Check::Spawns,
                Level::Warn,
                room,
                vec![tx(format!(
                    "Spawns from group {group} at {}, but {has}, so it spawns nothing.",
                    ranges([(s.min, s.max)].into_iter())
                ))],
            )
            .column("Mon Type"),
        );
    }
    // monsters no room spawns, and what brings them instead
    let mut keeps: HashMap<i64, Vec<Rec>> = HashMap::new();
    for row in &rooms.rows.rows {
        let npc = rooms.n(row, "Perm NPC");
        if npc > 0 {
            keeps
                .entry(npc)
                .or_default()
                .push(rec_of("Rooms", rooms, row));
        }
    }
    let mut summons: HashMap<i64, Vec<i64>> = HashMap::new();
    for (t, row) in world.each("Spells") {
        for i in 0..10 {
            if t.n(row, &format!("Ability {i}")) == SUMMON {
                let m = t.n(row, &format!("Ability Value {i}"));
                let s = t.n(row, "Number");
                let e = summons.entry(m).or_default();
                if !e.contains(&s) {
                    e.push(s);
                }
            }
        }
    }
    let mut scripted: HashMap<i64, Vec<Rec>> = HashMap::new();
    for (rec, c) in script_clauses(world) {
        let m = i64::from(c.get("monster"));
        if c.kw == Some("summon") && m > 0 {
            let e = scripted.entry(m).or_default();
            if !e.contains(&rec) {
                e.push(rec);
            }
        }
    }
    let cast_by = casters(world);
    for row in &mons.rows.rows {
        if mons.s(row, "Name").trim().is_empty() {
            continue;
        }
        let n = mons.n(row, "Number");
        let (group, index) = (mons.n(row, "Group"), mons.n(row, "Index"));
        let list = spawning.get(&group);
        if list.is_some_and(|l| l.iter().any(|(s, _)| s.takes(index))) {
            continue;
        }
        let me = Rec::of("Monsters", n);
        let why = match list {
            None => format!("No room spawns from group {group}"),
            Some(l) => format!(
                "No room spawns it: group {group}'s rooms spawn {}, and its index is {index}",
                ranges(l.iter().map(|(s, _)| (s.min, s.max)))
            ),
        };
        let mut bits = vec![tx(format!("{why}. "))];
        let mut level = Level::Ok;
        let mut ways = 0;
        if let Some(k) = keeps.get(&n) {
            ways += 1;
            bits.extend(listed_some(k, 3));
            bits.push(tx(if k.len() == 1 {
                " keeps it as its NPC"
            } else {
                " keep it as their NPC"
            }));
        }
        if let Some(sp) = summons.get(&n) {
            for &s in sp.iter().take(2) {
                if ways > 0 {
                    bits.push(tx("; "));
                }
                ways += 1;
                bits.push(rc(Rec::of("Spells", s)));
                match cast_by.get(&s).filter(|c| !c.is_empty()) {
                    Some(c) => {
                        bits.push(tx(" summons it, cast by "));
                        let (who, how) = &c[0];
                        bits.push(rc(who.clone()));
                        bits.push(tx(how.clone()));
                        if c.len() > 1 {
                            bits.push(tx(format!(", and by {} more", c.len() - 1)));
                        }
                    }
                    None => {
                        bits.push(tx(" summons it, but nothing casts that spell"));
                        if level == Level::Ok && ways == 1 {
                            level = Level::Note;
                        }
                    }
                }
            }
        }
        if let Some(sc) = scripted.get(&n) {
            if ways > 0 {
                bits.push(tx("; "));
            }
            ways += 1;
            bits.extend(listed_some(sc, 3));
            bits.push(tx(if sc.len() == 1 {
                " summons it in a script"
            } else {
                " summon it in scripts"
            }));
            level = Level::Ok;
        }
        if keeps.contains_key(&n) {
            level = Level::Ok;
        }
        if ways == 0 {
            level = Level::Note;
            bits.push(tx(
                "No room keeps it, and nothing summons it, so players never meet it",
            ));
        }
        bits.push(tx("."));
        out.push(Finding::new(Check::Spawns, level, me, bits).column("Group"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(name: &str, cols: &[&str], rows: Vec<Vec<Value>>) -> NmrRows {
        NmrRows {
            name: name.into(),
            columns: cols.iter().map(|c| c.to_string()).collect(),
            rows,
        }
    }
    fn n(v: i32) -> Value {
        Value::Long(v)
    }
    fn s(v: &str) -> Value {
        Value::Text(v.into())
    }

    fn merchant() -> World {
        let blocks = table(
            "Textblocks",
            &["Number", "Part #", "Link To", "Data"],
            vec![
                vec![n(907), n(0), n(908), s("wares:909\naxes:910\nhide:920\n")],
                vec![n(908), n(0), n(0), s("\x1b[0mGreetings, traveler.")],
                vec![
                    n(909),
                    n(0),
                    n(0),
                    s("\x1b[0mThe axes are 2,000 gold each, the mail a thousand gold."),
                ],
                vec![n(910), n(0), n(911), s("\x1b[0mHe holds his hand out.")],
                vec![
                    n(911),
                    n(0),
                    n(0),
                    s("price 200000 1458:giveitem 868:message 1459\n"),
                ],
                vec![n(920), n(0), n(921), s("\x1b[0mHe eyes the hide.")],
                vec![
                    n(921),
                    n(0),
                    n(0),
                    s("checkitem 920 1710:price 20000 1711:giveitem 967\n"),
                ],
                vec![
                    n(930),
                    n(0),
                    n(0),
                    s(
                        "play slots:price 100:random 931\npull lever:price 100:random 931\nplay crowns:price 1000:random 931\n",
                    ),
                ],
                vec![n(931), n(0), n(0), s("50:message 1713\n100:message 1458\n")],
            ],
        );
        let messages = table(
            "Messages",
            &["Number", "Line 1", "Line 2", "Line 3"],
            vec![
                vec![n(1), s(""), s(""), s("")],
                vec![n(1458), s("You scrounge for cash."), s(""), s("")],
                vec![
                    n(1459),
                    s("He hands you the axe in exhange for your money."),
                    s(""),
                    s(""),
                ],
                vec![n(1710), s("You show him nothing."), s(""), s("")],
                vec![
                    n(1711),
                    s("It will cost you 2000 gold to work this hide."),
                    s(""),
                    s(""),
                ],
                vec![
                    n(1712),
                    s("Your money is safe in exchange for nothing."),
                    s(""),
                    s(""),
                ],
                vec![n(1713), s("You win 5 gold!"), s(""), s("")],
            ],
        );
        let items = table(
            "Items",
            &[
                "Number",
                "Name",
                "Type",
                "Cost",
                "Cost Type",
                "Hit Msg",
                "Miss Msg",
            ],
            vec![
                vec![n(868), s("dwarven axe"), n(1), n(1000), n(2), n(1), n(0)],
                vec![
                    n(967),
                    s("golden hide tunic"),
                    n(0),
                    n(2000),
                    n(2),
                    n(0),
                    n(0),
                ],
            ],
        );
        let mut mon_cols = vec![
            "Number",
            "Name",
            "Group",
            "Index",
            "Greet Txt",
            "Create Spell",
        ];
        let attack: Vec<String> = (0..5)
            .flat_map(|i| {
                ["Type", "%", "Hit Msg", "Dodge Msg", "Miss Msg"].map(|c| format!("Attack {c} {i}"))
            })
            .collect();
        mon_cols.extend(attack.iter().map(String::as_str));
        let mon = |num: i32,
                   name: &str,
                   group: i32,
                   index: i32,
                   greet: i32,
                   create: i32,
                   slot0: [i32; 5]| {
            let mut v = vec![n(num), s(name), n(group), n(index), n(greet), n(create)];
            for i in 0..5 {
                for x in slot0 {
                    v.push(n(if i == 0 { x } else { 0 }));
                }
            }
            v
        };
        let monsters = table(
            "Monsters",
            &mon_cols,
            vec![
                mon(
                    381,
                    "dwarven merchant",
                    24,
                    3,
                    907,
                    483,
                    [1, 100, 1459, 1459, 1459],
                ),
                mon(382, "bodyguard", 24, 100, 0, 0, [1, 100, 1, 0, 1]),
                mon(383, "lost dinosaur", 31, 0, 0, 0, [0, 0, 0, 0, 0]),
                mon(812, "sdfsdfsfs", 24, 3, 0, 0, [0, 0, 0, 0, 0]),
            ],
        );
        let spells = table(
            "Spells",
            &["Number", "Name", "Ability 0", "Ability Value 0"],
            vec![vec![n(483), s("summon bodyguard"), n(12), n(382)]],
        );
        let rooms = table(
            "Rooms",
            &[
                "Map Number",
                "Room Number",
                "Name",
                "Mon Type",
                "Min Index",
                "Max Index",
                "Max Regen",
                "Type",
                "Perm NPC",
                "Command Text",
                "Spell",
            ],
            vec![
                vec![
                    n(1),
                    n(10),
                    s("Mountain Pass"),
                    n(24),
                    n(1),
                    n(5),
                    n(3),
                    n(0),
                    n(0),
                    n(0),
                    n(0),
                ],
                vec![
                    n(1),
                    n(11),
                    s("Dungeon Tunnel"),
                    n(6),
                    n(10),
                    n(10),
                    n(2),
                    n(0),
                    n(0),
                    n(0),
                    n(0),
                ],
                vec![
                    n(1),
                    n(12),
                    s("Casino"),
                    n(0),
                    n(0),
                    n(0),
                    n(0),
                    n(0),
                    n(0),
                    n(930),
                    n(0),
                ],
                vec![
                    n(1),
                    n(13),
                    s("Mine Shaft"),
                    n(24),
                    n(50),
                    n(60),
                    n(2),
                    n(0),
                    n(0),
                    n(0),
                    n(0),
                ],
            ],
        );
        World::from_rows(vec![blocks, messages, items, monsters, spells, rooms])
    }

    fn of(check: Check, all: &[Finding]) -> Vec<(Level, String)> {
        all.iter()
            .filter(|f| f.check == check)
            .map(|f| (f.level, f.text(|_| None)))
            .collect()
    }

    #[test]
    fn quotes_in_figures_and_words() {
        let q = |s: &str| {
            quotes(s)
                .iter()
                .map(|q| (q.copper, q.coin))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            q("2,000 gold and 8000 GOLD"),
            vec![(200_000, "gold"), (800_000, "gold")]
        );
        assert_eq!(
            q("me price be a thousand gold marks"),
            vec![(100_000, "gold")]
        );
        assert_eq!(
            q("two thousand five hundred silver"),
            vec![(25_000, "silver")]
        );
        assert_eq!(q("120 platinum"), vec![(1_200_000, "platinum")]);
        assert_eq!(q("gold, and a gold ring"), vec![]);
        assert_eq!(q("\x1b[1;33m50\x1b[0m copper"), vec![(50, "copper")]);
    }

    #[test]
    fn prices_against_what_is_said() {
        let w = merchant();
        let all = check(&w, &Options::default(), &[Check::Prices]);
        let got = of(Check::Prices, &all);
        assert_eq!(
            got,
            vec![
                (
                    Level::Ok,
                    "Charges 2,000 gold, as text block 909 quotes. That is twice the 1,000 gold item 868 costs.".into()
                ),
                (
                    Level::Warn,
                    "Charges 200 gold, but its own words quote 2,000 gold: message 1711. That is a tenth of the 2,000 gold item 967 costs.".into()
                ),
                (
                    Level::Note,
                    "Charges 1 gold (lines 1 and 2), but the commands of room 1/12 never quote that price; they quote 5 gold.".into()
                ),
                (
                    Level::Warn,
                    "Charges 10 gold (line 3), but lines 1 and 2 charge 1 gold to do the same.".into()
                ),
            ],
            "{got:#?}"
        );
        // the conversation's monster is among what it rests on
        assert!(all[1].concerns(&Rec::of("Monsters", 381)));
        assert!(all[1].concerns(&Rec::of("Messages", 1711)));
        assert_eq!(all[1].about, Rec::block(921, 0));
    }

    #[test]
    fn spelling_finds_a_rare_word_near_a_known_one() {
        let w = merchant();
        let all = check(&w, &Options::default(), &[Check::Spelling]);
        let words: Vec<_> = all
            .iter()
            .map(|f| (f.word.clone().unwrap(), f.suggest.clone(), f.about.clone()))
            .collect();
        assert_eq!(
            words,
            vec![(
                "exhange".into(),
                vec!["exchange".into()],
                Rec::of("Messages", 1459)
            )]
        );
        // an accepted word is left alone
        let opts = Options {
            words: HashSet::from(["exhange".to_string()]),
        };
        assert!(check(&w, &opts, &[Check::Spelling]).is_empty());
    }

    #[test]
    fn spawn_ranges_in_words() {
        let r = |v: &[(i64, i64)]| ranges(v.iter().copied());
        assert_eq!(r(&[(3, 3)]), "index 3");
        assert_eq!(r(&[(0, 4), (9, 12), (2, 6)]), "indexes up to 6 and 9 to 12");
        assert_eq!(r(&[(1, 5), (6, 8)]), "indexes 1 to 8");
        assert_eq!(
            r(&[(1, 1), (3, 3), (5, 5), (7, 7), (9, 9)]),
            "indexes 1 to 9, in 5 ranges"
        );
    }

    #[test]
    fn words_leave_codes_and_blanks_out() {
        assert_eq!(
            words_of("The %s %sspins at %-12s, \x1b[1;31mye're\x1b[0m Gorthuk's 'cept"),
            vec!["The", "spins", "at", "ye're", "Gorthuk", "'cept"]
        );
        assert!(excused("askin", false));
        assert!(excused("batlike", false));
        assert!(excused("elementals", false));
        assert!(excused("gorthuk", true));
        assert!(!excused("exhange", false));
    }

    #[test]
    fn placeholders_blank_blows_and_mashed_names() {
        let w = merchant();
        let all = check(&w, &Options::default(), &[Check::Placeholders]);
        let got = of(Check::Placeholders, &all);
        assert_eq!(
            got,
            vec![
                (
                    Level::Warn,
                    "Attack 1's hit and miss messages are message 1, which is blank, so players read an empty line for its blows. 0 tells them in the game's own words.".into()
                ),
                (
                    Level::Warn,
                    "Its hit verbs are message 1, which is blank, so its hits read with no verb (“You  the orc”). 0 uses the game's own.".into()
                ),
                (Level::Note, "Its name “sdfsdfsfs” reads like keys mashed: a placeholder left in?".into()),
            ],
            "{got:#?}"
        );
        assert_eq!(
            all[0].fix,
            vec![
                ("Attack Hit Msg 0".into(), 0),
                ("Attack Miss Msg 0".into(), 0)
            ]
        );
    }

    #[test]
    fn spawns_nobody_reaches_and_what_does() {
        let w = merchant();
        let all = check(&w, &Options::default(), &[Check::Spawns]);
        let got = of(Check::Spawns, &all);
        assert_eq!(
            got,
            vec![
                (
                    Level::Warn,
                    "Spawns from group 6 at index 10, but no monster is in group 6, so it spawns nothing.".into()
                ),
                (
                    Level::Warn,
                    "Spawns from group 24 at indexes 50 to 60, but group 24 has none there (its nearest are 3 and 100), so it spawns nothing.".into()
                ),
                (
                    Level::Ok,
                    "No room spawns it: group 24's rooms spawn indexes 1 to 5 and 50 to 60, and its index is 100. Spell 483 summons it, cast by monster 381's Create Spell.".into()
                ),
                (
                    Level::Note,
                    "No room spawns from group 31. No room keeps it, and nothing summons it, so players never meet it.".into()
                ),
            ],
            "{got:#?}"
        );
    }
}
