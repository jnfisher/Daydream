// Picking rooms on the World map with the Select tool: the rooms a box or a lasso holds, the
// region the wand takes (the rooms joined to one by exits, through rooms of its colour), how
// a pick changes the rooms picked, and what those rooms share, for the bar that sets their
// fields together (`POST /api/map/set`).
//
// Pure: no DOM, so `node --test` runs its tests (picks.test.ts); its imports name their
// files whole, as node needs.

import type { MapRoom } from './model.ts';

export type PickTool = 'box' | 'lasso' | 'brush' | 'wand';

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
