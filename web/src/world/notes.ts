// The builder's notes on a map, kept in its side file beside the working copy and never
// published (crates/daydream-server/src/pins.rs): a few words on a room, perhaps a thing to do;
// a label drawn by a room; and a region, a name drawn around rooms. What a note is and the
// outline a region is drawn with; notelayer.ts draws them. This and the rest of the notes' code
// (noting.ts) load the first time a map with notes is drawn, or a note written.

import type { Spot } from './layout.ts';
import type { MapNote, NoteColour, NoteKind } from './model.ts';

/** Each kind's own colour, for a note given none. */
export const KIND_COLOUR: Record<NoteKind, NoteColour> = { note: 'butter', label: 'lilac', region: 'mint' };

export const colourOf = (n: Pick<MapNote, 'kind' | 'colour'>): NoteColour => n.colour ?? KIND_COLOUR[n.kind];

/** A new note's id: random, so an undo and a redo put back that note and no other. */
export function newId(): string {
  const a = new Uint32Array(2);
  crypto.getRandomValues(a);
  return (a[0]!.toString(36) + a[1]!.toString(36)).slice(0, 12);
}

/** A room's note or label: it has one of each at most. */
export const noteOn = (notes: readonly MapNote[], room: number, kind: 'note' | 'label'): MapNote | undefined =>
  notes.find((n) => n.kind === kind && n.rooms[0] === room);

/** Whether a note is a thing still to do. */
export const toDo = (n: MapNote): boolean => !!n.todo && !n.done;

/** A corner of an outline, in half cells: cell (x, y)'s top left corner is (2x − 1, 2y − 1). */
export type Corner = [number, number];

/**
 * The outline around some cells, as loops of corners in half cells, each going clockwise on the
 * screen (its cells on its right): one around each group of cells joined side by side, and one
 * around each hole in one. Cells that touch only at a corner are outlined apart. Corners along a
 * straight side are left out.
 */
export function outline(cells: Iterable<Spot>): Corner[][] {
  const has = new Set<string>();
  for (const c of cells) has.add(`${c.x},${c.y}`);
  // the sides with no cell beyond them, from the corner each starts at
  const from = new Map<string, Corner[]>();
  const sides: [Corner, Corner][] = [];
  const side = (a: Corner, b: Corner) => {
    sides.push([a, b]);
    const k = `${a[0]},${a[1]}`;
    const list = from.get(k);
    if (list) list.push(b);
    else from.set(k, [b]);
  };
  for (const key of has) {
    const [x, y] = key.split(',').map(Number) as [number, number];
    const [l, r, t, b] = [2 * x - 1, 2 * x + 1, 2 * y - 1, 2 * y + 1];
    if (!has.has(`${x},${y - 1}`)) side([l, t], [r, t]);
    if (!has.has(`${x + 1},${y}`)) side([r, t], [r, b]);
    if (!has.has(`${x},${y + 1}`)) side([r, b], [l, b]);
    if (!has.has(`${x - 1},${y}`)) side([l, b], [l, t]);
  }
  const used = new Set<string>();
  const id = (a: Corner, b: Corner) => `${a[0]},${a[1]}>${b[0]},${b[1]}`;
  const loops: Corner[][] = [];
  for (const [a0, b0] of sides) {
    if (used.has(id(a0, b0))) continue;
    const loop: Corner[] = [];
    let [a, b] = [a0, b0];
    for (;;) {
      used.add(id(a, b));
      loop.push(a);
      if (b[0] === a0[0] && b[1] === a0[1]) break;
      const next = (from.get(`${b[0]},${b[1]}`) ?? []).filter((c) => !used.has(id(b, c)));
      if (!next.length) break;
      // where two cells touch at a corner, turn towards the cell this side is of: the right
      const [dx, dy] = [Math.sign(b[0] - a[0]), Math.sign(b[1] - a[1])];
      const right = next.find((c) => Math.sign(c[0] - b[0]) === -dy && Math.sign(c[1] - b[1]) === dx);
      [a, b] = [b, right ?? next[0]!];
    }
    // only the corners where it turns
    loops.push(
      loop.filter((c, i) => {
        const p = loop[(i + loop.length - 1) % loop.length]!;
        const q = loop[(i + 1) % loop.length]!;
        return (c[0] - p[0]) * (q[1] - c[1]) !== (c[1] - p[1]) * (q[0] - c[0]);
      }),
    );
  }
  return loops;
}

/** The cell a region's name goes on: its top row's first. */
export function topLeft(cells: Iterable<Spot>): Spot | null {
  let best: Spot | null = null;
  for (const c of cells) if (!best || c.y < best.y || (c.y === best.y && c.x < best.x)) best = c;
  return best;
}
