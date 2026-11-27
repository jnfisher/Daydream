//! MajorMUD's text block scripts, read as the game reads them, for the checks that read the
//! wiring ([`crate::wiring`]). A port of the web UI's `web/src/library/script.ts`, itself a
//! port of silvermere-engine's `scripts/mod.rs` (see UPSTREAM.md): an action line is clauses
//! cut at each `:`, and a clause is the first keyword it starts with (ASCII letters without
//! case, [`KEYWORDS`]' order deciding) and its words, each a number as the game reads one (a
//! missing word is 0). A clause with no keyword is skipped and the line goes on.
//!
//! How a block is run decides how it reads ([`Reading`]): a whole block (a monster's answer, a
//! death text, a spell's) tries its lines in turn; a room's command block runs the lines whose
//! `words:clauses` pattern matches what was typed; a random block rolls 0 to 99 for one line by
//! its percentage; a talk block's `words:block` lines are what a monster can be asked about.
//!
//! Only what a clause names is read here (its numbers), not what it does.

/// The keywords, in matching order: `nomonsters` before `monsters`; `teleport` takes every
/// `teleport_` form.
pub const KEYWORDS: [&str; 42] = [
    "price",
    "cast",
    "giveability",
    "addability",
    "teleport",
    "removeability",
    "nomonsters",
    "monsters",
    "needmonster",
    "summon",
    "message",
    "takeitem",
    "addexp",
    "giveitem",
    "hideitem",
    "checkability",
    "class",
    "race",
    "goodaligned",
    "evilaligned",
    "minlevel",
    "maxlevel",
    "checkitem",
    "failitem",
    "text",
    "roomtext",
    "roomitem",
    "failroomitem",
    "clearitem",
    "addevil",
    "testability",
    "failability",
    "remoteaction",
    "random",
    "adddelay",
    "testskill",
    "checkskill",
    "givecoins",
    "test_tournament",
    "checkspell",
    "flag",
    "learnspell",
];

/// The stock world's named teleports, in matching order (`teleport_slumsewers` before
/// `teleport_slums`): such a clause takes no numbers.
const TELEPORTS: [&str; 12] = [
    "teleport_silvermere",
    "teleport_sewers",
    "teleport_slumsewers",
    "teleport_slums",
    "teleport_darkwood",
    "teleport_eldergrove",
    "teleport_labyrinth",
    "teleport_blackcaverns",
    "teleport_fungushazy",
    "teleport_crackedlava",
    "teleport_blackfortress",
    "teleport_obsidian",
];

/// Each keyword's words, in order: a name is a number, 32-bit, or 16-bit with a `%`; a `~`
/// word is read as a word (a skill, a coin, a flag's verb).
fn args(kw: &str) -> &'static str {
    match kw {
        "price" => "copper msg",
        "cast" => "spell",
        "giveability" => "ability value",
        "addability" => "ability amount",
        "teleport" => "room map",
        "removeability" => "ability",
        "nomonsters" | "monsters" | "message" => "msg",
        "needmonster" => "monster msg",
        "summon" => "monster",
        "takeitem" | "checkitem" | "failitem" | "roomitem" | "failroomitem" | "clearitem" => {
            "item msg"
        }
        "addexp" => "exp",
        "giveitem" | "hideitem" => "item",
        "checkability" => "ability% min%",
        "class" => "class",
        "race" => "race",
        "goodaligned" | "evilaligned" => "limit msg",
        "minlevel" | "maxlevel" => "level msg",
        "text" | "roomtext" | "random" => "block",
        "addevil" => "points%",
        "testability" => "ability% max%",
        "failability" => "ability% msg%",
        "remoteaction" => "room msg action exit%",
        "adddelay" => "ticks",
        "testskill" => "~skill modifier% block%",
        "checkskill" => "~skill min% msg%",
        "givecoins" => "amount ~coin",
        "checkspell" => "spell block",
        "flag" => "flag% ~verb msg",
        "learnspell" => "spell%",
        _ => "",
    }
}

/// At most this many `|` alternatives of a pattern are tried.
pub const MAXALTS: usize = 20;

fn is_blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0c | b'\r')
}

/// Past the word (to the first white space or the end).
pub fn skip_word(s: &str) -> &str {
    let i = s.bytes().position(is_blank).unwrap_or(s.len());
    &s[i..]
}

