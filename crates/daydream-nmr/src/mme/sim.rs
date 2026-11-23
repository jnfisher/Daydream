//! NMR's monster attack simulation (`CalculateMonsterAvgDmg` and MMUD Explorer's
//! `clsMonsterAttackSim`, which NMR's export carries): each monster's average damage a round,
//! physical and by spells, and how often it uses each attack (`AttTrue%`).
//!
//! The simulation is random: it fights rounds against a player of 50 MR until the running
//! average settles (changes less than 1% over a thousand rounds), so NMR's own numbers move
//! a little from one export to the next. This port draws from VB's own generator with a
//! fixed seed, so Daydream's export is the same each time, and its averages sit where NMR's
//! do to within that settling.
//!
//! Like NMR's, the simulator is one object reused monster after monster, and a few of its
//! values are not reset between them (an attack's spell duration, and the physical and spell
//! averages when there is nothing to simulate); the port carries them over the same way.

use std::collections::HashMap;

use super::Src;
use super::walk::{Reader, Row};

/// VB's `Rnd`: its 24-bit linear congruential generator.
struct VbRnd(u32);

impl VbRnd {
    fn next(&mut self) -> f64 {
        self.0 = (self.0.wrapping_mul(0x43FD_43FD).wrapping_add(0x00C3_9EC3)) & 0x00FF_FFFF;
        f64::from(self.0) / 16_777_216.0
    }

    /// `RandomNumber(a, b)`: `Int((b - a + 1) * Rnd + a)`.
    fn number(&mut self, a: i64, b: i64) -> i64 {
        ((b - a + 1) as f64 * self.next() + a as f64).floor() as i64
    }
}

/// VB's `Round(x, digits)`: half to even.
fn round(x: f64, digits: i32) -> f64 {
    super::lairs::round(x, digits)
}

/// A spell as the simulation reads it.
struct Spell {
    target: i64,
    resists: i64,
    duration: i64,
    min: i64,
    max: i64,
    min_inc: (i64, i64),
    max_inc: (i64, i64),
    dur_inc: (i64, i64),
    abilities: Vec<(i64, i64)>,
}

impl Spell {
    /// `SpellHasAbility`: the last such ability's value, or -1.
    fn ability(&self, a: i64) -> i64 {
        self.abilities
            .iter()
            .filter(|&&(x, _)| x == a)
            .map(|&(_, v)| v)
            .next_back()
            .unwrap_or(-1)
    }

    fn scaled(base: i64, (lvls, inc): (i64, i64), level: i64) -> i64 {
        if lvls == 0 || level <= 0 {
            base
        } else {
            base + ((inc as f64 / lvls as f64) * level as f64).trunc() as i64
        }
    }

    fn min_damage(&self, level: i64) -> i64 {
        Spell::scaled(self.min, self.min_inc, level)
    }

    fn max_damage(&self, level: i64) -> i64 {
        Spell::scaled(self.max, self.max_inc, level)
    }

    fn duration(&self, level: i64) -> i64 {
        Spell::scaled(self.duration, self.dur_inc, level)
    }
}

struct Item {
    limit: i64,
    abilities: Vec<(i64, i64)>,
}

impl Item {
    /// `ItemHasAbility`.
    fn ability(&self, a: i64) -> i64 {
        self.abilities
            .iter()
            .filter(|&&(x, _)| x == a)
            .map(|&(_, v)| v)
            .next_back()
            .unwrap_or(-1)
    }
}

/// A monster's attacks.
pub(super) struct Monster {
    pub number: i64,
    energy: i64,
    weapon: i64,
    /// Type, energy, %, min hit / cast %, max hit / cast level, accuracy / spell, hit spell.
    attacks: [[i64; 7]; 5],
    /// Spell, cast %, cast level.
    between: [[i64; 3]; 5],
    drops: Vec<(i64, i64)>,
}

/// What the simulation needs of the world.
pub(super) struct Book {
    spells: HashMap<i64, Spell>,
    items: HashMap<i64, Item>,
    pub monsters: Vec<Monster>,
}

fn abilities(r: &Row, n: usize) -> Vec<(i64, i64)> {
    (0..n)
        .map(|x| {
            (
                r.n(&format!("Ability {x}")),
                r.n(&format!("Ability Value {x}")),
            )
        })
        .collect()
}

