import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { TableInfo } from '../api.ts';
import { gridHref, parse, recordHref } from './routes.ts';

const tables: TableInfo[] = [
  { name: 'Items', key: ['Number'], title: 'Name', columns: [{ name: 'Number', type: 'long', size: null }] },
  {
    name: 'Rooms',
    key: ['Map Number', 'Room Number'],
    title: 'Name',
    columns: [
      { name: 'Map Number', type: 'long', size: null },
      { name: 'Room Number', type: 'long', size: null },
    ],
  },
];

test('a record and a grid each have a link', () => {
  assert.deepEqual(parse('#/items/868', tables), { place: 'library', table: 'Items', key: [868], rest: [] });
  assert.deepEqual(parse(recordHref('Rooms', [1, 224]), tables).key, [1, 224]);
  assert.equal(gridHref('Items'), '#/items?grid');
  assert.deepEqual(parse('#/items?grid', tables), { place: 'library', table: 'Items', key: undefined, rest: [], view: 'grid' });
  // a record's link is the record, whatever follows it
  assert.equal(parse('#/items/868?grid', tables).view, undefined);
  assert.equal(parse('#/items?other', tables).view, undefined);
});
