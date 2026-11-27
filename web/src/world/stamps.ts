// Stamps: room shapes put down on the map at once. A shape is cells on the grid and the links
// between them, its first cell (its way in) under the pointer. Put down at a cell, each of its
// cells is a new room where the map has none, or the room drawn there, which keeps everything
// it has and only gains exits in its free slots. A link is made both ways only where both
// slots are free (or already lead there): a link with another exit in its way is left whole,
// so no one-way exit comes of it by surprise, unless the exits in the way are to be replaced.
// Rooms that merely touch the stamp stay apart until its seam is joined. New rooms take the
// numbers after the map's highest, in walking order from the first cell, and copy a room as a
// dig does: the first room the stamp lands on or joins, else the one picked.
//
// The server makes the same checks as it writes the stamp (`POST /api/map/stamp`,
// crates/daydream-nmr/src/map.rs's `stamp`).
//
// Pure: no DOM, so `node --test` runs its tests (stamps.test.ts); its imports name their
// files whole, as node needs.

import { cellKey, dirOf, OPPOSITE, STEP, type Layout } from './layout.ts';
import type { MapExit, MapRoom } from './model.ts';

/** A link between two cells of a shape: from cell `a` in `dir` to cell `b`. */
export interface ShapeLink {
  a: number;
  dir: number;
  b: number;
  oneWay?: boolean;
}

export interface Shape {
  /** Each cell as [column, row] from the first, the cell under the pointer. */
  cells: [number, number][];
  links: ShapeLink[];
}

export interface Size {
  /** What it measures, for a person: `long`, `wide`. */
  word: string;
  min: number;
  max: number;
  start: number;
}

export interface StampKind {
  id: string;
  name: string;
  /** What it is, in a few words. */
  about: string;
  /** Up to two: [ and ] change the first, { and } the second. */
  sizes: Size[];
  /** Drawn afresh by each roll. */
  random?: boolean;
  /** A stamp saved from a map: its id (`.daydream/stamps.json`). */
  saved?: string;
  /** The room its new rooms copy on that map when it lands on none and no room is picked (a saved stamp's own way in). */
  brush?: { map: number; room: number };
  make(sizes: readonly number[], seed: number): Shape;
}

const [N, S, E, W, NE, NW, SE, SW, U] = [0, 1, 2, 3, 4, 5, 6, 7, 8];

/** A shape being drawn, cell by cell. */
class Sketch {
  private readonly cells: [number, number][] = [];
  private readonly at = new Map<string, number>();
  private readonly links: ShapeLink[] = [];
  private readonly joined = new Set<string>();

  cell(x: number, y: number): number {
    const k = `${x},${y}`;
    let i = this.at.get(k);
    if (i === undefined) {
      i = this.cells.length;
      this.cells.push([x, y]);
      this.at.set(k, i);
    }
    return i;
  }

  has(x: number, y: number): boolean {
    return this.at.has(`${x},${y}`);
  }

  get size(): number {
    return this.cells.length;
  }

  /** Every cell, in the order drawn. */
  all(): readonly [number, number][] {
    return this.cells;
  }

  /** Joins two neighbouring cells (making them), the way the second lies from the first. */
  join(x1: number, y1: number, x2: number, y2: number, oneWay = false): void {
    const dir = dirOf(x2 - x1, y2 - y1);
    if (dir < 0) throw new Error(`cells ${x1},${y1} and ${x2},${y2} are not neighbours`);
    this.joinWay(x1, y1, x2, y2, dir, oneWay);
  }

  /** Joins two cells by a way of its own (up or down). */
  joinWay(x1: number, y1: number, x2: number, y2: number, dir: number, oneWay = false): void {
    const a = this.cell(x1, y1);
    const b = this.cell(x2, y2);
    const k = a < b ? `${a}-${b}` : `${b}-${a}`;
    if (a === b || this.joined.has(k)) return;
    this.joined.add(k);
    this.links.push(oneWay ? { a, dir, b, oneWay } : { a, dir, b });
  }

