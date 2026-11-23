//! NMR's lairs: what `ScanRooms` gathers of each monster group's index range
//! (`GetLairInfo`, `SetLairInfo`), the averages and script values of `CalculateScriptValue`,
//! and the `Lairs` table of `ExportLairs` with its walk between lairs (`GetLairWalk`).

use std::collections::{HashMap, HashSet};

use crate::data::NmrRows;
use crate::schema::Value;

use super::num;

/// VB's `Round(x, digits)`: half to even.
pub(super) fn round(x: f64, digits: i32) -> f64 {
    let m = 10f64.powi(digits);
    (x * m).round_ties_even() / m
}

/// One group's index range (`LairInfoType`).
#[derive(Debug, Default, Clone)]
pub(super) struct Lair {
    pub group_index: String,
    pub mob_list: String,
    pub mobs: i64,
    pub avg_exp: f64,
    pub avg_dmg: f64,
    pub avg_dmg_phys: f64,
    pub avg_dmg_spell: f64,
    pub avg_hp: f64,
    pub avg_ac: i64,
    pub avg_dr: i64,
    pub avg_mr: i64,
    pub avg_dodge: i64,
    pub script_value: f64,
    pub avg_delay: i64,
    pub total_lairs: i64,
    pub lair_rooms: String,
    pub non_lair_rooms: String,
}

/// `colLairs`, by `type-min-max` (NMR's first entry is an empty one).
#[derive(Debug)]
pub(super) struct Lairs {
    list: Vec<Lair>,
    index: HashMap<String, usize>,
}

impl Default for Lairs {
    fn default() -> Self {
        Lairs {
            list: vec![Lair::default()],
            index: HashMap::new(),
        }
    }
}

/// A room as the lair code reads it.
pub(super) struct LairRoom {
    pub map: i64,
    pub room: i64,
    pub ty: i64,
    pub delay: i64,
    pub mon_type: i64,
    pub min_index: i64,
    pub max_index: i64,
    pub max_regen: i64,
    /// Exit room, type, Para1.
    pub exits: Vec<(i64, i64, i64)>,
}

impl LairRoom {
    fn is_lair(&self) -> bool {
        self.min_index <= self.max_index && self.max_index > 0 && self.max_regen > 0 && self.ty == 3
    }
}

impl Lairs {
    /// `GetLairInfoIndex`: the entry for `type-min-max`, made when new.
    fn at(&mut self, key: &str) -> usize {
        if let Some(&i) = self.index.get(key) {
            return i;
        }
        let i = self.list.len();
        self.list.push(Lair {
            group_index: key.to_string(),
            ..Lair::default()
        });
        self.index.insert(key.to_string(), i);
        i
    }

    /// `ScanRooms`' part: a room with a monster index range. `mobs` lists a group index's
    /// monsters; `regen` and `exp` read a monster.
    pub(super) fn room(
        &mut self,
        r: &LairRoom,
        mobs: impl Fn(i64, i64) -> Vec<i64>,
        regen: impl Fn(i64) -> i64,
        exp: impl Fn(i64) -> f64,
    ) {
        let key = format!("{}-{}-{}", r.mon_type, r.min_index, r.max_index);
        let i = self.at(&key);
        let l = &mut self.list[i];
        let here = format!("{}/{}", r.map, r.room);
        if r.max_regen > 0 && r.ty == 3 {
            if l.mobs == 0 {
                let mut list = String::new();
                let mut total = 0f64;
                for x in r.min_index..=r.max_index {
                    for n in mobs(r.mon_type, x) {
                        if n != 0 && regen(n) == 0 {
                            list.push_str(&format!("{n},"));
                            total += exp(n);
                            l.mobs += 1;
                        }
                    }
                }
                if l.mobs > 0 {
                    list.pop();
                    l.mob_list = list;
                    l.avg_exp = round(total / l.mobs as f64, 0);
                }
            }
            let delay = if r.delay == 0 { 5 } else { r.delay };
            // NMR's total is an Integer: past 32,767 the addition fails and is skipped
            if l.avg_delay + delay <= i64::from(i16::MAX) {
                l.avg_delay += delay;
            }
            l.total_lairs += 1;
            note_room(&mut l.lair_rooms, &here);
        } else {
            note_room(&mut l.non_lair_rooms, &here);
        }
    }

