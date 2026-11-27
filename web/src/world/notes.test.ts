import { test } from 'node:test';
import assert from 'node:assert/strict';
import { colourOf, newId, noteOn, outline, toDo, topLeft, type Corner } from './notes.ts';
import type { MapNote } from './model.ts';

const cells = (...xy: [number, number][]) => xy.map(([x, y]) => ({ x, y }));
/** A loop as a set of corners, whatever corner it starts at. */
const corners = (loop: Corner[]) => new Set(loop.map(([x, y]) => `${x},${y}`));

test('one cell is outlined by its four corners, clockwise on the screen', () => {
  const loops = outline(cells([0, 0]));
  assert.equal(loops.length, 1);
  const loop = loops[0]!;
  assert.deepEqual(corners(loop), new Set(['-1,-1', '1,-1', '1,1', '-1,1']));
  // from the top left corner the outline goes right, along the top
  const i = loop.findIndex(([x, y]) => x === -1 && y === -1);
  assert.deepEqual(loop[(i + 1) % 4], [1, -1]);
});

test('cells side by side share one outline, with corners only where it turns', () => {
  const row = outline(cells([0, 0], [1, 0], [2, 0]));
  assert.equal(row.length, 1);
  assert.deepEqual(corners(row[0]!), new Set(['-1,-1', '5,-1', '5,1', '-1,1']));
  // an L turns six times
  const l = outline(cells([0, 0], [0, 1], [1, 1]));
  assert.equal(l.length, 1);
  assert.equal(l[0]!.length, 6);
  assert.deepEqual(corners(l[0]!), new Set(['-1,-1', '1,-1', '1,1', '3,1', '3,3', '-1,3']));
});

test('cells touching only at a corner are outlined apart', () => {
  const loops = outline(cells([0, 0], [1, 1]));
  assert.equal(loops.length, 2);
  assert.deepEqual(
    loops.map((l) => [...corners(l)].sort()).sort(),
    [
      ['-1,-1', '-1,1', '1,-1', '1,1'],
      ['1,1', '1,3', '3,1', '3,3'],
    ],
  );
});

test('a ring of cells has its hole outlined too, the other way round', () => {
  const ring = cells([0, 0], [1, 0], [2, 0], [0, 1], [2, 1], [0, 2], [1, 2], [2, 2]);
  const loops = outline(ring);
  assert.equal(loops.length, 2);
  const area = (l: Corner[]) => l.reduce((a, [x, y], i) => a + x * l[(i + 1) % l.length]![1] - l[(i + 1) % l.length]![0] * y, 0) / 2;
  const [outer, inner] = [...loops].sort((a, b) => Math.abs(area(b)) - Math.abs(area(a)));
  assert.deepEqual(corners(outer!), new Set(['-1,-1', '5,-1', '5,5', '-1,5']));
  assert.deepEqual(corners(inner!), new Set(['1,1', '3,1', '3,3', '1,3']));
  // clockwise on the screen (y down) is a positive area; the hole goes the other way
  assert.ok(area(outer!) > 0 && area(inner!) < 0);
  assert.deepEqual(outline([]), []);
});

test("a region's name goes on its top row's first cell", () => {
  assert.deepEqual(topLeft(cells([3, 2], [1, 2], [0, 4], [2, 1], [5, 1])), { x: 2, y: 1 });
  assert.equal(topLeft([]), null);
});

test('notes: a room has one note and one label, colours fall back to the kind, to-dos until done', () => {
  const notes: MapNote[] = [
    { id: 'a', kind: 'label', text: 'Gate', rooms: [5] },
    { id: 'b', kind: 'note', text: 'Add a guard', rooms: [5], todo: true },
    { id: 'c', kind: 'note', text: 'Done', rooms: [6], todo: true, done: true, colour: 'pink' },
    { id: 'd', kind: 'region', text: 'Docks', rooms: [5, 6] },
  ];
  assert.equal(noteOn(notes, 5, 'note')?.id, 'b');
  assert.equal(noteOn(notes, 5, 'label')?.id, 'a');
  assert.equal(noteOn(notes, 6, 'label'), undefined);
  assert.deepEqual(notes.map(colourOf), ['lilac', 'butter', 'pink', 'mint']);
  assert.deepEqual(notes.filter(toDo).map((n) => n.id), ['b']);
  const ids = new Set(Array.from({ length: 200 }, newId));
  assert.equal(ids.size, 200);
  for (const id of ids) assert.match(id, /^[0-9a-z]{1,12}$/);
});
