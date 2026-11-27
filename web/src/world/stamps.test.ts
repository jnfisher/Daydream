// Stamps: their shapes, turned and mirrored, and what one does put down on a map (stamps.ts),
// with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dirOf, layOut, type Spot } from './layout.ts';
import type { MapRoom } from './model.ts';
import { brushOf, named, placed, request, STAMPS, stamped, summary, turned, UPRIGHT, type Shape } from './stamps.ts';

const [N, S, E, W] = [0, 1, 2, 3];

/** A room from `[dir, to]` exits. */
function room(n: number, name: string, ...exits: [number, number][]): MapRoom {
  return {
    room: n,
    name,
    exits: exits.map(([dir, to]) => ({ dir, to, kind: 0, para1: 0, para2: 0, para3: 0, para4: 0 })),
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

const kind = (id: string) => STAMPS.find((k) => k.id === id)!;
const shapeOf = (id: string, sizes: number[] = [], seed = 1) => kind(id).make(sizes, seed);

/** Every cell once, links between its cells, neighbours' ways as they lie, and one piece. */
function wellMade(s: Shape, what: string) {
  assert.deepEqual(s.cells[0], [0, 0], `${what}: the first cell is under the pointer`);
  assert.equal(new Set(s.cells.map((c) => c.join())).size, s.cells.length, `${what}: a cell twice`);
  const next = new Map<number, number[]>();
  for (const l of s.links) {
    const [a, b] = [s.cells[l.a], s.cells[l.b]];
    assert.ok(a && b, `${what}: a link to no cell`);
    if (l.dir < 8) assert.equal(dirOf(b[0] - a[0], b[1] - a[1]), l.dir, `${what}: a link ${l.dir} from ${a} to ${b}`);
    for (const [p, q] of [[l.a, l.b], [l.b, l.a]] as const) next.set(p, [...(next.get(p) ?? []), q]);
  }
  const seen = new Set([0]);
  const queue = [0];
  while (queue.length) for (const j of next.get(queue.shift()!) ?? []) if (!seen.has(j)) seen.add(j), queue.push(j);
  assert.equal(seen.size, s.cells.length, `${what}: in one piece`);
}

test('every stamp is one piece, at its least, its first and its most', () => {
  for (const k of STAMPS) {
    for (const pick of ['min', 'start', 'max'] as const) {
      for (const seed of k.random ? [1, 2, 3, 99] : [1]) {
        const sizes = k.sizes.map((s) => s[pick]);
        const s = k.make(sizes, seed);
        wellMade(s, `${k.name} ${sizes.join('x')} #${seed}`);
        for (const t of [{ turns: 1, flipX: false, flipY: false }, { turns: 3, flipX: true, flipY: false }, { turns: 2, flipX: false, flipY: true }]) {
          wellMade(turned(s, t), `${k.name} turned`);
        }
      }
    }
  }
  assert.equal(shapeOf('corridor', [5]).cells.length, 5);
  assert.equal(shapeOf('hall', [3, 2]).cells.length, 6);
  assert.equal(shapeOf('hall', [3, 2]).links.length, 7);
  assert.equal(shapeOf('ring', [4, 3]).cells.length, 10);
  assert.equal(shapeOf('ring', [4, 3]).links.length, 10, 'a ring closes');
  assert.equal(shapeOf('maze', [5, 5], 7).links.length, 24, 'a maze is a tree');
  assert.deepEqual(shapeOf('maze', [5, 5], 7), shapeOf('maze', [5, 5], 7), 'the same seed, the same maze');
  assert.notDeepEqual(shapeOf('maze', [5, 5], 7), shapeOf('maze', [5, 5], 8));
  assert.equal(shapeOf('forest', [30], 4).cells.length, 30);
  const drop = shapeOf('drop', [2]);
  assert.ok(drop.links.every((l) => l.oneWay && l.dir === S));
  const stairs = shapeOf('stairs', [3]);
  assert.deepEqual(stairs.links.map((l) => l.dir), [8, 8], 'floors go up');
  assert.equal(named(kind('hall')), 'an open hall');
  assert.equal(named(kind('ring')), 'a ring');
});

test('a stamp turns clockwise and mirrors, its ways with it', () => {
  const c = shapeOf('corridor', [3]);
  assert.deepEqual(turned(c, UPRIGHT).cells, [[0, 0], [1, 0], [2, 0]]);
  assert.deepEqual(turned(c, { turns: 1, flipX: false, flipY: false }).cells, [[0, 0], [0, 1], [0, 2]]);
  assert.deepEqual(turned(c, { turns: 1, flipX: false, flipY: false }).links.map((l) => l.dir), [S, S]);
  assert.deepEqual(turned(c, { turns: 0, flipX: true, flipY: false }).cells, [[0, 0], [-1, 0], [-2, 0]]);
  assert.deepEqual(turned(c, { turns: 0, flipX: true, flipY: false }).links.map((l) => l.dir), [W, W]);
  assert.deepEqual(turned(c, { turns: 3, flipX: false, flipY: false }).links.map((l) => l.dir), [N, N]);
});

/**
 * The map:   1 2      1 Gate (0,0), 2 Road (1,0): joined east and west
 *            . .
 *            3 .      3 Well (0,2): its north leads to room 9, elsewhere
 */
function map() {
  const rooms = [room(1, 'Gate', [E, 2]), room(2, 'Road', [W, 1]), room(3, 'Well', [N, 9])];
  const pins = new Map<number, Spot>([[1, { x: 0, y: 0 }], [2, { x: 1, y: 0 }], [3, { x: 0, y: 2 }]]);
  const layout = layOut(rooms, 1, pins);
  return { rooms, by: new Map(rooms.map((r) => [r.room, r])), layout };
}

test('a stamp joins the room under its first cell and numbers its new rooms after the highest', () => {
  const { rooms, by, layout } = map();
  // a corridor of three from the road east: the road gains an exit, two new rooms after it
  const p = placed(by, layout, shapeOf('corridor', [3]), 1, 0);
  assert.deepEqual(p.cells.map((c) => [c.sort, c.room]), [['merge', 2], ['new', 4], ['new', 5]]);
  assert.deepEqual(p.links.map((l) => [l.from, l.dir, l.to, l.sort]), [[2, E, 4, 'make'], [4, E, 5, 'make']]);
  assert.deepEqual(p.count, { rooms: 2, exits: 4, joined: 1, left: 0, skipped: 0 });
  assert.equal(summary(1, p), '2 rooms 1/4–5 · 4 exits · joined 1');
  // the map as it leaves it: new rooms copy the brush, without its permanent NPC
  const after = stamped(rooms, p, by.get(2)!);
  assert.deepEqual(after.find((r) => r.room === 2)!.exits.map((e) => [e.dir, e.to]), [[E, 4], [W, 1]]);
  const four = after.find((r) => r.room === 4)!;
  assert.deepEqual([four.name, four.npc, four.mon_type], ['Road', 0, 3]);
  assert.deepEqual(four.exits.map((e) => [e.dir, e.to]), [[E, 5], [W, 2]]);
  assert.deepEqual(request(1, 2, p, { keep: false, overwrite: false, seam: false }, 'a corridor'), {
    map: 1,
    brush: 2,
    rooms: [4, 5],
    links: [
      { from: 2, dir: E, to: 4, one_way: false },
      { from: 4, dir: E, to: 5, one_way: false },
    ],
    overwrite: false,
    what: 'a corridor',
  });

  // over the gate and the road: they are already joined, so only the new room is
  const q = placed(by, layout, shapeOf('corridor', [3]), 0, 0);
  assert.deepEqual(q.links.map((l) => l.sort), ['have', 'make']);
  assert.deepEqual(q.count, { rooms: 1, exits: 2, joined: 1, left: 0, skipped: 0 });

  // a T in the open: its rooms numbered walking from its foot
  const t = placed(by, layout, shapeOf('tee', [1]), 5, 5);
  assert.deepEqual(t.cells.map((c) => c.room), [4, 5, 6, 7]);
  assert.deepEqual(t.rooms, [4, 5, 6, 7]);

  // new rooms copy the first room the stamp lands on, else the one picked
  assert.equal(brushOf(p, 3), 2, 'the road under its way in');
  assert.equal(brushOf(placed(by, layout, shapeOf('corridor', [3]), -1, 0), 3), 1, 'the gate, its second cell');
  assert.equal(brushOf(t, 3), 3, 'in the open: the room picked');
});

test('a link with an exit in its way is left whole, unless told to replace it', () => {
  const { by, layout } = map();
  // north from the well: its north leads to room 9, so the way up is left, and no one-way exit made
  const north = turned(shapeOf('corridor', [2]), { turns: 3, flipX: false, flipY: false });
  const p = placed(by, layout, north, 0, 2);
  assert.deepEqual(p.links.map((l) => [l.from, l.dir, l.to, l.sort]), [[3, N, 4, 'clash']]);
  assert.deepEqual(p.count, { rooms: 1, exits: 0, joined: 0, left: 1, skipped: 0 });
  assert.deepEqual(p.exits.get(4), []);
  assert.equal(request(1, 3, p, { keep: false, overwrite: false, seam: false }, 'x').links.length, 0);
  const over = placed(by, layout, north, 0, 2, { keep: false, overwrite: true, seam: false });
  assert.deepEqual(over.links.map((l) => l.sort), ['replace']);
  assert.deepEqual(over.exits.get(3)!.map((e) => [e.dir, e.to]), [[N, 4]]);
  assert.equal(request(1, 3, over, { keep: false, overwrite: true, seam: false }, 'x').overwrite, true);
  // kept: the well's cell is left alone, and so is the link to it
  const kept = placed(by, layout, north, 0, 2, { keep: true, overwrite: false, seam: false });
  assert.deepEqual(kept.cells.map((c) => c.sort), ['skip', 'new']);
  assert.deepEqual(kept.links.map((l) => l.sort), ['skip']);
  assert.deepEqual(kept.count, { rooms: 1, exits: 0, joined: 0, left: 0, skipped: 1 });
});

test('rooms the stamp only touches are joined across the seam when asked', () => {
  const { by, layout } = map();
  // a corridor along the row between the road and the well
  const c = shapeOf('corridor', [2]);
  const apart = placed(by, layout, c, 0, 1);
  assert.equal(apart.links.length, 1, 'touching is not joining');
  const p = placed(by, layout, c, 0, 1, { keep: false, overwrite: false, seam: true });
  const seams = p.links.filter((l) => l.seam).map((l) => [l.from, l.dir, l.to, l.sort]);
  // the gate above the first, the road above the second; the well's north is in use
  assert.deepEqual(seams, [[4, N, 1, 'make'], [4, S, 3, 'clash'], [5, N, 2, 'make']]);
  assert.deepEqual(p.count, { rooms: 2, exits: 6, joined: 2, left: 1, skipped: 0 });
  // its new rooms copy the first room joined across the seam; apart, the room picked
  assert.equal(brushOf(p, 3), 1);
  assert.equal(brushOf(apart, 3), 3);
});
