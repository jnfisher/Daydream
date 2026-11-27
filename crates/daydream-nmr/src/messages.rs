//! Messages as the game reads them. A message is three lines, and each use of one reads some
//! of them, for someone, with their blanks (`%s`, `%d`) filled in order by what the game
//! passes. [`reads`] says which for each column that names a message, as silvermere-engine
//! reads them (see UPSTREAM.md): `combat/messages.rs` (a monster's blows),
//! `monsters/generate.rs` (its arrival), `combat/kill_monster.rs` (its death), `combat/mod.rs`
//! (a weapon's verbs), `items/inventory.rs` and `cycles/medium.rs` (an item that breaks or
//! burns out), `spells/messages.rs` (a spell that takes hold) and `scripts/actions.rs` (a
//! script's message).
//!
//! [`fill`] is the game's formatter, [`Names`] finds the names written into a line, and
//! [`swap_name`] writes another name in their place (a monster cloned with its messages).

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use daydream_store::Realm;

use crate::NmrRows;
use crate::refs::{self, Tables};
use crate::rows::{self, RowError};
use crate::schema::{NmrTable, Value};
use crate::{script, wiring};

/// One line a use reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    /// Which line, 1 to 3.
    pub line: usize,
    /// When the game shows it (`it dies`, `attack 2 hits`).
    pub when: String,
    /// Who reads it (`the room`, `the player`).
    pub to: &'static str,
    /// What fills its blanks, in order; none for a line shown as written.
    pub fills: Vec<&'static str>,
}

/// What fills a blank with the name of the record using the message.
pub const ITS_NAME: &str = "its name";
/// What fills a blank of an arrival line with where the monster came from.
pub const CAME_FROM: &str = "where it came from (“the north”)";
const MISS_VERB: &str = "its weapon's miss verb";
const WEAPON: &str = "its weapon's name";
const PLAYER: &str = "the player's name";
/// What the game fills a monster's attack line's spare blanks with: two stars after the rest.
pub const STAR: &str = "a star (*)";

fn read(line: usize, when: impl Into<String>, to: &'static str, fills: &[&'static str]) -> Read {
    Read {
        line,
        when: when.into(),
        to,
        fills: fills.to_vec(),
    }
}

/// A column's name without the slot number at its end (`Attack Hit Msg 2` is
/// `Attack Hit Msg`), and the slot counted from 1.
fn family(column: &str) -> (&str, Option<usize>) {
    let f = column
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .trim_end();
    let slot = column[f.len()..]
        .trim()
        .parse::<usize>()
        .ok()
        .map(|i| i + 1);
    (f, slot)
}

