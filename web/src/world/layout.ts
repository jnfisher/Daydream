// Where each room of a map is drawn. A walk from the start room places each room one step
// from the room it is reached from, by its exit's direction (N is one row up, NE one row up and
// one column right), and by the exits that lead into it too. Unlike NMR's 41 by 41 grid the
// map has no edge, and no room vanishes: a room whose cell is taken goes to the nearest free
// cell once the walk has placed every room that fits, and the walk goes on from it. Rooms the
// walk never reaches (joined only by up and down, by map changes or not at all) are walked in
// turn as islands of their own, laid out below the start room's. The same rooms always give
// the same layout, so the map holds still between visits and edits.
//
// Rooms moved by hand are pinned: each is drawn in its own cell (the lowest room number keeps a
// cell two pins name), and the walk starts from them instead of the start room, so the rooms
// they lead to are placed around them as before, and a room dug from one lands beside it.
// Rooms no pinned room reaches are islands below them.
//
// A map can be drawn in levels instead (layOutLevels): the walk goes up and down too, each a
// level up or down into the same cell, so a tower's floors stack and the rooms on each are laid
// out around their stairs. Each level has a layout of its own, and pins name a level too.
//
// Pure: no DOM, so `node --test` runs its tests (layout.test.ts).

import type { PinCells } from './model.ts';

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
  /** The level, when the map is drawn in levels: 0 is the start room's, and Up leads one higher. */
  z?: number;
}

/** A place in levels: the cell and the level. */
export type Spot3 = Required<Spot>;

/** Places of rooms, as the map's side file keeps them: `{"14": [3, -2]}`, or `{"14": [3, -2, 1]}` in levels. */
export const cellsOf = (p: ReadonlyMap<number, Spot>): PinCells =>
  Object.fromEntries([...p].map(([n, s]) => [String(n), s.z === undefined ? [s.x, s.y] : [s.x, s.y, s.z]]));

/** Places as the map's side file keeps them, by room number. */
export const spotsOf = (c: PinCells | undefined): Map<number, Spot> =>
  new Map(Object.entries(c ?? {}).map(([k, [x, y, z]]) => [Number(k), z === undefined ? { x, y } : { x, y, z }]));

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
  /** The rooms drawn where they were pinned. */
  pinned: Set<number>;
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

/** A map drawn in levels: Up and Down lead a level up and down, to a room drawn in the same cell
 * as the room they leave, when it is free. */
export interface Levels {
  /** Where each room is, and on what level. */
  at: Map<number, Spot3>;
  /** Each level's own layout, lowest first. */
  of: Map<number, Layout>;
}

/** A cell's key in `Layout.cell`. */
export const cellKey = (x: number, y: number) => (x + 1_000_000) * 2_097_152 + (y + 1_000_000);

/** Whether the layout walks through an exit of this type: not a map change, nor a remote action. */
const follows = (kind: number) => kind !== 8 && kind !== 12;

/** How many levels Up and Down climb. */
const RISE: readonly number[] = [0, 0, 0, 0, 0, 0, 0, 0, 1, -1];

/** Blank cells between islands. */
const GAP = 2;

