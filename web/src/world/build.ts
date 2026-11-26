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
