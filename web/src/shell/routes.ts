// Where the page is, from its hash. Every record has a stable link (`#/items/868`,
// `#/rooms/1/224`, `#/users/Sysop`), so a link pasted into a chat, a note or an agent's
// report opens the same record, and a table's grid has one too (`#/items?grid`). Today's links (`#/Items/[868]`, `#/map/1/224`, `#/tools`)
// keep working; the old map is `#/map-classic/1/224`.

import type { Cell, TableInfo } from '../api';

export type Place = 'home' | 'world' | 'library' | 'story' | 'players' | 'workshop' | 'worlds' | 'publish';

export interface Route {
  place: Place;
  /** Library and Players: the table (`Items`) and the record's key, when one is open. */
  table?: string;
  key?: Cell[];
  /** Library: the table as a spreadsheet grid. */
  view?: 'grid';
  /** What follows the place: the World's map and room, the Workshop's tab. */
  rest: string[];
}

/** The tables the Players place edits; the Library has the rest. */
export const PLAYER_TABLES = ['Users', 'BankBooks', 'Gangs'];

export const slug = (table: string) => table.toLowerCase();

const PLACES: Record<string, Place> = {
  '': 'home',
  home: 'home',
  world: 'world',
  map: 'world',
  library: 'library',
  story: 'story',
  players: 'players',
  workshop: 'workshop',
  tools: 'workshop',
  lists: 'workshop',
  worlds: 'worlds',
  games: 'worlds',
  publish: 'publish',
};

/** The route a hash names; a table the realm does not have goes Home. */
export function parse(hash: string, tables: TableInfo[]): Route {
  const [path = '', query = ''] = hash.split('?');
  const r = parsePath(path, tables);
  return (r.place === 'library' || r.place === 'players') && !r.key && r.table && query.split('&').includes('grid') ? { ...r, view: 'grid' } : r;
}

function parsePath(hash: string, tables: TableInfo[]): Route {
  const parts = hash.replace(/^#\/?/, '').split('/');
  const head = safeDecode(parts[0] ?? '').toLowerCase();
  const rest = parts.slice(1);
  const place = PLACES[head];
  if (head === 'map-classic') return { place: 'world', rest: ['classic', ...rest.map(safeDecode)] };
  if (place) {
    if (head === 'lists') return { place, rest: ['lists'] };
    if (place === 'library' || place === 'players') {
      // `#/library` and `#/players` go back to where they were left (or their first table);
      // `#/library/items/868` opens its record
      const t = rest[0] ? tableOf(rest[0], tables) : undefined;
      return t ? recordRoute(t, rest.slice(1)) : { place, rest: [] };
    }
    return { place, rest: rest.map(safeDecode) };
  }
  const t = tableOf(parts[0] ?? '', tables);
  return t ? recordRoute(t, rest) : { place: 'home', rest: [] };
}

function tableOf(part: string, tables: TableInfo[]): TableInfo | undefined {
  const name = safeDecode(part).toLowerCase();
  return tables.find((t) => slug(t.name) === name);
}

function recordRoute(t: TableInfo, parts: string[]): Route {
  return {
    place: PLAYER_TABLES.includes(t.name) ? 'players' : 'library',
    table: t.name,
    key: keyOf(t, parts),
    rest: [],
  };
}

/** A record's key from its link's parts, typed as its index columns are. */
function keyOf(t: TableInfo, parts: string[]): Cell[] | undefined {
  const raw = parts.map(safeDecode);
  if (!raw.length || raw[0] === '') return undefined;
  if (raw.length === 1 && raw[0]!.startsWith('[')) {
    // today's links: the key as JSON
    try {
      const v: unknown = JSON.parse(raw[0]!);
      return Array.isArray(v) ? (v as Cell[]) : undefined;
    } catch {
      return undefined;
    }
  }
  if (raw.length !== t.key.length) return undefined;
  const key: Cell[] = [];
  for (const [i, part] of raw.entries()) {
    const col = t.columns.find((c) => c.name === t.key[i]);
    if (col?.type === 'long' || col?.type === 'double') {
      const n = Number(part);
      if (part.trim() === '' || !Number.isFinite(n)) return undefined;
      key.push(n);
    } else key.push(part);
  }
  return key;
}

function safeDecode(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}

/** A record's stable link, or its table's when there is no key. */
export function recordHref(table: string, key?: Cell[] | null): string {
  const base = `#/${slug(table)}`;
  return key?.length ? `${base}/${key.map((k) => encodeURIComponent(String(k ?? ''))).join('/')}` : base;
}

/** A table's spreadsheet grid. */
export const gridHref = (table: string) => `${recordHref(table)}?grid`;

/** The tables a story starts at: a monster's conversation, a room's commands, a text block. */
export const STORY_TABLES = ['Monsters', 'Rooms', 'Textblocks'];

/** The story that starts at a record: `#/story/monsters/381`. */
export const storyHref = (table: string, key: Cell[]) => `#/story${recordHref(table, key).slice(1)}`;

/** The record a story starts at, from what follows `#/story`; a text block's part may be left
 * out. Null for none, or one no story starts at. */
export function storyStart(rest: string[], tables: TableInfo[]): { table: string; key: Cell[] } | null {
  const t = rest[0] ? tableOf(rest[0], tables) : undefined;
  if (!t || !STORY_TABLES.includes(t.name)) return null;
  const parts = rest.slice(1).map((p) => encodeURIComponent(p));
  if (t.name === 'Textblocks' && parts.length === 1) parts.push('0');
  const key = keyOf(t, parts);
  return key ? { table: t.name, key } : null;
}

/** A key as people write it: `868`, `1/224`. */
export const keyLabel = (key: Cell[]) => key.map((k) => String(k ?? '')).join('/');