/// Past the white space.
pub fn skip_blanks(s: &str) -> &str {
    let i = s.bytes().position(|b| !is_blank(b)).unwrap_or(s.len());
    &s[i..]
}

/// `word` starts with `prefix` (lower case), ASCII letters compared without case.
pub fn starts_with(prefix: &str, word: &str) -> bool {
    word.len() >= prefix.len()
        && word.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// A number as the game reads one: white space, a sign, then digits; anything else ends it (0
/// when there is none). It wraps at 32 bits.
pub fn leading_int32(s: &str) -> i32 {
    let t = skip_blanks(s).as_bytes();
    let (neg, digits) = match t.first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let mut v: i32 = 0;
    for &d in digits {
        if !d.is_ascii_digit() {
            break;
        }
        v = v.wrapping_mul(10).wrapping_add(i32::from(d - b'0'));
    }
    if neg { v.wrapping_neg() } else { v }
}

/// [`leading_int32`] cut to 16 bits.
pub fn leading_int(s: &str) -> i32 {
    i32::from(leading_int32(s) as i16)
}

/// The keyword a clause starts with, if any.
pub fn keyword_of(clause: &str) -> Option<&'static str> {
    KEYWORDS.iter().copied().find(|k| starts_with(k, clause))
}

/// One clause, its numbers read as the game reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    /// Its keyword, or `None` for a clause the game skips.
    pub kw: Option<&'static str>,
    /// The clause as written.
    pub text: String,
    /// Its numbers by name (`copper`, `msg`, `item`, ...), in the keyword's order.
    pub n: Vec<(&'static str, i32)>,
    /// A named teleport's keyword, or a coin's or skill's word.
    pub word: Option<String>,
    /// Whether a `:` follows it.
    pub more: bool,
}

impl Clause {
    /// The number called `name` (0 when the clause has none).
    pub fn get(&self, name: &str) -> i32 {
        self.n
            .iter()
            .find(|(k, _)| *k == name)
            .map_or(0, |(_, v)| *v)
    }

    /// One clause with its numbers.
    pub fn parse(text: &str, more: bool) -> Clause {
        let kw = keyword_of(text);
        let mut c = Clause {
            kw,
            text: text.to_string(),
            n: Vec::new(),
            word: None,
            more,
        };
        let Some(kw) = kw else {
            return c;
        };
        if kw == "teleport"
            && let Some(named) = TELEPORTS.iter().find(|t| starts_with(t, text))
        {
            c.word = Some((*named).to_string());
            return c;
        }
        let mut rest = skip_blanks(skip_word(text));
        for spec in args(kw).split(' ').filter(|s| !s.is_empty()) {
            match spec {
                "~skill" | "~verb" => {
                    c.word = rest
                        .split(|ch: char| ch.is_ascii_whitespace())
                        .next()
                        .map(str::to_string);
                }
                "~coin" => {
                    c.word = rest
                        .chars()
                        .next()
                        .map(|ch| ch.to_ascii_lowercase().to_string())
                }
                s if s.ends_with('%') => c.n.push((&s[..s.len() - 1], leading_int(rest))),
                s => c.n.push((s, leading_int32(rest))),
            }
            if spec != "~coin" {
                rest = skip_blanks(skip_word(rest));
            }
        }
        c
    }
}

/// The clauses of an action line, as the game cuts them: at each `:`, a last empty piece
/// ending the line.
pub fn clauses(line: &str) -> Vec<Clause> {
    let mut pieces: Vec<&str> = line.split(':').collect();
    let n = pieces.len();
    if pieces.last() == Some(&"") {
        pieces.pop();
    }
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| Clause::parse(p, i + 1 < n))
        .collect()
}

/// The lines of a block run as a whole: the first line always, then each next one while a
/// `:` follows anywhere from its start.
pub fn whole_lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    loop {
        let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        out.push(&text[start..end]);
        if end >= text.len() {
            break;
        }
        start = end + 1;
        if !text[start..].contains(':') {
            break;
        }
    }
    out
}

/// The lines of a pattern block (a talk block, a room's command block, a random block): the
/// pattern runs from a line's start to the next `:` anywhere further on (so a line with no
/// `:` joins the next one's pattern), the action to the line's end; the scan ends when no `:`
/// follows.
pub fn pattern_lines(text: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut line = 0;
    while let Some(c) = text[line..].find(':') {
        let colon = line + c;
        let end = text[colon + 1..]
            .find('\n')
            .map_or(text.len(), |i| colon + 1 + i);
        out.push((&text[line..colon], &text[colon + 1..end]));
        if end >= text.len() {
            break;
        }
        line = end + 1;
    }
    out
}

