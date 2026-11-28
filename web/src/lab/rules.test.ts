// The cases are silvermere-rules' own (combat/tests.rs, spells/tests.rs), on the same sheets.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  ATTACK,
  BASH,
  dodgeChance,
  dynamicStats,
  effectiveCrit,
  energyRound,
  energyUsed,
  expNeeded,
  hitChance,
  isqrt,
  KICK,
  monsterFighter,
  mrDamage,
  playerFighter,
  PUNCH,
  resolve,
  secondary,
  SMASH,
  type Character,
  type ClassRec,
  type Fighter,
  type ItemRec,
  type Kit,
  type MonsterRec,
  type Rng,
} from './rules.ts';

/** Scripted rolls, each checked against its range. */
function fixed(rolls: number[]): Rng {
  return {
    range(lo, hi) {
      const v = rolls.shift();
      assert.ok(v !== undefined, 'a scripted roll');
      assert.ok(lo <= v && (v < hi || hi <= lo), `roll ${v} outside [${lo}, ${hi})`);
      return v;
    },
  };
}

/** Kasimir, the Kang Witchunter test sheet at level 1. */
const kang = (): Character => ({ level: 1, str: 79, intel: 30, wil: 45, agi: 42, hea: 74, cha: 30, randomhp: 0, encumpct: 18, evil: 0 });
const witchunter: ClassRec = { combat: 7, basehp: 0, hplevel: 0, magery: 0, magelvl: 0, exp: 0, slots: [] };
const item = (o: Partial<ItemRec>): ItemRec => ({ number: 0, name: '', type: 0, ac: 0, dr: 0, accuracy: 0, armtype: 0, mindmg: 0, maxdmg: 0, speed: 0, strreq: 0, hitmsg: 0, missmsg: 0, slots: [], negates: [], ...o });
const quarterstaff = item({ number: 100, type: 1, speed: 1200, mindmg: 2, maxdmg: 12, ac: 10, strreq: 30, hitmsg: 8237, missmsg: 8289 });

function kit(c = kang(), weapon: ItemRec | null = quarterstaff, armour: ItemRec[] = []): Kit {
  const sources = [...armour.map((a) => ({ item: a.slots })), ...(weapon ? [{ item: weapon.slots }] : [])];
  return { c, cls: witchunter, weapon, armour, sources, dyn: dynamicStats(sources), mabase: 1, hp: 20 };
}

const fighter = (av: number, dv: number): Fighter => ({ av, dv, defalign: 0, dr: 0, expval: 0, energy: 0, mindmg: 10, maxdmg: 20, dodge: 0, crit: 0, defmod: 0, fast: false });

test('energy a blow: skill, quick, divisor, cost', () => {
  assert.equal(energyUsed(1, 7, 42, 1200, 0), 721);
  assert.equal(energyUsed(0, 0, -150, 1200, 0), 50);
});

test('blows a round carry the energy left over', () => {
  assert.deepEqual(energyRound(0, 605), { blows: 1, leftover: 395 });
  assert.deepEqual(energyRound(395, 605), { blows: 2, leftover: 185 });
  assert.equal(energyRound(0, 100).blows, 6);
  assert.equal(energyRound(0, 0).blows, 5);
});

test('the Kang Witchunter sheet', () => {
  const f = playerFighter(kit(), ATTACK, 'evil');
  assert.equal(f.av, 58);
  assert.equal(f.energy, 605);
  assert.equal(f.crit, 1);
  assert.equal(f.dodge, 3);
  assert.equal(f.dv, 1);
  assert.deepEqual([f.mindmg, f.maxdmg], [2, 14]);
  assert.equal(playerFighter(kit(), BASH, 'evil').energy, 1000);
  assert.equal(playerFighter(kit(), SMASH, 'evil').energy, 1000);
});

test('stat terms truncate toward zero', () => {
  const at = (o: Partial<Character>) => playerFighter(kit({ ...kang(), ...o }), ATTACK, 'evil');
  assert.equal(at({}).av, at({ agi: 50 }).av - 1);
  assert.equal(at({ agi: 45 }).av, at({ agi: 50 }).av);
  assert.equal(at({}).dodge, at({ cha: 50 }).dodge - 4);
  assert.equal(at({ cha: 46 }).dodge, at({ cha: 50 }).dodge);
  assert.equal(at({ cha: 45 }).dodge, at({ cha: 50 }).dodge - 1);
});

