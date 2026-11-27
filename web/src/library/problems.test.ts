import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Finding } from '../api.ts';
import type { Grid } from '../layouts.ts';
import {
  allFixes,
  byLevel,
  byteOf,
  caseLike,
  checkValue,
  clampTo,
  foundProblems,
  labelsOf,
  leftoverProblem,
  leftoverSlots,
  refOf,
  replaceWord,
  storable,
  trailingSpaces,
  TWO_SPACES,
  type Problem,
  type Targets,
} from './problems.ts';
import type { Cell } from './sections.ts';

const integer = { type: 'long', size: null, range: [-32768, 32767] } as const;
const byte = { type: 'long', size: null, range: [0, 255] } as const;
const unsigned = { type: 'double', size: null, range: null } as const;
const name = { type: 'text', size: 29, range: null } as const;

test('a number column takes a whole number in its range', () => {
  assert.equal(checkValue(integer, '400'), null);
  assert.equal(checkValue(integer, ''), null);
  assert.equal(checkValue(integer, '-32768'), null);
  assert.equal(checkValue(integer, '40000'), 'Takes -32,768 to 32,767.');
  assert.equal(checkValue(byte, '-1'), 'Takes 0 to 255.');
  assert.equal(checkValue(byte, '2.5'), 'Takes whole numbers only.');
  assert.equal(checkValue(byte, 'ten'), 'Not a number.');
  // NMR clamps an unsigned value, so any number will do
  assert.equal(checkValue(unsigned, '5000000000'), null);
  assert.deepEqual([clampTo([0, 255], 300), clampTo([0, 255], -4), clampTo([0, 255], 2.6)], [255, 0, 3]);
});

test('text takes what Windows-1252 has, and a text block drops what is past ß', () => {
  assert.equal(checkValue(name, 'café “Dwarven” axe…'), null);
  assert.equal(checkValue(name, 'axe ✨'), "The game can't store “✨”.");
  assert.equal(checkValue(name, 'x'.repeat(30)), 'Takes 29 characters at most.');
  const memo = { type: 'memo', size: null, range: null } as const;
  assert.equal(checkValue(memo, 'Straße', true), null);
  assert.equal(checkValue(memo, 'café', true), 'A text block drops “é” (it keeps characters up to ß).');
  assert.equal(byteOf('€'), 0x80);
  assert.equal(byteOf('Ÿ'), 0x9f);
  assert.equal(byteOf('✨'), null);
  // ␛ is how an input shows ESC: it stays
  assert.equal(storable('␛[1mcafé ✨'), '␛[1mcafé ');
  assert.equal(storable('café', true), 'caf');
});

const targets: Targets = {
  targetOf: (table, col) => ({ 'Monsters/Weapon Number': 'Items', 'Rooms/Shop Number': 'Shops', 'Monsters/Greet Txt': 'Textblocks' })[`${table}/${col}`],
  exitLinks: (type, para) => (type === 2 && para === 'Para1' ? 'Items' : undefined),
};

test('a number names a record by its table and key', () => {
  const room: Record<string, Cell> = { 'Map Number': 3, 'Exit 0': 12, 'Type 0': 0, 'Exit 1': 40, 'Type 1': 8, 'Para1 1': 5, 'Exit 2': 7, 'Type 2': 2, 'Para1 2': 868, 'Shop Number': 4 };
  const get = (c: string) => room[c] ?? 0;
  assert.deepEqual(refOf('Rooms', 'Exit 0', get, targets), { table: 'Rooms', key: [3, 12] });
  // a Map Change exit goes to the map its Para1 names
  assert.deepEqual(refOf('Rooms', 'Exit 1', get, targets), { table: 'Rooms', key: [5, 40] });
  // a key exit's Para1 is its key
  assert.deepEqual(refOf('Rooms', 'Para1 2', get, targets), { table: 'Items', key: [868] });
  assert.equal(refOf('Rooms', 'Para1 1', get, targets), null);
  assert.deepEqual(refOf('Rooms', 'Shop Number', get, targets), { table: 'Shops', key: [4] });
  const mon = (c: string) => ({ 'Weapon Number': 868, 'Greet Txt': 907 })[c] ?? 0;
  assert.deepEqual(refOf('Monsters', 'Greet Txt', mon, targets), { table: 'Textblocks', key: [907, 0] });
  assert.equal(refOf('Monsters', 'Death Msg', mon, targets), null);
});

const abilities: Grid = {
  columns: [
    { template: 'Ability #', label: 'Ability' },
    { template: 'Ability Value #', label: 'Value' },
  ],
  from: 0,
  to: 3,
};

test('an empty slot that still holds a value is a leftover, cleared in one fix', () => {
  const item: Record<string, Cell> = { 'Ability 0': 22, 'Ability Value 0': 5, 'Ability Value 1': TWO_SPACES, 'Ability Value 3': 10 };
  const get = (c: string) => item[c] ?? 0;
  assert.deepEqual(leftoverSlots(abilities, get), [1, 3]);
  const p = leftoverProblem(abilities, 'abilities', get, () => true)!;
  assert.equal(p.level, 'tidy');
  assert.equal(p.text, '2 empty slots of abilities still hold values (8224 is two spaces an old tool left). The game skips them.');
  assert.deepEqual(p.fix!.set, { 'Ability 1': '0', 'Ability Value 1': '0', 'Ability 3': '0', 'Ability Value 3': '0' });
  assert.equal(leftoverProblem(abilities, 'abilities', (c) => (c === 'Ability 0' ? 3 : 0), () => true), null);
  // who may use an item: separate lists, no slot to leave over
  assert.deepEqual(leftoverSlots({ ...abilities, separate: true }, get), []);
});

