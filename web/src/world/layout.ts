// Where each room of a map is drawn. A walk from the start room places each room one step
// from the room it is reached from, by its exit's direction (N is one row up, NE one row up and
// one column right), and by the exits that lead into it too. Unlike NMR's 41 by 41 grid the
// map has no edge, and no room vanishes: a room whose cell is taken goes to the nearest free
// cell once the walk has placed every room that fits, and the walk goes on from it. Rooms the
// walk never reaches (joined only by up and down, by map changes or not at all) are walked in
// turn as islands of their own, laid out below the start room's. The same rooms always give
// the same layout, so the map holds still between visits and edits.
//
// Pure: no DOM, so `node --test` runs its tests (layout.test.ts).

/** A room as the layout needs it. */
export interface LRoom {
  room: number;
  name: string;
  exits: readonly { dir: number; to: number; kind: number }[];
}

/** Grid steps of directions 0 to 7 (N S E W NE NW SE SW), as [columns, rows]. */
export const STEP: readonly (readonly [number, number])[] = [
  [0, -1],
  [0, 1],
  [1, 0],
  [-1, 0],
  [1, -1],
  [-1, -1],
  [1, 1],
  [-1, 1],
];

/** The exit back: S for N, SW for NE, D for U. */
export const OPPOSITE = [1, 0, 3, 2, 7, 6, 5, 4, 9, 8];

/** The direction from one cell to its neighbour, or -1. */
export function dirOf(dx: number, dy: number): number {
  return STEP.findIndex(([x, y]) => x === dx && y === dy);
}

export interface Spot {
  x: number;
  y: number;
}

/** A name shared by a run of joined rooms, drawn once where the run is. */
export interface Label {
  x: number;
  y: number;
  text: string;
  /** How many rooms share it. */
  size: number;
}

export interface Layout {
  /** Where each room is. */
  at: Map<number, Spot>;
  /** The room in each cell, by `cellKey`. */
  cell: Map<number, number>;
  /** Rooms put away from where an exit wanted them, their cell being taken. */
  displaced: Set<number>;
  /** Each room's island: 0 is the start room's. */
  island: Map<number, number>;
  islands: number;
  bounds: { x0: number; y0: number; x1: number; y1: number };
  /** Longest runs first. */
  labels: Label[];
}

/** A cell's key in `Layout.cell`. */
export const cellKey = (x: number, y: number) => (x + 1_000_000) * 2_097_152 + (y + 1_000_000);

/** Whether the layout walks through an exit of this type: not a map change, nor a remote action. */
const follows = (kind: number) => kind !== 8 && kind !== 12;

/** Blank cells between islands. */
const GAP = 2;

