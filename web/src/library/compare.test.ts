import assert from 'node:assert/strict';
import { test } from 'node:test';
import { around, closest, compareFields, plainText, same } from './compare.ts';
import type { Cell } from './sections.ts';

const row = (label: string, vals: Record<string, number>) => ({ label, get: (c: string): Cell => vals[c] ?? 0 });

test('the closest records are nearest by every number, each by its spread', () => {
  const mine = row('me', { 'Hit Points': 100, Experience: 500 });
  const rows = [
    row('a', { 'Hit Points': 400, Experience: 500 }),
    row('b', { 'Hit Points': 110, Experience: 900 }),
    row('c', { 'Hit Points': 120, Experience: 520 }),
    row('d', { 'Hit Points': 100, Experience: 500 }),
    row('e', { 'Hit Points': 1000, Experience: 2000 }),
  ];
  assert.deepEqual(
    closest(mine.get, rows, ['Hit Points', 'Experience'], 3).map((r) => r.label),
    ['d', 'c', 'b'],
  );
  // without numbers to go by, the first ones
  assert.deepEqual(
    closest(mine.get, rows, [], 2).map((r) => r.label),
    ['a', 'b'],
  );
});

test("the fields compared are the card's, under its headings, without the key or slots", () => {
  const has = (c: string) => c !== 'Gone';
  const groups = compareFields(
    [
      { id: 'who', title: 'Who it is', fields: [{ col: 'Number' }, { col: 'Name' }, { col: 'Cost', with: 'Cost Type' }, { col: 'Gone' }] },
      { id: 'fight', title: 'In a fight', fields: [{ col: 'Min Hit', label: 'Min dmg' }, { col: 'Name' }], grid: { columns: [{ template: 'Ability #' }], from: 0, to: 9 } },
      { id: 'see', title: 'What players see', text: { cols: ['Desc1', 'Desc2'] } },
      { id: 'where', title: 'Where it is', where: true },
    ],
    ['Number'],
    has,
  );
  assert.deepEqual(
    groups.map((g) => [g.title, g.fields.map((f) => `${f.col}:${f.label}`)]),
    [
      ['Who it is', ['Name:Name', 'Cost:Cost', 'Cost Type:Cost Type']],
      ['In a fight', ['Min Hit:Min dmg']],
      ['What players see', ['Desc1:Desc1', 'Desc2:Desc2']],
    ],
  );
});

test('values are the same as a record holds them', () => {
  assert.equal(same(5, 5), true);
  assert.equal(same(5, '5'), true);
  assert.equal(same(0, null), true);
  assert.equal(same('axe', 'axe'), true);
  assert.equal(same('axe', 'Axe'), false);
  assert.equal(same(5, 'five'), false);
  assert.equal(same(null, ''), true);
});

test('a text reads whole, its colours and what follows a NUL left out', () => {
  assert.equal(plainText('\x1b[1;33mdwarven\x1b[0m axe\0junk'), 'dwarven axe');
  assert.equal(plainText('one\r\ntwo\r\n'), 'one\ntwo');
  assert.equal(plainText('x'.repeat(80)), 'x'.repeat(80));
});

test('without numbers to measure by, the records beside it in the list', () => {
  const rows = ['a', 'b', 'c', 'd', 'e', 'f'];
  assert.deepEqual(around(rows, 2, 3), ['b', 'd', 'e']);
  assert.deepEqual(around(rows, 0, 3), ['b', 'c', 'd']);
  assert.deepEqual(around(rows, 5, 3), ['c', 'd', 'e']);
  assert.deepEqual(around(rows, 1, 9), ['a', 'c', 'd', 'e', 'f']);
  assert.deepEqual(around(rows, -1, 2), ['a', 'b']);
});
