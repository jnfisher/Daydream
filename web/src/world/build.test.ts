// Build mode's moves and the changes they make to the map's rooms (build.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { alongWay, dug, freeNumbers, joined, lineTo, moveFrom, reachCount, reached, reachJoin, reachLine, reachLinks, renamed } from './build.ts';
import { layOut } from './layout.ts';
import type { MapRoom } from './model.ts';

const N = 0;
const S = 1;
const E = 2;
const W = 3;
const NE = 4;
const NW = 5;
const SE = 6;
const U = 8;
const D = 9;

/** A room from `[dir, to, kind?, para1?]` exits. */
function room(n: number, name: string, ...exits: [number, number, number?, number?][]): MapRoom {
  return {
    room: n,
    name,
    exits: exits.map(([dir, to, kind, para1]) => ({ dir, to, kind: kind ?? 0, para1: para1 ?? 0, para2: 0, para3: 0, para4: 0 })),
    shop: 0,
    spell: 0,
    command: 0,
    npc: 7,
    control: 0,
    death: 0,
    exit_room: 0,
    room_type: 0,
    mon_type: 3,
    min_index: 0,
    max_index: 0,
    max_regen: 0,
    light: -20,
  };
}

/**
 * The map, drawn:      6 . .        6 Tower  (0,-3)
 *                      . 4 .        4 Hill   (1,-2)
 *                      2 5 .        2 Road, 5 Hut
 *                      1 3 .        1 Gate, 3 Field: no exit between them
 * The gate's west leads to room 40, which the map does not have, and its south to map 2; the
 * hut's east is a door to room 41, which it does not have either.
 */
function map() {
  const rooms = [
    room(1, 'Gate', [N, 2], [W, 40], [S, 9, 8, 2]),
    room(2, 'Road', [S, 1], [E, 5]),
    room(3, 'Field', [N, 5]),
    room(4, 'Hill', [S, 5], [NW, 6]),
    room(5, 'Hut', [N, 4], [S, 3], [E, 41, 7], [W, 2]),
    room(6, 'Tower', [SE, 4]),
  ];
  const by = new Map(rooms.map((r) => [r.room, r]));
  return { rooms, by, layout: layOut(rooms, 1) };
}

const exits = (r: MapRoom | undefined) => r?.exits.map((e) => [e.dir, e.to]);

test('the map is drawn as the tests think', () => {
  const { layout } = map();
  const at = (n: number) => layout.at.get(n);
  assert.deepEqual([at(1), at(2), at(3), at(4), at(5), at(6)], [
    { x: 0, y: 0 },
    { x: 0, y: -1 },
    { x: 1, y: 0 },
    { x: 1, y: -2 },
    { x: 1, y: -1 },
    { x: 0, y: -3 },
  ]);
});

test('a way with an exit is walked, to this map or another', () => {
  const { by, layout } = map();
  assert.deepEqual(moveFrom(by, layout, 1, N), { kind: 'walk', to: 2 });
  const leave = moveFrom(by, layout, 1, S);
  assert.equal(leave.kind, 'leave');
  assert.equal(leave.kind === 'leave' && leave.exit.para1, 2);
});

test('a room drawn next door with no exit between is joined', () => {
  const { by, layout } = map();
  assert.deepEqual(moveFrom(by, layout, 1, E), { kind: 'join', to: 3 });
});

test('free cells are dug, numbered after the highest room, a line as far as the cells stay free', () => {
  const { by, layout } = map();
  assert.deepEqual(freeNumbers(by, 2), [7, 8]);
  assert.deepEqual(moveFrom(by, layout, 3, S), { kind: 'dig', rooms: [7] });
  assert.deepEqual(moveFrom(by, layout, 3, E, 3), { kind: 'dig', rooms: [7, 8, 9] });
  // north of the road one cell is free, and the tower has the next
  assert.deepEqual(moveFrom(by, layout, 2, N, 3), { kind: 'dig', rooms: [7] });
  // up and down have no cells
  assert.deepEqual(moveFrom(by, layout, 1, U, 2), { kind: 'dig', rooms: [7, 8] });
});

test('an exit to a room the map does not have digs that room, when it is a plain exit', () => {
  const { by, layout } = map();
  assert.deepEqual(moveFrom(by, layout, 1, W, 5), { kind: 'dig', rooms: [40] });
  assert.equal(moveFrom(by, layout, 5, E).kind, 'stop');
});

