// NMR's lists as the Workshop shows them: the record each place an item is names, the order of
// the limited items, finding in a list, and each list as CSV.

import type { Cell, ControlRoom, GroupIndex, ItemPlace, LimitedItem, NpcRoom } from '../api';

/** The lists the Workshop shows, by their tools' ids. */
export const LISTS = ['limited', 'where', 'areas', 'npcs', 'spawns'] as const;
export type ListId = (typeof LISTS)[number];
export const isList = (id: string): id is ListId => (LISTS as readonly string[]).includes(id);

/** The record a place names: a room (on its floor or hidden), a shop, or a user. */
export function placeRef(p: ItemPlace): { table: string; key: Cell[] } {
  if (p.kind === 'room' || p.kind === 'hidden') {
    const [map, room] = p.key.split('/').map(Number);
    return { table: 'Rooms', key: [map ?? 0, room ?? 0] };
  }
  if (p.kind === 'shop') return { table: 'Shops', key: [Number(p.key)] };
  return { table: 'Users', key: [p.key] };
}

/** A place in words, as a CSV cell holds it: `Room 1/224 Town Square ×2`. */
export function placeText(p: ItemPlace): string {
  const what = p.kind === 'shop' ? 'Shop' : p.kind === 'user' ? 'User' : p.kind === 'hidden' ? 'Hidden in room' : 'Room';
  return `${what} ${p.key}${p.name ? ` ${p.name}` : ''} ×${p.count}`;
}

/** Whether a row's text holds every word of what is being found, in any order. */
export function finds(text: string, q: string): boolean {
  const t = text.toLowerCase();
  return q
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((w) => t.includes(w));
}

/** How many copies of a limited item are beyond its limit. */
export const overBy = (i: LimitedItem) => Math.max(0, i.in_use - i.limit);

/** The limited items: those over their limit first, furthest over first, then by number. */
export function limitedOrder(items: LimitedItem[]): LimitedItem[] {
  return [...items].sort((a, b) => Number(b.over) - Number(a.over) || overBy(b) - overBy(a) || a.number - b.number);
}

type Csv = { header: string[]; rows: (string | number)[][] };

/** Each list as CSV. */
export const csvOf = {
  limited: (items: LimitedItem[]): Csv => ({
    header: ['Number', 'Name', 'Limit', 'In use', 'Over by', 'Where'],
    rows: items.map((i) => [i.number, i.name, i.limit, i.in_use, overBy(i), i.places.map(placeText).join('; ')]),
  }),
  places: (places: ItemPlace[]): Csv => ({
    header: ['Where', 'Key', 'Name', 'Count'],
    rows: places.map((p) => [p.kind, p.key, p.name, p.count]),
  }),
  areas: (rooms: ControlRoom[]): Csv => ({
    header: ['Map', 'Control room', 'Name', 'Rooms that name it', 'Their numbers'],
    rows: rooms.map((c) => [c.map, c.room, c.name, c.rooms.length, c.rooms.join(' ')]),
  }),
  npcs: (rooms: NpcRoom[]): Csv => ({
    header: ['Map', 'Room', 'Room name', 'Monster', 'Monster name'],
    rows: rooms.map((r) => [r.map, r.room, r.room_name, r.monster, r.monster_name]),
  }),
  groups: (groups: GroupIndex[], groupName: (g: number) => string): Csv => ({
    header: ['Group', 'Group name', 'Index', 'Monsters'],
    rows: groups.map((g) => [g.group, groupName(g.group), g.index, g.monsters.map(([n, name]) => `${n} ${name}`).join('; ')]),
  }),
};