test('a text that ends in spaces is trimmed at its end only', () => {
  assert.equal(trailingSpaces('Data', 'The text', 'All  is well.'), null);
  assert.deepEqual(trailingSpaces('Data', 'The text', 'spits to the side.  \n')!.fix!.set, { Data: 'spits to the side.\n' });
  assert.deepEqual(trailingSpaces('Data', 'The text', 'a \nb\t')!.fix!.set, { Data: 'a \nb' });
});

test('problems come worst first, and their fixes join', () => {
  const p = (level: Problem['level'], col: string): Problem => ({ level, text: col, cols: [col], fix: { label: 'Fix', set: { [col]: '0' } } });
  assert.deepEqual(
    byLevel([p('tidy', 'a'), p('error', 'b'), p('warn', 'c')]).map((x) => x.level),
    ['error', 'warn', 'tidy'],
  );
  assert.deepEqual(allFixes([p('tidy', 'a'), p('error', 'b')]), { a: '0', b: '0' });
});

test("a card's columns are named as it names them", () => {
  const labels = labelsOf([
    { id: 'a', title: 'Who', fields: [{ col: 'Hit Points', label: 'Hitpoints' }, { col: 'AC' }] },
    {
      id: 'b',
      title: 'Attacks',
      grid: { columns: [{ template: 'Attack Hit Msg #', label: 'Hit Msg' }, { template: 'Attack % #' }], from: 0, to: 4, across: 'Attack' },
    },
    { id: 'c', title: 'Abilities', grid: abilities },
  ]);
  assert.equal(labels.get('Hit Points'), 'Hitpoints');
  assert.equal(labels.get('AC'), 'AC');
  assert.equal(labels.get('Attack Hit Msg 2'), 'Attack 3 Hit Msg');
  assert.equal(labels.get('Attack % 0'), 'Attack 1 Attack %');
  // a grid not laid across slots keeps the columns' own names
  assert.equal(labels.get('Ability 2'), undefined);
});

test('a typo is put right in the case it was written in', () => {
  assert.equal(caseLike('exchange', 'Exhange'), 'Exchange');
  assert.equal(caseLike('exchange', 'EXHANGE'), 'EXCHANGE');
  assert.equal(caseLike('exchange', 'exhange'), 'exchange');
  assert.equal(replaceWord('Exhange it in exhange, not exhanged.', 'exhange', 'exchange'), 'Exchange it in exchange, not exhanged.');
  assert.equal(replaceWord('␛[0mexhange', 'exhange', 'exchange'), '␛[0mexchange');
});

test('the checks across the world become the problems of the records they concern', () => {
  const base = { refs: [], fix: {}, word: null, suggest: [], text: '' };
  const found: Finding[] = [
    { ...base, check: 'spelling', level: 'note', about: { table: 'Messages', key: [1459] }, column: 'Line 1', bits: ['“exhange” may be “exchange”.'], word: 'exhange', suggest: ['exchange'], text: 'typo' },
    { ...base, check: 'placeholders', level: 'warn', about: { table: 'Messages', key: [1459] }, column: 'Line 2', bits: ['blank'], fix: { 'Line 2': '0' }, text: 'blank' },
    { ...base, check: 'prices', level: 'warn', about: { table: 'Textblocks', key: [921, 0] }, column: 'Data', bits: ['Charges 200 gold, but ', { ref: { t: 'Messages', n: 1459 } }, ' says 2,000.'], text: 'price' },
    { ...base, check: 'spawns', level: 'ok', about: { table: 'Messages', key: [1459] }, column: null, bits: ['fine'], text: 'fine' },
  ];
  const get = (c: string): Cell => (c === 'Line 1' ? 'He hands you the axe in exhange.' : '');
  const ps = foundProblems(found, 'Messages', [1459], get);
  assert.equal(ps.length, 3);
  assert.deepEqual(ps[0], {
    level: 'tidy',
    text: 'typo',
    cols: ['Line 1'],
    bits: ['“exhange” may be “exchange”.'],
    fix: { label: 'Make it “exchange”', set: { 'Line 1': 'He hands you the axe in exchange.' } },
    accept: 'exhange',
  });
  assert.deepEqual(ps[1]!.fix, { label: "Use the game's own words", set: { 'Line 2': '0' } });
  // one that rests on the record names what it is about, and fixes nothing here
  assert.deepEqual(ps[2], {
    level: 'warn',
    text: 'price',
    cols: [],
    bits: [{ ref: { t: 'Textblocks', n: 921 } }, ': ', 'Charges 200 gold, but ', { ref: { t: 'Messages', n: 1459 } }, ' says 2,000.'],
  });
});
