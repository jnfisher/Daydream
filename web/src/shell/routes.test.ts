import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { TableInfo } from '../api.ts';
import { gridHref, parse, recordHref, storyHref, storyStart } from './routes.ts';

const tables: TableInfo[] = [
  { name: 'Items', key: ['Number'], title: 'Name', columns: [{ name: 'Number', type: 'long', size: null }] },
  { name: 'Monsters', key: ['Number'], title: 'Name', columns: [{ name: 'Number', type: 'long', size: null }] },
  {
    name: 'Textblocks',
    key: ['Number', 'Part'],
    title: null,
    columns: [
      { name: 'Number', type: 'long', size: null },
      { name: 'Part', type: 'long', size: null },
    ],
  },
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

test('a story has a link from the record it starts at', () => {
  assert.equal(storyHref('Monsters', [381]), '#/story/monsters/381');
  const route = parse('#/story/rooms/6/1664', tables);
  assert.deepEqual([route.place, storyStart(route.rest, tables)], ['story', { table: 'Rooms', key: [6, 1664] }]);
  // a text block's first part by its number alone
  assert.deepEqual(storyStart(['textblocks', '911'], tables), { table: 'Textblocks', key: [911, 0] });
  assert.deepEqual(storyStart(['textblocks', '911', '2'], tables), { table: 'Textblocks', key: [911, 2] });
  // no story starts at an item, nor at a room with half a key
  assert.equal(storyStart(['items', '868'], tables), null);
  assert.equal(storyStart(['rooms', '6'], tables), null);
  assert.equal(storyStart([], tables), null);
});