interface Grown {
  seed: number;
  at: Map<number, Spot>;
  displaced: Set<number>;
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

export function layOut(given: readonly LRoom[], start: number): Layout {
  // in room order, whatever order they came in, so the same rooms give the same layout
  const rooms = [...given].sort((a, b) => a.room - b.room);
  const by = new Map(rooms.map((r) => [r.room, r]));
  // the exits into each room, so a room is placed by its way in as well as its way out
  const into = new Map<number, { from: number; dir: number }[]>();
  for (const r of rooms) {
    for (const e of r.exits) {
      if (e.dir > 7 || !follows(e.kind) || e.to === r.room || !by.has(e.to)) continue;
      let list = into.get(e.to);
      if (!list) into.set(e.to, (list = []));
      list.push({ from: r.room, dir: e.dir });
    }
  }
  const taken = new Set<number>();

  function grow(seed: number): Grown {
    const at = new Map<number, Spot>();
    const occupied = new Set<number>();
    const displaced = new Set<number>();
    const queue: number[] = [];
    const deferred: [number, number, number][] = [];
    const place = (n: number, x: number, y: number) => {
      at.set(n, { x, y });
      occupied.add(cellKey(x, y));
      taken.add(n);
      queue.push(n);
    };
    const want = (n: number, x: number, y: number) => {
      if (taken.has(n)) return;
      if (occupied.has(cellKey(x, y))) deferred.push([n, x, y]);
      else place(n, x, y);
    };
    place(seed, 0, 0);
    let next = 0;
    for (;;) {
      while (next < queue.length) {
        const n = queue[next++]!;
        const { x, y } = at.get(n)!;
        for (const e of by.get(n)!.exits) {
          if (e.dir > 7 || !follows(e.kind) || !by.has(e.to)) continue;
          const [dx, dy] = STEP[e.dir]!;
          want(e.to, x + dx, y + dy);
        }
        for (const w of into.get(n) ?? []) {
          const [dx, dy] = STEP[w.dir]!;
          want(w.from, x - dx, y - dy);
        }
      }
      // every room that fits is placed: the first one whose cell was taken goes to the
      // nearest free cell, and the walk goes on from it
      let d = deferred.shift();
      while (d && taken.has(d[0])) d = deferred.shift();
      if (!d) break;
      const free = nearestFree(occupied, d[1], d[2]);
      place(d[0], free.x, free.y);
      displaced.add(d[0]);
    }
    let x0 = Infinity;
    let y0 = Infinity;
    let x1 = -Infinity;
    let y1 = -Infinity;
    for (const { x, y } of at.values()) {
      x0 = Math.min(x0, x);
      y0 = Math.min(y0, y);
      x1 = Math.max(x1, x);
      y1 = Math.max(y1, y);
    }
    return { seed, at, displaced, x0, y0, x1, y1 };
  }

  const grown: Grown[] = [];
  if (by.has(start)) grown.push(grow(start));
  for (const r of rooms) if (!taken.has(r.room)) grown.push(grow(r.room));

  // the start room's island where it is, the others in rows below it, biggest first
  const offsets: Spot[] = [];
  const main = grown[0];
  if (main) {
    offsets[0] = { x: 0, y: 0 };
    const order = grown.slice(1).sort((a, b) => b.at.size - a.at.size || a.seed - b.seed);
    const width = Math.max(main.x1 - main.x0 + 1, 24);
    let cx = main.x0;
    let cy = main.y1 + GAP + 1;
    let row = 0;
    for (const g of order) {
      const w = g.x1 - g.x0 + 1;
      const h = g.y1 - g.y0 + 1;
      if (cx > main.x0 && cx + w > main.x0 + width) {
        cx = main.x0;
        cy += row + GAP;
        row = 0;
      }
      offsets[grown.indexOf(g)] = { x: cx - g.x0, y: cy - g.y0 };
      cx += w + GAP;
      row = Math.max(row, h);
    }
  }

  const at = new Map<number, Spot>();
  const cell = new Map<number, number>();
  const island = new Map<number, number>();
  const displaced = new Set<number>();
  const bounds = { x0: 0, y0: 0, x1: 0, y1: 0 };
  grown.forEach((g, i) => {
    const o = offsets[i]!;
    for (const [n, { x, y }] of g.at) {
      const s = { x: x + o.x, y: y + o.y };
      at.set(n, s);
      cell.set(cellKey(s.x, s.y), n);
      island.set(n, i);
      bounds.x0 = Math.min(bounds.x0, s.x);
      bounds.y0 = Math.min(bounds.y0, s.y);
      bounds.x1 = Math.max(bounds.x1, s.x);
      bounds.y1 = Math.max(bounds.y1, s.y);
    }
    for (const n of g.displaced) displaced.add(n);
  });
  return { at, cell, displaced, island, islands: grown.length, bounds, labels: labels(rooms, by, island, at) };
}

/** The free cell nearest (x, y), ring by ring, in a fixed order. */
function nearestFree(occupied: Set<number>, x: number, y: number): Spot {
  for (let r = 1; ; r++) {
    let best: Spot | null = null;
    let bestD = Infinity;
    for (let i = -r; i <= r; i++) {
      for (const [cx, cy] of [
        [x + i, y - r],
        [x + i, y + r],
        [x - r, y + i],
        [x + r, y + i],
      ] as const) {
        if (occupied.has(cellKey(cx, cy))) continue;
        // the nearest of the ring as the crow flies, so a room stays beside where it belongs
        const d = (cx - x) ** 2 + (cy - y) ** 2;
        if (d < bestD) {
          best = { x: cx, y: cy };
          bestD = d;
        }
      }
    }
    if (best) return best;
  }
}

const norm = (name: string) => name.trim().replace(/\s+/g, ' ').toLowerCase();

/** One label for each run of joined rooms with the same name, at the room nearest its middle. */
function labels(
  rooms: readonly LRoom[],
  by: Map<number, LRoom>,
  island: Map<number, number>,
  at: Map<number, Spot>,
): Label[] {
  const parent = new Map<number, number>();
  const find = (n: number): number => {
    let p = parent.get(n) ?? n;
    while (p !== (parent.get(p) ?? p)) p = parent.get(p) ?? p;
    parent.set(n, p);
    return p;
  };
  for (const r of rooms) {
    const name = norm(r.name);
    if (!name) continue;
    for (const e of r.exits) {
      const t = by.get(e.to);
      if (!t || e.dir > 7 || island.get(t.room) !== island.get(r.room) || norm(t.name) !== name) continue;
      const [a, b] = [find(r.room), find(t.room)];
      if (a !== b) parent.set(Math.max(a, b), Math.min(a, b));
    }
  }
  const runs = new Map<number, number[]>();
  for (const r of rooms) {
    if (!norm(r.name)) continue;
    const root = find(r.room);
    let list = runs.get(root);
    if (!list) runs.set(root, (list = []));
    list.push(r.room);
  }
  const out: Label[] = [];
  for (const [root, members] of runs) {
    let sx = 0;
    let sy = 0;
    for (const n of members) {
      sx += at.get(n)!.x;
      sy += at.get(n)!.y;
    }
    const [mx, my] = [sx / members.length, sy / members.length];
    let best = members[0]!;
    let bestD = Infinity;
    for (const n of members) {
      const s = at.get(n)!;
      const d = (s.x - mx) ** 2 + (s.y - my) ** 2;
      if (d < bestD) [best, bestD] = [n, d];
    }
    const s = at.get(best)!;
    out.push({ x: s.x, y: s.y, text: by.get(root)!.name.trim(), size: members.length });
  }
  return out.sort((a, b) => b.size - a.size || a.y - b.y || a.x - b.x);
}