/// How a text block reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reading {
    /// A script run as a whole.
    Script,
    /// A talk block's topics (`words:block`).
    Topics,
    /// A room's commands (`words:clauses`).
    Commands,
    /// A random block's chances (`percent:clauses`).
    Chances,
    /// Text to show.
    Text,
}

fn is_number(s: &str) -> bool {
    let t = s.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\r');
    let t = t.strip_prefix(['+', '-']).unwrap_or(t);
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// How a block's text reads by its shape alone: shown text has colour codes; each line of a
/// script starts with a keyword; a talk block's lines are `words:number`, a random block's
/// `number:clauses` and a room's `words:clauses`.
pub fn reading_of(text: &str) -> Reading {
    if text.trim().is_empty() || text.contains('\x1b') {
        return Reading::Text;
    }
    let lines: Vec<&str> = whole_lines(text)
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if !lines.is_empty() && lines.iter().all(|l| keyword_of(l).is_some()) {
        return Reading::Script;
    }
    let pl: Vec<(&str, &str)> = pattern_lines(text)
        .into_iter()
        .filter(|(p, a)| !p.trim().is_empty() || !a.trim().is_empty())
        .collect();
    if pl.is_empty() {
        return Reading::Text;
    }
    if pl
        .iter()
        .all(|(p, a)| !p.trim().is_empty() && keyword_of(p).is_none() && is_number(a))
    {
        return Reading::Topics;
    }
    if pl
        .iter()
        .all(|(p, a)| is_number(p) && keyword_of(a).is_some())
    {
        return Reading::Chances;
    }
    if pl
        .iter()
        .all(|(p, a)| !p.trim().is_empty() && keyword_of(a).is_some())
    {
        return Reading::Commands;
    }
    Reading::Text
}

/// One line of a block as the game reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Its number among the block's lines, from 1.
    pub no: usize,
    /// What the game matches it by: a command's words, a topic's, a random line's percentage.
    pub pattern: Option<String>,
    /// A topic's text block.
    pub block: Option<i32>,
    pub clauses: Vec<Clause>,
    /// The game never runs it: past where a whole block stops reading, or a random line the
    /// lines before it leave no roll.
    pub never: bool,
}

