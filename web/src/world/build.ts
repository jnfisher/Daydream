// Build mode's moves: what a direction key does from a room. An exit that way is walked; a
// room drawn in the next cell with no exit between is joined to it, an exit each way; free
// cells are dug, a new room in each (a line of them for a count, as far as the cells stay
// free), every one a copy of the room it is dug from without its exits, items, coins and
// permanent NPC, as NMR's map editor makes them; and an exit to a room the map does not have
// digs that room. Up and down have no cells, so they always dig.
//
// The same changes the server makes, made to the map's rooms at once, so the map shows them
// before the server has written them.
//
// The mouse reaches too: a free cell beside the room dragged out along its way is a corridor
// of new rooms that long, joining each room drawn in its way to the one before; dragged onto a
// room anywhere, the room is joined to it that way. Shift and a click on a cell in a straight
// line from the room is the same corridor.
//
// Pure: no DOM, so `node --test` runs its tests (build.test.ts); its imports name their
// files whole, as node needs.

import { cellKey, OPPOSITE, STEP, type Layout } from './layout.ts';
import type { MapExit, MapRoom } from './model.ts';

export type Move =
  /** An exit leads that way to a room of this map. */
  | { kind: 'walk'; to: number }
  /** An exit leads that way to another map. */
  | { kind: 'leave'; exit: MapExit }
  /** A room is drawn in the next cell with no exit to it: join the two. */
  | { kind: 'join'; to: number }
  /** New rooms that way, numbered `rooms`, each from the one before. */
  | { kind: 'dig'; rooms: number[] }
  /** Nothing to do, and why. */
  | { kind: 'stop'; why: string };

const WORDS = ['north', 'south', 'east', 'west', 'northeast', 'northwest', 'southeast', 'southwest', 'up', 'down'];

/**
 * What a direction key does from room `from`, `count` times over at most (a line of new rooms
 * stops before a cell that is taken). `by` and `layout` are the map's rooms and where each is.
 */
export function moveFrom(by: ReadonlyMap<number, MapRoom>, layout: Layout, from: number, dir: number, count = 1): Move {
  const r = by.get(from);
  if (!r) return { kind: 'stop', why: `Room ${from} is not on this map.` };
  const e = r.exits.find((x) => x.dir === dir);
  if (e) {
    if (e.kind === 8) return { kind: 'leave', exit: e };
    if (by.has(e.to)) return { kind: 'walk', to: e.to };
    // the room it leads to is not there: dig it, with the number the exit gives it (an exit of
    // another type keeps it, so the server would change what it is)
    if (e.kind !== 0) return { kind: 'stop', why: `The exit ${WORDS[dir]} leads to room ${e.to}, which this map does not have. Make the room in the Library, or change the exit.` };
    return { kind: 'dig', rooms: [e.to] };
  }
  if (dir > 7) return { kind: 'dig', rooms: freeNumbers(by, count) };
  const p = layout.at.get(from);
  const step = STEP[dir];
  if (!p || !step) return { kind: 'stop', why: `Room ${from} is not drawn.` };
  const [dx, dy] = step;
  const next = layout.cell.get(cellKey(p.x + dx, p.y + dy));
  if (next !== undefined) return { kind: 'join', to: next };
  let free = 1;
  while (free < count && !layout.cell.has(cellKey(p.x + dx * (free + 1), p.y + dy * (free + 1)))) free++;
  return { kind: 'dig', rooms: freeNumbers(by, free) };
}

/** The numbers `count` new rooms take: after the map's highest, as NMR's map editor numbers them. */
export function freeNumbers(by: ReadonlyMap<number, unknown>, count: number): number[] {
  let top = 0;
  for (const n of by.keys()) if (n > top) top = n;
  return Array.from({ length: count }, (_, i) => top + 1 + i);
}

/** An exit to a room of the same map, as a dig makes it. */
export const exitTo = (dir: number, to: number): MapExit => ({ dir, to, kind: 0, para1: 0, para2: 0, para3: 0, para4: 0 });

/** `exits` with `e` in its direction, in direction order. */
function withExit(exits: readonly MapExit[], e: MapExit): MapExit[] {
  return [...exits.filter((x) => x.dir !== e.dir), e].sort((a, b) => a.dir - b.dir);
}

