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
  /** `Mon Type`: which monsters come. */
  mon_type: number;
  light: number;
}

export interface MapAnswer {
  map: number;
  /** Every map with rooms, and how many. */
  maps: [number, number][];
  rooms: MapRoom[];
}

export const DIRS = ['N', 'S', 'E', 'W', 'NE', 'NW', 'SE', 'SW', 'U', 'D'];
export const DIR_WORDS = ['north', 'south', 'east', 'west', 'northeast', 'northwest', 'southeast', 'southwest', 'up', 'down'];

/** The exit types' names (EXIT_TYPE in the editors' lists). */
export const EXIT_KINDS = [
  'Normal', 'Spell', 'Key', 'Item', 'Toll', 'Action', 'Hidden', 'Door', 'Map change', 'Trap', 'Text', 'Gate',
  'Remote action', 'Class', 'Race', 'Level', 'Timed', 'Ticket', 'User count', 'Block guard', 'Alignment', 'Delay',
  'Cast', 'Ability', 'Spell trap',
];

export const kindName = (kind: number) => EXIT_KINDS[kind] ?? `Type ${kind}`;