test('fists keep 1-4 and the martial arts their own ranges', () => {
  const f = playerFighter(kit(kang(), null), ATTACK, 'evil');
  assert.deepEqual([f.mindmg, f.maxdmg], [1, 6]); // +2 most damage for 79 strength
  const k = kit(kang(), null);
  k.sources = [{ slots: [[29, 5], [30, 3]] }];
  const p = playerFighter(k, PUNCH, 'evil');
  // level 1, punch 5: (1*5/8+2, (1+3)*5/4+6) = (2, 11), +2 for strength
  assert.deepEqual([p.mindmg, p.maxdmg], [2, 13]);
  const kk = playerFighter(k, KICK, 'evil');
  assert.deepEqual([kk.mindmg, kk.maxdmg], [2, 9]);
});

test('worn armour adds its armour class, resistance and accuracy', () => {
  const chain = item({ type: 0, ac: 40, dr: 5, accuracy: -4, armtype: 7 });
  const f = playerFighter(kit(kang(), quarterstaff, [chain]), ATTACK, 'evil');
  assert.equal(f.dv, 5); // (10 + 40) / 10
  assert.equal(f.dr, 5);
  assert.equal(f.av, 58 - 4); // accuracy 0 -> 1 floor gone; -4 + 1 + 9 = 6, /2 = 3 vs 10/2 = 5
});

test('hit chance: two divisions, then held to 10..99', () => {
  assert.equal(hitChance(58, 0, 30, 0, false), 63);
  assert.equal(hitChance(58, 0, 40, 0, false), 34);
  assert.equal(hitChance(58, 0, 0, 0, false), 99);
  assert.equal(hitChance(58, 0, 30, 10, false), 34);
  assert.equal(hitChance(58, -15, 30, 0, false), 31);
  assert.equal(hitChance(11, 0, 0, 0, false), 10);
  assert.equal(hitChance(80, 0, 35, 0, true), 45);
  assert.equal(hitChance(300, 0, 1, 0, false), 99);
});

test('critical chance over 40 counts a third', () => {
  assert.equal(effectiveCrit(40), 40);
  assert.equal(effectiveCrit(70), 50);
});

test('dodge chance: ten times the rating over an eighth of the attack value', () => {
  assert.equal(dodgeChance(10, 80, false), 10);
  assert.equal(dodgeChance(50, 16, false), 95);
  assert.equal(dodgeChance(10, 8, false), 0);
  assert.equal(dodgeChance(50, 80, true), 10);
});

test('a blow: the roll under the chance lands', () => {
  const def = fighter(0, 30);
  assert.equal(resolve(fighter(58, 0), def, PUNCH, fixed([62, 99, 0])).outcome, 'hit');
  assert.equal(resolve(fighter(58, 0), def, PUNCH, fixed([63])).outcome, 'miss');
});

test('a negative dodge is hit at 99', () => {
  const def = { ...fighter(0, 30), dodge: -25 };
  assert.equal(resolve(fighter(20, 0), def, PUNCH, fixed([98, 76, 99, 0])).outcome, 'hit');
  assert.equal(resolve(fighter(20, 0), def, PUNCH, fixed([98, 75])).outcome, 'miss');
});

test('damage less a tenth of resistance; bash triples after it; armour that takes it all', () => {
  const def = { ...fighter(0, 0), dr: 50 };
  assert.deepEqual(resolve(fighter(58, 0), def, ATTACK, fixed([1, 99, 4])), { outcome: 'hit', damage: 9, critical: false });
  // bash: the range is raised 10% (11-22), the roll 0 is 11, less 5 is 6, tripled 18
  assert.deepEqual(resolve(fighter(58, 0), def, BASH, fixed([1, 99, 0])), { outcome: 'hit', damage: 18, critical: false });
  const thick = { ...fighter(0, 0), dr: 300 };
  assert.equal(resolve(fighter(58, 0), thick, ATTACK, fixed([1, 99, 0])).outcome, 'armour');
});

test('a critical runs from twice to four times the most', () => {
  const atk = { ...fighter(58, 0), crit: 30 };
  assert.deepEqual(resolve(atk, fighter(0, 0), ATTACK, fixed([1, 10, 0])), { outcome: 'hit', damage: 40, critical: true });
  assert.equal(atk.crit, 30, 'the attacker given is left as it was');
});

test('a dodged blow does nothing', () => {
  const def = { ...fighter(0, 0), dodge: 20 };
  assert.equal(resolve(fighter(80, 0), def, ATTACK, fixed([1, 99, 0, 5])).outcome, 'dodge');
});

test('isqrt', () => {
  assert.deepEqual([0, 1, 3, 4, 15, 16, 99].map(isqrt), [0, 1, 1, 2, 3, 4, 9]);
});