/// The lines a use of a message reads: the use in `column` of a `table` row, `get` giving the
/// row's numbers that decide it (a monster's Desc Txt, a spell's MSG Style). `None` for a
/// column whose lines silvermere does not read one by one (an exit's, a spell's Resist
/// Message) or that names no message.
pub fn reads(table: &str, column: &str, get: &dyn Fn(&str) -> i64) -> Option<Vec<Read>> {
    let (f, slot) = family(column);
    let attack = |s: &str| {
        s.replace(
            "{a}",
            &slot.map_or("an attack".to_string(), |n| format!("attack {n}")),
        )
    };
    // an attack line's blanks past the rest take a star each, two of them
    let starred = |v: Vec<Read>| {
        v.into_iter()
            .map(|mut r| {
                r.fills.extend([STAR, STAR]);
                r
            })
            .collect()
    };
    Some(match (table, f) {
        ("Monsters", "Attack Hit Msg") => starred(vec![
            read(
                1,
                attack("{a} hits"),
                "the player",
                &[ITS_NAME, "the damage"],
            ),
            read(
                2,
                attack("{a} hits"),
                "the room",
                &[ITS_NAME, PLAYER, "the damage"],
            ),
        ]),
        ("Monsters", "Attack Dodge Msg") => starred(vec![
            read(
                1,
                attack("armour takes {a}"),
                "the player",
                &[ITS_NAME, MISS_VERB],
            ),
            read(
                2,
                attack("armour takes {a}"),
                "the room",
                &[ITS_NAME, MISS_VERB, PLAYER, "his or her"],
            ),
            read(
                3,
                attack("the player dodges {a} (when it has a miss message too)"),
                "the player",
                &[ITS_NAME, MISS_VERB, WEAPON],
            ),
        ]),
        ("Monsters", "Attack Miss Msg") => starred(vec![
            read(
                1,
                attack("the player dodges {a} (when it has a dodge message too)"),
                "the room",
                &[ITS_NAME, MISS_VERB, PLAYER, WEAPON, "he or she"],
            ),
            read(
                2,
                attack("{a} misses"),
                "the player",
                &[ITS_NAME, MISS_VERB, WEAPON],
            ),
            read(
                3,
                attack("{a} misses"),
                "the room",
                &[ITS_NAME, MISS_VERB, PLAYER, WEAPON],
            ),
        ]),
        // with a name list the name goes first, else where it came from
        ("Monsters", "Move Msg") => vec![if get("Desc Txt") != 0 {
            read(1, "it appears", "the room", &[ITS_NAME, CAME_FROM])
        } else {
            read(1, "it appears", "the room", &[CAME_FROM, ITS_NAME])
        }],
        ("Monsters", "Death Msg") => vec![read(3, "it dies", "the room", &[])],
        // a weapon's verbs: a word each, `|` between alternatives picked together
        ("Items", "Hit Msg") => vec![
            read(
                1,
                "its wielder hits",
                "the wielder, as the verb (“slash”)",
                &[],
            ),
            read(2, "its wielder hits", "the one hit (“slashes”)", &[]),
            read(3, "its wielder hits", "the room (“slashes”)", &[]),
        ],
        ("Items", "Miss Msg") => vec![
            read(
                1,
                "its wielder misses",
                "the wielder, as the verb (“swing”)",
                &[],
            ),
            read(2, "its wielder misses", "the one missed (“swings”)", &[]),
            read(3, "its wielder misses", "the room (“swings”)", &[]),
        ],
        ("Items", "Distruct Msg") => vec![
            read(1, "it breaks or burns out", "the player", &[ITS_NAME]),
            read(2, "it breaks or burns out", "the room", &[PLAYER, ITS_NAME]),
        ],
        // a spell that takes hold; MSG Style 1 leaves the names out
        ("Spells", "Cast MSG B") => {
            let when = "it takes hold";
            if get("MSG Style") != 0 {
                vec![
                    read(1, when, "the caster", &["the target", "the amount"]),
                    read(2, when, "the target", &["the amount"]),
                    read(3, when, "the room", &["the target", "the amount"]),
                ]
            } else {
                vec![
                    read(
                        1,
                        when,
                        "the caster",
                        &[ITS_NAME, "the target", "the amount"],
                    ),
                    read(
                        2,
                        when,
                        "the target",
                        &["the caster", ITS_NAME, "the amount"],
                    ),
                    read(
                        3,
                        when,
                        "the room",
                        &["the caster", ITS_NAME, "the target", "the amount"],
                    ),
                ]
            }
        }
        _ => return None,
    })
}

/// The lines a script's clause reads from the message it names (`message 5`, a check's
/// message when it fails): the first to the player as written, the second to the room with
/// the player's name.
pub fn script_reads(keyword: &str) -> Vec<Read> {
    let when = format!("its {keyword} clause runs");
    vec![
        read(1, when.clone(), "the player", &[]),
        read(2, when, "the room", &[PLAYER]),
    ]
}

/// Each blank of a line, where it is: a `%s`, `%d`, `%u` or `%i`, with any `l` or `h` before
/// the letter, as [`fill`] reads them.
pub fn blanks(line: &str) -> Vec<Range<usize>> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < b.len() && (b[i] == b'l' || b[i] == b'h') {
            i += 1;
        }
        match b.get(i) {
            Some(b'%') => i += 1,
            Some(b's' | b'd' | b'u' | b'i') => {
                i += 1;
                out.push(start..i);
            }
            _ => {}
        }
    }
    out
}