  /** Whether two cells are joined. */
  joins(x1: number, y1: number, x2: number, y2: number): boolean {
    const a = this.at.get(`${x1},${y1}`);
    const b = this.at.get(`${x2},${y2}`);
    return a !== undefined && b !== undefined && this.joined.has(a < b ? `${a}-${b}` : `${b}-${a}`);
  }

  /** A line of `n` steps from (x, y) going (dx, dy); answers where it ends. */
  line(x: number, y: number, dx: number, dy: number, n: number): [number, number] {
    this.cell(x, y);
    for (let i = 0; i < n; i++) {
      this.join(x, y, x + dx, y + dy);
      x += dx;
      y += dy;
    }
    return [x, y];
  }

  /** The shape, its first cell the one at (x, y), every cell counted from it. */
  done(x: number, y: number): Shape {
    const first = this.at.get(`${x},${y}`) ?? 0;
    const order = [first, ...[...this.cells.keys()].filter((i) => i !== first)];
    const index = new Map(order.map((old, i) => [old, i]));
    const [ox, oy] = this.cells[first] ?? [0, 0];
    return {
      cells: order.map((i) => [this.cells[i]![0] - ox, this.cells[i]![1] - oy]),
      links: this.links.map((l) => ({ ...l, a: index.get(l.a)!, b: index.get(l.b)! })),
    };
  }
}

/** Numbers from a seed, the same each time (mulberry32). */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const shuffled = <T>(list: readonly T[], r: () => number): T[] => {
  const out = [...list];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(r() * (i + 1));
    [out[i], out[j]] = [out[j]!, out[i]!];
  }
  return out;
};

/** Joins every pair of neighbouring cells of a sketch: the four ways, or all eight. */
function joinAll(s: Sketch, eight: boolean): void {
  for (const [x, y] of [...s.all()]) {
    for (const d of eight ? [E, S, SE, SW] : [E, S]) {
      const [dx, dy] = STEP[d]!;
      if (s.has(x + dx, y + dy)) s.join(x, y, x + dx, y + dy);
    }
  }
}

/** The bottom row's cell nearest the middle: the way into an area. */
function bottomMiddle(s: Sketch): [number, number] {
  let best: [number, number] = [0, 0];
  let bestY = -Infinity;
  const xs = s.all().map(([x]) => x);
  const mid = (Math.min(...xs) + Math.max(...xs)) / 2;
  for (const [x, y] of s.all()) {
    if (y > bestY || (y === bestY && Math.abs(x - mid) < Math.abs(best[0] - mid))) {
      best = [x, y];
      bestY = y;
    }
  }
  return best;
}

const size = (word: string, min: number, max: number, start: number): Size => ({ word, min, max, start });

