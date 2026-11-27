// The map's layout (layout.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { layOut, layOutLevels, type LRoom, type Spot } from './layout.ts';

const N = 0;
const S = 1;
const E = 2;
const W = 3;
const NE = 4;
const U = 8;
const D = 9;

/** Rooms from `[room, name, [dir, to, kind?]...]`. */
function rooms(...list: [number, string, ...[number, number, number?][]][]): LRoom[] {
  return list.map(([room, name, ...exits]) => ({
    room,
    name,
    exits: exits.map(([dir, to, kind]) => ({ dir, to, kind: kind ?? 0 })),
  }));
}

const spot = (l: ReturnType<typeof layOut>, n: number) => {
  const s = l.at.get(n);
  return s ? [s.x, s.y] : null;
};

test('rooms are placed a step away by their exits, from the start room', () => {
  const l = layOut(rooms([1, 'Gate', [N, 2], [E, 3]], [2, 'Road', [S, 1], [NE, 4]], [3, 'Road', [W, 1]], [4, 'Hill', [U, 9]]), 1);
  assert.deepEqual(spot(l, 1), [0, 0]);
  assert.deepEqual(spot(l, 2), [0, -1]);
  assert.deepEqual(spot(l, 3), [1, 0]);
  assert.deepEqual(spot(l, 4), [1, -2]);
  assert.equal(l.islands, 1);
  assert.equal(l.displaced.size, 0);
});

test('a room reached only by its own exit in is placed by that exit', () => {
  // 5 leads into 1 from the west, and nothing leads to 5
  const l = layOut(rooms([1, 'Gate'], [5, 'Drop', [E, 1]]), 1);
  assert.deepEqual(spot(l, 5), [-1, 0]);
  assert.equal(l.islands, 1);
});

test('no room vanishes: one whose cell is taken goes to the nearest free cell', () => {
  // 1 -> E 2 -> N 3 -> W 4, and 1 -> N 5: 4 and 5 both want (0, -1)
  const l = layOut(rooms([1, 'A', [E, 2], [N, 5]], [2, 'B', [N, 3]], [3, 'C', [W, 4]], [4, 'D'], [5, 'E']), 1);
  assert.equal(l.at.size, 5);
  assert.deepEqual(spot(l, 5), [0, -1]);
  assert.ok(l.displaced.has(4));
  const cells = new Set([...l.at.values()].map((s) => `${s.x},${s.y}`));
  assert.equal(cells.size, 5, 'one room per cell');
  // beside where it belongs
  const [x, y] = spot(l, 4)!;
  assert.ok(Math.max(Math.abs(x - 0), Math.abs(y + 1)) === 1);
});

test('rooms the walk never reaches are islands below the start room', () => {
  const l = layOut(rooms([1, 'A', [E, 2]], [2, 'B'], [7, 'Far', [E, 8]], [8, 'Far']), 1);
  assert.equal(l.islands, 2);
  assert.equal(l.island.get(7), 1);
  assert.ok(spot(l, 7)![1] > 0, 'below');
  // the island keeps its own shape
  assert.equal(spot(l, 8)![0] - spot(l, 7)![0], 1);
});

test('map changes and up and down exits are not walked', () => {
  const l = layOut(rooms([1, 'A', [E, 2, 8], [U, 3]], [2, 'B'], [3, 'C']), 1);
  assert.equal(l.islands, 3);
});

test('the same rooms give the same layout', () => {
  const list = rooms([1, 'A', [E, 2], [N, 5]], [2, 'B', [N, 3]], [3, 'C', [W, 4]], [4, 'D'], [5, 'E'], [9, 'F']);
  const a = layOut(list, 1);
  const b = layOut([...list].reverse(), 1);
  for (const n of [1, 2, 3, 4, 5, 9]) assert.deepEqual(spot(a, n), spot(b, n));
});

test('a run of joined rooms with one name has one label, in its middle', () => {
  const l = layOut(rooms([1, 'Main Street', [E, 2]], [2, 'Main Street', [E, 3], [W, 1]], [3, 'Main Street', [W, 2], [E, 4]], [4, 'Inn', [W, 3]]), 1);
  const main = l.labels.find((x) => x.text === 'Main Street')!;
  assert.equal(main.size, 3);
  assert.deepEqual([main.x, main.y], [1, 0]);
  assert.equal(l.labels[0], main, 'longest first');
});