/// A line with its blanks filled as the game's formatter fills them: each blank takes the
/// next of `args`, `%%` is a percent sign, a blank past the last argument prints nothing,
/// before any other character the `%` is dropped, and a `%` that ends the line is printed.
pub fn fill(line: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(line.len() + 32);
    let mut next = 0;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut spec = String::new();
        while let Some(&p) = chars.peek() {
            if p == 'l' || p == 'h' {
                spec.push(p);
                chars.next();
            } else {
                break;
            }
        }
        match chars.peek().copied() {
            Some('%') => {
                chars.next();
                out.push('%');
            }
            Some('s' | 'd' | 'u' | 'i') => {
                chars.next();
                if let Some(a) = args.get(next) {
                    out.push_str(a);
                    next += 1;
                }
            }
            None if spec.is_empty() => out.push('%'),
            _ => out.push_str(&spec),
        }
    }
    out
}

/// One use of a message: the row that names it, and the lines it reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub reference: refs::Reference,
    /// What it reads; `None` when silvermere does not read its lines one by one (an exit's
    /// message, a spell's Resist Message).
    pub reads: Option<Vec<Read>>,
}

/// Every use of message `number` among the tables `search` ([`refs::pointing_at`]), with what
/// each reads, and whether there were more than [`refs::MAX`].
pub fn uses(
    tables: &mut Tables,
    search: &[&'static NmrTable],
    number: i64,
) -> Result<(Vec<Use>, bool), RowError> {
    let target = rows::record_table("Messages")?;
    let key =
        [Value::Long(i32::try_from(number).map_err(|_| {
            RowError::Bad(format!("no message {number}"))
        })?)];
    let found = refs::pointing_at(tables, search, target, &key)?;
    let mut out = Vec::with_capacity(found.list.len());
    for r in found.list {
        let reads = match (r.table, r.column.as_deref()) {
            // a script's clause: `line 2: message 1711`
            ("Textblocks", Some("Data")) => {
                let clause = r.by.split_once(": ").map_or("", |(_, c)| c);
                script::keyword_of(clause).map(script_reads)
            }
            (table, Some(column)) => {
                let t = rows::record_table(table)?;
                let all = tables.rows(t)?;
                let row = rows::find_row(all, &rows::index_columns(t), &r.key);
                let get = |c: &str| {
                    row.and_then(|row| all.position(c).and_then(|i| refs::num(&row[i])))
                        .unwrap_or(0)
                };
                reads(table, column, &get)
            }
            _ => None,
        };
        out.push(Use {
            reference: r,
            reads,
        });
    }
    Ok((out, found.more))
}

// ---- names in lines ------------------------------------------------------------------------

/// A word of a text: where it is, in lower case.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Word {
    at: Range<usize>,
    lower: String,
}

/// The words of a text: letters and digits, with `'` or `-` inside a word (`half-ogre`),
/// colour codes left out.
fn words(text: &str) -> Vec<Word> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let part = |c: u8| c.is_ascii_alphanumeric() || c == b'\'' || c == b'-';
    while i < b.len() {
        if b[i] == 0x1b {
            // a colour code: ESC [ ... letter
            i += 1;
            if b.get(i) == Some(&b'[') {
                i += 1;
                while i < b.len() && !b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                i += 1;
            }
            continue;
        }
        if !part(b[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && part(b[i]) {
            i += 1;
        }
        let (mut s, mut e) = (start, i);
        while s < e && !b[s].is_ascii_alphanumeric() {
            s += 1;
        }
        while e > s && !b[e - 1].is_ascii_alphanumeric() {
            e -= 1;
        }
        if s < e {
            out.push(Word {
                at: s..e,
                lower: text[s..e].to_ascii_lowercase(),
            });
        }
    }
    out
}

/// A name's words, in lower case.
pub fn name_words(name: &str) -> Vec<String> {
    words(name).into_iter().map(|w| w.lower).collect()
}

/// Whether `b` is `a` as a name may be written: the same word, its plural (`wolf`,
/// `wolves`), or a word with the same stem (`dwarf`, `dwarven`).
pub fn alike(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let plural = |x: &str, y: &str| {
        y.strip_prefix(x).is_some_and(|r| r == "s" || r == "es")
            || x.strip_suffix('f').is_some_and(|s| y == format!("{s}ves"))
            || x.strip_suffix('y').is_some_and(|s| y == format!("{s}ies"))
            || x.strip_suffix("man")
                .is_some_and(|s| y == format!("{s}men"))
    };
    if plural(a, b) || plural(b, a) {
        return true;
    }
    let common = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    common >= 4 && common + 1 >= a.len().min(b.len())
}

/// Whether every word of the name `part` is a word of the name `whole` (`dwarf` of
/// `dwarven miner`, `troll` of `cave troll`).
pub fn within(part: &[String], whole: &[String]) -> bool {
    !part.is_empty() && part.iter().all(|p| whole.iter().any(|w| alike(p, w)))
}

/// A name found in a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found<'a, T> {
    /// Where it is written.
    pub at: Range<usize>,
    /// The name's words, as listed.
    pub name: &'a [String],
    /// What carries the name.
    pub of: &'a [T],
}

