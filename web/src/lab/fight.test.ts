import assert from 'node:assert/strict';
import { test } from 'node:test';
import { averageRandomHp, blowsAverage, fightOut, kitOf, movesOf, oddsOf, roundOf, sheetOf, startOf, youLine, type Fight, type Player } from './fight.ts';
import { seeded, type ItemRec, type MonsterRec, type Rng, type SpellRec } from './rules.ts';

const plain = (s: string) => s.replace(/\x1b\[[0-9;]*m/g, '');

const item = (o: Partial<ItemRec>): ItemRec => ({ number: 0, name: '', type: 0, ac: 0, dr: 0, accuracy: 0, armtype: 0, mindmg: 0, maxdmg: 0, speed: 0, strreq: 0, hitmsg: 0, missmsg: 0, slots: [], negates: [], ...o });
const quarterstaff = item({ number: 100, name: 'quarterstaff', type: 1, speed: 1200, mindmg: 2, maxdmg: 12, ac: 10, strreq: 30, hitmsg: 1 });

const kasimir = (): Player => ({
  name: 'Kasimir',
  c: { level: 1, str: 79, intel: 30, wil: 45, agi: 42, hea: 74, cha: 30, randomhp: 0, encumpct: 18, evil: 0 },
  cls: { combat: 7, basehp: 6, hplevel: 6, magery: 0, magelvl: 0, exp: 0, slots: [] },
  race: { hpbonus: 0, exp: 0, slots: [] },
  weapon: quarterstaff,
  armour: [],
  move: 5,
});

const rat: MonsterRec = {
  number: 1, name: 'giant rat', ac: 0, dr: 1, mr: 30, hp: 9, energy: 1000, alignment: 2, bsdefense: 0, exp: 9, level: 1, weapon: 0, spells: [], sources: [],
  attacks: [{ type: 1, pct: 100, acc: 10, min: 2, max: 10, energy: 500, hitmsg: 0, defmsg: 0, missmsg: 0, hitspell: 0 }],
};

const bolt: SpellRec = { number: 7, name: 'chill', level: 1, resmode: 2, type: 0, element: 0, target: 0, min: 5, max: 5, minInc: 0, minLvls: 0, maxInc: 0, maxLvls: 0, msg: 0, alt: false, slots: [[17, 0]] };

const world = { lines: (n: number) => (n === 1 ? (['whap|smack', 'whaps|smacks', 'whaps|smacks'] as [string, string, string]) : null), spell: (n: number) => (n === 7 ? bolt : null), weapon: () => null };
const fight = (o: Partial<MonsterRec> = {}, p = kasimir()): Fight => ({ player: p, monster: { ...rat, ...o }, world });

/** Rolls from a list, then 0s. */
const rolls = (list: number[]): Rng => ({ range: (lo, hi) => Math.min(Math.max(list.length ? list.shift()! : lo, lo), Math.max(lo, hi - 1)) });

test('the sheet: blows a round, hit chances both ways', () => {
  const s = sheetOf(kasimir(), rat);
  assert.equal(s.you.av, 58);
  assert.equal(s.you.energy, 605);
  assert.ok(Math.abs(s.you.blows - 1000 / 605) < 0.05);
  assert.equal(s.you.hit, 99);
  assert.equal(s.mon.attacks.length, 1);
  assert.equal(s.mon.attacks[0]!.hit, 10); // AV 10 has no attack power
  assert.equal(s.mon.attacks[0]!.blows, 2);
  assert.equal(s.mon.exp, 9);
  assert.equal(s.you.maxhp, kitOf(kasimir()).maxhp);
});

test('one blow, then two: the energy carries', () => {
  assert.ok(Math.abs(blowsAverage(605) - 1000 / 605) < 0.05);
  const f = fight({ hp: 1000 });
  let st = startOf(f);
  const rng = seeded(3);
  const mine = (lines: { who: string }[]) => lines.filter((l) => l.who === 'you').length;
  let r = roundOf(f, st, rng);
  assert.equal(mine(r.lines), 1);
  assert.equal(r.state.carry, 395);
  r = roundOf(f, (st = r.state), rng);
  assert.equal(mine(r.lines), 2);
});

test('the monster spends its energy: two blows of 500', () => {
  const f = fight({ hp: 1000 });
  const r = roundOf(f, startOf(f), seeded(9));
  assert.equal(r.lines.filter((l) => l.who === 'mon').length, 2);
});

test('the player reads the game’s lines', () => {
  const v = { hit: ['whap', 'whaps', 'whaps'] as [string, string, string], miss: ['whap', 'whaps', 'whaps'] as [string, string, string] };
  assert.equal(plain(youLine(v, 'giant rat', { outcome: 'hit', damage: 7, critical: true })), 'You critically whap giant rat for 7 damage!');
  assert.equal(plain(youLine(v, 'giant rat', { outcome: 'armour', damage: 0, critical: false })), 'Your whap giant rat hits, but glances off its armour.');
  assert.equal(plain(youLine(v, 'giant rat', { outcome: 'dodge', damage: 0, critical: false })), 'You whap giant rat who dodges your attack!');
  assert.equal(plain(youLine(v, 'giant rat', { outcome: 'miss', damage: 0, critical: false })), 'You whap giant rat!');
});

test('a kill ends the round with the experience', () => {
  const f = fight({ hp: 1 });
  const r = roundOf(f, startOf(f), seeded(1));
  // the rat is hit 98 times in 99: try seeds until one lands first
  const end = r.state.monHp <= 0 ? r : roundOf(f, startOf(f), seeded(2));
  assert.equal(end.state.monHp, 0);
  assert.equal(plain(end.lines.at(-1)!.text), 'You gain 9 experience.');
});

test('a spell attack: cast roll, resist roll, magic resistance', () => {
  const caster = { attacks: [{ type: 2, pct: 100, acc: 7, min: 100, max: 1, energy: 1000, hitmsg: 0, defmsg: 0, missmsg: 0, hitspell: 0 }] };
  const f = fight({ ...caster, hp: 1000 });
  // the verb pick 0; the player's blow: hit 1, crit 99, damage 0, and the blow the energy does not cover rolled all the same; the monster: slot roll 0, cast roll 0, resist roll 98, damage roll 0
  const r = roundOf(f, startOf(f), rolls([0, 1, 99, 0, 1, 99, 0, 0, 0, 98, 0]));
  const cast = r.lines.find((l) => l.who === 'mon')!;
  assert.equal(cast.outcome, 'cast');
  assert.equal(plain(cast.text), 'Giant rat cast chill on you.');
  // mr (30 + 45*3)/4 = 41: below 50 the unsigned test gives the full cut (sic), 5 -> 3
  assert.equal(cast.damage, 3);
  // resisted: a roll at or under 41/2 = 20
  const res = roundOf(f, startOf(f), rolls([0, 1, 99, 0, 1, 99, 0, 0, 0, 20]));
  assert.equal(res.lines.find((l) => l.who === 'mon')!.outcome, 'resisted');
});

test('moves come from the abilities', () => {
  const p = kasimir();
  assert.deepEqual(movesOf(p).map((m) => m.move), [5]);
  p.cls.slots = [[31, 1], [29, 2]];
  assert.deepEqual(movesOf(p).map((m) => m.move), [5, 6, 1]);
});

test('many fights from one seed come out the same', () => {
  const f = fight({ hp: 30 });
  const a = oddsOf(f, 200, 5);
  assert.deepEqual(a, oddsOf(f, 200, 5));
  assert.equal(a.wins + a.stalled <= 200, true);
  assert.ok(a.wins > 150, `Kasimir beats a rat: ${a.wins}`);
  const one = fightOut(f, seeded(4), 200, true);
  assert.equal(one.rounds_.length, one.rounds);
});

test('average rolled hit points', () => {
  assert.equal(averageRandomHp(1, 6), 0);
  assert.equal(averageRandomHp(11, 6), 30);
});
