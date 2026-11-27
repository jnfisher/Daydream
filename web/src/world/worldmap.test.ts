// The world view's layout (worldmap.ts), with `npm test`.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { lineUnder, mapAt, worldLayout, type MapLink } from './worldmap.ts';

const link = (from: number, to: number, exits = 1, room = 1, to_room = 1): MapLink => ({ from, to, exits, room, to_room });

/** A world like a realm's: a town (map 1) every other map leads back to, a ring of maps around
 * it, a few ways one way only, an island no map change reaches, and an add-on's map. */
function world() {
  const maps: [number, number][] = [];
  for (let m = 1; m <= 16; m++) maps.push([m, m === 1 ? 3400 : 40 + ((m * 397) % 2900)]);
  const links: MapLink[] = [];
  for (let m = 2; m <= 12; m++) links.push(link(1, m, m % 5 + 1), link(m, 1, m % 4 + 1));
  for (let m = 2; m < 12; m++) links.push(link(m, m + 1), link(m + 1, m));
  links.push(link(13, 1), link(14, 1), link(15, 2), link(2, 15), link(12, 30, 2));
  return { maps, links };
}

const indexOf = (l: ReturnType<typeof worldLayout>) => {
  const at = new Map(l.nodes.map((n, i) => [n.map, i]));
  return l.edges.map((e) => [at.get(e.a)!, at.get(e.b)!] as [number, number]);
};

test('every map is a disc, larger for more rooms, and a map exits lead to that has no rooms is one too', () => {
  const { maps, links } = world();
  const l = worldLayout(maps, links);
  assert.deepEqual(
    l.nodes.map((n) => n.map),
    [...maps.map(([m]) => m), 30],
  );
  const [town, small] = [l.nodes.find((n) => n.map === 1)!, l.nodes.find((n) => n.map === 7)!];
  assert.ok(town.r > small.r, `the town's disc (${town.r}) is larger than map 7's (${small.r})`);
  const addOn = l.nodes.find((n) => n.map === 30)!;
  assert.equal(addOn.missing, true);
  assert.equal(addOn.rooms, 0);
  assert.ok(l.nodes.filter((n) => n.map !== 30).every((n) => !n.missing));
});

test('the exits each way between two maps are one line, and a map change onto its own map none', () => {
  const l = worldLayout(
    [
      [1, 10],
      [2, 10],
      [3, 10],
    ],
    [link(2, 1, 3, 40, 7), link(1, 2, 5, 9, 12), link(3, 1), link(2, 2)],
  );
  assert.equal(l.edges.length, 2);
  const [a, b] = l.edges;
  assert.deepEqual([a!.a, a!.b, a!.ab?.exits, a!.ba?.exits, a!.ab?.room, a!.ba?.room], [1, 2, 5, 3, 9, 40]);
  // one way only: from 3 into 1
  assert.deepEqual([b!.a, b!.b, b!.ab, b!.ba?.from], [1, 3, null, 3]);
});

test('no two discs overlap, and no line passes under a disc not its own', () => {
  const { maps, links } = world();
  for (const aspect of [1, 1.8, 0.6]) {
    const l = worldLayout(maps, links, aspect);
    for (let i = 0; i < l.nodes.length; i++)
      for (let j = i + 1; j < l.nodes.length; j++) {
        const [p, q] = [l.nodes[i]!, l.nodes[j]!];
        assert.ok(Math.hypot(p.x - q.x, p.y - q.y) >= p.r + q.r, `maps ${p.map} and ${q.map} overlap`);
      }
    assert.equal(lineUnder(l.nodes, indexOf(l), 0), 0);
    // the bounds hold every disc
    for (const n of l.nodes) assert.ok(n.x - n.r >= l.bounds.x0 - 1e-9 && n.x + n.r <= l.bounds.x1 + 1e-9 && n.y - n.r >= l.bounds.y0 - 1e-9 && n.y + n.r <= l.bounds.y1 + 1e-9);
  }
});

test('the same world is laid out the same way every time', () => {
  const { maps, links } = world();
  const a = worldLayout(maps, links, 1.5);
  const b = worldLayout(maps, links, 1.5);
  assert.deepEqual(
    a.nodes.map((n) => [n.map, n.x, n.y]),
    b.nodes.map((n) => [n.map, n.x, n.y]),
  );
});

test('two clusters of maps joined by one way lie along the box, wide or tall', () => {
  const maps: [number, number][] = [1, 2, 3, 4, 5, 6, 7, 8].map((m) => [m, 100]);
  const pairs = [[1, 2], [1, 3], [2, 3], [1, 4], [2, 4], [3, 4], [5, 6], [5, 7], [6, 7], [5, 8], [6, 8], [7, 8], [4, 5]];
  const links = pairs.flatMap(([a, b]) => [link(a!, b!), link(b!, a!)]);
  const wide = worldLayout(maps, links, 3).bounds;
  const tall = worldLayout(maps, links, 1 / 3).bounds;
  assert.ok(wide.x1 - wide.x0 > 1.4 * (wide.y1 - wide.y0), 'wide');
  assert.ok(tall.y1 - tall.y0 > 1.4 * (tall.x1 - tall.x0), 'tall');
});

test('a lone map sits in the middle, and a world with none is empty', () => {
  const one = worldLayout([[4, 12]], []);
  assert.deepEqual([one.nodes.length, one.nodes[0]!.x, one.nodes[0]!.y], [1, 0, 0]);
  const none = worldLayout([], []);
  assert.deepEqual([none.nodes, none.edges, none.bounds], [[], [], { x0: 0, y0: 0, x1: 0, y1: 0 }]);
});

test('the disc under a point, with a little more for a finger', () => {
  const { maps, links } = world();
  const l = worldLayout(maps, links);
  const town = l.nodes.find((n) => n.map === 1)!;
  assert.equal(mapAt(l, town.x, town.y)?.map, 1);
  assert.equal(mapAt(l, town.x + town.r + 5, town.y)?.map ?? null, mapAt(l, town.x + town.r + 5, town.y, 0)?.map ?? null);
  assert.equal(mapAt(l, town.x + town.r + 5, town.y, 6)?.map, 1);
  assert.equal(mapAt(l, l.bounds.x1 + 50, l.bounds.y1 + 50), null);
});
