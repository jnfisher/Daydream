//! Merging an edit onto a record as the game has it now, member by member.
//!
//! A working copy is read from the game's files and edited while the game runs, and the game
//! keeps changing its own records meanwhile (a room's floor and monsters, a shop's stock, an
//! item's count in play, a player's everything). Writing the working copy's record over the
//! game's would undo all of that. So an edit is written as what it changed: each member of
//! the record (by its layout) that the edit changed from the record as it was read (`base`)
//! takes the edit's bytes (`ours`), and every other member keeps the game's (`theirs`). A
//! member both changed takes the edit's: the person asked for it. An array of numbers (a
//! room's 17 item slots) merges element by element; text merges whole.

use daydream_types::records::layout::{Field, Layout};

/// One range a record merges by, and the member it is (of).
struct Span {
    range: std::ops::Range<usize>,
    field: Option<&'static Field>,
}

/// The byte ranges a record of `len` bytes is merged by: its layout's members (an array of
/// numbers by element), then one range for any bytes past the layout. Without a layout, the
/// whole record is one.
fn spans(layout: Option<&Layout>, len: usize) -> Vec<Span> {
    let mut out = Vec::new();
    let mut end = 0;
    if let Some(layout) = layout {
        for f in layout.fields {
            if f.offset >= len {
                break;
            }
            let to = (f.offset + f.size).min(len);
            let step = if f.elem > 1 { f.elem } else { f.size.max(1) };
            let mut at = f.offset;
            while at < to {
                let next = (at + step).min(to);
                out.push(Span {
                    range: at..next,
                    field: Some(f),
                });
                at = next;
            }
            end = to;
        }
    }
    if end < len {
        out.push(Span {
            range: end..len,
            field: None,
        });
    }
    out
}

/// `theirs` with every member `ours` changed from `base` taken from `ours`. Records whose
/// lengths differ (a variable-length text block) are not merged: `ours` is the result.
pub fn merge(layout: Option<&Layout>, base: &[u8], ours: &[u8], theirs: &[u8]) -> Vec<u8> {
    if base.len() != ours.len() || ours.len() != theirs.len() {
        return ours.to_vec();
    }
    let mut out = theirs.to_vec();
    for s in spans(layout, ours.len()) {
        let r = s.range;
        if ours[r.clone()] != base[r.clone()] {
            out[r.clone()].copy_from_slice(&ours[r]);
        }
    }
    out
}

/// The members `ours` changed from `base`, by name, each once (`"(bytes past the record)"`
/// for bytes past the layout, `"(record)"` without one).
pub fn changed_members(layout: Option<&Layout>, base: &[u8], ours: &[u8]) -> Vec<&'static str> {
    let len = base.len().max(ours.len());
    let mut out: Vec<&'static str> = Vec::new();
    let byte = |r: &[u8], i: usize| r.get(i).copied();
    for s in spans(layout, len) {
        if s.range.clone().any(|i| byte(base, i) != byte(ours, i)) {
            let name = match (s.field, layout) {
                (Some(f), _) => f.name,
                (None, Some(_)) => "(bytes past the record)",
                (None, None) => "(record)",
            };
            if out.last() != Some(&name) {
                out.push(name);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use daydream_types::records::layout::{RACE, ROOM};

    fn race(name: &[u8], exp: u8, hp: u8) -> Vec<u8> {
        let mut r = vec![0; RACE.len];
        r[..2].copy_from_slice(&1u16.to_le_bytes());
        let n = RACE.field("name").unwrap();
        r[n.offset..n.offset + name.len()].copy_from_slice(name);
        r[RACE.field("exp").unwrap().offset] = exp;
        r[RACE.field("hpbonus").unwrap().offset] = hp;
        r
    }

    #[test]
    fn the_edit_takes_its_members_and_the_game_keeps_the_rest() {
        let base = race(b"Human", 10, 1);
        let ours = race(b"Elf", 10, 1);
        let theirs = race(b"Human", 10, 7);
        let merged = merge(Some(&RACE), &base, &ours, &theirs);
        assert_eq!(merged, race(b"Elf", 10, 7));
        assert_eq!(changed_members(Some(&RACE), &base, &ours), ["name"]);
        // both changed the same member: the edit's
        let theirs = race(b"Hume", 12, 1);
        assert_eq!(
            merge(Some(&RACE), &base, &ours, &theirs),
            race(b"Elf", 12, 1)
        );
        // a whole member, not its bytes one by one
        let ours = race(b"Huwan", 10, 1);
        let theirs = race(b"Hxman", 10, 1);
        assert_eq!(merge(Some(&RACE), &base, &ours, &theirs), ours);
    }

    #[test]
    fn an_array_of_numbers_merges_by_element() {
        let items = ROOM.field("items").unwrap();
        let slot = |r: &mut Vec<u8>, i: usize, v: i32| {
            let at = items.offset + 4 * i;
            r[at..at + 4].copy_from_slice(&v.to_le_bytes());
        };
        let base = vec![0; ROOM.len];
        // the edit puts an item in slot 0; the game drops one in slot 3 meanwhile
        let mut ours = base.clone();
        slot(&mut ours, 0, 12);
        let mut theirs = base.clone();
        slot(&mut theirs, 3, 40);
        let mut both = base.clone();
        slot(&mut both, 0, 12);
        slot(&mut both, 3, 40);
        assert_eq!(merge(Some(&ROOM), &base, &ours, &theirs), both);
        // named once, however many of its elements changed
        slot(&mut ours, 5, 7);
        assert_eq!(changed_members(Some(&ROOM), &base, &ours), ["items"]);
    }

    #[test]
    fn without_a_layout_or_with_lengths_that_differ_the_edit_wins_whole() {
        assert_eq!(merge(None, b"abc", b"abd", b"xyz"), b"abd");
        assert_eq!(merge(None, b"abc", b"abc", b"xyz"), b"xyz");
        assert_eq!(merge(None, b"abc", b"abcd", b"xyz"), b"abcd");
        assert_eq!(changed_members(None, b"abc", b"abd"), ["(record)"]);
        assert!(changed_members(None, b"abc", b"abc").is_empty());
    }
}