/** The shapes, in the drawer's order: keys 1 to 9 pick the first nine. */
export const STAMPS: readonly StampKind[] = [
  {
    id: 'corridor',
    name: 'Corridor',
    about: 'A straight run of rooms',
    sizes: [size('long', 2, 40, 5)],
    make([n = 5]) {
      const s = new Sketch();
      s.line(0, 0, 1, 0, n - 1);
      return s.done(0, 0);
    },
  },
  {
    id: 'bend',
    name: 'L-bend',
    about: 'A corridor that turns a corner',
    sizes: [size('arm', 2, 20, 3)],
    make([n = 3]) {
      const s = new Sketch();
      const [x, y] = s.line(0, 0, 1, 0, n - 1);
      s.line(x, y, 0, 1, n - 1);
      return s.done(0, 0);
    },
  },
  {
    id: 'tee',
    name: 'T',
    about: 'A corridor that meets another',
    sizes: [size('arm', 1, 15, 2)],
    make([n = 2]) {
      const s = new Sketch();
      s.line(0, 0, 0, -1, n);
      s.line(-n, -n, 1, 0, n * 2);
      return s.done(0, 0);
    },
  },
  {
    id: 'cross',
    name: 'Crossroads',
    about: 'Four ways from the middle',
    sizes: [size('arm', 1, 15, 2)],
    make([n = 2]) {
      const s = new Sketch();
      for (const [dx, dy] of [[0, 1], [0, -1], [1, 0], [-1, 0]] as const) s.line(0, 0, dx, dy, n);
      return s.done(0, n);
    },
  },
  {
    id: 'hall',
    name: 'Open hall',
    about: 'A block of rooms, each joined to its four neighbours',
    sizes: [size('wide', 2, 20, 3), size('deep', 2, 20, 3)],
    make([w = 3, h = 3]) {
      const s = new Sketch();
      for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) s.cell(x, y);
      joinAll(s, false);
      return s.done(Math.floor((w - 1) / 2), h - 1);
    },
  },
  {
    id: 'ring',
    name: 'Ring',
    about: 'A loop around an empty middle',
    sizes: [size('wide', 3, 20, 4), size('deep', 3, 20, 4)],
    make([w = 4, h = 4]) {
      const s = new Sketch();
      const [x1, y1] = s.line(0, 0, 1, 0, w - 1);
      const [x2, y2] = s.line(x1, y1, 0, 1, h - 1);
      const [x3, y3] = s.line(x2, y2, -1, 0, w - 1);
      s.line(x3, y3, 0, -1, h - 1);
      return s.done(Math.floor((w - 1) / 2), h - 1);
    },
  },
  {
    id: 'plaza',
    name: 'Plaza',
    about: 'An open square with its corners cut, every way open',
    sizes: [size('across', 3, 15, 5)],
    make([n = 5]) {
      const s = new Sketch();
      const cut = Math.floor(n / 3);
      for (let y = 0; y < n; y++) {
        for (let x = 0; x < n; x++) if (Math.min(x, n - 1 - x) + Math.min(y, n - 1 - y) >= cut) s.cell(x, y);
      }
      joinAll(s, true);
      return s.done(Math.floor((n - 1) / 2), n - 1);
    },
  },
  {
    id: 'spiral',
    name: 'Spiral',
    about: 'A path winding in to the middle',
    sizes: [size('turns', 2, 10, 3)],
    make([t = 3]) {
      const s = new Sketch();
      let [x, y] = [0, 0];
      const ways: [number, number][] = [[1, 0], [0, 1], [-1, 0], [0, -1]];
      for (let k = 0; k < t * 2; k++) {
        const [dx, dy] = ways[k % 4]!;
        [x, y] = s.line(x, y, dx, dy, 2 * (Math.floor(k / 2) + 1));
      }
      return s.done(x, y);
    },
  },
  {
    id: 'maze',
    name: 'Maze',
    about: 'Twisty little passages, one way between any two',
    sizes: [size('wide', 3, 15, 5), size('deep', 3, 15, 5)],
    random: true,
    make([w = 5, h = 5], seed) {
      const s = new Sketch();
      const r = rng(seed);
      const start: [number, number] = [Math.floor((w - 1) / 2), h - 1];
      const seen = new Set<string>([start.join()]);
      const stack: [number, number][] = [start];
      s.cell(...start);
      while (stack.length) {
        const [x, y] = stack.at(-1)!;
        const next = shuffled([N, S, E, W], r)
          .map((d) => [x + STEP[d]![0], y + STEP[d]![1]] as [number, number])
          .find(([nx, ny]) => nx >= 0 && ny >= 0 && nx < w && ny < h && !seen.has(`${nx},${ny}`));
        if (!next) {
          stack.pop();
          continue;
        }
        seen.add(next.join());
        s.join(x, y, ...next);
        stack.push(next);
      }
      return s.done(...start);
    },
  },
  {
    id: 'stairs',
    name: 'Stairwell',
    about: 'Floors joined by up and down',
    sizes: [size('floors', 2, 9, 3)],
    make([n = 3]) {
      const s = new Sketch();
      for (let k = 0; k + 1 < n; k++) s.joinWay(k, -k, k + 1, -k - 1, U);
      return s.done(0, 0);
    },
  },
  {
    id: 'gate',
    name: 'Gatehouse',
    about: 'A way through, with a guard room each side',
    sizes: [],
    make() {
      const s = new Sketch();
      s.line(0, 0, 0, -1, 2);
      s.join(0, -1, -1, -1);
      s.join(0, -1, 1, -1);
      return s.done(0, 0);
    },
  },
  {
    id: 'spur',
    name: 'Lair spur',
    about: 'A winding dead end that opens into a den',
    sizes: [size('long', 1, 12, 3)],
    make([n = 3]) {
      const s = new Sketch();
      const zig: [number, number][] = [[0, -1], [1, -1], [0, -1], [-1, -1]];
      let [x, y] = [0, 0];
      s.cell(0, 0);
      for (let i = 0; i < n; i++) {
        const [dx, dy] = zig[i % 4]!;
        s.join(x, y, x + dx, y + dy);
        [x, y] = [x + dx, y + dy];
      }
      s.join(x, y, x, y - 1);
      s.line(x - 1, y - 1, 1, 0, 2);
      return s.done(0, 0);
    },
  },
  {
    id: 'shops',
    name: 'Shop row',
    about: 'A street with a door off it at every step, side to side',
    sizes: [size('long', 2, 20, 5)],
    make([n = 5]) {
      const s = new Sketch();
      s.line(0, 0, 1, 0, n - 1);
      for (let x = 0; x < n; x++) s.join(x, 0, x, x % 2 ? 1 : -1);
      return s.done(0, 0);
    },
  },
  {
    id: 'forest',
    name: 'Forest patch',
    about: 'An open, ragged wood, every way open where the trees allow',
    sizes: [size('rooms', 5, 120, 18)],
    random: true,
    make([n = 18], seed) {
      const r = rng(seed);
      const s = new Sketch();
      s.cell(0, 0);
      for (let tries = 0; s.size < n && tries < n * 60; tries++) {
        const [x, y] = s.all()[Math.floor(r() * s.size)]!;
        const [dx, dy] = STEP[Math.floor(r() * 8)]!;
        if (!s.has(x + dx, y + dy)) s.cell(x + dx, y + dy);
      }
      // a way to every tree first, then more ways at random, no two crossing
      treeOf(s, r, true);
      for (const [x, y] of shuffled(s.all(), r)) {
        for (const d of [E, S, SE, SW]) {
          const [dx, dy] = STEP[d]!;
          if (!s.has(x + dx, y + dy) || r() > 0.55 || crosses(s, x, y, d)) continue;
          s.join(x, y, x + dx, y + dy);
        }
      }
      return s.done(...bottomMiddle(s));
    },
  },
  {
    id: 'cave',
    name: 'Cave',
    about: 'Narrow tunnels that wander and now and then meet',
    sizes: [size('rooms', 5, 120, 14)],
    random: true,
    make([n = 14], seed) {
      const r = rng(seed);
      const s = new Sketch();
      let [x, y] = [0, 0];
      s.cell(0, 0);
      let way = Math.floor(r() * 4);
      for (let tries = 0; s.size < n && tries < n * 60; tries++) {
        // a tunnel keeps on its way more often than not
        if (r() < 0.4) way = Math.floor(r() * 4);
        const [dx, dy] = STEP[way]!;
        const fresh = !s.has(x + dx, y + dy);
        if (fresh || r() < 0.12) s.join(x, y, x + dx, y + dy);
        // back into a tunnel already dug: carry on from where it leads
        if (fresh || s.joins(x, y, x + dx, y + dy)) [x, y] = [x + dx, y + dy];
      }
      return s.done(...bottomMiddle(s));
    },
  },
  {
    id: 'drop',
    name: 'One-way drop',
    about: 'A fall with no way back up',
    sizes: [size('fall', 1, 6, 1)],
    make([n = 1]) {
      const s = new Sketch();
      for (let y = 0; y < n; y++) s.join(0, y, 0, y + 1, true);
      return s.done(0, 0);
    },
  },
];