    /// `CalculateScriptValue`: each lair's averages over its monsters, from the MME Monsters
    /// table, then each lair monster's `AvgLairExp` and `ScriptValue`.
    pub(super) fn script_values(&mut self, monsters: &mut NmrRows) {
        let col = |c: &str| {
            monsters
                .position(c)
                .unwrap_or_else(|| panic!("Monsters has {c}"))
        };
        let (number, regen, summoned) = (col("Number"), col("RegenTime"), col("Summoned By"));
        let (dmg, phys, spell, hp) = (
            col("AvgDmg"),
            col("AvgDmgPhys"),
            col("AvgDmgSpell"),
            col("HP"),
        );
        let (ac, dr, mr) = (col("ArmourClass"), col("DamageResist"), col("MagicRes"));
        let (lair_exp, script) = (col("AvgLairExp"), col("ScriptValue"));
        let abil: Vec<(usize, usize)> = (0..10)
            .map(|x| (col(&format!("Abil-{x}")), col(&format!("AbilVal-{x}"))))
            .collect();
        let by_number: HashMap<i64, usize> = monsters
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| (num(&r[number]), i))
            .collect();
        // a Double read into NMR's Long
        let long = |v: &Value| match v {
            Value::Double(d) => d.round_ties_even(),
            v => num(v) as f64,
        };
        for l in self.list.iter_mut() {
            if l.mobs <= 0 {
                continue;
            }
            let mut s = [0f64; 8];
            for m in l.mob_list.split(',') {
                let n: i64 = m.trim().parse().unwrap_or(0);
                if n <= 0 {
                    continue;
                }
                let Some(&i) = by_number.get(&n) else {
                    continue;
                };
                let r = &monsters.rows[i];
                let dodge = abil
                    .iter()
                    .filter(|&&(a, _)| num(&r[a]) == 34)
                    .map(|&(_, v)| num(&r[v]))
                    .next_back()
                    .unwrap_or(0);
                for (k, v) in [
                    long(&r[dmg]),
                    long(&r[phys]),
                    long(&r[spell]),
                    num(&r[hp]) as f64,
                    num(&r[ac]) as f64,
                    num(&r[dr]) as f64,
                    num(&r[mr]) as f64,
                    dodge as f64,
                ]
                .into_iter()
                .enumerate()
                {
                    s[k] += v;
                }
            }
            let avg = |t: f64| round(t / l.mobs as f64, 0);
            l.avg_dmg_phys = avg(s[1]);
            l.avg_dmg_spell = avg(s[2]);
            l.avg_dmg = avg(s[0]);
            l.avg_hp = avg(s[3]);
            l.avg_ac = avg(s[4]) as i64;
            l.avg_dr = avg(s[5]) as i64;
            l.avg_mr = avg(s[6]) as i64;
            l.avg_dodge = avg(s[7]) as i64;
            if l.total_lairs > 0 && l.avg_delay > 0 {
                l.avg_delay = round(l.avg_delay as f64 / l.total_lairs as f64, 0) as i64;
            }
        }