impl Book {
    pub(super) fn read(monsters: &Src, spells: &Src, items: &Src) -> Book {
        let spells = Reader::new(spells)
            .rows()
            .map(|r| {
                (
                    r.n("Number"),
                    Spell {
                        target: r.n("Target"),
                        resists: r.n("Type of Resists"),
                        duration: r.n("Duration"),
                        min: r.n("Min"),
                        max: r.n("Max"),
                        min_inc: (r.n("LVLS Min Increase"), r.n("Min Increase")),
                        max_inc: (r.n("LVLS Max Increase"), r.n("Max Increase")),
                        dur_inc: (r.n("LVLS Dur Increase"), r.n("Dur Increase")),
                        abilities: abilities(&r, 10),
                    },
                )
            })
            .collect();
        let items = Reader::new(items)
            .rows()
            .map(|r| {
                (
                    r.n("Number"),
                    Item {
                        limit: r.n("Game Limit"),
                        abilities: abilities(&r, 20),
                    },
                )
            })
            .collect();
        let monsters = Reader::new(monsters)
            .rows()
            .map(|r| Monster {
                number: r.n("Number"),
                energy: r.n("Energy"),
                weapon: r.n("Weapon Number"),
                attacks: std::array::from_fn(|x| {
                    [
                        "Attack Type",
                        "Attack Energy",
                        "Attack %",
                        "Attack Min Hit/Cast %",
                        "Attack Max Hit/Cast LVL",
                        "Attack Accu/Spell",
                        "Attack Hit Spell",
                    ]
                    .map(|c| r.n(&format!("{c} {x}")))
                }),
                between: std::array::from_fn(|x| {
                    ["Spell Number", "Spell Cast %", "Spell Cast LVL"]
                        .map(|c| r.n(&format!("{c} {x}")))
                }),
                drops: (0..10)
                    .map(|x| {
                        (
                            r.n(&format!("Item Number {x}")),
                            r.n(&format!("Item Drop % {x}")),
                        )
                    })
                    .collect(),
            })
            .collect();
        Book {
            spells,
            items,
            monsters,
        }
    }

    /// `CalculateMonsterItemBonuses`: what the monster's weapon and drops (unlimited ones)
    /// add of these abilities, the drops at their drop chance.
    fn item_bonus(&self, m: &Monster, wanted: &[i64]) -> i64 {
        // NMR's total is an Integer: each addition rounds
        let mut total = 0i64;
        let unlimited = |n: i64| self.items.get(&n).filter(|i| i.limit == 0);
        if m.weapon > 0
            && let Some(item) = unlimited(m.weapon)
        {
            for &a in wanted {
                let v = item.ability(a);
                if v > 0 {
                    total += v;
                }
            }
        }
        for &(n, pct) in &m.drops {
            if n <= 0 {
                continue;
            }
            let Some(item) = unlimited(n) else { continue };
            for &a in wanted {
                let v = item.ability(a);
                if v > 0 {
                    let pct = pct.min(100) as f64;
                    total = (total as f64 + v as f64 * (pct / 100.0)).round_ties_even() as i64;
                }
            }
        }
        total
    }
}

/// One monster's results.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Results {
    pub avg: f64,
    pub phys: f64,
    pub spell: f64,
    /// `AttTrue%-x`.
    pub att_true: [f64; 5],
}

#[derive(Default, Clone, Copy)]
struct Attack {
    ty: i64,
    energy: i64,
    min: i64,
    max: i64,
    chance: i64,
    success: i64,
    hit_min: i64,
    hit_max: i64,
    resist: i64,
    mr_resist: i64,
    duration: i64,
}

#[derive(Default, Clone, Copy)]
struct Between {
    min: i64,
    max: i64,
    chance: i64,
    resist: i64,
    mr_resist: i64,
    duration: i64,
}

#[derive(Default, Clone, Copy)]
struct Active {
    ticks: i64,
    left: i64,
    value: i64,
    original: i64,
}

impl Active {
    fn reset(&mut self) {
        self.left = 0;
        self.ticks = 0;
        self.value = 0;
    }
}

/// The simulator object, kept from one monster to the next as NMR keeps it.
pub(super) struct Sim {
    rnd: VbRnd,
    atk: [Attack; 5],
    between: [Between; 5],
    avg_phys: f64,
    avg_spell: f64,
}

