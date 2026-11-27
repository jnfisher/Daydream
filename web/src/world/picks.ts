// Picking rooms on the World map with the Select tool: the rooms a box or a lasso holds, the
// region the wand takes (the rooms joined to one by exits, through rooms of its colour), how
// a pick changes the rooms picked, and what those rooms share, for the bar that sets their
// fields together (`POST /api/map/set`); Paint, which puts a field straight onto the rooms a
// stroke goes over; and the shape of the rooms picked, saved as a stamp of one's own.
//
// Pure: no DOM, so `node --test` runs its tests (picks.test.ts); its imports name their
// files whole, as node needs.

import { OPPOSITE } from './layout.ts';
import type { ColourBy } from './look.ts';
import type { MapRoom } from './model.ts';
import type { Shape, ShapeLink } from './stamps.ts';

/** The ways the Select tool works: four that pick rooms, and Paint, which sets a field on them. */
export type PickTool = 'box' | 'lasso' | 'brush' | 'wand' | 'paint';

/** How a pick changes the rooms picked: replaces them, adds to them, takes from them, or (a click) flips one. */
export type PickHow = 'set' | 'add' | 'drop' | 'flip';

/** What a pick does with no key held: switched in the bar, for a finger (Shift and Alt change it for a moment). */
export type PickMode = 'set' | 'add' | 'drop';

export const PICK_MODES: { id: PickMode; name: string; about: string }[] = [
  { id: 'set', name: 'New', about: 'A pick replaces the rooms picked' },
  { id: 'add', name: 'Add', about: 'A pick adds to the rooms picked (Shift)' },
  { id: 'drop', name: 'Take away', about: 'A pick takes rooms away (Alt)' },
];

/** The brush's sizes, in rooms across. */
export const BRUSH_SIZES = [1, 3, 5, 7, 9, 13];

export const PICK_TOOLS: { id: PickTool; name: string; about: string }[] = [
  { id: 'box', name: 'Box', about: 'Drag a box around rooms' },
  { id: 'lasso', name: 'Lasso', about: 'Draw around rooms' },
  { id: 'brush', name: 'Brush', about: 'Paint over rooms' },
  { id: 'wand', name: 'Wand', about: 'Click a room for the rooms joined to it in its colour' },
  { id: 'paint', name: 'Paint', about: 'Paint light, monsters or a control room onto rooms' },
];

/** How a pick's modifier keys change it: Shift adds, Alt takes away. */
export const howOf = (m: { shiftKey: boolean; altKey: boolean }, plain: PickHow = 'set'): PickHow => (m.altKey ? 'drop' : m.shiftKey ? 'add' : plain);

/**
 * How a click on a room changes the rooms picked: with no key held, it is picked alone (or,
 * adding, flipped: a second tap lets it go), Shift or Ctrl flips it, and Alt takes it away.
 */
export function clickHow(m: { shiftKey: boolean; altKey: boolean; ctrlKey: boolean; metaKey: boolean }, mode: PickMode): PickHow {
  if (m.altKey) return 'drop';
  if (m.shiftKey || m.ctrlKey || m.metaKey) return 'flip';
  return mode === 'add' ? 'flip' : mode;
}

/** Whether (x, y) is inside the polygon (the even-odd rule). */
export function inside(x: number, y: number, poly: readonly (readonly [number, number])[]): boolean {
  let yes = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const [xi, yi] = poly[i]!;
    const [xj, yj] = poly[j]!;
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) yes = !yes;
  }
  return yes;
}

/**
 * The wand's region: the rooms joined to `start` by exits (either way, map changes aside),
 * through rooms whose key (the colour the map gives them) is its own. In walking order.
 */
export function regionOf(by: ReadonlyMap<number, MapRoom>, start: number, key: (n: number) => string | undefined): number[] {
  if (!by.has(start)) return [];
  const mine = key(start);
  // exits lead one way: the rooms that lead in count as neighbours too
  const into = new Map<number, number[]>();
  for (const r of by.values()) {
    for (const e of r.exits) {
      if (e.kind === 8 || e.to === r.room) continue;
      const list = into.get(e.to);
      if (list) list.push(r.room);
      else into.set(e.to, [r.room]);
    }
  }
  const seen = new Set([start]);
  const queue = [start];
  for (let i = 0; i < queue.length; i++) {
    const n = queue[i]!;
    const next = [...(by.get(n)?.exits ?? []).filter((e) => e.kind !== 8).map((e) => e.to), ...(into.get(n) ?? [])];
    for (const m of next) {
      if (seen.has(m) || !by.has(m) || key(m) !== mine) continue;
      seen.add(m);
      queue.push(m);
    }
  }
  return queue;
}