        for r in monsters.rows.iter_mut() {
            if num(&r[regen]) != 0 {
                continue;
            }
            let Value::Text(by) = &r[summoned] else {
                continue;
            };
            if !by.contains("(lair)") {
                continue;
            }
            let found = lair_tags(by);
            if found.is_empty() {
                continue;
            }
            let lairs = found.len() as f64;
            let (mut total_exp, mut value, mut mobs) = (0f64, 0f64, 0i64);
            for (group_index, max_regen) in &found {
                if *max_regen > 0 && group_index.len() > 4 {
                    let (exp, sv) = self.lair_info(group_index, *max_regen);
                    total_exp += exp * *max_regen as f64;
                    value += sv;
                    mobs += max_regen;
                } else if *max_regen > 0 {
                    mobs += max_regen;
                }
            }
            let mut value = round(value / lairs, 0);
            // the monsters a lair holds on average (an Integer)
            let possy = round(round(mobs as f64 / lairs, 1), 0);
            let mut per_hour = 36f64;
            if possy > 1.0 {
                per_hour = round(per_hour / possy + 1.0, 2);
            }
            if lairs < per_hour {
                let pct = round(lairs / per_hour, 2);
                value = round(value * pct, 0);
            }
            if value >= 1.0 || total_exp > 0.0 {
                r[lair_exp] = Value::Double(round(total_exp / lairs, 0));
                r[script] = Value::Double(value);
            }
        }
    }

    /// `GetLairInfo(type-min-max-maxregen)`: the lair's average exp and the script value it
    /// had before this call; the call then works the value out again for this max regen.
    fn lair_info(&mut self, group_index: &str, max_regen: i64) -> (f64, f64) {
        let i = self.at(group_index);
        let l = &mut self.list[i];
        let before = (l.avg_exp, l.script_value);
        let m = max_regen as f64;
        l.script_value = if max_regen > 0 && l.avg_exp > 0.0 {
            if l.avg_hp + l.avg_dmg <= 0.0 {
                l.avg_exp * m * 100.0
            } else {
                round(
                    (l.avg_exp * m)
                        / ((l.avg_hp * m) + (l.avg_dmg * 2.0 * ((m * (m + 1.0)) / 2.0)))
                        * 100.0,
                    0,
                )
            }
        } else {
            0.0
        };
        before
    }

    /// `ExportLairs`: the Lairs table's rows.
    pub(super) fn export(&self, t: &mut NmrRows, rooms: &[LairRoom]) {
        let at: HashMap<(i64, i64), usize> = rooms
            .iter()
            .enumerate()
            .map(|(i, r)| ((r.map, r.room), i))
            .collect();
        let mut seen: HashSet<String> = HashSet::new();
        for l in &self.list {
            let parts: Vec<&str> = l.group_index.split('-').collect();
            if l.mobs <= 0 || parts.len() != 3 || !seen.insert(l.group_index.clone()) {
                continue;
            }
            let group = super::walk::vb_val(parts[0]);
            let strip = |s: &str| -> Vec<String> {
                if s.is_empty() {
                    return Vec::new();
                }
                s[1..s.len() - 1].split(',').map(str::to_string).collect()
            };
            let non_lairs = strip(&l.non_lair_rooms).len() as f64;
            let mut lairs_to_rooms = 0.0;
            if l.total_lairs > 0 && non_lairs > 0.0 {
                lairs_to_rooms =
                    round((non_lairs + l.total_lairs as f64) / l.total_lairs as f64, 2);
            }
            let lair_list: Vec<(i64, i64)> =
                strip(&l.lair_rooms).iter().map(|s| map_room(s)).collect();
            let mut walk_distance = 0.0;
            if lair_list.len() > 1 {
                let (mut total, mut count) = (0i64, 0i64);
                for &start in &lair_list {
                    let mut chained = Chain::default();
                    chained.push(start);
                    let (t, c) = lair_walk(start, rooms, &at, &mut chained, group, 1);
                    if c > 0 {
                        // NMR's total is a Long
                        total = (total as f64 + t as f64 / c as f64).round_ties_even() as i64;
                        count += 1;
                    }
                }
                if count > 0 {
                    walk_distance = round(total as f64 / count as f64, 2);
                }
            }
            let avg_walk = if lairs_to_rooms > 0.0 && lairs_to_rooms < walk_distance {
                lairs_to_rooms
            } else if walk_distance > 0.0 {
                walk_distance
            } else {
                lairs_to_rooms
            };
            t.rows.push(vec![
                Value::Text(l.group_index.clone()),
                Value::Text(l.mob_list.clone()),
                Value::Long(l.mobs as i32),
                Value::Long(l.total_lairs as i32),
                Value::Long(l.avg_delay as i32),
                Value::Double(avg_walk),
                Value::Double(l.avg_exp),
                Value::Double(l.avg_dmg),
                Value::Double(l.avg_dmg_phys),
                Value::Double(l.avg_dmg_spell),
                Value::Double(l.avg_hp),
                Value::Long(l.avg_ac as i32),
                Value::Long(l.avg_dr as i32),
                Value::Long(l.avg_mr as i32),
                Value::Long(l.avg_dodge as i32),
            ]);
        }
    }
}

/// NMR's room lists: `,m/r,` and each new room then `m/r,`.
fn note_room(list: &mut String, here: &str) {
    if list.is_empty() {
        *list = format!(",{here},");
    } else if !list.contains(&format!(",{here},")) {
        list.push_str(&format!("{here},"));
    }
}