/// A block's lines, as the game reads them for `reading` (none for text).
pub fn lines_of(text: &str, reading: Reading) -> Vec<Line> {
    match reading {
        Reading::Script => {
            let read = whole_lines(text).len();
            text.split('\n')
                .enumerate()
                .filter(|(_, l)| !l.trim().is_empty())
                .map(|(i, l)| Line {
                    no: i + 1,
                    pattern: None,
                    block: None,
                    clauses: if i < read { clauses(l) } else { Vec::new() },
                    never: i >= read,
                })
                .collect()
        }
        Reading::Topics => pattern_lines(text)
            .into_iter()
            .filter(|(p, a)| !p.is_empty() || !a.trim().is_empty())
            .enumerate()
            .map(|(i, (p, a))| Line {
                no: i + 1,
                pattern: Some(p.to_string()),
                block: Some(leading_int32(a)),
                clauses: Vec::new(),
                never: false,
            })
            .collect(),
        Reading::Chances => {
            let mut top = 0;
            pattern_lines(text)
                .into_iter()
                .enumerate()
                .map(|(i, (p, a))| {
                    let at = leading_int32(p).clamp(0, 100);
                    let pct = (at - top).max(0);
                    top = top.max(at);
                    Line {
                        no: i + 1,
                        pattern: Some(p.trim().to_string()),
                        block: None,
                        clauses: clauses(a),
                        never: pct == 0,
                    }
                })
                .collect()
        }
        Reading::Commands => pattern_lines(text)
            .into_iter()
            .enumerate()
            .map(|(i, (p, a))| Line {
                no: i + 1,
                pattern: Some(p.to_string()),
                block: None,
                clauses: clauses(a),
                never: false,
            })
            .collect(),
        Reading::Text => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // the tests of script.ts (the web UI's port), which carries silvermere's own
    #[test]
    fn numbers_are_read_as_the_game_reads_them() {
        assert_eq!(leading_int32("  -42x"), -42);
        assert_eq!(leading_int32("+7"), 7);
        assert_eq!(leading_int32(""), 0);
        assert_eq!(leading_int32("lever"), 0);
        assert_eq!(leading_int32("4294967297"), 1);
        assert_eq!(leading_int("70000"), 4464);
        let c = Clause::parse("checkitem 123   45", false);
        assert_eq!(c.n, vec![("item", 123), ("msg", 45)]);
        assert_eq!(
            Clause::parse("price", false).n,
            vec![("copper", 0), ("msg", 0)]
        );
    }

    #[test]
    fn a_line_is_cut_at_each_colon() {
        let cut = |line: &str| -> Vec<(String, bool)> {
            clauses(line)
                .into_iter()
                .map(|c| (c.text, c.more))
                .collect()
        };
        assert_eq!(cut("a:b"), vec![("a".into(), true), ("b".into(), false)]);
        assert_eq!(
            cut("a::b:"),
            vec![("a".into(), true), ("".into(), true), ("b".into(), true)]
        );
        assert!(cut("").is_empty());
    }

    #[test]
    fn a_keyword_is_the_first_in_order() {
        assert_eq!(keyword_of("nomonsters 5"), Some("nomonsters"));
        assert_eq!(keyword_of("MONSTERS"), Some("monsters"));
        assert_eq!(keyword_of("teleport_slumsewers"), Some("teleport"));
        assert_eq!(keyword_of("check class 3"), None);
        assert_eq!(keyword_of(" message 5"), None);
        assert_eq!(
            Clause::parse("teleport_slums", false).word.as_deref(),
            Some("teleport_slums")
        );
        let skill = Clause::parse("testskill picklocks 10 77", false);
        assert_eq!(skill.n, vec![("modifier", 10), ("block", 77)]);
        let line = clauses("price 200000 1458:giveitem 868:message 1459");
        assert_eq!(line[0].get("copper"), 200_000);
        assert_eq!(line[0].get("msg"), 1458);
        assert_eq!(line[1].get("item"), 868);
        assert_eq!(line[2].get("msg"), 1459);
        assert_eq!(
            Clause::parse("remoteaction 2051 77 3 1", false).get("exit"),
            1
        );
    }

    #[test]
    fn a_block_reads_as_its_shape_says() {
        assert_eq!(
            reading_of("wares:909\naxes:910\nmail:912\nhammers:914\n"),
            Reading::Topics
        );
        assert_eq!(
            reading_of("price 200000 1458:giveitem 868:message 1459"),
            Reading::Script
        );
        assert_eq!(
            reading_of("maxlevel 29:cast 818\nmaxlevel 39:testskill wisdom 40 2765\n"),
            Reading::Script
        );
        assert_eq!(
            reading_of("go hole:nomonsters 266:message 267:teleport 2126 1\n"),
            Reading::Commands
        );
        assert_eq!(
            reading_of("50:giveitem 1593\n100:giveitem 1594\n\n"),
            Reading::Chances
        );
        assert_eq!(
            reading_of("The old man looks you over, and says \"Ah\""),
            Reading::Text
        );
        assert_eq!(reading_of("\x1b[0mNote: run!"), Reading::Text);
        assert_eq!(reading_of("\n\n"), Reading::Text);
    }

    #[test]
    fn a_blocks_lines_as_the_game_reads_them() {
        let whole = lines_of("message 5\ntext 6", Reading::Script);
        let got: Vec<_> = whole
            .iter()
            .map(|l| (l.no, l.clauses.len(), l.never))
            .collect();
        assert_eq!(got, vec![(1, 1, false), (2, 0, true)]);
        let n: Vec<_> = lines_of("message 5\ntext 6:text 7\n\n", Reading::Script)
            .iter()
            .map(|l| l.clauses.len())
            .collect();
        assert_eq!(n, vec![1, 2]);
        let never: Vec<_> = lines_of(
            "30:text 1\n20:text 2\n60:text 3\n150:text 4",
            Reading::Chances,
        )
        .iter()
        .map(|l| l.never)
        .collect();
        assert_eq!(never, vec![false, true, false, false]);
        let topics: Vec<_> = lines_of("wares:909\naxes:910\n", Reading::Topics)
            .into_iter()
            .map(|l| (l.pattern.unwrap(), l.block.unwrap()))
            .collect();
        assert_eq!(topics, vec![("wares".into(), 909), ("axes".into(), 910)]);
    }
}