/** Whether a diagonal from (x, y) would cross one already made. */
function crosses(s: Sketch, x: number, y: number, d: number): boolean {
  if (d === NE) return s.joins(x, y - 1, x + 1, y);
  if (d === NW) return s.joins(x, y - 1, x - 1, y);
  if (d === SE) return s.joins(x + 1, y, x, y + 1);
  if (d === SW) return s.joins(x - 1, y, x, y + 1);
  return false;
}

/**
 * Joins every cell of a sketch to one already joined, from its first, in a random order (a
 * spanning tree), no diagonal crossing another unless a cell has no other way in.
 */
function treeOf(s: Sketch, r: () => number, eight: boolean): void {
  const [first] = s.all();
  if (!first) return;
  const seen = new Set<string>([first.join()]);
  const queue: [number, number][] = [first];
  const ways = eight ? [N, S, E, W, NE, NW, SE, SW] : [N, S, E, W];
  for (;;) {
    while (queue.length) {
      const [x, y] = queue.splice(Math.floor(r() * queue.length), 1)[0]!;
      for (const d of shuffled(ways, r)) {
        const [dx, dy] = STEP[d]!;
        const k = `${x + dx},${y + dy}`;
        if (!s.has(x + dx, y + dy) || seen.has(k) || crosses(s, x, y, d)) continue;
        seen.add(k);
        s.join(x, y, x + dx, y + dy);
        queue.push([x + dx, y + dy]);
      }
    }
    // a cell walled off by crossings: joined across one after all, and the walk goes on
    const left = s.all().find(([x, y]) => !seen.has(`${x},${y}`) && ways.some((d) => seen.has(`${x + STEP[d]![0]},${y + STEP[d]![1]}`)));
    if (!left) return;
    const [x, y] = left;
    const d = ways.find((w) => seen.has(`${x + STEP[w]![0]},${y + STEP[w]![1]}`))!;
    s.join(x + STEP[d]![0], y + STEP[d]![1], x, y);
    seen.add(`${x},${y}`);
    queue.push([x, y]);
  }
}