/// `ExtractMapRoom` of `m/r`.
fn map_room(s: &str) -> (i64, i64) {
    match s.split_once('/') {
        Some((m, r)) => (super::walk::vb_val(m), super::walk::vb_val(r)),
        None => (0, 0),
    }
}

/// The `[type-min-max][maxregen]Group(lair): m/r` tags in a Summoned By note.
fn lair_tags(s: &str) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while let Some(p) = s[i..].find('[') {
        let start = i + p;
        i = start + 1;
        let Some(m) = parse_tag(b, start) else {
            continue;
        };
        out.push((m.0, m.1));
        i = m.2;
    }
    out
}

/// `\[([\d\-]+)\]\[(\d+)\]Group\(lair\): (\d+)\/(\d+)` at `at`: the two captures and where
/// the match ends.
fn parse_tag(b: &[u8], at: usize) -> Option<(String, i64, usize)> {
    let mut i = at + 1;
    let g0 = i;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'-') {
        i += 1;
    }
    if i == g0 || b.get(i) != Some(&b']') || b.get(i + 1) != Some(&b'[') {
        return None;
    }
    let group = String::from_utf8_lossy(&b[g0..i]).into_owned();
    i += 2;
    let r0 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == r0 || b.get(i) != Some(&b']') {
        return None;
    }
    let regen: i64 = std::str::from_utf8(&b[r0..i]).ok()?.parse().ok()?;
    i += 1;
    let lit = b"Group(lair): ";
    if !b[i..].starts_with(lit) {
        return None;
    }
    i += lit.len();
    let m0 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == m0 || b.get(i) != Some(&b'/') {
        return None;
    }
    i += 1;
    let n0 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == n0 {
        return None;
    }
    Some((group, regen, i))
}

/// The rooms a walk has been to (`aChainedRooms`).
#[derive(Default)]
struct Chain {
    set: HashSet<(i64, i64)>,
}

impl Chain {
    fn push(&mut self, r: (i64, i64)) {
        self.set.insert(r);
    }

    fn contains(&self, r: (i64, i64)) -> bool {
        self.set.contains(&r)
    }
}

/// `GetLairWalk`: from `start`, the steps to each lair reached through rooms of `group`
/// (the total of the depths, and how many), depth first, each room once.
fn lair_walk(
    start: (i64, i64),
    rooms: &[LairRoom],
    at: &HashMap<(i64, i64), usize>,
    chained: &mut Chain,
    group: i64,
    depth: i64,
) -> (i64, i64) {
    if depth > 99 || start.0 == 0 || start.1 == 0 {
        return (0, 0);
    }
    let Some(&i) = at.get(&start) else {
        return (0, 0);
    };
    let r = &rooms[i];
    let (mut total, mut count) = (0i64, 0i64);
    let mut queue = Vec::new();
    for &(to, ty, para1) in &r.exits {
        // GetRoomExits(…, bForLairs): no remote actions, broken hidden exits or map changes
        if to <= 0 || ty == 12 || (ty == 6 && para1 == 0) || ty == 8 {
            continue;
        }
        let next = (r.map, to);
        // the group is the room walked from's
        if group > 0 && group != r.mon_type {
            continue;
        }
        if chained.contains(next) {
            continue;
        }
        if at.get(&next).is_some_and(|&j| rooms[j].is_lair()) {
            total += depth;
            count += 1;
        } else {
            queue.push(next);
        }
        chained.push(next);
    }
    for q in queue {
        let (t, c) = lair_walk(q, rooms, at, chained, group, depth + 1);
        total += t;
        count += c;
    }
    if total > 0 && count > 0 {
        (total, count)
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lair_tags_are_found() {
        let s = "Group: 1/2,[3-4-5][2]Group(lair): 6/7,[3-4-5][x]Group(lair): 1/1,[9-1-1][4]Group(lair): 8/9";
        assert_eq!(
            lair_tags(s),
            vec![("3-4-5".to_string(), 2), ("9-1-1".to_string(), 4)]
        );
    }

    #[test]
    fn rounding_is_bankers() {
        assert_eq!(round(2.5, 0), 2.0);
        assert_eq!(round(3.5, 0), 4.0);
    }
}