test('a line is dug as the server digs it: copies of the room, joined both ways, without its NPC', () => {
  const { rooms } = map();
  const after = dug(rooms, 3, S, [7, 8]);
  const by = new Map(after.map((r) => [r.room, r]));
  assert.deepEqual(exits(by.get(3)), [[N, 5], [S, 7]]);
  assert.deepEqual(exits(by.get(7)), [[N, 3], [S, 8]]);
  assert.deepEqual(exits(by.get(8)), [[N, 7]]);
  assert.equal(by.get(8)!.name, 'Field');
  assert.equal(by.get(8)!.mon_type, 3);
  assert.equal(by.get(8)!.npc, 0);
  // the rooms it did not touch are the same objects
  assert.equal(by.get(5), rooms[4]);
  // drawn where they were dug, the rest holding still
  const l = layOut(after, 1);
  assert.deepEqual([l.at.get(7), l.at.get(8)], [
    { x: 1, y: 1 },
    { x: 1, y: 2 },
  ]);
  assert.deepEqual(l.at.get(6), { x: 0, y: -3 });
});

test('a join makes the way back, unless the room has an exit that way to another room', () => {
  const { rooms } = map();
  const a = joined(rooms, 1, E, 3);
  assert.equal(a.oneWay, false);
  assert.deepEqual(exits(a.rooms.find((r) => r.room === 1)), [[N, 2], [S, 9], [E, 3], [W, 40]]);
  assert.deepEqual(exits(a.rooms.find((r) => r.room === 3)), [[N, 5], [W, 1]]);
  // the hut's west leads to the road
  const b = joined(rooms, 1, E, 5);
  assert.equal(b.oneWay, true);
  assert.equal(b.rooms.find((r) => r.room === 5), rooms[4]);
});

test('down digs too, and a rename changes only the name', () => {
  const { rooms } = map();
  const after = dug(rooms, 1, D, [7]);
  assert.deepEqual(exits(after.find((r) => r.room === 7)), [[U, 1]]);
  const named = renamed(after, 7, 'Cellar');
  assert.equal(named.find((r) => r.room === 7)!.name, 'Cellar');
  assert.equal(named.find((r) => r.room === 1), after.find((r) => r.room === 1));
});

test('on a map laid out by hand, rooms dug from a moved room land beside it', () => {
  const { rooms, by, layout } = map();
  const pins = new Map([...layout.at].map(([n, s]) => [n, { ...s }]));
  pins.set(3, { x: 4, y: 0 });
  const moved = layOut(rooms, 1, pins);
  assert.deepEqual(moveFrom(by, moved, 3, S, 2), { kind: 'dig', rooms: [7, 8] });
  const l = layOut(dug(rooms, 3, S, [7, 8]), 1, pins);
  assert.deepEqual([l.at.get(7), l.at.get(8)], [
    { x: 4, y: 1 },
    { x: 4, y: 2 },
  ]);
  assert.deepEqual(l.at.get(5), layout.at.get(5));
});

test('a corridor dragged out of a room digs the free cells and joins the rooms in its way', () => {
  const { rooms, by, layout } = map();
  // east of the gate: the field (no exit between them yet), then two free cells
  const r = reachLine(by, layout, 1, E, 3)!;
  assert.equal(r.kind, 'line');
  assert.ok(r.kind === 'line');
  assert.deepEqual(r.cells, [
    { x: 1, y: 0, room: 3, made: false },
    { x: 2, y: 0, room: 7, made: true },
    { x: 3, y: 0, room: 8, made: true },
  ]);
  assert.deepEqual(r.links.map((l) => [l.from, l.to, l.out, l.back]), [
    [1, 3, true, 'make'],
    [3, 7, true, 'make'],
    [7, 8, true, 'make'],
  ]);
  assert.equal(r.stop, null);
  assert.deepEqual(reachCount(r), { made: [7, 8], exits: 6, joined: [3] });
  assert.deepEqual(reachLinks(r), [
    { from: 1, dir: E, to: 3, one_way: false },
    { from: 3, dir: E, to: 7, one_way: false },
    { from: 7, dir: E, to: 8, one_way: false },
  ]);
  // made as the server makes it: copies of the gate without its NPC, joined both ways
  const after = new Map(reached(rooms, r, by.get(1)!).map((x) => [x.room, x]));
  assert.deepEqual(exits(after.get(1)), [[N, 2], [S, 9], [E, 3], [W, 40]]);
  assert.deepEqual(exits(after.get(3)), [[N, 5], [E, 7], [W, 1]]);
  assert.deepEqual(exits(after.get(7)), [[E, 8], [W, 3]]);
  assert.deepEqual(exits(after.get(8)), [[W, 7]]);
  assert.equal(after.get(8)!.name, 'Gate');
  assert.equal(after.get(8)!.npc, 0);
  assert.equal(after.get(5), rooms[4]);
});

