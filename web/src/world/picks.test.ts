// Picking rooms on the World map: lassos, the wand's region, picks added and taken, what the
// rooms picked share (picks.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { MapRoom } from './model.ts';
import { lightWord } from './light.ts';
import { after, clickHow, howOf, inside, PICK_FIELDS, regionOf, shared, sharedOf, withValues } from './picks.ts';

const [N, S, E, W] = [0, 1, 2, 3];

/** A room from `[dir, to]` exits (an exit to another map as `[dir, to, 8]`). */
function room(n: number, name: string, ...exits: [number, number, number?][]): MapRoom {
  return {
    room: n,
    name,
    exits: exits.map(([dir, to, kind]) => ({ dir, to, kind: kind ?? 0, para1: kind === 8 ? 2 : 0, para2: 0, para3: 0, para4: 0 })),
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
  };
}

test('a lasso holds the points inside it', () => {
  const l: [number, number][] = [[0, 0], [10, 0], [10, 10], [5, 4], [0, 10]];
  assert.ok(inside(2, 2, l));
  assert.ok(inside(9.5, 8, l), 'in a lobe beside the notch');
  assert.ok(!inside(5, 8, l), 'in the notch');
  assert.ok(!inside(11, 5, l));
  assert.ok(!inside(5, 5, []), 'no lasso, nothing inside');
});

test('the wand takes the rooms joined to one through rooms of its colour, either way along an exit', () => {
  // 1 Wood ⇄ 2 Wood → 3 Wood (one way), 4 Road ⇄ 2, 5 Wood ← 4 only (not through the road),
  // 6 Wood leads into 1 one way, and 1 has an exit to map 2's room 3
  const rooms = [
    room(1, 'Wood', [E, 2], [N, 3, 8]),
    room(2, 'Wood', [W, 1], [E, 3], [S, 4]),
    room(3, 'Wood'),
    room(4, 'Road', [N, 2], [E, 5]),
    room(5, 'Wood'),
    room(6, 'Wood', [S, 1]),
  ];
  const by = new Map(rooms.map((r) => [r.room, r]));
  const key = (n: number) => by.get(n)?.name;
  assert.deepEqual(regionOf(by, 1, key), [1, 2, 6, 3]);
  assert.deepEqual(regionOf(by, 3, key), [3, 2, 1, 6], 'from the end of a one-way exit too');
  assert.deepEqual(regionOf(by, 4, key), [4]);
  assert.deepEqual(regionOf(by, 5, key), [5], 'the road between keeps it apart');
  assert.deepEqual(regionOf(by, 9, key), []);
});

test('a pick replaces, adds to, takes from or flips the rooms picked', () => {
  const was = new Set([1, 2, 3]);
  assert.deepEqual([...after(was, [7, 8], 'set')], [7, 8]);
  assert.deepEqual([...after(was, [3, 4], 'add')], [1, 2, 3, 4]);
  assert.deepEqual([...after(was, [2, 9], 'drop')], [1, 3]);
  assert.deepEqual([...after(was, [3], 'flip')], [1, 2]);
  assert.deepEqual([...after(was, [5], 'flip')], [1, 2, 3, 5]);
  assert.deepEqual([...was], [1, 2, 3], 'the rooms picked before are left as they were');
  assert.equal(howOf({ shiftKey: true, altKey: false }), 'add');
  assert.equal(howOf({ shiftKey: false, altKey: true }), 'drop');
  assert.equal(howOf({ shiftKey: false, altKey: false }, 'flip'), 'flip');
  const keys = (k = '') => ({ shiftKey: k === 'shift', altKey: k === 'alt', ctrlKey: k === 'ctrl', metaKey: false });
  assert.deepEqual(
    (['set', 'add', 'drop'] as const).map((mode) => ['', 'shift', 'ctrl', 'alt'].map((k) => clickHow(keys(k), mode))),
    [
      ['set', 'flip', 'flip', 'drop'],
      ['flip', 'flip', 'flip', 'drop'],
      ['drop', 'flip', 'flip', 'drop'],
    ],
  );
});

test('what the rooms picked share, and the map room a set leaves', () => {
  assert.deepEqual(shared([5, 5, 5]), { one: 5 });
  assert.equal(shared([5, 5, -20]), 'mixed');
  assert.equal(shared([]), null);
  assert.deepEqual(shared(['']), { one: '' });
  const r = withValues(room(1, 'Wood'), [['Name', 'Dark wood'], ['Light', -40], ['Mon Type', 6], ['Min Index', 3], ['Type', 3]]);
  assert.deepEqual([r.name, r.light, r.mon_type, r.min_index, r.max_index, r.room_type, r.control], ['Dark wood', -40, 6, 3, 0, 3, 0]);
});

test('a field is shared when every column of it is, and light has the game words', () => {
  const field = (id: string) => PICK_FIELDS.find((f) => f.id === id)!;
  const a = withValues(room(1, 'Wood '), [['Mon Type', 1], ['Min Index', 3], ['Max Index', 5], ['Max Regen', 4]]);
  const b = withValues(room(2, 'Wood'), [['Mon Type', 1], ['Min Index', 3], ['Max Index', 5], ['Max Regen', 4]]);
  const c = withValues(room(3, 'Wood'), [['Mon Type', 1], ['Min Index', 3], ['Max Index', 6], ['Max Regen', 4]]);
  assert.deepEqual(sharedOf([a, b], field('monsters')), { one: [1, 3, 5, 4] });
  assert.equal(sharedOf([a, b, c], field('monsters')), 'mixed', 'one index differs');
  assert.deepEqual(sharedOf([a, b, c], field('name')), { one: ['Wood'] }, 'names compare without their trailing spaces');
  assert.equal(sharedOf([], field('light')), null);
  assert.deepEqual(
    [250, 0, -1, -100, -101, -150, -151, -200, -201, 900].map(lightWord),
    ['Daylight', 'Regular light', 'Dimly lit', 'Dimly lit', 'Barely visible', 'Barely visible', 'Very dark', 'Very dark', 'Pitch black', 'Blinding'],
  );
});
