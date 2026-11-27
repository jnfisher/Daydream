import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { ItemPlace, LimitedItem } from '../api.ts';
import { csvOf, finds, limitedOrder, placeRef, placeText } from './reports.ts';

const place = (kind: ItemPlace['kind'], key: string, name = '', count = 1): ItemPlace => ({ kind, key, name, count });
const item = (number: number, limit: number, in_use: number): LimitedItem => ({
  number,
  name: `item ${number}`,
  limit,
  in_use,
  over: in_use > limit,
  places: [],
});

test('a place names its room, shop or user', () => {
  assert.deepEqual(placeRef(place('room', '1/224')), { table: 'Rooms', key: [1, 224] });
  assert.deepEqual(placeRef(place('hidden', '3/17')), { table: 'Rooms', key: [3, 17] });
  assert.deepEqual(placeRef(place('shop', '100')), { table: 'Shops', key: [100] });
  assert.deepEqual(placeRef(place('user', 'Sysop')), { table: 'Users', key: ['Sysop'] });
  assert.equal(placeText(place('hidden', '3/17', 'A dusty cellar', 2)), 'Hidden in room 3/17 A dusty cellar ×2');
  assert.equal(placeText(place('shop', '100', '', 1)), 'Shop 100 ×1');
});

test('the items furthest over their limit come first, then by number', () => {
  const order = limitedOrder([item(5, 1, 1), item(2, 1, 0), item(9, 1, 2), item(7, 2, 5)]).map((i) => i.number);
  assert.deepEqual(order, [7, 9, 2, 5]);
});

test('finding needs every word, in any order and case', () => {
  assert.ok(finds('868 dwarven axe Room 1/224', 'AXE dwarven'));
  assert.ok(finds('anything', '  '));
  assert.ok(!finds('868 dwarven axe', 'elven axe'));
});

test('a list comes out as CSV rows', () => {
  const crown = { ...item(1, 1, 3), places: [place('room', '1/1', 'Square', 2), place('user', 'Sysop', 'Sysop', 1)] };
  assert.deepEqual(csvOf.limited([crown]).rows, [[1, 'item 1', 1, 3, 2, 'Room 1/1 Square ×2; User Sysop Sysop ×1']]);
  assert.deepEqual(csvOf.groups([{ group: 3, index: 2, monsters: [[12, 'goblin'], [13, 'orc']] }], (g) => `G${g}`).rows, [[3, 'G3', 2, '12 goblin; 13 orc']]);
});