/** The rooms picked once `rooms` are picked `how`. */
export function after(picked: ReadonlySet<number>, rooms: readonly number[], how: PickHow): Set<number> {
  if (how === 'set') return new Set(rooms);
  const out = new Set(picked);
  for (const n of rooms) {
    if (how === 'add' || (how === 'flip' && !out.has(n))) out.add(n);
    else out.delete(n);
  }
  return out;
}

/** What the rooms hold in a field: its one value, `mixed` when they differ, or null for no rooms. */
export function shared<T>(values: Iterable<T>): { one: T } | 'mixed' | null {
  let first: { one: T } | null = null;
  for (const v of values) {
    if (!first) first = { one: v };
    else if (first.one !== v) return 'mixed';
  }
  return first;
}

/** A field the bar sets on the rooms picked, and the Rooms columns it writes. */
export interface PickField {
  id: 'name' | 'light' | 'monsters' | 'control' | 'type';
  label: string;
  columns: string[];
}

export const PICK_FIELDS: PickField[] = [
  { id: 'name', label: 'Name', columns: ['Name'] },
  { id: 'light', label: 'Light', columns: ['Light'] },
  { id: 'monsters', label: 'Monsters', columns: ['Mon Type', 'Min Index', 'Max Index', 'Max Regen'] },
  { id: 'control', label: 'Control room', columns: ['Control Room'] },
  { id: 'type', label: 'Room type', columns: ['Type'] },
];

/** The words for a set on `n` rooms: "Light", "on 14 rooms". */
export const onRooms = (map: number, rooms: readonly number[]) => (rooms.length === 1 ? `on room ${map}/${rooms[0]}` : `on ${rooms.length.toLocaleString()} rooms`);

/** The map room's own name for each Rooms column the bar sets. */
const KEYS = {
  'Mon Type': 'mon_type',
  'Min Index': 'min_index',
  'Max Index': 'max_index',
  'Max Regen': 'max_regen',
  'Control Room': 'control',
  Type: 'room_type',
  Light: 'light',
} as const satisfies Record<string, keyof MapRoom>;

/** A Rooms column's value as a map room holds it (the columns the map reads). */
export function mapValue(r: MapRoom, column: string): string | number | undefined {
  if (column === 'Name') return r.name.trim();
  const k = KEYS[column as keyof typeof KEYS];
  return k ? r[k] : undefined;
}

/** Whether a room holds these values already. */
export const holds = (r: MapRoom, values: readonly (readonly [string, string | number])[]) => values.every(([c, v]) => mapValue(r, c) === (typeof v === 'string' ? v.trim() : v));

/** The rooms of `rooms` that a set of these values changes (those that hold them already are left out), in room order. */
export function changing(by: ReadonlyMap<number, MapRoom>, rooms: Iterable<number>, values: readonly (readonly [string, string | number])[]): number[] {
  const out: number[] = [];
  for (const n of rooms) {
    const r = by.get(n);
    if (r && !holds(r, values)) out.push(n);
  }
  return out.sort((a, b) => a - b);
}

/** A map room with the columns the map reads set (the rest wait for the map to be read again). */
export function withValues(r: MapRoom, values: readonly (readonly [string, string | number])[]): MapRoom {
  const out = { ...r };
  for (const [c, v] of values) {
    if (c === 'Name') out.name = String(v);
    const k = KEYS[c as keyof typeof KEYS];
    if (k) out[k] = Number(v);
  }
  return out;
}

/** What a field's columns hold across the rooms: one value for each column, `mixed` when the rooms differ, or null for no rooms. */
export function sharedOf(rooms: readonly MapRoom[], f: PickField): { one: (string | number)[] } | 'mixed' | null {
  const s = shared(rooms.map((r) => JSON.stringify(f.columns.map((c) => mapValue(r, c) ?? null))));
  return s && s !== 'mixed' ? { one: JSON.parse(s.one) as (string | number)[] } : s;
}