/** The map's rooms once `made` are dug from `from` going `dir`, a line, each joined both ways to the one before. */
export function dug(rooms: readonly MapRoom[], from: number, dir: number, made: readonly number[]): MapRoom[] {
  const src = rooms.find((r) => r.room === from);
  if (!src || !made.length) return [...rooms];
  const back = OPPOSITE[dir]!;
  const out = rooms.map((r) => (r.room === from ? { ...r, exits: withExit(r.exits, exitTo(dir, made[0]!)) } : r));
  made.forEach((n, i) => {
    const exits = [exitTo(back, i ? made[i - 1]! : from)];
    if (i < made.length - 1) exits.push(exitTo(dir, made[i + 1]!));
    out.push({ ...src, room: n, npc: 0, exits: exits.sort((a, b) => a.dir - b.dir) });
  });
  return out;
}

/**
 * The map's rooms once `from` is joined to `to` going `dir`. The way back is made unless `to`
 * already has an exit that way (to another room): then the join is one-way, as the server
 * leaves it.
 */
export function joined(rooms: readonly MapRoom[], from: number, dir: number, to: number): { rooms: MapRoom[]; oneWay: boolean } {
  const back = OPPOSITE[dir]!;
  const t = rooms.find((r) => r.room === to);
  const taken = t?.exits.find((e) => e.dir === back);
  const oneWay = !!taken && taken.to !== from;
  return {
    rooms: rooms.map((r) => {
      if (r.room === from) return { ...r, exits: withExit(r.exits, exitTo(dir, to)) };
      if (r.room === to && !taken) return { ...r, exits: withExit(r.exits, exitTo(back, from)) };
      return r;
    }),
    oneWay,
  };
}

/** The map's rooms with room `n` named `name`. */
export function renamed(rooms: readonly MapRoom[], n: number, name: string): MapRoom[] {
  return rooms.map((r) => (r.room === n ? { ...r, name } : r));
}

// ---- reaching with the mouse ------------------------------------------------------------

/** The longest corridor one drag makes. */
export const REACH_MOST = 99;

/** A cell a corridor goes through: a new room (its number), or the room drawn there. */
export interface ReachCell {
  x: number;
  y: number;
  room: number;
  made: boolean;
}

/** A corridor's way from one of its rooms to the next. */
export interface ReachLink {
  from: number;
  to: number;
  /** Whether the exit from `from` is made: false when it is there already, and nothing changes. */
  out: boolean;
  /** The way back: made, there already, or taken by an exit to another room (the way is one-way). */
  back: 'make' | 'have' | 'taken';
}

export type Reach =
  /** A corridor from room `from` going `dir`: its cells after the room, and where it stopped short. */
  | { kind: 'line'; from: number; dir: number; cells: ReachCell[]; links: ReachLink[]; stop: { x: number; y: number; why: string } | null }
  /** Room `from` joined going `dir` to room `to`, wherever it is drawn: one-way when `to` has its way back to another room. */
  | { kind: 'join'; from: number; dir: number; to: number; oneWay: boolean };

/** The highest room number of the map. */
function highest(by: ReadonlyMap<number, unknown>): number {
  let top = 0;
  for (const n of by.keys()) if (n > top) top = n;
  return top;
}

/**
 * A corridor dragged from room `from` going `dir` (0 to 7), `count` cells long: each free cell a
 * new room (numbered after the map's highest, in order), each room drawn in its way joined to the
 * one before (one-way when its way back leads to another room) and gone through. It stops short
 * where a room has an exit that way to somewhere else.
 */
export function reachLine(by: ReadonlyMap<number, MapRoom>, layout: Layout, from: number, dir: number, count: number): Reach | null {
  const p = layout.at.get(from);
  const step = STEP[dir];
  if (!p || !step || !by.has(from)) return null;
  const [dx, dy] = step;
  const cells: ReachCell[] = [];
  const links: ReachLink[] = [];
  let stop: { x: number; y: number; why: string } | null = null;
  let top = highest(by);
  let prev = from;
  for (let i = 1; i <= Math.min(count, REACH_MOST); i++) {
    const [x, y] = [p.x + dx * i, p.y + dy * i];
    const there = layout.cell.get(cellKey(x, y));
    const out = by.get(prev)?.exits.find((e) => e.dir === dir);
    if (out) {
      if (there !== undefined && out.to === there && out.kind !== 8) {
        // the way is there already: go through it
        cells.push({ x, y, room: there, made: false });
        links.push({ from: prev, to: there, out: false, back: 'have' });
        prev = there;
        continue;
      }
      const to = out.kind === 8 ? `map ${out.para1}` : by.has(out.to) ? `room ${out.to}` : `room ${out.to}, which the map does not have`;
      stop = { x: x - dx / 2, y: y - dy / 2, why: `Room ${prev} has an exit ${WORDS[dir]} already, to ${to}.` };
      break;
    }
    if (there === undefined) {
      const n = ++top;
      cells.push({ x, y, room: n, made: true });
      links.push({ from: prev, to: n, out: true, back: 'make' });
      prev = n;
      continue;
    }
    const back = by.get(there)?.exits.find((e) => e.dir === OPPOSITE[dir]);
    cells.push({ x, y, room: there, made: false });
    links.push({ from: prev, to: there, out: true, back: !back ? 'make' : back.to === prev && back.kind !== 8 ? 'have' : 'taken' });
    prev = there;
  }
  return { kind: 'line', from, dir, cells, links, stop };
}