interface Grown {
  seed: number;
  at: Map<number, Spot3>;
  displaced: Set<number>;
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** Where a walk put every room, before it is cut into levels. */
interface Walked {
  rooms: LRoom[];
  by: Map<number, LRoom>;
  at: Map<number, Spot3>;
  pinned: Set<number>;
  displaced: Set<number>;
  island: Map<number, number>;
}

export function layOut(given: readonly LRoom[], start: number, pins?: ReadonlyMap<number, Spot>): Layout {
  const w = walk(given, start, pins, false);
  return layoutOf(w, w.rooms, true);
}

/** The map drawn in levels: the walk goes up and down as well, and pins name their level. */
export function layOutLevels(given: readonly LRoom[], start: number, pins?: ReadonlyMap<number, Spot>): Levels {
  const w = walk(given, start, pins, true);
  const on = new Map<number, LRoom[]>();
  for (const r of w.rooms) {
    const z = w.at.get(r.room)!.z;
    let list = on.get(z);
    if (!list) on.set(z, (list = []));
    list.push(r);
  }
  const of = new Map([...on].sort((a, b) => a[0] - b[0]).map(([z, list]) => [z, layoutOf(w, list, false)]));
  return { at: w.at, of };
}

/** The layout of a walk's rooms: all of them, or one level's. */
function layoutOf(w: Walked, rooms: readonly LRoom[], origin: boolean): Layout {
  const at = new Map<number, Spot>();
  const cell = new Map<number, number>();
  const island = new Map<number, number>();
  const pinned = new Set<number>();
  const displaced = new Set<number>();
  const islands = new Set<number>();
  // a whole map's bounds hold the start room's cell, where its walk began
  const first = origin ? { x: 0, y: 0 } : (w.at.get(rooms[0]?.room ?? 0) ?? { x: 0, y: 0 });
  const bounds = { x0: first.x, y0: first.y, x1: first.x, y1: first.y };
  for (const r of rooms) {
    const n = r.room;
    const s = w.at.get(n)!;
    at.set(n, { x: s.x, y: s.y });
    cell.set(cellKey(s.x, s.y), n);
    const i = w.island.get(n)!;
    island.set(n, i);
    islands.add(i);
    if (w.pinned.has(n)) pinned.add(n);
    if (w.displaced.has(n)) displaced.add(n);
    bounds.x0 = Math.min(bounds.x0, s.x);
    bounds.y0 = Math.min(bounds.y0, s.y);
    bounds.x1 = Math.max(bounds.x1, s.x);
    bounds.y1 = Math.max(bounds.y1, s.y);
  }
  return { at, pinned, cell, displaced, island, islands: islands.size, bounds, labels: labels(rooms, w.by, island, at) };
}

/** Places every room: a walk from the pins or the start room, then islands below. With `levels`, up and down too. */
function walk(given: readonly LRoom[], start: number, pins: ReadonlyMap<number, Spot> | undefined, levels: boolean): Walked {
  // in room order, whatever order they came in, so the same rooms give the same layout
  const rooms = [...given].sort((a, b) => a.room - b.room);
  const by = new Map(rooms.map((r) => [r.room, r]));
  /** The exits a walk goes through: along the grid, and in levels up and down too. */
  const walked = (dir: number, kind: number) => (dir <= 7 || (levels && dir <= 9)) && follows(kind);
  // the exits into each room, so a room is placed by its way in as well as its way out
  const into = new Map<number, { from: number; dir: number }[]>();
  for (const r of rooms) {
    for (const e of r.exits) {
      if (!walked(e.dir, e.kind) || e.to === r.room || !by.has(e.to)) continue;
      let list = into.get(e.to);
      if (!list) into.set(e.to, (list = []));
      list.push({ from: r.room, dir: e.dir });
    }
  }
  const taken = new Set<number>();
  const pinned = new Set<number>();

  /** Walks from rooms placed first: the start room, or the pinned rooms in their cells. */
  function grow(seeds: readonly (readonly [number, number, number, number])[], pin: boolean): Grown {
    const at = new Map<number, Spot3>();
    /** The cells taken, level by level. */
    const occupied = new Map<number, Set<number>>();
    const cells = (z: number) => {
      let c = occupied.get(z);
      if (!c) occupied.set(z, (c = new Set()));
      return c;
    };
    const displaced = new Set<number>();
    const queue: number[] = [];
    const deferred: [number, number, number, number][] = [];
    const place = (n: number, x: number, y: number, z: number) => {
      at.set(n, { x, y, z });
      cells(z).add(cellKey(x, y));
      taken.add(n);
      queue.push(n);
    };
    const want = (n: number, x: number, y: number, z: number) => {
      if (taken.has(n)) return;
      if (cells(z).has(cellKey(x, y))) deferred.push([n, x, y, z]);
      else place(n, x, y, z);
    };
    for (const [n, x, y, z] of seeds) {
      // two pins in one cell: the lower room number keeps it, and the walk places the other
      if (cells(z).has(cellKey(x, y))) continue;
      place(n, x, y, z);
      if (pin) pinned.add(n);
    }
    let next = 0;
    for (;;) {
      while (next < queue.length) {
        const n = queue[next++]!;
        const { x, y, z } = at.get(n)!;
        for (const e of by.get(n)!.exits) {
          if (!walked(e.dir, e.kind) || !by.has(e.to)) continue;
          const [dx, dy] = STEP[e.dir] ?? [0, 0];
          want(e.to, x + dx, y + dy, z + RISE[e.dir]!);
        }
        for (const w of into.get(n) ?? []) {
          const [dx, dy] = STEP[w.dir] ?? [0, 0];
          want(w.from, x - dx, y - dy, z - RISE[w.dir]!);
        }
      }
      // every room that fits is placed: the first one whose cell was taken goes to the
      // nearest free cell of its level, and the walk goes on from it
      let d = deferred.shift();
      while (d && taken.has(d[0])) d = deferred.shift();
      if (!d) break;
      const free = nearestFree(cells(d[3]), d[1], d[2]);
      place(d[0], free.x, free.y, d[3]);
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
    return { seed: seeds[0]![0], at, displaced, x0, y0, x1, y1 };
  }

  const grown: Grown[] = [];
  const pinsGiven: [number, number, number, number][] = [];
  if (pins?.size) {
    for (const r of rooms) {
      const p = pins.get(r.room);
      const z = levels ? (p?.z ?? 0) : 0;
      if (p && Number.isInteger(p.x) && Number.isInteger(p.y) && Number.isInteger(z)) pinsGiven.push([r.room, p.x, p.y, z]);
    }
  }
  if (pinsGiven.length) grown.push(grow(pinsGiven, true));
  else if (by.has(start)) grown.push(grow([[start, 0, 0, 0]], false));
  for (const r of rooms) if (!taken.has(r.room)) grown.push(grow([[r.room, 0, 0, 0]], false));

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

  const at = new Map<number, Spot3>();
  const island = new Map<number, number>();
  const displaced = new Set<number>();
  grown.forEach((g, i) => {
    const o = offsets[i]!;
    for (const [n, { x, y, z }] of g.at) {
      at.set(n, { x: x + o.x, y: y + o.y, z });
      island.set(n, i);
    }
    for (const n of g.displaced) displaced.add(n);
  });
  return { rooms, by, at, pinned, displaced, island };
}

/** The free cell nearest (x, y), ring by ring, in a fixed order. */
function nearestFree(occupied: ReadonlySet<number>, x: number, y: number): Spot {
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
  by: ReadonlyMap<number, LRoom>,
  island: ReadonlyMap<number, number>,
  at: ReadonlyMap<number, Spot>,
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
