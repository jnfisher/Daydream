// Picking rooms on the World map: lassos, the wand's region, picks added and taken, what the
// rooms picked share, the rooms a stroke of Paint changes, and the rooms picked saved as a
// stamp's shape (picks.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { layOut, type Spot } from './layout.ts';
import type { MapRoom } from './model.ts';
import { lightWord } from './light.ts';
import { after, changing, clickHow, drawnNear, holds, howOf, inside, lookOf, PAINT_COLOUR, PAINT_FIELDS, PICK_FIELDS, PICK_TOOLS, regionOf, shapeOf, shared, sharedOf, stampName, valuesOf, wayIn, withValues } from './picks.ts';
import { placed, stamped } from './stamps.ts';

const [N, S, E, W, U, D] = [0, 1, 2, 3, 8, 9];

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

test('a stroke of Paint changes only the rooms without its values, and each field it paints colours the map', () => {
  const field = (id: string) => PICK_FIELDS.find((f) => f.id === id)!;
  const by = new Map(
    [
      withValues(room(1, 'Cave'), [['Light', -175]]),
      withValues(room(2, 'Cave'), [['Light', 0]]),
      withValues(room(3, 'Cave'), [['Light', -175], ['Mon Type', 6], ['Min Index', 1], ['Max Index', 4], ['Max Regen', 2]]),
      room(4, 'Cave'),
    ].map((r) => [r.room, r]),
  );
  assert.deepEqual(changing(by, [4, 3, 2, 1, 9], [['Light', -175]]), [2, 4], 'room 9 is not on the map; rooms 1 and 3 are dark already');
  assert.deepEqual(changing(by, new Set([3, 1]), [['Light', -175]]), []);
  const monsters: [string, number][] = [['Mon Type', 6], ['Min Index', 1], ['Max Index', 4], ['Max Regen', 2]];
  assert.deepEqual(changing(by, [1, 2, 3], monsters), [1, 2], 'every column of the field counts');
  assert.ok(holds(by.get(3)!, monsters));
  assert.ok(!holds(by.get(3)!, [['Max Regen', 3]]));
  // the pipette takes a room's values
  assert.deepEqual(valuesOf(by.get(3)!, field('monsters')), [6, 1, 4, 2]);
  assert.deepEqual(valuesOf(by.get(1)!, field('light')), [-175]);
  assert.deepEqual(valuesOf(withValues(room(5, 'Den'), [['Control Room', 12]]), field('control')), [12]);
  // Paint is the fifth way of the Select tool, and each of its fields has a colouring that shows it
  assert.equal(PICK_TOOLS[4]!.id, 'paint');
  assert.deepEqual(PAINT_FIELDS.map((f) => PAINT_COLOUR[f]), ['light', 'monsters', 'area']);
  for (const f of PAINT_FIELDS) assert.ok(field(f), `${f} is a field the bar sets too`);
});

/**
 * The map:   1 2 .     1 Gate (0,0): south to the well (one way), east to the road
 *          7 3 4 .     2 Road (1,0): west to the gate, down to the cellar, east off the map
 *                      3 Well (0,1): east to the cellar, west to the field (not picked)
 *                      4 Cellar (1,1): west to the well, up to the road
 */
