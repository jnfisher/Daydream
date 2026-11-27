import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Lines } from './combat.ts';
import { countedTables, countedUsers, filledFor, groupUses, namesIt, readAs, whenSaid } from './msguse.ts';
import type { MessageRead, Reference } from './where.ts';

const hit = (slot: number): MessageRead[] => [
  { line: 1, when: `attack ${slot} hits`, to: 'the player', fills: ['its name', 'the damage', 'a star (*)', 'a star (*)'] },
  { line: 2, when: `attack ${slot} hits`, to: 'the room', fills: ['its name', "the player's name", 'the damage', 'a star (*)', 'a star (*)'] },
];
const death: MessageRead[] = [{ line: 3, when: 'it dies', to: 'the room', fills: [] }];
const ref = (table: string, key: (number | string)[], title: string, column: string, reads: MessageRead[] | null): Reference => ({
  table,
  key,
  title,
  column,
  by: column,
  reads,
});

test('a line reads as the game fills it for each use', () => {
  assert.equal(readAs('The %s bites you for %d damage!', ['its name', 'the damage'], 'cave newt'), 'The cave newt bites you for 12 damage!');
  // a spare blank takes a star, as the game's attack lines do
  assert.equal(readAs('The %s claws you for %d %s', ['its name', 'the damage', 'a star (*)'], 'troll'), 'The troll claws you for 12 *');
  assert.equal(readAs('A %s comes in from %s.', ['where it came from (“the north”)', 'its name'], 'troll'), 'A the north comes in from troll.');
  assert.equal(readAs('%s and %s', ['its name', 'a fill no one knows'], 'x'), 'x and ‹a fill no one knows›');
});

test('uses gather by what they read, any attack slot as one', () => {
  const groups = groupUses([
    ref('Monsters', [1], 'cave newt', 'Death Msg', death),
    ref('Monsters', [1], 'cave newt', 'Attack Hit Msg 0', hit(1)),
    ref('Monsters', [2], 'marsh toad', 'Attack Hit Msg 3', hit(4)),
    ref('Monsters', [2], 'marsh toad', 'Death Msg', death),
    ref('Spells', [7], 'resist', 'Cast MSG A', null),
    ref('Monsters', [2], 'marsh toad', 'Attack Hit Msg 4', hit(5)),
  ]);
  assert.deepEqual(
    groups.map((g) => [g.table, g.reads?.[0]?.when ?? null, g.users.map((u) => u.title)]),
    [
      ['Monsters', 'it dies', ['cave newt', 'marsh toad']],
      ['Monsters', 'an attack hits', ['cave newt', 'marsh toad']],
      ['Spells', null, ['resist']],
    ],
  );
  assert.deepEqual(groups[1]!.users[1]!.by, ['Attack Hit Msg 3', 'Attack Hit Msg 4']);
  assert.equal(whenSaid(groups[1]!.reads!), 'When an attack hits');
});

test('the lines a use reads, filled in from the lines as typed', () => {
  const lines: Lines = ['The %s bites you for %d damage!', 'The %s bites %s for %s damage!', 'The cave newt squeaks, and dies.'];
  assert.deepEqual(
    filledFor(lines, hit(1), 'marsh toad').map((l) => `${l.line} ${l.to}: ${l.text}`),
    ['1 the player: The marsh toad bites you for 12 damage!', '2 the room: The marsh toad bites Rowan for 12 damage!'],
  );
  assert.equal(filledFor(lines, death, 'marsh toad')[0]!.text, 'The cave newt squeaks, and dies.');
  // an attack line goes out with a capital, as the game sends it
  assert.equal(filledFor(['%s bites you!', '', ''], hit(2), 'barmaid')[0]!.text, 'Barmaid bites you!');
  assert.equal(filledFor(['', '', '%s dies.'], death, 'barmaid')[0]!.text, ' dies.');
  assert.equal(namesIt(hit(1)), true);
  assert.equal(namesIt(death), false);
});

test('the records a change reaches, counted in words', () => {
  const counted = (t: string, n: number) => `${n} ${t.toLowerCase().replace(/s$/, '')}${n === 1 ? '' : 's'}`;
  assert.equal(countedUsers([{ table: 'Monsters' }, { table: 'Textblocks' }, { table: 'Monsters' }], counted), '2 monsters and 1 textblock');
  assert.equal(countedUsers([{ table: 'Items' }], counted), '1 item');
  assert.equal(countedUsers([], counted), '');
  assert.equal(countedTables({ Spells: 1, Monsters: 1510, Items: 4 }, counted), '1510 monsters, 4 items and 1 spell');
});
