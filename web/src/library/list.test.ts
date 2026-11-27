import assert from 'node:assert/strict';
import { test } from 'node:test';
import { found, keyProblem, listRow, nextKey } from './list.ts';

test('a row answers the values it was read with', () => {
  const r = listRow([868], 'dwarven axe', ['Type', 'Max Hit'], [1, 16]);
  assert.equal(r.label, '868');
  assert.equal(r.get('Max Hit'), 16);
  assert.equal(r.get('Speed'), null);
});

test('a search finds a record by its number or its name', () => {
  const axe = listRow([868], 'dwarven axe', [], []);
  const room = listRow([1, 224], 'Silvermere Square', [], []);
  assert.ok(found(axe, ''));
  assert.ok(found(axe, '86'));
  assert.ok(!found(axe, '68'));
  assert.ok(found(axe, 'AXE'));
  assert.ok(found(room, '224'), 'a room by its number on its map');
  assert.ok(found(room, '1/2'));
  assert.ok(found(room, 'square'));
  assert.ok(!found(room, 'dwarf'));
});

test('a new record is offered the number after the highest', () => {
  const rows = [{ key: [3] }, { key: [868] }, { key: [12] }];
  assert.deepEqual(nextKey('Items', [true], rows), [869]);
  assert.deepEqual(nextKey('Items', [true], []), [1]);
});

test("a new room is offered the next number on the open room's map", () => {
  const rows = [{ key: [1, 5] }, { key: [1, 224] }, { key: [2, 900] }, { key: [2, 3] }];
  assert.deepEqual(nextKey('Rooms', [true, true], rows, [2, 3]), [2, 901]);
  assert.deepEqual(nextKey('Rooms', [true, true], rows), [1, 225]);
  assert.deepEqual(nextKey('Rooms', [true, true], [], null), [1, 1]);
});

test('a new text block is a new block, at its first part', () => {
  const rows = [{ key: [7, 0] }, { key: [7, 1] }, { key: [9, 0] }];
  assert.deepEqual(nextKey('Textblocks', [true, true], rows, [7, 1]), [10, 0]);
});

test('a key of text has nothing to offer', () => {
  assert.deepEqual(nextKey('Actions', [false], [{ key: ['smile'] }]), ['']);
});

test("a typed key's problems are said", () => {
  const taken = (l: string) => l === '868' || l === '1/224';
  assert.equal(keyProblem(['869'], [true], taken, 'Item'), '');
  assert.equal(keyProblem(['868'], [true], taken, 'Item'), 'Item 868 is there already.');
  assert.equal(keyProblem(['8x'], [true], taken, 'Item'), '8x is not a whole number.');
  assert.equal(keyProblem([''], [true], taken, 'Item'), 'Fill in every part of the key.');
  assert.equal(keyProblem(['1', '224'], [true, true], taken, 'Room'), 'Room 1/224 is there already.');
  assert.equal(keyProblem(['wave'], [false], taken, 'Action'), '');
});
