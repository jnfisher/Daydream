import assert from 'node:assert/strict';
import { test } from 'node:test';
import { columnGroups, compareCells, csvField, defaultColumns, inOrder, toCsv } from './grid.ts';
import type { Section } from './sections.ts';

const sections: Section[] = [
  { id: 'a', title: 'The room', fields: [{ col: 'Name' }, { col: 'Type' }, { col: 'Shop Number' }] },
  { id: 'b', title: 'Monsters here', fields: [{ col: 'Monster Type' }, { col: 'Max Regen' }] },
  { id: 'c', title: 'What players see', text: { cols: ['Desc 0', 'Desc 1'] } },
];
const all = ['Map Number', 'Room Number', 'Name', 'Desc 0', 'Desc 1', 'Type', 'Shop Number', 'Monster Type', 'Max Regen', 'Light'];
const key = ['Map Number', 'Room Number'];

test("a grid starts with the name and the list's numbers, else the card's first fields", () => {
  assert.deepEqual(defaultColumns(sections, all, key, 'Name', ['Type', 'Max Regen', 'Gone']), ['Name', 'Type', 'Max Regen']);
  assert.deepEqual(defaultColumns(sections, all, key, 'Name', undefined, 4), ['Name', 'Type', 'Shop Number', 'Monster Type']);
  // never the key
  assert.deepEqual(defaultColumns(sections, all, key, 'Room Number', ['Map Number']), ['Name', 'Type', 'Shop Number', 'Monster Type', 'Max Regen']);
});

test("the picker offers every column under the card's headings", () => {
  const groups = columnGroups(sections, all, key);
  assert.deepEqual(
    groups.map((g) => [g.title, g.cols.length]),
    [
      ['The room', 3],
      ['Monsters here', 2],
      ['What players see', 2],
      ['More fields', 1],
    ],
  );
  assert.deepEqual(inOrder(new Set(['Light', 'Desc 0', 'Name']), groups), ['Name', 'Desc 0', 'Light']);
});

test('rows sort by number, by text ignoring case, blanks last', () => {
  assert.deepEqual([3, 12, null, 1].sort(compareCells), [1, 3, 12, null]);
  assert.deepEqual(['b', 'A', '', 'a2', 'a10'].sort(compareCells), ['A', 'a2', 'a10', 'b', '']);
});

test('a CSV quotes what needs it and runs no formulas', () => {
  assert.equal(csvField('plain'), 'plain');
  assert.equal(csvField('a, b'), '"a, b"');
  assert.equal(csvField('say "hi"'), '"say ""hi"""');
  assert.equal(csvField('two\nlines'), '"two\nlines"');
  assert.equal(csvField('=SUM(A1)'), "'=SUM(A1)");
  assert.equal(csvField('-1 sword'), "'-1 sword");
  assert.equal(csvField(-1), '-1');
  assert.equal(csvField(null), '');
  assert.equal(toCsv(['Number', 'Name'], [[868, 'dwarven axe'], [1, null]]), '﻿Number,Name\r\n868,dwarven axe\r\n1,\r\n');
});