/** Room `from` joined going `dir` to room `to`, wherever it is drawn; null when it has an exit that way already. */
export function reachJoin(by: ReadonlyMap<number, MapRoom>, from: number, dir: number, to: number): Reach | null {
  const a = by.get(from);
  const b = by.get(to);
  if (!a || !b || from === to || a.exits.some((e) => e.dir === dir)) return null;
  const back = b.exits.find((e) => e.dir === OPPOSITE[dir]);
  return { kind: 'join', from, dir, to, oneWay: !!back && (back.to !== from || back.kind === 8) };
}

/** The way from a room's cell to a cell in a straight line from it (across, down or corner to corner), and how far: null when it is in none. */
export function lineTo(dx: number, dy: number): { dir: number; count: number } | null {
  if ((!dx && !dy) || (dx && dy && Math.abs(dx) !== Math.abs(dy))) return null;
  const dir = STEP.findIndex(([x, y]) => x === Math.sign(dx) && y === Math.sign(dy));
  return dir < 0 ? null : { dir, count: Math.max(Math.abs(dx), Math.abs(dy)) };
}

/** How far along its way a drag from a room's cell has gone, in cells (the cell under the pointer `dx`, `dy` from it, onto the way's line). */
export function alongWay(dir: number, dx: number, dy: number): number {
  const [sx, sy] = STEP[dir] ?? [0, 0];
  return Math.round((dx * sx + dy * sy) / (sx * sx + sy * sy || 1));
}

/** What a reach does, in numbers: the new rooms, the exits made, and the rooms joined. */
export function reachCount(r: Reach): { made: number[]; exits: number; joined: number[] } {
  if (r.kind === 'join') return { made: [], exits: r.oneWay ? 1 : 2, joined: [r.to] };
  const made = r.cells.filter((c) => c.made).map((c) => c.room);
  let exits = 0;
  const joined = new Set<number>();
  for (const l of r.links) {
    if (l.out) exits++;
    if (l.back === 'make') exits++;
    if (l.out) for (const n of [l.from, l.to]) if (n !== r.from && !made.includes(n)) joined.add(n);
  }
  return { made, exits, joined: [...joined] };
}

/** The map's rooms once a corridor is made: new rooms copy `brush`, without its exits, items, coins and permanent NPC. */
export function reached(rooms: readonly MapRoom[], r: Reach, brush: MapRoom): MapRoom[] {
  if (r.kind === 'join') return joined(rooms, r.from, r.dir, r.to).rooms;
  const add = new Map<number, MapExit[]>();
  const put = (n: number, e: MapExit) => add.set(n, [...(add.get(n) ?? []), e]);
  for (const l of r.links) {
    if (l.out) put(l.from, exitTo(r.dir, l.to));
    if (l.back === 'make') put(l.to, exitTo(OPPOSITE[r.dir]!, l.from));
  }
  const out = rooms.map((x) => {
    const e = add.get(x.room);
    return e ? { ...x, exits: e.reduce(withExit, x.exits) } : x;
  });
  for (const c of r.cells) if (c.made) out.push({ ...brush, room: c.room, npc: 0, exits: (add.get(c.room) ?? []).reduce(withExit, [] as MapExit[]) });
  return out;
}

/** The exits a corridor makes, as the stamp endpoint takes them (`POST /api/map/stamp`). */
export function reachLinks(r: Reach): { from: number; dir: number; to: number; one_way: boolean }[] {
  if (r.kind === 'join') return [{ from: r.from, dir: r.dir, to: r.to, one_way: r.oneWay }];
  return r.links.filter((l) => l.out).map((l) => ({ from: l.from, dir: r.dir, to: l.to, one_way: l.back !== 'make' }));
}
