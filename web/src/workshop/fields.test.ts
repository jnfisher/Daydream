import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { TableInfo } from '../api.ts';
import { columnsOf, decimal, fieldName, numberFields, rangeOf, whole } from './fields.ts';

const monsters: TableInfo = {
  name: 'Monsters',
  key: ['Number'],
  title: 'Name',
  columns: [
    { name: 'Number', type: 'long', size: null },
    { name: 'Name', type: 'text', size: 30 },
    { name: 'Experience', type: 'long', size: null },
    { name: 'Item Drop % 0', type: 'long', size: null },
    { name: 'Item Drop % 1', type: 'long', size: null },
    { name: 'Regen Number0', type: 'long', size: null },
    { name: 'Regen Number1', type: 'long', size: null },
    { name: 'Weapon Number', type: 'long', size: null },
  ],
};

test('number fields leave out the key and the text, and offer each numbered family whole', () => {
  const f = numberFields(monsters);
  assert.ok(!f.some(([c]) => c === 'Number' || c === 'Name'));
  assert.deepEqual(
    f.filter(([c]) => c.endsWith('*')),
    [
      ['Item Drop % *', 'Item Drop % (every one)'],
      ['Regen Number*', 'Regen Number (every one)'],
    ],
  );
  // one numbered column is no family
  assert.ok(!f.some(([c]) => c === 'Weapon *'));
  assert.equal(numberFields(monsters, false).length, 6);
});

test("a family's choice stands for each of its columns", () => {
  assert.deepEqual(columnsOf(monsters, 'Item Drop % *'), ['Item Drop % 0', 'Item Drop % 1']);
  assert.deepEqual(columnsOf(monsters, 'Regen Number*'), ['Regen Number0', 'Regen Number1']);
  assert.deepEqual(columnsOf(monsters, 'Experience'), ['Experience']);
  assert.equal(fieldName('Item Drop % *'), 'Item Drop %');
});

test('numbers are read as typed', () => {
  assert.equal(whole(' 12 '), 12);
  assert.equal(whole('-3'), -3);
  assert.ok(Number.isNaN(whole('1.5')));
  assert.equal(decimal('1.5'), 1.5);
  assert.equal(decimal('.25'), 0.25);
  assert.ok(Number.isNaN(decimal('x')));
});

test('a span of records is a range, all of them, or a reason it is neither', () => {
  assert.deepEqual(rangeOf('Items', { all: true, map: '', from: '', to: '' }), { range: null });
  assert.deepEqual(rangeOf('Items', { all: false, map: '', from: '5', to: '9' }), { range: { table: 'Items', from: 5, to: 9 } });
  assert.deepEqual(rangeOf('Rooms', { all: false, map: '2', from: '1', to: '40' }), { range: { table: 'Rooms', map: 2, from: 1, to: 40 } });
  assert.ok('error' in rangeOf('Rooms', { all: false, map: '', from: '1', to: '40' }));
  assert.ok('error' in rangeOf('Items', { all: false, map: '', from: '9', to: '5' }));
  assert.ok('error' in rangeOf('Items', { all: false, map: '', from: '', to: '5' }));
});
