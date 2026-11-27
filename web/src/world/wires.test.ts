// The map's scripted links (wires.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { MapRoom, Scripts } from './model.ts';
import { FAN, SPRAY, wiresOf, type Wire } from './wires.ts';

const room = (n: number, more: Partial<MapRoom> = {}): MapRoom => ({
  room: n,
  name: `Room ${n}`,
  exits: [],
  shop: 0,
  spell: 0,
  command: 0,
  npc: 0,
  control: 0,
  death: 0,
  exit_room: 0,
  room_type: 0,
  mon_type: 0,
  min_index: 0,
  max_index: 0,
  max_regen: 0,
  light: 0,
  ...more,
});

const rooms = (...list: MapRoom[]) => new Map(list.map((r) => [r.room, r]));
const said = (w: Wire) => `${w.kind} ${w.from.map}/${w.from.room}>${w.to.map}/${w.to.room}${w.exit === undefined ? '' : `.${w.exit}`}${w.among ? ` of ${w.among}` : ''}`;

/** Map 1: room 1 types a teleport to 3 and to map 2's room 9, and works the north exit of 4;
 * room 2's spell scatters players over rooms 10 to 14 (the map has 10 to 12); map 2's room 9
 * teleports to 1. */
const scripts: Scripts = {
  sources: [
    {
      table: 'Textblocks',
      key: [50],
      line: [0, 1],
      targets: [
        { kind: 'teleport', to_map: 1, to_room: 3, table: 'Textblocks', record: 50, what: 'teleport 3 1' },
        { kind: 'teleport', to_map: 2, to_room: 9, table: 'Textblocks', record: 50, what: 'teleport 9 2' },
        { kind: 'remote', to_map: 0, to_room: 4, exit: 0, table: 'Textblocks', record: 50, what: 'remoteaction 4 0 0 0' },
      ],
    },
    { table: 'Spells', key: [7], name: 'scatter', targets: [{ kind: 'spell', to_map: 1, to_room: 10, to_last: 14, table: 'Spells', record: 7, what: 'scatter' }] },
    { table: 'Textblocks', key: [51], line: [0, 1], targets: [{ kind: 'teleport', to_map: 1, to_room: 1, table: 'Textblocks', record: 51, what: 'teleport 1 1' }] },
  ],
  uses: [
    { map: 1, room: 1, when: { by: 'typed', words: 'pull lever' }, source: 0 },
    { map: 1, room: 2, when: { by: 'here' }, source: 1 },
    { map: 2, room: 9, when: { by: 'typed', words: 'go home' }, source: 2 },
  ],
};
const here = rooms(room(1, { control: 5, death: 3 }), room(2, { control: 5 }), room(3, { exit_room: 1 }), room(4), room(5, { control: 5 }), room(10), room(11), room(12));

test('a room’s wires go both ways: what it sets off, then what leads to it', () => {
  const { wires, more } = wiresOf(1, here, scripts, new Set([1]));
  assert.deepEqual(wires.map(said), [
    'teleport 1/1>1/3',
    'teleport 1/1>2/9',
    'remote 1/1>1/4.0',
    'death 1/1>1/3',
    'control 1/1>1/5',
    // into room 1: map 2's teleport, and room 3's hang-up room
    'teleport 2/9>1/1',
    'hangup 1/3>1/1',
  ]);
  assert.equal(more, 0);
  // what a wire says about itself comes with it
  assert.deepEqual(wires[0]!.when, { by: 'typed', words: 'pull lever' });
  assert.equal(wires[0]!.target?.what, 'teleport 3 1');
});

test('a control room shows the rooms it controls, and itself no spoke', () => {
  const { wires } = wiresOf(1, here, scripts, new Set([5]));
  assert.deepEqual(wires.map(said), ['control 1/1>1/5', 'control 1/2>1/5']);
});