const rat: MonsterRec = {
  number: 1, name: 'giant rat', ac: 0, dr: 1, mr: 30, hp: 9, energy: 1000, alignment: 2, bsdefense: 0, exp: 9, level: 1, weapon: 0, spells: [], sources: [],
  attacks: [
    { type: 1, pct: 50, acc: 10, min: 2, max: 10, energy: 1000, hitmsg: 0, defmsg: 0, missmsg: 0, hitspell: 0 },
    { type: 1, pct: 100, acc: 70, min: 4, max: 20, energy: 1000, hitmsg: 0, defmsg: 0, missmsg: 0, hitspell: 0 },
  ],
};

test('the stock monster sheets', () => {
  const f = monsterFighter(rat, 0, 'neutral');
  assert.deepEqual([f.av, f.dv, f.dr, f.expval, f.energy, f.mindmg, f.maxdmg], [10, 0, 10, 9, 1000, 2, 10]);
  const k = monsterFighter({ ...rat, ac: 10, exp: 13, attacks: [{ ...rat.attacks[0]!, acc: 15, min: 1, max: 8, energy: 666 }] }, 0, 'neutral');
  assert.deepEqual([k.av, k.dv, k.dr, k.expval, k.energy, k.mindmg, k.maxdmg], [15, 10, 10, 13, 666, 1, 8]);
});

test('a monster defends against the other alignment with Defence vs Evil (sic)', () => {
  const m = { ...rat, sources: [{ slots: [[25, 7], [24, 3]] as [number, number][] }] };
  assert.equal(monsterFighter(m, 0, 'good').defalign, 3);
  assert.equal(monsterFighter(m, 0, 'evil').defalign, 0); // chaotic evil against evil: one group
  assert.equal(monsterFighter(m, 0, 'neutral').defalign, 0);
});

test('hit points, martial-arts base and magic resistance', () => {
  const c = { ...kang(), level: 10, randomhp: 12 };
  const s = secondary(c, { ...witchunter, basehp: 6 }, { hpbonus: 2, exp: 0, slots: [] }, []);
  // 74/2 + 10*6 + 10*2 + (24*10)/16 + 12 = 37 + 60 + 20 + 15 + 12
  assert.equal(s.maxhp, 144);
  assert.equal(s.mabase, 1);
  assert.equal(s.mr, Math.trunc((30 + 45 * 3) / 4));
});

test('magic resistance softens Inflict Damage on Armour', () => {
  assert.equal(mrDamage(100, 50, false), 100);
  assert.equal(mrDamage(100, 51, false), 99);
  assert.equal(mrDamage(100, 90, false), 80);
  assert.equal(mrDamage(100, 200, false), 50);
  // (sic) below 50 the unsigned test wraps: the full cut
  assert.equal(mrDamage(100, 40, false), 50);
  assert.equal(mrDamage(100, 100, true), 50);
  assert.equal(mrDamage(100, 0, true), 100);
});

test('dynamic stats: the best Alter AV, energy use averaged', () => {
  const d = dynamicStats([{ slots: [[22, 5], [105, 9], [87, 80]] }, { item: [[87, 120], [2, 4], [2, 3]] }]);
  assert.equal(d.avbonus, 9);
  assert.equal(d.energyuse, 100);
  assert.equal(d.acbonus, 7);
});

test('experience to reach a level', () => {
  const kang = 65;
  assert.deepEqual([1, 2, 3, 4, 5, 6, 7, 8, 9].map((l) => expNeeded(l, kang)), [0, 1_650, 3_300, 6_050, 11_091, 19_013, 32_593, 52_963, 86_064]);
  // the 64-bit oracle of silvermere's tests, past the 32-bit rescaling
  let rem = 1650;
  let bil = 0;
  const T = [[1, 1], [40, 20], [44, 24], [44, 24], [48, 28], [48, 28], [52, 32], [52, 32], [56, 36], [56, 36], [60, 40], [60, 40], [65, 45], [65, 45], [70, 50], [70, 50], [75, 55], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [23, 20]];
  for (let step = 1; step < 75; step++) {
    const [n, d] = step <= 25 ? T[step]! : step <= 53 ? [115, 100] : step <= 56 ? [109, 100] : [108, 100];
    let h = 0;
    while (rem * n! >= 2 ** 32) {
      rem = Math.floor(rem / 100);
      h++;
    }
    rem = Math.floor((rem * n!) / d!) * 100 ** h;
    const carry = (bil * n! * 1_000_000) % 2 ** 32;
    bil += Math.floor(carry / 1e9);
    const total = (rem + (carry % 1e9)) % 2 ** 32;
    bil += Math.floor(total / 1e9);
    rem = total % 1e9;
    assert.equal(expNeeded(step + 2, kang), bil * 1e9 + rem, `level ${step + 2}`);
  }
});