// ---- turning and mirroring ---------------------------------------------------------------

/** How a stamp is turned: flipped across, flipped up and down, then turned clockwise `turns` quarters. */
export interface Turn {
  turns: number;
  flipX: boolean;
  flipY: boolean;
}

export const UPRIGHT: Turn = { turns: 0, flipX: false, flipY: false };

/** A direction a quarter turn clockwise: N to E, NE to SE; up and down stay. */
const CLOCKWISE = [E, W, S, N, SE, NE, SW, NW, 8, 9];
/** A direction mirrored across: E to W. */
const ACROSS = [N, S, W, E, NW, NE, SW, SE, 8, 9];
/** A direction mirrored up and down: N to S. */
const UPDOWN = [S, N, E, W, SE, SW, NE, NW, 8, 9];

/** A shape as turned. */
export function turned(shape: Shape, t: Turn): Shape {
  const q = ((t.turns % 4) + 4) % 4;
  const dir = (d: number) => {
    let out = d;
    if (t.flipX) out = ACROSS[out]!;
    if (t.flipY) out = UPDOWN[out]!;
    for (let i = 0; i < q; i++) out = CLOCKWISE[out]!;
    return out;
  };
  const cell = ([x, y]: [number, number]): [number, number] => {
    if (t.flipX) x = -x;
    if (t.flipY) y = -y;
    for (let i = 0; i < q; i++) [x, y] = [-y, x];
    // no negative zeros: they key cells the same
    return [x + 0, y + 0];
  };
  return {
    cells: shape.cells.map(cell),
    links: shape.links.map((l) => ({ ...l, dir: dir(l.dir) })),
  };
}

// ---- putting one down ----------------------------------------------------------------

export type CellSort = 'new' | 'merge' | 'skip';

export interface PlacedCell {
  x: number;
  y: number;
  /** `new` a room the stamp makes, `merge` the room drawn there, `skip` a cell left alone. */
  sort: CellSort;
  /** Its room (a new one's number to be), or null when skipped. */
  room: number | null;
}

/** `make` new exits, `have` already there, `clash` left for an exit in its way, `replace` that exit replaced, `skip` a cell left alone. */
export type LinkSort = 'make' | 'have' | 'clash' | 'replace' | 'skip';

export interface PlacedLink {
  /** Where its two ends are drawn. */
  ax: number;
  ay: number;
  bx: number;
  by: number;
  from: number | null;
  to: number | null;
  dir: number;
  oneWay: boolean;
  sort: LinkSort;
  /** A join across the seam, to a room the stamp only touches. */
  seam?: boolean;
}

export interface StampOptions {
  /** Change no room already on the map: their cells are left alone. */
  keep: boolean;
  /** Replace the exits in a link's way instead of leaving the link. */
  overwrite: boolean;
  /** Join new rooms to the rooms they touch (north, south, east and west). */
  seam: boolean;
}