impl Sim {
    pub(super) fn new() -> Sim {
        Sim {
            // VB's seed before any Randomize
            rnd: VbRnd(0x05_0000),
            atk: [Attack::default(); 5],
            between: [Between::default(); 5],
            avg_phys: 0.0,
            avg_spell: 0.0,
        }
    }

    /// `ResetValues`: all but the durations.
    fn reset(&mut self) {
        for a in &mut self.atk {
            *a = Attack {
                duration: a.duration,
                ..Attack::default()
            };
        }
        for b in &mut self.between {
            *b = Between {
                duration: b.duration,
                ..Between::default()
            };
        }
    }

    /// `CalculateMonsterAvgDmg`, then `RunSim`.
    pub(super) fn monster(&mut self, book: &Book, m: &Monster) -> Results {
        self.reset();
        let damage_bonus = book.item_bonus(m, &[4]);
        let accy_bonus = book.item_bonus(m, &[22, 105, 106]);
        for (x, &[ty, energy, chance, min_hit, max_hit, accu_spell, hit_spell]) in
            m.attacks.iter().enumerate()
        {
            if ty <= 0 || ty >= 4 {
                continue;
            }
            let a = &mut self.atk[x];
            a.ty = ty;
            a.energy = energy;
            a.chance = chance;
            if ty == 2 {
                match book.spells.get(&accu_spell) {
                    Some(s) => {
                        if s.target == 12 && s.ability(1) > -1 {
                            // an area spell doing damage through ability 1: the game will
                            // not cast it
                            a.duration = 0;
                            a.min = 0;
                            a.max = 0;
                            a.energy = 0;
                            continue;
                        }
                        a.resist = s.resists;
                        a.duration = s.duration(max_hit);
                        a.min = 0;
                        a.max = 0;
                        for (ab, mr) in [(1, 0), (17, 1), (8, 0)] {
                            let v = s.ability(ab);
                            if v >= 0 {
                                a.mr_resist = mr;
                                if v > 0 {
                                    a.min = v;
                                    a.max = v;
                                } else {
                                    a.min = s.min_damage(max_hit);
                                    a.max = s.max_damage(max_hit);
                                }
                            }
                        }
                    }
                    None => {
                        a.min = 0;
                        a.max = 0;
                    }
                }
                a.success = min_hit;
            } else {
                a.min = min_hit + damage_bonus;
                a.max = max_hit + damage_bonus;
                a.success = accu_spell + accy_bonus;
                if hit_spell > 0
                    && let Some(s) = book.spells.get(&hit_spell)
                {
                    a.resist = s.resists;
                    a.duration = s.duration(0);
                    if s.ability(1) >= 0 {
                        a.mr_resist = 0;
                        a.hit_min = s.min_damage(0);
                        a.hit_max = s.max_damage(0);
                    } else if s.ability(17) >= 0 {
                        a.mr_resist = 1;
                        a.hit_min = s.min_damage(0);
                        a.hit_max = s.max_damage(0);
                    } else {
                        a.hit_min = 0;
                        a.hit_max = 0;
                    }
                }
            }
        }
        for (x, &[spell, chance, level]) in m.between.iter().enumerate() {
            if spell <= 0 {
                continue;
            }
            let Some(s) = book.spells.get(&spell) else {
                continue;
            };
            let b = &mut self.between[x];
            b.resist = s.resists;
            b.chance = chance;
            b.duration = s.duration(level);
            for (ab, mr) in [(1, 0), (17, 1), (8, 0)] {
                let v = s.ability(ab);
                if v >= 0 {
                    b.mr_resist = mr;
                    if v > 0 {
                        b.min = v;
                        b.max = v;
                    } else {
                        b.min = s.min_damage(level);
                        b.max = s.max_damage(level);
                    }
                }
            }
        }
        self.run(m.energy)
    }

    /// `IsSpellResisted` against a player of 50 MR and no anti-magic.
    fn resisted(&mut self, resist: i64) -> bool {
        const MR: f64 = 50.0;
        if resist == 2 {
            let roll = self.rnd.number(1, 100);
            return roll as f64 <= MR / 2.0;
        }
        false
    }

