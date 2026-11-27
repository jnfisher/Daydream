// A map's rooms as the server sends them (`GET /api/map`, from crates/daydream-nmr/src/map.rs).

export interface MapExit {
  /** 0 to 9: N S E W NE NW SE SW U D. */
  dir: number;
  /** The room it leads to (on map `para1` when `kind` is 8, a map change). */
  to: number;
  /** The exit type (`Type n`). */
  kind: number;
  para1: number;
  para2: number;
  para3: number;
  para4: number;
}

export interface MapRoom {
  room: number;
  name: string;
  exits: MapExit[];
  shop: number;
  spell: number;
  command: number;
  npc: number;
  control: number;
  death: number;
  exit_room: number;
  /** `Type`: 0 normal, 1 shop, 2 arena, 3 lair, 4 hotel, 5 colliseum, 6 jail, 7 library. */
  room_type: number;
  /** `Mon Type`: which monsters come, and which of the group (`Min Index` to `Max Index`) and how many at most (`Max Regen`). */
  mon_type: number;
  min_index: number;
  max_index: number;
  max_regen: number;
  light: number;
}

export interface MapAnswer {
  map: number;
  /** Every map with rooms, and how many. */
  maps: [number, number][];
  /** Each map's commonest room name. */
  names?: MapName[];
  rooms: MapRoom[];
  /** The map change exits between maps, by the two maps and the way they go. */
  links?: MapLink[];
  /** Where its rooms moved by hand are drawn. */
  pins?: PinCells;
}

/** The name most of a map's rooms share, and how many do. */
export interface MapName {
  map: number;
  name: string;
  rooms: number;
}

/** The map change exits from one map to another. */
export interface MapLink {
  from: number;
  to: number;
  /** How many lead that way. */
  exits: number;
  /** The first of them: its room on `from`, and the room it leads to on `to`. */
  room: number;
  to_room: number;
}

/** What a map is mostly, by the name of its rooms (MajorMUD's maps have only numbers): the
 * commonest name, when a fair share of the map's rooms have it, else null. */
export function mapName(names: ReadonlyMap<number, MapName>, map: number, rooms: number): string | null {
  const n = names.get(map);
  return n && n.rooms * 20 >= rooms ? n.name : null;
}

/** Cells by room number, as the map's side file keeps them. */
export type PinCells = Record<string, [number, number]>;

/** The map's side file (`.daydream/map.json`, crates/daydream-server/src/pins.rs), as exported. */
export interface MapFile {
  daydream: 'map';
  version: 1;
  maps: Record<string, { pins: PinCells }>;
}

/** A stamp saved from the map (`.daydream/stamps.json`, crates/daydream-server/src/stamps.rs): a shape, as the drawer's own. */
export interface SavedStamp {
  /** `s` and a number, given when it is first saved. */
  id: string;
  name: string;
  /** The map it was saved from, and the room there its new rooms copy (one with the name most of its rooms had): put down on that map, they copy it unless it lands on a room. */
  map?: number;
  room?: number;
  /** Each cell as [column, row] from the first, its way in. */
  cells: [number, number][];
  links: { a: number; dir: number; b: number; one_way?: boolean }[];
}

/** The stamps saved, in the order they were first saved. */
export interface StampsFile {
  daydream: 'stamps';
  version: 1;
  next: number;
  stamps: SavedStamp[];
}

/** A dig (`POST /api/map/dig`, crates/daydream-nmr/src/map.rs's `Dig`). */
export interface Dig {
  map: number;
  room: number;
  dir: number;
  to_map: number;
  to_room: number;
  /** Join the room `to_room` (else new rooms are made). */
  existing?: boolean;
  /** A line of new rooms, each from the one before: `to_room` and the next numbers the map does not use. */
  count?: number;
}

export const DIRS = ['N', 'S', 'E', 'W', 'NE', 'NW', 'SE', 'SW', 'U', 'D'];
export const DIR_WORDS = ['north', 'south', 'east', 'west', 'northeast', 'northwest', 'southeast', 'southwest', 'up', 'down'];

/** The direction each key goes on the map: the arrows and the keys beside them, and the number pad. */
export const KEY_DIR: Record<string, number> = {
  ArrowUp: 0, ArrowDown: 1, ArrowRight: 2, ArrowLeft: 3, PageUp: 4, Home: 5, PageDown: 6, End: 7,
  Numpad8: 0, Numpad2: 1, Numpad6: 2, Numpad4: 3, Numpad9: 4, Numpad7: 5, Numpad3: 6, Numpad1: 7,
};

/** The exit types' names (EXIT_TYPE in the editors' lists). */
export const EXIT_KINDS = [
  'Normal', 'Spell', 'Key', 'Item', 'Toll', 'Action', 'Hidden', 'Door', 'Map change', 'Trap', 'Text', 'Gate',
  'Remote action', 'Class', 'Race', 'Level', 'Timed', 'Ticket', 'User count', 'Block guard', 'Alignment', 'Delay',
  'Cast', 'Ability', 'Spell trap',
];

export const kindName = (kind: number) => EXIT_KINDS[kind] ?? `Type ${kind}`;