test('a corridor goes through the ways there already, and stops short of an exit to elsewhere', () => {
  const { by, layout } = map();
  // north of the field: the hut and the hill, joined already, then a free cell
  const up = reachLine(by, layout, 3, N, 3)!;
  assert.ok(up.kind === 'line');
  assert.deepEqual(up.cells.map((c) => [c.room, c.made]), [[5, false], [4, false], [7, true]]);
  assert.deepEqual(up.links.map((l) => [l.from, l.to, l.out, l.back]), [
    [3, 5, false, 'have'],
    [5, 4, false, 'have'],
    [4, 7, true, 'make'],
  ]);
  assert.deepEqual(reachCount(up), { made: [7], exits: 2, joined: [4] });
  // east of the road: the hut, then the hut's door to a room the map does not have
  const east = reachLine(by, layout, 2, E, 4)!;
  assert.ok(east.kind === 'line');
  assert.deepEqual(east.cells.map((c) => c.room), [5]);
  assert.deepEqual(east.stop, { x: 1.5, y: -1, why: 'Room 5 has an exit east already, to room 41, which the map does not have.' });
  assert.deepEqual(reachLinks(east), []);
});

test('a corridor into a room whose way back leads elsewhere is one-way there', () => {
  const rooms = [room(1, 'A', [E, 2]), room(2, 'B', [W, 1], [E, 4]), room(3, 'C'), room(4, 'D')];
  const pins = new Map([
    [1, { x: 0, y: 0 }],
    [2, { x: 1, y: 0 }],
    [3, { x: 2, y: 0 }],
    [4, { x: 5, y: 5 }],
  ]);
  const by = new Map(rooms.map((r) => [r.room, r]));
  const r = reachLine(by, layOut(rooms, 1, pins), 3, W, 2)!;
  assert.ok(r.kind === 'line');
  assert.deepEqual(r.links.map((l) => [l.from, l.to, l.out, l.back]), [
    [3, 2, true, 'taken'],
    [2, 1, false, 'have'],
  ]);
  assert.deepEqual(reachLinks(r), [{ from: 3, dir: W, to: 2, one_way: true }]);
  const after = reached(rooms, r, rooms[2]!);
  // the room it reaches keeps its way back as it was
  assert.equal(after.find((x) => x.room === 2), rooms[1]);
  assert.deepEqual(exits(after.find((x) => x.room === 3)), [[W, 2]]);
});

test('dragged onto a room anywhere, the room is joined that way', () => {
  const { by } = map();
  assert.deepEqual(reachJoin(by, 3, E, 6), { kind: 'join', from: 3, dir: E, to: 6, oneWay: false });
  // the road's east leads to the hut, so the join is one-way
  assert.deepEqual(reachJoin(by, 3, W, 2), { kind: 'join', from: 3, dir: W, to: 2, oneWay: true });
  // the gate has its north already; a room is not joined to itself
  assert.equal(reachJoin(by, 1, N, 6), null);
  assert.equal(reachJoin(by, 3, E, 3), null);
  const r = reachJoin(by, 3, E, 6)!;
  assert.deepEqual(reachCount(r), { made: [], exits: 2, joined: [6] });
  assert.deepEqual(reachLinks(r), [{ from: 3, dir: E, to: 6, one_way: false }]);
});

test('a cell is in a line from a room across, down or corner to corner, and a drag goes along a way', () => {
  assert.deepEqual(lineTo(3, 0), { dir: E, count: 3 });
  assert.deepEqual(lineTo(-2, -2), { dir: NW, count: 2 });
  assert.deepEqual(lineTo(0, -4), { dir: N, count: 4 });
  assert.equal(lineTo(1, 2), null);
  assert.equal(lineTo(0, 0), null);
  assert.equal(alongWay(E, 3, 1), 3);
  assert.equal(alongWay(NE, 2, -3), 3);
  assert.equal(alongWay(E, -2, 0), -2);
});
