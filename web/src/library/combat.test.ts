import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  arrivalLine,
  attacksOf,
  blowLines,
  blowsOf,
  COLOUR,
  fmtMsg,
  nameOdds,
  roundOf,
  SAMPLE_VICTIM,
  shares,
  talkOdds,
  topicsOf,
  verbsOf,
  type Attack,
  type Lines,
} from './combat.ts';

const plain = (s: string) => s.replace(/\x1b\[[0-9;]*m/g, '');

test('a line is filled in as the game fills it', () => {
  assert.equal(fmtMsg('The %s bites you for %d damage!', ['giant rat', 7]), 'The giant rat bites you for 7 damage!');
  assert.equal(fmtMsg('100%% sure, %ld left', [3]), '100% sure, 3 left');
  // past the last argument: nothing
  assert.equal(fmtMsg('%s and %s', ['one']), 'one and ');
  // a percent before anything else is dropped; one at the end is printed
  assert.equal(fmtMsg('50%x off%', []), '50x off%');
  assert.equal(fmtMsg('%l', []), 'l');
});

const attack = (a: Partial<Attack>): Attack => ({ type: 1, pct: 0, acc: 10, min: 1, max: 4, energy: 500, hit: 0, dodge: 0, miss: 0, spell: 0, ...a });

test('attack chances add up as they go', () => {
  assert.deepEqual(shares([attack({ pct: 50 }), attack({ pct: 80 }), attack({ pct: 100 })]), { share: [50, 30, 20], rest: 0 });
  // a later slot under an earlier one is never picked
  assert.deepEqual(shares([attack({ pct: 100 }), attack({ pct: 20 }), attack({ pct: 30 })]), { share: [100, 0, 0], rest: 0 });
  assert.deepEqual(shares([attack({ pct: 30 }), attack({ pct: 0 })]), { share: [30, 0], rest: 70 });
  assert.deepEqual(shares([attack({ pct: 250 })]), { share: [100], rest: 0 });
});

test("an attack's blows are what its energy buys, at most six", () => {
  assert.equal(blowsOf(1000, 1000), 1);
  assert.equal(blowsOf(1000, 300), 3);
  assert.equal(blowsOf(1000, 250), 4);
  assert.equal(blowsOf(1000, 0), 6);
  // dearer than the whole round: it never lands
  assert.equal(blowsOf(500, 1000), 0);
  assert.equal(blowsOf(1000, -1), 0);
});

test('a record names its attacks by slot', () => {
  const rec: Record<string, number> = { 'Attack Type 1': 2, 'Attack % 1': 40, 'Attack Accu/Spell 1': 23 };
  const a = attacksOf((c) => rec[c] ?? 0);
  assert.equal(a.length, 5);
  assert.deepEqual([a[1]!.type, a[1]!.pct, a[1]!.acc], [2, 40, 23]);
});

// a giant rat's messages in the stock world
const BITE: Lines = ['The %s bites you for %d damage!', 'The %s bites %s for %s damage!', 'The giant rat falls to the ground with a tortured squeak.'];
const LUNGE_DEF: Lines = [
  'The %s %slunges at you, but your armour deflects the blow!',
  'The %s %slunges at %s, but %s armour deflects the blow!',
  'The %s %slunges at %syou, but you dodge out of the way!',
];
const LUNGE_MISS: Lines = ['The %s %slunges at %s, %sbut %s dodges out of the way!', 'The %s %slunges at %syou!', 'The %s %slunges at %s! %s'];

test('each outcome reads as the game sends it', () => {
  const texts = { hit: BITE, def: LUNGE_DEF, miss: LUNGE_MISS };
  const verbs = verbsOf(null);
  const line = (o: 'hit' | 'armour' | 'dodge' | 'miss') => {
    const l = blowLines('giant rat', verbs, texts, SAMPLE_VICTIM, o, 7);
    return [plain(l.you), plain(l.room)];
  };
  assert.deepEqual(line('hit'), ['The giant rat bites you for 7 damage!', 'The giant rat bites Rowan for 7 damage!']);
  assert.deepEqual(line('armour'), ['The giant rat lunges at you, but your armour deflects the blow!', 'The giant rat lunges at Rowan, but her armour deflects the blow!']);
  assert.deepEqual(line('dodge'), ['The giant rat lunges at you, but you dodge out of the way!', 'The giant rat lunges at Rowan, but she dodges out of the way!']);
  assert.deepEqual(line('miss'), ['The giant rat lunges at you!', 'The giant rat lunges at Rowan! ']);
  assert.ok(blowLines('giant rat', verbs, texts, SAMPLE_VICTIM, 'hit', 7).you.startsWith(COLOUR.hitYou));
});

test("without messages a blow reads in the game's own words, with the weapon's", () => {
  const verbs = verbsOf({ name: 'club', hit: ['smack', 'smacks', 'smacks'], miss: ['swing', 'swings at', 'swings at'] });
  const none = { hit: null, def: null, miss: null };
  const l = (o: 'hit' | 'armour' | 'dodge' | 'miss') => plain(blowLines('orc', verbs, none, SAMPLE_VICTIM, o, 5).you);
  assert.equal(l('hit'), 'Orc smacks you for 5 damage!');
  assert.equal(l('armour'), "Orc's swings at hits you, but your armour deflects.");
  assert.equal(l('dodge'), 'Orc swings at you with club, but you dodge!');
  assert.equal(l('miss'), 'Orc swings at you with club.');
  // a dodge needs both messages
  assert.equal(plain(blowLines('orc', verbs, { ...none, def: LUNGE_DEF }, SAMPLE_VICTIM, 'dodge', 0).you), 'Orc swings at you with club, but you dodge!');
  // without a miss message the misses use the hit's words
  assert.deepEqual(verbsOf({ name: 'x', hit: ['a', 'b', 'c'], miss: null }).miss, ['a', 'b', 'c']);
});

/** A rand() that answers these rolls (0 to 99) in turn. */
const rolls = (...r: number[]) => {
  let i = 0;
  return () => (r[i++ % r.length]! + 0.5) / 100;
};

test('a round is blows while the energy lasts', () => {
  // one attack at 300 a blow out of 1000: three blows
  const one = [attack({ pct: 100, energy: 300 }), attack({}), attack({}), attack({}), attack({})];
  const r = roundOf(one, 1000, rolls(0, 10, 0));
  assert.equal(r.steps.length, 3);
  assert.ok(r.steps.every((s) => s.kind === 'blow' && s.slot === 0 && s.outcome === 'hit'));
  assert.equal(r.after, null);
  // an attack dearer than the round never lands
  assert.equal(roundOf([attack({ pct: 100, energy: 1500 })], 1000, rolls(0)).steps.length, 0);
});

test('a roll above every chance keeps the last attack', () => {
  const two = [attack({ pct: 30, energy: 200 }), attack({ pct: 60, energy: 200 })];
  // 40 picks the second, 90 keeps it, 10 picks the first
  const r = roundOf(two, 600, rolls(40, 99, 90, 99, 10, 99));
  assert.deepEqual(
    r.steps.map((s) => s.slot),
    [1, 1, 0],
  );
});

test('slots without a type do nothing, spells are cast, rob strikes with the first', () => {
  const mixed = [attack({ pct: 20, energy: 500 }), attack({ type: 0, pct: 40 }), attack({ type: 2, pct: 60, acc: 23, min: 50, max: 3, energy: 200 }), attack({ type: 3, pct: 100 })];
  const r = roundOf(mixed, 1000, rolls(30, 50, 70, 99, 10));
  assert.deepEqual(
    r.steps.map((s) => s.kind),
    ['idle', 'cast', 'blow'],
  );
  assert.deepEqual(r.steps[1], { kind: 'cast', slot: 2, spell: 23, chance: 50, level: 3 });
  // the rob became the first attack's blow, which left too little for another
  assert.equal(r.steps[2]!.slot, 0);
  // the rob is idle when the first attack is no normal one
  const robs = [attack({ type: 0, pct: 0 }), attack({ type: 3, pct: 100 })];
  assert.deepEqual(roundOf(robs, 1000, rolls(50)).steps[0], { kind: 'idle', slot: 1, rob: true });
});

test('after the round a spell may be cast', () => {
  const spells = [
    { spell: 7, pct: 30, level: 2 },
    { spell: 8, pct: 50, level: 4 },
  ];
  const a = [attack({ pct: 100, energy: 1000 })];
  assert.deepEqual(roundOf(a, 1000, rolls(0, 20, 0, 40), spells).after, spells[1]);
  assert.equal(roundOf(a, 1000, rolls(0, 20, 0, 80), spells).after, null);
});

test('the arrival line puts the name first only with a name list', () => {
  const creeps: Lines = ['A %s creeps into the room from %s.', '', ''];
  assert.equal(plain(arrivalLine(26, creeps, 'nasty giant rat', true, 'east')), 'A nasty giant rat creeps into the room from the east.');
  assert.equal(plain(arrivalLine(26, creeps, 'giant rat', false, 'east')), 'A the east creeps into the room from giant rat.');
  assert.equal(plain(arrivalLine(0, null, 'giant rat', false, null)), 'giant rat just arrived from nowhere.');
  assert.equal(plain(arrivalLine(9, null, 'giant rat', false, 'west')), 'giant rat moves into the room from the west.');
  assert.equal(arrivalLine(9, ['', '', ''], 'giant rat', false, 'west'), '');
});

test('a name list favours its first lines', () => {
  const odds = nameOdds('B:nasty\nB:angry\nN:\n\n', 'giant rat');
  assert.deepEqual(
    odds.map((o) => o.name),
    ['nasty giant rat', 'angry giant rat', 'giant rat'],
  );
  assert.equal(Math.round(odds[0]!.pct), 10);
  assert.equal(Math.round(odds[1]!.pct), 9);
  // N: and the blank last line are both its own name
  assert.equal(Math.round(odds[2]!.pct), 81);
  assert.deepEqual(nameOdds('', 'bat'), [{ name: 'bat', pct: 100 }]);
});

test('talk lines take the rolls under their chance', () => {
  assert.deepEqual(talkOdds('10 The rat squeaks.\r\n30 The rat sniffs the air.\n20 Never said.\n'), [
    { text: 'The rat squeaks.', pct: 10 },
    { text: 'The rat sniffs the air.', pct: 20 },
  ]);
});

test('greet text lists what a monster may be asked about', () => {
  assert.deepEqual(topicsOf('wares:80\naxes:81 \nmail:82'), [
    { words: 'wares', block: 80 },
    { words: 'axes', block: 81 },
    { words: 'mail', block: 82 },
  ]);
  assert.deepEqual(topicsOf('no topics here'), []);
});