/** The fields Paint puts on rooms. */
export type PaintField = 'light' | 'monsters' | 'control';

export const PAINT_FIELDS: PaintField[] = ['light', 'monsters', 'control'];

/** How the map is coloured while each field is painted, so the paint shows as it goes on. */
export const PAINT_COLOUR: Record<PaintField, ColourBy> = { light: 'light', monsters: 'monsters', control: 'area' };

/** A room's values in a field's columns, as numbers (for the fields Paint puts on rooms). */
export const valuesOf = (r: MapRoom, f: PickField): number[] => f.columns.map((c) => Number(mapValue(r, c) ?? 0));

// ---- the rooms picked as a stamp ------------------------------------------------------------

/** The most rooms a stamp makes (the server's `MAX_STAMP_ROOMS`), and the longest name a saved one has. */
export const STAMP_ROOMS_MAX = 2000;
export const STAMP_NAME_MAX = 40;

/** Whether the map walks an exit of this type, as its layout does: not a map change, nor a remote action. */
const walked = (kind: number) => kind !== 8 && kind !== 12;

/** The rooms picked that the map has and draws, in room order. */
const drawn = (by: ReadonlyMap<number, MapRoom>, at: ReadonlyMap<number, unknown>, rooms: Iterable<number>) =>
  [...new Set(rooms)].filter((n) => by.has(n) && at.has(n)).sort((a, b) => a - b);

/** The way in of a stamp of the rooms picked: the first with an exit out of them, else the first. */
export function wayIn(by: ReadonlyMap<number, MapRoom>, at: ReadonlyMap<number, unknown>, rooms: Iterable<number>): number | undefined {
  const mine = drawn(by, at, rooms);
  const picked = new Set(mine);
  return mine.find((n) => by.get(n)!.exits.some((e) => walked(e.kind) && !picked.has(e.to))) ?? mine[0];
}

/** How far apart two cells of a stamp may be drawn and stay where they are to each other. */
const NEAR = 8;

/**
 * A stamp's cells with its far parts brought in: cells within `NEAR` of each other stay as they
 * are drawn, and each part drawn further from the first cell's (a floor up or down, which the map
 * draws apart) is put beside the parts before it, two cells clear, level with the first's top.
 */
export function drawnNear(cells: readonly (readonly [number, number])[]): [number, number][] {
  const out = cells.map(([x, y]): [number, number] => [x, y]);
  // the parts: cells within NEAR of one another, through buckets of NEAR + 1 cells
  const part = cells.map((_, i) => i);
  const root = (i: number): number => (part[i] === i ? i : (part[i] = root(part[i]!)));
  const size = NEAR + 1;
  const buckets = new Map<string, number[]>();
  cells.forEach(([x, y], i) => {
    const [bx, by] = [Math.floor(x / size), Math.floor(y / size)];
    for (let dx = -1; dx <= 1; dx++) {
      for (let dy = -1; dy <= 1; dy++) {
        for (const j of buckets.get(`${bx + dx},${by + dy}`) ?? []) {
          if (Math.max(Math.abs(cells[j]![0] - x), Math.abs(cells[j]![1] - y)) <= NEAR) part[root(i)] = root(j);
        }
      }
    }
    const k = `${bx},${by}`;
    const list = buckets.get(k);
    if (list) list.push(i);
    else buckets.set(k, [i]);
  });
  // each part's cells, in the order of its first
  const parts = new Map<number, number[]>();
  cells.forEach((_, i) => {
    const r = root(i);
    const list = parts.get(r);
    if (list) list.push(i);
    else parts.set(r, [i]);
  });
  const [main, ...rest] = [...parts.values()];
  if (!main || !rest.length) return out;
  const box = (ids: number[]) => {
    const b = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
    for (const i of ids) {
      const [x, y] = out[i]!;
      b.x0 = Math.min(b.x0, x);
      b.y0 = Math.min(b.y0, y);
      b.x1 = Math.max(b.x1, x);
      b.y1 = Math.max(b.y1, y);
    }
    return b;
  };
  const home = box(main);
  let right = home.x1;
  for (const ids of rest) {
    const b = box(ids);
    const [dx, dy] = [right + 3 - b.x0, home.y0 - b.y0];
    for (const i of ids) out[i] = [out[i]![0] + dx, out[i]![1] + dy];
    right += 3 + b.x1 - b.x0;
  }
  return out;
}

