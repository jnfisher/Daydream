// The map's layout (layout.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { layOut, type LRoom } from './layout.ts';

const N = 0;
const S = 1;
const E = 2;
const W = 3;
const NE = 4;
const U = 8;

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