/// A name's words, and what carries it.
type Carried<T> = (Vec<String>, Vec<T>);

/// Names to look for in texts: whole words in any case, the longest at each place, the last
/// word in its plural too.
pub struct Names<T> {
    /// By a name's first word: each name's words and what carries it, longest first.
    first: HashMap<String, Vec<Carried<T>>>,
}

impl<T> Names<T> {
    /// Names and what carries each (a name many records share is one name).
    pub fn new(list: impl IntoIterator<Item = (String, T)>) -> Names<T> {
        let mut by: HashMap<Vec<String>, Vec<T>> = HashMap::new();
        for (name, of) in list {
            let ws = name_words(&name);
            if !ws.is_empty() {
                by.entry(ws).or_default().push(of);
            }
        }
        let mut first: HashMap<String, Vec<Carried<T>>> = HashMap::new();
        for (ws, of) in by {
            first.entry(ws[0].clone()).or_default().push((ws, of));
        }
        for v in first.values_mut() {
            v.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
        }
        Names { first }
    }

    /// Each name written in `text`, in order: at each word the longest name that starts
    /// there, its words with only spaces between them.
    pub fn find<'a>(&'a self, text: &str) -> Vec<Found<'a, T>> {
        let ws = words(text);
        let mut out = Vec::new();
        let mut i = 0;
        while i < ws.len() {
            // the names starting with this word, then the one-word names it is the plural of
            let w = ws[i].lower.as_str();
            let plural_of = [w.strip_suffix("es"), w.strip_suffix('s')]
                .into_iter()
                .flatten()
                .filter_map(|stem| self.first.get(stem))
                .flatten()
                .filter(|(name, _)| name.len() == 1);
            let mut cands = self.first.get(w).into_iter().flatten().chain(plural_of);
            let hit = cands.find(|(name, _)| {
                name.len() <= ws.len() - i
                    && name.iter().enumerate().all(|(k, n)| {
                        let w = &ws[i + k];
                        let last = k + 1 == name.len();
                        let spaced = k == 0
                            || text[ws[i + k - 1].at.end..w.at.start]
                                .bytes()
                                .all(|c| c == b' ');
                        spaced
                            && (w.lower == *n
                                || (last
                                    && w.lower
                                        .strip_prefix(n.as_str())
                                        .is_some_and(|r| r == "s" || r == "es")))
                    })
            });
            match hit {
                Some((name, of)) => {
                    out.push(Found {
                        at: ws[i].at.start..ws[i + name.len() - 1].at.end,
                        name,
                        of,
                    });
                    i += name.len();
                }
                None => i += 1,
            }
        }
        out
    }
}

/// `text` with the name `from` written as `to` instead: the whole name, and its last word on
/// its own (`warrior` of `orc warrior`), in any case and in the plural, keeping a capital
/// first letter and setting `a` or `an` before it to fit. Returns the text and how many
/// times the name was there.
pub fn swap_name(text: &str, from: &str, to: &str) -> (String, usize) {
    swap_names(text, &[from], to)
}

