// The links between rooms that no exit draws, for the map's layer of scripted links: where a
// room's scripts and spells send players and the exits its remote actions work (the server
// reads them: crates/daydream-nmr/src/scripted.rs), and the rooms its own fields name: where
// those who die in it wake (Death Room), where those who hang up in it come back (Exit Room),
// and its control room. Shown for the selected rooms, both ways; and for every room, their
// scripts too: only those that lead to a few rooms, since a spell can scatter players over a
// hundred rooms (and the fields, set on whole areas, stay the selected rooms').
//
// Pure: no DOM, so `node --test` runs its tests (wires.test.ts).

import type { MapRoom, ScriptSource, ScriptTarget, Scripts, ScriptUse, When } from './model.ts';

export type WireKind = 'teleport' | 'spell' | 'remote' | 'death' | 'hangup' | 'control';

/** A room at one end of a wire. */
export interface End {
  map: number;
  room: number;
}

export interface Wire {
  kind: WireKind;
  from: End;
  to: End;
  /** The exit of `to` a remote action works. */
  exit?: number;
  /** A teleport to a room rolled from a range: how many rooms the range has. */
  among?: number;
  /** One of more links of its kind from its room than are drawn: how many there are. */
  fan?: number;
  /** A script's or spell's: how its room sets it off, what it is and where it leads. */
  when?: When;
  source?: ScriptSource;
  target?: ScriptTarget;
}

export type WiresFor = 'selected' | 'all' | 'off';

/** The most wires drawn at once: the selected rooms', and with every room's. */
export const MOST = 400;
export const MOST_ALL = 2500;
/** For every room, a source that leads to more rooms than this is left out: a scatter over an area. */
export const FAN = 8;
/** A teleport rolled from a range: at most this many of its rooms are drawn. */
const RANGE = 60;
/** A selected room's links of one kind: at most this many are drawn (a room can scatter players over hundreds of rooms). */
export const SPRAY = 60;

export interface Wired {
  wires: Wire[];
  /** How many more there were than are drawn. */
  more: number;
}

/**
 * The wires drawn on map `map`: those of the rooms `rooms`, each way (from them, then to them),
 * and with `all` every room's scripts. `here` is the map's rooms by number: a range lands only
 * on rooms the map has.
 */
export function wiresOf(map: number, here: ReadonlyMap<number, MapRoom>, scripts: Scripts | undefined, rooms: ReadonlySet<number>, all = false, most = all ? MOST_ALL : MOST): Wired {
  const out: Wire[] = [];
  let more = 0;
  const seen = new Set<string>();
  const key = (w: Wire) => `${w.kind} ${w.from.map}/${w.from.room} ${w.to.map}/${w.to.room} ${w.exit ?? ''}`;
  const add = (w: Wire) => {
    const k = key(w);
    if (seen.has(k)) return;
    seen.add(k);
    if (out.length < most) out.push(w);
    else more++;
  };
  const picked = (e: End) => e.map === map && rooms.has(e.room);
  // a target's rooms: one, or those of a range (the map's own, when it lands on this map)
  const ends = (u: ScriptUse, t: ScriptTarget): { to: End; among?: number }[] => {
    const m = t.to_map || u.map;
    if (t.to_last === undefined || t.to_last <= t.to_room) return [{ to: { map: m, room: t.to_room } }];
    const among = t.to_last - t.to_room + 1;
    if (m !== map) return [{ to: { map: m, room: t.to_room }, among }];
    const list: { to: End; among?: number }[] = [];
    for (let r = t.to_room; r <= t.to_last && list.length < RANGE; r++) if (here.has(r)) list.push({ to: { map: m, room: r }, among });
    return list;
  };
  const uses = scripts?.uses ?? [];
  const wiresOfUse = (u: ScriptUse) => {
    const source = scripts!.sources[u.source]!;
    return source.targets.flatMap((target) =>
      ends(u, target).map(({ to, among }): Wire => ({ kind: target.kind, from: { map: u.map, room: u.room }, to, exit: target.exit, among, when: u.when, source, target })),
    );
  };
  const fields = (r: MapRoom): Wire[] =>
    (
      [
        ['death', r.death],
        ['hangup', r.exit_room],
        ['control', r.control],
      ] as const
    )
      .filter(([, n]) => n > 0 && n !== r.room)
      .map(([kind, n]) => ({ kind, from: { map, room: r.room }, to: { map, room: n } }));

  // from the rooms picked (so many of each kind from each), then into them
  if (rooms.size) {
    const sprays = new Map<string, Map<string, Wire>>();
    for (const u of uses) {
      if (!picked({ map: u.map, room: u.room })) continue;
      for (const w of wiresOfUse(u)) {
        const g = `${u.room} ${w.kind}`;
        const s = sprays.get(g) ?? sprays.set(g, new Map()).get(g)!;
        if (!s.has(key(w))) s.set(key(w), w);
      }
    }
    for (const s of sprays.values()) {
      const list = [...s.values()];
      if (list.length > SPRAY) {
        more += list.length - SPRAY;
        list.length = SPRAY;
        for (const w of list) w.fan = s.size;
      }
      list.forEach(add);
    }
    for (const n of rooms) {
      const r = here.get(n);
      if (r) fields(r).forEach(add);
    }
    for (const u of uses) for (const w of wiresOfUse(u)) if (picked(w.to)) add(w);
    for (const r of here.values()) for (const w of fields(r)) if (picked(w.to)) add(w);
  }
  if (all)
    for (const u of uses) {
      const ws = wiresOfUse(u);
      if (ws.length <= FAN) ws.forEach(add);
    }
  return { wires: out, more };
}