    /// `RunSim`, dynamic, against a player of 50 MR (no AC, DR, dodge or anti-magic).
    fn run(&mut self, energy_per_round: i64) -> Results {
        const MAX_ROUNDS: i64 = 100_000;
        const DIFFERENCE: f64 = 0.01;
        let mut attempted = [0f64; 5];
        let mut total_attacks = 0f64;
        let done = |s: &Sim, avg: f64, attempted: [f64; 5], total: f64| Results {
            avg,
            phys: s.avg_phys,
            spell: s.avg_spell,
            att_true: std::array::from_fn(|x| {
                if attempted[x] > 0.0 && total > 0.0 {
                    round(attempted[x] / total, 3) * 100.0
                } else {
                    0.0
                }
            }),
        };
        if energy_per_round < 1 {
            return done(self, 0.0, attempted, 0.0);
        }
        let mut active_atk = [Active::default(); 5];
        let mut active_btw = [Active::default(); 5];
        let nothing = self
            .atk
            .iter()
            .all(|a| a.min <= 0 && a.max <= 0 && a.hit_min <= 0 && a.hit_max <= 0)
            && self.between.iter().all(|b| b.min <= 0 && b.max <= 0);
        if nothing {
            return done(self, 0.0, attempted, 0.0);
        }
        let (mut total, mut phys, mut spell) = (0f64, 0f64, 0f64);
        let mut remaining = 0i64;
        let mut current_avg = 0f64;
        let mut dynamic_count = 0i64;
        let mut round_no = 1i64;
        let mut finished_early = false;
        while round_no <= MAX_ROUNDS {
            // the second tick of spells cast 15 seconds ago
            for x in 0..5 {
                let a = &mut active_atk[x];
                if a.left > 0 && a.ticks == 3 {
                    total += a.value as f64;
                    spell += a.value as f64;
                    a.left -= 1;
                    if a.left < 1 {
                        a.reset();
                    }
                }
                let b = &mut active_btw[x];
                if b.left > 0 && b.ticks == 3 {
                    total += b.value as f64;
                    spell += b.value as f64;
                    b.left -= 1;
                    if b.left < 1 {
                        b.reset();
                    }
                }
            }
            remaining += energy_per_round;
            'attempts: for _ in 0..6 {
                let roll = self.rnd.number(1, 100);
                let mut last_type = 0;
                let mut last_energy = 0;
                for x in 0..5 {
                    let a = self.atk[x];
                    if !(roll <= a.chance && a.ty > 0) {
                        continue;
                    }
                    last_energy = a.energy;
                    last_type = a.ty;
                    if remaining < a.energy {
                        break;
                    }
                    attempted[x] += 1.0;
                    total_attacks += 1.0;
                    let mut hit = false;
                    let mut resisted = false;
                    let mut success = a.success as f64;
                    if a.ty != 2 {
                        // no AC
                        success = 99.0;
                    }
                    let hit_roll = self.rnd.number(1, 100);
                    if hit_roll as f64 <= success {
                        hit = true;
                    }
                    if hit {
                        let mut hit_spell_damage = 0i64;
                        let mut damage = self.rnd.number(a.min, a.max);
                        if a.ty == 2 {
                            active_atk[x].original = damage;
                            // MR 50 and no anti-magic reduce damage by nothing
                            if a.duration > 0 {
                                let act = &mut active_atk[x];
                                if act.left < 1 || act.value != damage {
                                    if self.resisted(a.resist) {
                                        resisted = true;
                                        hit = false;
                                        damage = 0;
                                    } else {
                                        let act = &mut active_atk[x];
                                        act.reset();
                                        act.left = a.duration;
                                        act.value = damage;
                                        damage = 0;
                                    }
                                } else {
                                    attempted[x] -= 1.0;
                                    total_attacks -= 1.0;
                                    continue;
                                }
                            } else if self.resisted(a.resist) {
                                resisted = true;
                                hit = false;
                                damage = 0;
                            }
                        } else {
                            // no DR
                            if a.hit_min > 0 || a.hit_max > 0 || a.duration > 0 {
                                hit_spell_damage = self.rnd.number(a.hit_min, a.hit_max);
                                active_atk[x].original = hit_spell_damage;
                                let mut show = false;
                                if a.duration > 0 {
                                    let act = &active_atk[x];
                                    if act.left < 1 || act.value != hit_spell_damage {
                                        if self.resisted(a.resist) {
                                            show = true;
                                            hit_spell_damage = 0;
                                        } else {
                                            let act = &mut active_atk[x];
                                            act.reset();
                                            act.left = a.duration;
                                            act.value = hit_spell_damage;
                                            show = true;
                                            hit_spell_damage = 0;
                                        }
                                    }
                                } else {
                                    if self.resisted(a.resist) {
                                        hit_spell_damage = 0;
                                    }
                                    show = true;
                                }
                                if !show {
                                    hit_spell_damage = 0;
                                }
                            }
                        }
                        if damage <= 0 {
                            damage = 0;
                        }
                        if hit {
                            total += (damage + hit_spell_damage) as f64;
                            spell += hit_spell_damage as f64;
                            if a.ty == 2 {
                                spell += damage as f64;
                            } else {
                                phys += damage as f64;
                            }
                            remaining -= a.energy;
                        }
                    }
                    if !hit {
                        if a.ty == 2 {
                            if a.duration == 0 || resisted {
                                remaining -= round(a.energy as f64 / 2.0, 0) as i64;
                            }
                        } else {
                            remaining -= a.energy;
                        }
                    }
                    break;
                }
                if last_type == 1 && remaining < last_energy {
                    break 'attempts;
                }
            }
            // between rounds: the first spell that goes off
            let roll = self.rnd.number(1, 100);
            for x in 0..5 {
                let b = self.between[x];
                if b.chance <= 0 || roll > b.chance {
                    continue;
                }
                let mut damage = self.rnd.number(b.min, b.max);
                active_btw[x].original = damage;
                let (mut applied, mut resisted, mut hit) = (false, false, false);
                if b.duration > 0 {
                    let act = &active_btw[x];
                    if act.left < 1 || act.value != damage {
                        if self.resisted(b.resist) {
                            resisted = true;
                        } else {
                            let act = &mut active_btw[x];
                            act.reset();
                            act.left = b.duration;
                            act.value = damage;
                            applied = true;
                        }
                    }
                    damage = 0;
                } else if self.resisted(b.resist) {
                    resisted = true;
                } else {
                    hit = true;
                }
                if applied || resisted {
                    break;
                }
                if hit {
                    total += damage as f64;
                    spell += damage as f64;
                    break;
                }
            }
            // spells running
            for x in 0..5 {
                for a in [&mut active_atk[x], &mut active_btw[x]] {
                    if a.left > 0 {
                        total += a.value as f64;
                        spell += a.value as f64;
                        a.left -= 1;
                        if a.ticks >= 3 {
                            a.ticks = 1;
                        } else {
                            a.ticks += 1;
                        }
                        if a.left < 1 {
                            a.reset();
                        }
                    }
                }
            }
            if dynamic_count > 1000 {
                let avg = round(total / round_no as f64, 3);
                if current_avg == 0.0 {
                    current_avg = avg;
                    if current_avg < 1.0 {
                        finished_early = true;
                        break;
                    }
                    dynamic_count = 0;
                } else {
                    let difference = round((1.0 - avg / current_avg).abs(), 4);
                    if difference < DIFFERENCE {
                        finished_early = true;
                        break;
                    }
                    current_avg = avg;
                    dynamic_count = 0;
                }
            }
            dynamic_count += 1;
            round_no += 1;
        }
        // a VB For loop that runs out leaves its counter one past the end
        let rounds = if finished_early {
            round_no
        } else {
            MAX_ROUNDS + 1
        } as f64;
        self.avg_phys = round(phys / rounds, 1);
        self.avg_spell = round(spell / rounds, 1);
        let avg = round(total / rounds, 1);
        done(self, avg, attempted, total_attacks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vb_rnd_is_vbs() {
        // what VB's Rnd gives first, unseeded
        let mut r = VbRnd(0x05_0000);
        assert_eq!((r.next() * 1e7).round(), 7_055_475.0);
        assert_eq!((r.next() * 1e7).round(), 5_334_240.0);
        assert!(
            (0..1000)
                .map(|_| r.number(1, 6))
                .all(|x| (1..=6).contains(&x))
        );
    }
}