test('a spell’s roll lands on each room of its range the map has', () => {
  const { wires } = wiresOf(1, here, scripts, new Set([2]));
  assert.deepEqual(wires.map(said), ['spell 1/2>1/10 of 5', 'spell 1/2>1/11 of 5', 'spell 1/2>1/12 of 5', 'control 1/2>1/5']);
});

test('for every room: their scripts, not those scattering players over many rooms, after the selected room’s own', () => {
  assert.deepEqual(wiresOf(1, here, scripts, new Set(), true).wires.map(said), [
    'teleport 1/1>1/3',
    'teleport 1/1>2/9',
    'remote 1/1>1/4.0',
    'spell 1/2>1/10 of 5',
    'spell 1/2>1/11 of 5',
    'spell 1/2>1/12 of 5',
    'teleport 2/9>1/1',
  ]);
  const wide: Scripts = {
    sources: [{ table: 'Spells', key: [8], targets: [{ kind: 'spell', to_map: 1, to_room: 10, to_last: 10 + FAN, table: 'Spells', record: 8, what: 'wide' }] }],
    uses: [{ map: 1, room: 4, when: { by: 'here' }, source: 0 }],
  };
  const many = rooms(...Array.from({ length: FAN + 1 }, (_, i) => room(10 + i)));
  assert.deepEqual(wiresOf(1, many, wide, new Set(), true).wires, []);
  // picked, it shows
  assert.equal(wiresOf(1, many, wide, new Set([4]), true).wires.length, FAN + 1);
  // and the room selected keeps its fields
  assert.deepEqual(wiresOf(1, here, scripts, new Set([5]), true).wires.map(said).slice(0, 3), ['control 1/1>1/5', 'control 1/2>1/5', 'teleport 1/1>1/3']);
});

test('a room scattering players over many rooms has so many of them drawn, each saying how many there are', () => {
  const targets = Array.from({ length: SPRAY + 5 }, (_, i) => ({ kind: 'teleport' as const, to_map: 2, to_room: 100 + i, table: 'Textblocks', record: 60, what: `teleport ${100 + i} 2` }));
  const spray: Scripts = {
    sources: [{ table: 'Textblocks', key: [60], line: [0, 1], targets }, scripts.sources[0]!],
    uses: [
      { map: 1, room: 1, when: { by: 'typed', words: 'scatter' }, source: 0 },
      { map: 1, room: 1, when: { by: 'typed', words: 'again' }, source: 0 },
      { map: 1, room: 1, when: { by: 'typed', words: 'pull lever' }, source: 1 },
    ],
  };
  const { wires, more } = wiresOf(1, here, spray, new Set([1]));
  const teleports = wires.filter((w) => w.kind === 'teleport');
  assert.equal(teleports.length, SPRAY);
  assert.ok(teleports.every((w) => w.fan === SPRAY + 7));
  assert.equal(more, 7);
  // the room's other kinds still show, and say nothing of the scatter, and so do those leading to it
  assert.deepEqual(wires.filter((w) => w.kind !== 'teleport').map(said), ['remote 1/1>1/4.0', 'death 1/1>1/3', 'control 1/1>1/5', 'hangup 1/3>1/1']);
  assert.equal(wires.find((w) => w.kind === 'remote')!.fan, undefined);
});

test('at most so many are drawn, and the rest are counted', () => {
  const { wires, more } = wiresOf(1, here, scripts, new Set([1, 2, 5]), false, 4);
  assert.equal(wires.length, 4);
  assert.ok(more > 0);
  // the rooms’ own come first
  assert.deepEqual(wires.map(said), ['teleport 1/1>1/3', 'teleport 1/1>2/9', 'remote 1/1>1/4.0', 'spell 1/2>1/10 of 5']);
});

test('no rooms picked, or no scripts: only the fields', () => {
  assert.deepEqual(wiresOf(1, here, scripts, new Set()).wires, []);
  assert.deepEqual(wiresOf(1, here, undefined, new Set([3])).wires.map(said), ['hangup 1/3>1/1', 'death 1/1>1/3']);
});