/**
 * The rooms picked as a stamp's shape: each room's cell as the map draws it (`at`), counted from
 * its way in, and the exits between them, one link for both ways where the way back is there too.
 * Its way in is the first room with an exit out of the rooms picked, else the first room; the
 * others follow it in walking order. Parts drawn far apart are brought in (`drawnNear`). Exits out
 * of them, map changes and remote actions are left out, and a stamp makes every exit plain.
 */
export function shapeOf(by: ReadonlyMap<number, MapRoom>, at: ReadonlyMap<number, { x: number; y: number }>, rooms: Iterable<number>): Shape {
  const mine = drawn(by, at, rooms);
  const picked = new Set(mine);
  const inside = (n: number) => by.get(n)!.exits.filter((e) => walked(e.kind) && e.to !== n && picked.has(e.to));
  const first = wayIn(by, at, mine);
  if (first === undefined) return { cells: [], links: [] };
  // walking order from the way in, over exits either way; rooms it does not reach follow in turn
  const next = new Map<number, number[]>();
  const join = (p: number, q: number) => {
    const list = next.get(p);
    if (list) list.push(q);
    else next.set(p, [q]);
  };
  for (const n of mine) {
    for (const e of inside(n)) {
      join(n, e.to);
      join(e.to, n);
    }
  }
  const order: number[] = [];
  const seen = new Set<number>();
  for (const start of [first, ...mine]) {
    if (seen.has(start)) continue;
    seen.add(start);
    const queue = [start];
    for (let i = 0; i < queue.length; i++) {
      const n = queue[i]!;
      order.push(n);
      for (const m of next.get(n) ?? []) {
        if (!seen.has(m)) {
          seen.add(m);
          queue.push(m);
        }
      }
    }
  }
  const index = new Map(order.map((n, i) => [n, i]));
  const o = at.get(first)!;
  const cells = drawnNear(order.map((n): [number, number] => [at.get(n)!.x - o.x, at.get(n)!.y - o.y]));
  const links: ShapeLink[] = [];
  for (const n of order) {
    const a = index.get(n)!;
    for (const e of inside(n)) {
      const b = index.get(e.to)!;
      const back = by.get(e.to)!.exits.some((x) => x.dir === OPPOSITE[e.dir] && x.to === n && walked(x.kind));
      // both ways: one link, from the room that comes first
      if (!back) links.push({ a, dir: e.dir, b, oneWay: true });
      else if (a < b) links.push({ a, dir: e.dir, b });
    }
  }
  return { cells, links };
}

/** The name most of the rooms have (trimmed; on a tie, the first to get there), or '' when none has one. */
function commonName(rooms: Iterable<MapRoom>): string {
  const counts = new Map<string, number>();
  let best = '';
  for (const r of rooms) {
    const name = r.name.trim();
    if (!name) continue;
    const c = (counts.get(name) ?? 0) + 1;
    counts.set(name, c);
    if (c > (counts.get(best) ?? 0)) best = name;
  }
  return best;
}

/** A name for the rooms picked as a stamp: the one most of them have, else the map's. */
export function stampName(rooms: readonly MapRoom[], map: number): string {
  return [...(commonName(rooms) || `Rooms of map ${map}`)].slice(0, STAMP_NAME_MAX).join('').trim();
}

/**
 * The room a stamp of the rooms picked has its new rooms copy: its way in when its name is as
 * common among them as any, else the first whose name is, so a block of fields comes out as
 * fields and not as the gate at its edge.
 */
export function lookOf(by: ReadonlyMap<number, MapRoom>, at: ReadonlyMap<number, unknown>, rooms: Iterable<number>): number | undefined {
  const mine = drawn(by, at, rooms);
  const way = wayIn(by, at, mine);
  if (way === undefined) return undefined;
  const counts = new Map<string, number>();
  let most = 0;
  for (const n of mine) {
    const name = by.get(n)!.name.trim();
    const c = (counts.get(name) ?? 0) + 1;
    counts.set(name, c);
    if (name && c > most) most = c;
  }
  const common = (n: number) => {
    const name = by.get(n)!.name.trim();
    return !!name && counts.get(name) === most;
  };
  return most === 0 || common(way) ? way : (mine.find(common) ?? way);
}
