import assert from 'node:assert/strict';
import { test } from 'node:test';
import { family, groupReferences, type Reference } from './where.ts';

const counted = (table: string, n: number) => `${n} ${table.toLowerCase().replace(/s$/, '')}${n === 1 ? '' : 's'}`;
const ref = (table: string, key: (number | string)[], column: string | null, title = 'x', by = column ?? 'spawns it'): Reference => ({
  table,
  key,
  title,
  column,
  by,
});

test('a numbered column is of its family', () => {
  assert.equal(family('Item Number 3'), 'Item Number #');
  assert.equal(family('Room Item 12 QTY'), 'Room Item # QTY');
  assert.equal(family('Weapon Number'), 'Weapon Number');
  assert.equal(family('Para1 3'), 'Para1 #');
});

test('references group by how they name the record, in a fixed order', () => {
  const groups = groupReferences(
    [
      ref('Monsters', [381], 'Weapon Number', 'dwarven merchant'),
      ref('Shops', [100], 'Item 2', 'Dwarven Axe Store'),
      ref('Monsters', [382], 'Weapon Number', 'bodyguard'),
      ref('Monsters', [381], 'Item Number 0', 'dwarven merchant'),
      ref('Rooms', [1, 5], 'Para1 3', 'Gate', 'Para1 3 (Key: the key item)'),
    ],
    counted,
  );
  assert.deepEqual(
    groups.map((g) => g.says),
    ['Sold in 1 shop', 'Wielded by 2 monsters', 'Dropped by 1 monster', 'Used by exits of 1 room'],
  );
  assert.deepEqual(groups[1]!.rows.map((r) => r.title), ['dwarven merchant', 'bodyguard']);
  assert.deepEqual(groups[3]!.rows[0]!.by, ['Para1 3 (Key: the key item)']);
});

test('a row naming the record twice is listed once, with both ways', () => {
  const groups = groupReferences(
    [
      ref('Monsters', [9], 'Attack Hit Msg 0', 'orc'),
      ref('Monsters', [9], 'Attack Miss Msg 0', 'orc'),
      ref('Monsters', [10], 'Attack Dodge Msg 2', 'kobold'),
      ref('Monsters', [9], 'Death Msg', 'orc'),
    ],
    counted,
  );
  assert.deepEqual(
    groups.map((g) => [g.says, g.rows.length]),
    [
      ['Attack messages of 2 monsters', 2],
      ['The deaths of 1 monster', 1],
    ],
  );
  assert.deepEqual(groups[0]!.rows[0]!.by, ['Attack Hit Msg 0', 'Attack Miss Msg 0']);
});

test('rooms that spawn a monster, and columns no phrase covers', () => {
  const groups = groupReferences([ref('Rooms', [6, 84], null, 'Rocky Path'), ref('Gangs', ['g'], 'Leader')], counted);
  assert.deepEqual(
    groups.map((g) => g.says),
    ['Spawns in 1 room', '1 gang, by Leader'],
  );
});