export const NO_OPTIONS: StampOptions = { keep: false, overwrite: false, seam: false };

export interface Placed {
  cells: PlacedCell[];
  links: PlacedLink[];
  /** The new rooms' numbers, in walking order. */
  rooms: number[];
  /** The exits of each room the stamp makes or changes, as it leaves them. */
  exits: Map<number, MapExit[]>;
  /** Rooms made, exits set (each way counted), rooms already there that gain one, links left, cells left alone. */
  count: { rooms: number; exits: number; joined: number; left: number; skipped: number };
}

/** An exit as a stamp makes it. */
const plain = (dir: number, to: number): MapExit => ({ dir, to, kind: 0, para1: 0, para2: 0, para3: 0, para4: 0 });

/** Shape `shape` put down with its first cell at (x, y), on the map whose rooms are `by`, drawn as `layout`. */
export function placed(by: ReadonlyMap<number, MapRoom>, layout: Layout, shape: Shape, x: number, y: number, o: StampOptions = NO_OPTIONS): Placed {
  const mine = new Set<number>();
  const cells: PlacedCell[] = shape.cells.map(([dx, dy]) => {
    const [cx, cy] = [x + dx, y + dy];
    mine.add(cellKey(cx, cy));
    const there = layout.cell.get(cellKey(cx, cy));
    if (there === undefined) return { x: cx, y: cy, sort: 'new', room: null };
    return o.keep ? { x: cx, y: cy, sort: 'skip', room: null } : { x: cx, y: cy, sort: 'merge', room: there };
  });
  // the new rooms' numbers, after the map's highest, in walking order from the first cell
  let top = 0;
  for (const n of by.keys()) if (n > top) top = n;
  const next = new Map<number, number[]>();
  for (const l of shape.links) {
    for (const [p, q] of [[l.a, l.b], [l.b, l.a]] as const) {
      let list = next.get(p);
      if (!list) next.set(p, (list = []));
      list.push(q);
    }
  }
  const order: number[] = [];
  const seen = new Set<number>();
  const walk = (from: number) => {
    const queue = [from];
    seen.add(from);
    while (queue.length) {
      const i = queue.shift()!;
      order.push(i);
      for (const j of next.get(i) ?? []) {
        if (!seen.has(j)) {
          seen.add(j);
          queue.push(j);
        }
      }
    }
  };
  for (let i = 0; i < cells.length; i++) if (!seen.has(i)) walk(i);
  const rooms: number[] = [];
  for (const i of order) {
    const c = cells[i]!;
    if (c.sort !== 'new') continue;
    c.room = ++top;
    rooms.push(c.room);
  }
  // each link in turn, against the exits as the links before it leave them
  const exits = new Map<number, MapExit[]>();
  const exitsOf = (n: number) => exits.get(n) ?? by.get(n)?.exits ?? [];
  const slot = (n: number, dir: number, to: number): 'free' | 'have' | 'taken' => {
    const e = exitsOf(n).find((x) => x.dir === dir);
    if (!e) return 'free';
    return e.to === to && e.kind !== 8 ? 'have' : 'taken';
  };
  const set = (n: number, dir: number, to: number) => {
    exits.set(n, [...exitsOf(n).filter((x) => x.dir !== dir), plain(dir, to)].sort((a, b) => a.dir - b.dir));
  };
  for (const n of rooms) exits.set(n, []);
  const count = { rooms: rooms.length, exits: 0, joined: 0, left: 0, skipped: cells.filter((c) => c.sort === 'skip').length };
  const links: PlacedLink[] = [];
  const link = (a: PlacedCell, b: { x: number; y: number; room: number | null }, dir: number, oneWay: boolean, seam: boolean) => {
    const out: PlacedLink = { ax: a.x, ay: a.y, bx: b.x, by: b.y, from: a.room, to: b.room, dir, oneWay, sort: 'skip', ...(seam ? { seam } : {}) };
    links.push(out);
    if (a.room === null || b.room === null) return;
    const there = slot(a.room, dir, b.room);
    const back = oneWay ? 'have' : slot(b.room, OPPOSITE[dir]!, a.room);
    if (there === 'taken' || back === 'taken') {
      if (!o.overwrite) {
        out.sort = 'clash';
        count.left++;
        return;
      }
      out.sort = 'replace';
    } else out.sort = there === 'have' && back === 'have' ? 'have' : 'make';
    if (there !== 'have') {
      set(a.room, dir, b.room);
      count.exits++;
    }
    if (back !== 'have') {
      set(b.room, OPPOSITE[dir]!, a.room);
      count.exits++;
    }
  };
  for (const l of shape.links) link(cells[l.a]!, cells[l.b]!, l.dir, !!l.oneWay, false);
  if (o.seam) {
    for (const c of cells) {
      if (c.sort !== 'new') continue;
      for (const d of [N, S, E, W]) {
        const [dx, dy] = STEP[d]!;
        const k = cellKey(c.x + dx, c.y + dy);
        const there = layout.cell.get(k);
        if (there === undefined || mine.has(k)) continue;
        link(c, { x: c.x + dx, y: c.y + dy, room: there }, d, false, true);
      }
    }
  }
  for (const n of exits.keys()) if (by.has(n)) count.joined++;
  return { cells, links, rooms, exits, count };
}