/// [`swap_name`] for several names at once, in one pass, so a name written in is never read
/// again; `to` where it is already written is left as it is (a `bear` line that says `cave
/// bear` already, swapping `polar bear` for `cave bear`).
pub fn swap_names(text: &str, from: &[&str], to: &str) -> (String, usize) {
    let to = to.trim();
    if to.is_empty() {
        return (text.to_string(), 0);
    }
    // each name, and whether it is swapped (the new name is kept)
    let mut names = Vec::new();
    for f in from {
        let full = name_words(f);
        if full.is_empty() {
            continue;
        }
        if let Some(last) = full.last().filter(|l| full.len() > 1 && l.len() >= 3) {
            names.push((last.clone(), true));
        }
        names.push((full.join(" "), true));
    }
    if names.is_empty() {
        return (text.to_string(), 0);
    }
    let kept = name_words(to);
    if !kept.is_empty() {
        names.push((kept.join(" "), false));
    }
    let index = Names::new(names);
    let found: Vec<_> = index
        .find(text)
        .into_iter()
        .filter(|f| f.of.iter().all(|swap| *swap))
        .collect();
    if found.is_empty() {
        return (text.to_string(), 0);
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut at = 0;
    for f in &found {
        let was = &text[f.at.clone()];
        // what follows the name's last word: a plural's ending
        let name_len: usize = {
            let ws = words(was);
            let last = ws.last().map_or(0, |w| w.at.start);
            last + f.name.last().map_or(0, String::len)
        };
        let tail = &was[name_len.min(was.len())..];
        let mut new = String::new();
        let mut cs = to.chars();
        if was.starts_with(|c: char| c.is_ascii_uppercase())
            && let Some(c) = cs.next()
        {
            new.extend(c.to_uppercase());
            new.push_str(cs.as_str());
        } else {
            new.push_str(to);
        }
        new.push_str(tail);
        // `a` or `an` before it, to fit the new name
        let before = &text[at..f.at.start];
        let article = before
            .trim_end_matches(' ')
            .rsplit(|c: char| !c.is_ascii_alphabetic())
            .next()
            .unwrap_or("");
        let vowel = to.starts_with(|c: char| "aeiouAEIOU".contains(c));
        let swap_article = match article {
            "a" | "A" if vowel => Some(if article == "A" { "An" } else { "an" }),
            "an" | "An" if !vowel => Some(if article == "An" { "A" } else { "a" }),
            _ => None,
        };
        match swap_article {
            Some(a) if before.trim_end_matches(' ').ends_with(article) => {
                let cut = before.trim_end_matches(' ').len() - article.len();
                out.push_str(&before[..cut]);
                out.push_str(a);
                out.push_str(&before[cut + article.len()..]);
            }
            _ => out.push_str(before),
        }
        out.push_str(&new);
        at = f.at.end;
    }
    out.push_str(&text[at..]);
    (out, found.len())
}

// ---- who uses each message -----------------------------------------------------------------

/// A record that uses a message.
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub table: &'static str,
    pub key: Vec<Value>,
    /// Its title (`Name`, ...), as a row list shows it.
    pub title: Option<Value>,
}

/// Every message's users, by number.
pub type UsersIndex = HashMap<i64, Vec<User>>;

/// The realms whose index is kept.
const KEPT: usize = 4;

/// Who uses each message of the realm in `dir`: the records that name it, a record once
/// however many of its columns (or its script's lines) do. Kept, and made again only when the
/// realm's files change, so a card's message fields are answered at once.
pub fn users_index(dir: &Path) -> Result<Arc<UsersIndex>, RowError> {
    type Kept = Vec<(PathBuf, wiring::Fingerprint, Arc<UsersIndex>)>;
    static CACHE: Mutex<Kept> = Mutex::new(Vec::new());
    let print = wiring::fingerprint(dir);
    // held while an index is made, so two asks make it once
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, _, x)) = cache.iter().find(|(d, p, _)| d == dir && *p == print) {
        return Ok(x.clone());
    }
    let realm = Realm::open(dir)?;
    let mut tables = Tables::new(&realm);
    let mt = rows::record_table("Messages")?;
    let msgs = tables.rows(mt)?;
    let at = msgs.position("Number");
    let numbers: HashSet<refs::Named> = msgs
        .rows
        .iter()
        .filter_map(|r| at.and_then(|i| refs::num(&r[i])))
        .filter(|n| *n > 0)
        .map(refs::Named::Number)
        .collect();
    let targets = HashMap::from([("Messages", numbers)]);
    let search: Vec<_> = rows::record_tables().collect();
    let mut index = UsersIndex::new();
    refs::scan(&mut tables, &search, &targets, &mut |_, named, r| {
        let refs::Named::Number(n) = named else {
            return;
        };
        let list = index.entry(n).or_default();
        if !list
            .iter()
            .any(|u| u.table == r.table && rows::same_key(&u.key, &r.key))
        {
            list.push(User {
                table: r.table,
                key: r.key,
                title: r.title,
            });
        }
    })?;
    let x = Arc::new(index);
    cache.retain(|(d, _, _)| d != dir);
    if cache.len() >= KEPT {
        cache.remove(0);
    }
    cache.push((dir.to_path_buf(), print, x.clone()));
    Ok(x)
}