test('a start room the map does not have starts from its lowest room', () => {
  const l = layOut(rooms([3, 'A', [E, 4]], [4, 'B']), 1);
  assert.deepEqual(spot(l, 3), [0, 0]);
  assert.equal(l.islands, 1);
});

/** Every room's cell, as pins. */
const pinsOf = (l: ReturnType<typeof layOut>) => new Map([...l.at].map(([n, s]) => [n, { ...s }]));

test('pinned rooms are drawn in their cells, and the rest hold still', () => {
  const list = rooms([1, 'A', [E, 2], [N, 5]], [2, 'B', [N, 3]], [3, 'C', [W, 4]], [4, 'D'], [5, 'E'], [7, 'Far', [E, 8]], [8, 'Far']);
  const auto = layOut(list, 1);
  const pins = pinsOf(auto);
  pins.set(3, { x: 6, y: -6 });
  const l = layOut(list, 1, pins);
  assert.deepEqual(spot(l, 3), [6, -6]);
  for (const n of [1, 2, 4, 5, 7, 8]) assert.deepEqual(spot(l, n), spot(auto, n), `room ${n}`);
  assert.equal(l.pinned.size, 7);
  assert.equal(l.islands, 1, 'one walk, from the pins');
  assert.equal(l.displaced.size, 0);
});

test('a room joined to a pinned room is placed beside it, as one dug from it is', () => {
  const l = layOut(rooms([1, 'A', [E, 2]], [2, 'B', [W, 1], [S, 3]], [3, 'C', [N, 2]]), 1, new Map([[1, { x: 10, y: 10 }]]));
  assert.deepEqual(spot(l, 1), [10, 10]);
  assert.deepEqual(spot(l, 2), [11, 10]);
  assert.deepEqual(spot(l, 3), [11, 11]);
  assert.deepEqual([...l.pinned], [1]);
});

test('two pins in one cell: the lower room keeps it, and the walk places the other', () => {
  const l = layOut(rooms([1, 'A', [E, 2]], [2, 'B', [W, 1]]), 1, new Map([[1, { x: 0, y: 0 }], [2, { x: 0, y: 0 }]]));
  assert.deepEqual(spot(l, 1), [0, 0]);
  assert.deepEqual(spot(l, 2), [1, 0]);
  assert.deepEqual([...l.pinned], [1]);
});

test('rooms no pinned room reaches are islands below, and pins of rooms not on the map are let be', () => {
  const l = layOut(rooms([1, 'A', [E, 2]], [2, 'B'], [7, 'Far', [E, 8]], [8, 'Far']), 1, new Map([[1, { x: 0, y: -20 }], [99, { x: 5, y: 5 }]]));
  assert.deepEqual(spot(l, 1), [0, -20]);
  assert.equal(l.island.get(7), 1);
  assert.ok(spot(l, 7)![1] > -20, 'below');
  assert.equal(l.at.has(99), false);
});

// ---- in levels ---------------------------------------------------------------------------

const place = (l: ReturnType<typeof layOutLevels>, n: number) => {
  const s = l.at.get(n);
  return s ? [s.x, s.y, s.z] : null;
};

test('in levels, up and down lead a level up and down, into the same cell', () => {
  // a tower: 1 up to 2 up to 3, a balcony east of 2, and a cellar under 1
  const l = layOutLevels(rooms([1, 'Hall', [U, 2], [D, 5]], [2, 'Stair', [D, 1], [U, 3], [E, 4]], [3, 'Top', [D, 2]], [4, 'Balcony', [W, 2]], [5, 'Cellar', [U, 1]]), 1);
  assert.deepEqual(place(l, 1), [0, 0, 0]);
  assert.deepEqual(place(l, 2), [0, 0, 1]);
  assert.deepEqual(place(l, 3), [0, 0, 2]);
  assert.deepEqual(place(l, 4), [1, 0, 1]);
  assert.deepEqual(place(l, 5), [0, 0, -1]);
  assert.deepEqual([...l.of.keys()], [-1, 0, 1, 2], 'lowest first');
  const one = l.of.get(1)!;
  assert.deepEqual([...one.at.keys()].sort(), [2, 4]);
  assert.equal(one.cell.size, 2, 'its own cells only');
  assert.equal(one.islands, 1);
  assert.deepEqual(one.bounds, { x0: 0, y0: 0, x1: 1, y1: 0 });
  // drawn flat, the same rooms are islands
  assert.equal(layOut(rooms([1, 'Hall', [U, 2]], [2, 'Stair', [D, 1]]), 1).islands, 2);
});