/**
 * The room new rooms copy: the first room the stamp lands on (in its cells' order, so its way in
 * first), else the first it joins across the seam, else `fallback`: a stamp off a forest grows forest.
 */
export function brushOf(p: Placed, fallback: number | null): number | null {
  const on = p.cells.find((c) => c.sort === 'merge');
  if (on) return on.room;
  const seam = p.links.find((l) => l.seam && l.to !== null && l.sort !== 'clash' && l.sort !== 'skip');
  return seam?.to ?? fallback;
}

/** The map's rooms as a stamp leaves them: new rooms copy `brush`, without its exits, items, coins and permanent NPC. */
export function stamped(rooms: readonly MapRoom[], p: Placed, brush: MapRoom): MapRoom[] {
  const out = rooms.map((r) => {
    const e = p.exits.get(r.room);
    return e ? { ...r, exits: e } : r;
  });
  for (const n of p.rooms) out.push({ ...brush, room: n, npc: 0, exits: p.exits.get(n) ?? [] });
  return out;
}

/** What the server is asked to write (`POST /api/map/stamp`). */
export interface StampRequest {
  map: number;
  brush: number;
  rooms: number[];
  links: { from: number; dir: number; to: number; one_way: boolean }[];
  overwrite: boolean;
  what: string;
  /** Dug out of a room by hand (a corridor dragged out of it): the history says Dig, not Stamp. */
  dug?: boolean;
}

export function request(map: number, brush: number, p: Placed, o: StampOptions, what: string): StampRequest {
  return {
    map,
    brush,
    rooms: p.rooms,
    links: p.links
      .filter((l) => (l.sort === 'make' || l.sort === 'replace') && l.from !== null && l.to !== null)
      .map((l) => ({ from: l.from!, dir: l.dir, to: l.to!, one_way: l.oneWay })),
    overwrite: o.overwrite,
    what,
  };
}

/** A stamp's summary, as its toast says it: `9 rooms 5012–5020 · 22 exits · joined 2 · 1 left`. */
export function summary(map: number, p: Placed): string {
  const c = p.count;
  const parts: string[] = [];
  if (c.rooms === 1) parts.push(`1 room ${map}/${p.rooms[0]}`);
  else if (c.rooms > 1) parts.push(`${c.rooms} rooms ${map}/${Math.min(...p.rooms)}–${Math.max(...p.rooms)}`);
  parts.push(`${c.exits} exit${c.exits === 1 ? '' : 's'}`);
  if (c.joined) parts.push(`joined ${c.joined}`);
  if (c.left) parts.push(`${c.left} left`);
  if (c.skipped) parts.push(`${c.skipped} cell${c.skipped === 1 ? '' : 's'} kept`);
  return parts.join(' · ');
}

/** `a ring`, `an open hall`: a stamp for the history's label. */
export function named(kind: StampKind): string {
  const n = kind.name.toLowerCase();
  return `${/^[aeiou]/.test(n) ? 'an' : 'a'} ${n}`;
}