// ---- a record's own copy -------------------------------------------------------------------

/// A record's own copy of a message it shares, to be written as a new message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnCopy {
    pub lines: [String; 3],
    /// The record's columns that name the message, to name the copy instead.
    pub columns: Vec<String>,
    /// How many names were swapped for the record's own.
    pub swapped: usize,
}

/// The copy of message `number` that `table`'s record `key` takes for its own: the lines, and
/// for a monster, the names of the other monsters that use the message, written in its lines,
/// swapped for its own (unless its own name holds them: a `troll` line on a `cave troll`).
pub fn own_copy(
    tables: &mut Tables,
    table: &'static NmrTable,
    key: &[Value],
    number: i64,
) -> Result<OwnCopy, RowError> {
    let text = |all: &NmrRows, row: &[Value], c: &str| match all.position(c).map(|i| &row[i]) {
        Some(Value::Text(s)) => s.split('\0').next().unwrap_or("").to_string(),
        _ => String::new(),
    };
    let all = tables.rows(table)?;
    let row = rows::find_row(all, &rows::index_columns(table), key)
        .ok_or_else(|| RowError::NotFound(format!("{} {}", table.name, rows::key_display(key))))?;
    let columns: Vec<String> = all
        .columns
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            crate::fields::column(table.name, c).and_then(|f| f.target()) == Some("Messages")
                && refs::num(&row[*i]) == Some(number)
        })
        .map(|(_, c)| c.clone())
        .collect();
    if columns.is_empty() {
        return Err(RowError::Bad(format!(
            "{} {} does not name message {number}",
            table.name,
            rows::key_display(key)
        )));
    }
    let own = if table.name == "Monsters" {
        text(all, row, "Name").trim().to_string()
    } else {
        String::new()
    };
    let mt = rows::record_table("Messages")?;
    let msgs = tables.rows(mt)?;
    let wanted =
        [Value::Long(i32::try_from(number).map_err(|_| {
            RowError::NotFound(format!("Messages {number}"))
        })?)];
    let m = rows::find_row(msgs, &rows::index_columns(mt), &wanted)
        .ok_or_else(|| RowError::NotFound(format!("Messages {number}")))?;
    let mut lines = [1, 2, 3].map(|i| text(msgs, m, &format!("Line {i}")));
    let mut swapped = 0;
    if !own.is_empty() {
        // the other monsters that use it, by name
        let mons = rows::record_table("Monsters")?;
        let targets = HashMap::from([("Messages", HashSet::from([refs::Named::Number(number)]))]);
        let mut others: Vec<String> = Vec::new();
        refs::scan(tables, &[mons], &targets, &mut |_, _, r| {
            if !rows::same_key(&r.key, key)
                && let Some(Value::Text(n)) = &r.title
            {
                others.push(n.split('\0').next().unwrap_or("").trim().to_string());
            }
        })?;
        let mine = name_words(&own);
        others.sort();
        others.dedup();
        let from: Vec<&str> = others
            .iter()
            .map(String::as_str)
            .filter(|o| {
                let w = name_words(o);
                !w.is_empty() && !within(&w, &mine)
            })
            .collect();
        for l in &mut lines {
            let (s, n) = swap_names(l, &from, &own);
            *l = s;
            swapped += n;
        }
    }
    Ok(OwnCopy {
        lines,
        columns,
        swapped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_and_fill_as_the_game_reads_them() {
        let line = "The %s slashes %s for %d damage! 100%% sure, %ld";
        assert_eq!(blanks(line).len(), 4);
        assert_eq!(
            fill("The %s slashes %s for %s damage!", &["troll", "Kate", "12"]),
            "The troll slashes Kate for 12 damage!"
        );
        // a blank past the last argument prints nothing; a `%` at the end stays
        assert_eq!(fill("A %s comes from %s.", &["rat"]), "A rat comes from .");
        assert_eq!(fill("50%", &[]), "50%");
        assert_eq!(fill("100%% sure", &[]), "100% sure");
    }

    #[test]
    fn what_each_use_reads() {
        let none = |_: &str| 0;
        let hit = reads("Monsters", "Attack Hit Msg 1", &none).unwrap();
        assert_eq!(hit.len(), 2);
        assert_eq!(hit[0].when, "attack 2 hits");
        assert_eq!(
            hit[1].fills,
            vec![ITS_NAME, PLAYER, "the damage", STAR, STAR]
        );
        let death = reads("Monsters", "Death Msg", &none).unwrap();
        assert_eq!((death[0].line, death[0].fills.len()), (3, 0));
        // the arrival line's blanks swap with a name list
        let move_plain = reads("Monsters", "Move Msg", &none).unwrap();
        assert_eq!(move_plain[0].fills, vec![CAME_FROM, ITS_NAME]);
        let listed = |c: &str| i64::from(c == "Desc Txt") * 2003;
        let move_named = reads("Monsters", "Move Msg", &listed).unwrap();
        assert_eq!(move_named[0].fills, vec![ITS_NAME, CAME_FROM]);
        assert!(reads("Spells", "Cast MSG A", &none).is_none());
        assert!(reads("Rooms", "Para1 0", &none).is_none());
    }

    #[test]
    fn names_are_found_whole_and_longest_first() {
        let names = Names::new([
            ("troll".to_string(), 1),
            ("cave troll".to_string(), 2),
            ("half-ogre bodyguard".to_string(), 3),
            ("rat".to_string(), 4),
        ]);
        let found = names
            .find("The cave troll and two trolls watch a \x1b[1;33mhalf-ogre bodyguard. Pirate!");
        let got: Vec<(&[String], &[i32])> = found.iter().map(|f| (f.name, f.of)).collect();
        assert_eq!(got.len(), 3, "{got:?}");
        assert_eq!(got[0].1, &[2]);
        assert_eq!(got[1].1, &[1]);
        assert_eq!(got[2].1, &[3]);
        // across punctuation a name is two words
        assert!(names.find("The cave, troll").iter().all(|f| f.of == [1]));
    }

    #[test]
    fn alike_words_and_names_within_names() {
        assert!(alike("dwarf", "dwarven"));
        assert!(alike("wolf", "wolves"));
        assert!(alike("troll", "trolls"));
        assert!(alike("woman", "women"));
        assert!(!alike("rat", "rabbit"));
        assert!(!alike("orc", "ogre"));
        assert!(within(&name_words("dwarf"), &name_words("dwarven miner")));
        assert!(within(&name_words("troll"), &name_words("cave troll")));
        assert!(!within(&name_words("cave troll"), &name_words("troll")));
    }

    #[test]
    fn a_name_swapped_in_a_line() {
        assert_eq!(
            swap_name(
                "The troll spits up some black blood, and dies!",
                "troll",
                "mountain ape"
            ),
            (
                "The mountain ape spits up some black blood, and dies!".into(),
                1
            )
        );
        // the last word on its own, a capital, a plural, and the article
        assert_eq!(
            swap_name(
                "Warrior! A warrior falls; the orc warriors flee.",
                "orc warrior",
                "ape"
            ),
            ("Ape! An ape falls; the apes flee.".into(), 3)
        );
        assert_eq!(swap_name("A %s lumbers in from %s.", "troll", "ape").1, 0);
    }

    #[test]
    fn names_swapped_in_one_pass() {
        // the new name where it is written already stays, and is not read again
        assert_eq!(
            swap_names(
                "The bear growls; the cave bear sleeps.",
                &["polar bear"],
                "cave bear"
            ),
            ("The cave bear growls; the cave bear sleeps.".into(), 1)
        );
        assert_eq!(
            swap_names(
                "Aiken utters a sharp cry; the troll flees.",
                &["Aiken", "troll"],
                "orc"
            ),
            ("Orc utters a sharp cry; the orc flees.".into(), 2)
        );
        assert_eq!(swap_names("The troll dies.", &[], "orc").1, 0);
    }
}