test('the rooms picked make a stamp of their shape, which puts down the same rooms and exits', () => {
  const rooms = [
    room(1, 'Gate', [S, 3], [E, 2]),
    room(2, 'Road', [E, 5, 8], [W, 1], [D, 4]),
    room(3, 'Well', [E, 4], [W, 7]),
    room(4, 'Cellar', [W, 3], [U, 2]),
    room(7, 'Field', [E, 3]),
  ];
  const by = new Map(rooms.map((r) => [r.room, r]));
  const at = new Map<number, Spot>([[1, { x: 0, y: 0 }], [2, { x: 1, y: 0 }], [3, { x: 0, y: 1 }], [4, { x: 1, y: 1 }], [7, { x: -1, y: 1 }]]);
  // its way in is the well, the first with an exit out of them (the road's way off the map is no
  // way out); the others follow in walking order, each cell counted from it
  const shape = shapeOf(by, at, [4, 2, 3, 1, 4, 42]);
  assert.equal(wayIn(by, at, [4, 2, 3, 1, 4, 42]), 3);
  // no two have one name, so its new rooms copy the way in
  assert.equal(lookOf(by, at, [4, 2, 3, 1, 4, 42]), 3);
  assert.deepEqual(shape.cells, [[0, 0], [0, -1], [1, 0], [1, -1]]);
  assert.deepEqual(shape.links, [
    { a: 0, dir: E, b: 2 },
    { a: 1, dir: S, b: 0, oneWay: true },
    { a: 1, dir: E, b: 3 },
    { a: 2, dir: U, b: 3 },
  ]);
  // with no way out, the first room is the way in; nothing picked, no shape
  assert.deepEqual(shapeOf(by, at, [1, 2]).cells, [[0, 0], [1, 0]]);
  assert.deepEqual(shapeOf(by, at, []), { cells: [], links: [] });

  // put down in the open, it makes the same rooms joined the same ways
  const far = [room(50, 'Hut')];
  const p = placed(new Map(far.map((r) => [r.room, r])), layOut(far, 50), shape, 10, 10);
  assert.deepEqual(p.rooms, [51, 52, 53, 54]);
  const made = stamped(far, p, far[0]!);
  const exits = (n: number) => made.find((r) => r.room === n)!.exits.map((e) => [e.dir, e.to]);
  // the well 51, the cellar 52, the gate 53, the road 54
  assert.deepEqual([exits(51), exits(52), exits(53), exits(54)], [
    [[E, 52]],
    [[W, 51], [U, 54]],
    [[S, 51], [E, 54]],
    [[W, 53], [D, 52]],
  ]);
});

test('a stamp of rooms drawn far apart (a floor up) brings the far parts in beside the first', () => {
  // the first part keeps its cells; one 8 cells off stays where it is; one 40 off, and another
  // 90 off, come in beside the parts before them, two cells clear, level with the first's top
  assert.deepEqual(
    drawnNear([[0, 0], [1, 0], [1, 1], [9, 1], [40, 40], [41, 41], [-90, 90]]),
    [[0, 0], [1, 0], [1, 1], [9, 1], [12, 0], [13, 1], [16, 0]],
  );
  // near ones only: as they are
  assert.deepEqual(drawnNear([[0, 0], [8, -8]]), [[0, 0], [8, -8]]);
});

test('a stamp of the rooms picked has its new rooms copy one that looks like most of them', () => {
  // a block of fields with a gate at its edge: the gate is the way in, and a field is the look
  const rooms = [room(1, 'Gate', [E, 2], [W, 9]), room(2, 'Field', [W, 1], [E, 3]), room(3, ' Field', [W, 2]), room(9, 'Road', [E, 1])];
  const by = new Map(rooms.map((r) => [r.room, r]));
  const at = new Map<number, Spot>([[1, { x: 0, y: 0 }], [2, { x: 1, y: 0 }], [3, { x: 2, y: 0 }], [9, { x: -1, y: 0 }]]);
  assert.equal(wayIn(by, at, [3, 2, 1]), 1);
  assert.equal(lookOf(by, at, [3, 2, 1]), 2);
  // the way in when its name is as common as any; nothing picked, no room
  assert.equal(lookOf(by, at, [1, 2]), 1);
  assert.equal(lookOf(by, at, []), undefined);
});

test('a stamp of the rooms picked is named after most of them', () => {
  assert.equal(stampName([room(1, 'Road'), room(2, 'Gate'), room(3, ' Road '), room(4, '')], 1), 'Road');
  assert.equal(stampName([room(1, ''), room(2, '  ')], 3), 'Rooms of map 3');
  assert.equal(stampName([room(1, 'A very long name for a room that goes on and on')], 1), 'A very long name for a room that goes on');
});
