// Build mode's moves and the changes they make to the map's rooms (build.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dug, freeNumbers, joined, moveFrom, renamed } from './build.ts';
import { layOut } from './layout.ts';
import type { MapRoom } from './model.ts';

const N = 0;
const S = 1;
const E = 2;
const W = 3;
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