test('in levels, a room reached only by its own way up or down is placed by it', () => {
  // nothing leads to 6, which goes up into 1: it is a level below 1
  const l = layOutLevels(rooms([1, 'Hall'], [6, 'Pit', [U, 1]]), 1);
  assert.deepEqual(place(l, 6), [0, 0, -1]);
});

test('in levels, a room whose cell on its level is taken goes to the nearest free cell of that level', () => {
  // 1 up to 2; 1 east to 3, up to 4, west to 7: 7 wants 2's cell a level up
  const l = layOutLevels(rooms([1, 'A', [E, 3], [U, 2]], [2, 'B'], [3, 'C', [U, 4]], [4, 'D', [W, 7]], [7, 'E']), 1);
  assert.deepEqual(place(l, 2), [0, 0, 1]);
  assert.deepEqual(place(l, 4), [1, 0, 1]);
  const [x, y, z] = place(l, 7)!;
  assert.equal(z, 1);
  assert.ok(!(x === 0 && y === 0) && !(x === 1 && y === 0), 'a free cell');
  assert.ok(l.of.get(1)!.displaced.has(7));
  assert.equal(l.of.get(0)!.displaced.size, 0);
});

test('in levels, the shortest way decides a room’s level, and other ways to it lead off the level', () => {
  // 1 east to 2 east to 3, and 1 up to 3: 3 is a level up, and 2's way east leads there
  const l = layOutLevels(rooms([1, 'A', [E, 2], [U, 3]], [2, 'B', [E, 3]], [3, 'C']), 1);
  assert.deepEqual(place(l, 3), [0, 0, 1]);
  assert.deepEqual(place(l, 2), [1, 0, 0]);
  assert.equal(l.of.get(0)!.at.has(3), false);
});

test('in levels, pins name their level, and the rooms they lead to are placed around them', () => {
  const list = rooms([1, 'A', [U, 2]], [2, 'B', [D, 1], [E, 3]], [3, 'C', [W, 2]]);
  const pins = new Map<number, Spot>([[2, { x: 5, y: 5, z: 3 }]]);
  const l = layOutLevels(list, 1, pins);
  assert.deepEqual(place(l, 2), [5, 5, 3]);
  assert.deepEqual(place(l, 3), [6, 5, 3]);
  assert.deepEqual(place(l, 1), [5, 5, 2]);
  assert.deepEqual([...l.of.get(3)!.pinned], [2]);
  // pins without a level are on the start room's
  assert.deepEqual(place(layOutLevels(list, 1, new Map([[2, { x: 5, y: 5 }]])), 1), [5, 5, -1]);
  // drawn flat, a pin's level is let be
  assert.deepEqual(spot(layOut(list, 1, pins), 2), [5, 5]);
});

test('in levels, names are labelled level by level', () => {
  const l = layOutLevels(rooms([1, 'Tower', [U, 2], [E, 4]], [2, 'Tower', [D, 1], [U, 3]], [3, 'Tower', [D, 2]], [4, 'Tower', [W, 1]]), 1);
  assert.deepEqual(
    [...l.of].map(([z, lv]) => [z, lv.labels.map((x) => `${x.text} ${x.size}`)]),
    [
      [0, ['Tower 2']],
      [1, ['Tower 1']],
      [2, ['Tower 1']],
    ],
  );
});

test('in levels, rooms no way reaches are islands below, from the start room’s level', () => {
  const l = layOutLevels(rooms([1, 'A', [U, 2]], [2, 'B', [D, 1]], [7, 'Far', [U, 8]], [8, 'Far', [D, 7]]), 1);
  assert.equal(place(l, 7)![2], 0);
  assert.equal(place(l, 8)![2], 1);
  assert.ok(place(l, 7)![1] > 0, 'below');
  assert.deepEqual(place(l, 8)!.slice(0, 2), place(l, 7)!.slice(0, 2), 'stacked');
  assert.equal(l.of.get(0)!.island.get(7), 1);
  assert.equal(l.of.get(0)!.islands, 2);
});

test('in levels, the same rooms give the same layout', () => {
  const list = rooms([1, 'A', [E, 2], [U, 5]], [2, 'B', [U, 3]], [3, 'C', [W, 4]], [4, 'D'], [5, 'E'], [9, 'F', [D, 4]]);
  const a = layOutLevels(list, 1);
  const b = layOutLevels([...list].reverse(), 1);
  for (const n of [1, 2, 3, 4, 5, 9]) assert.deepEqual(place(a, n), place(b, n));
});
