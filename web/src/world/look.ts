// How the map looks: its colours in each theme, what a room is coloured by, and the symbol
// each kind of exit carries, so colour is never the only sign.

import { ICONS, type IconNode } from './icons';
import type { MapExit, MapRoom } from './model';
import { LIGHT_WORDS } from './picks';

// ---- colours -------------------------------------------------------------------------

export type Biome = 'street' | 'shop' | 'house' | 'park' | 'forest' | 'farm' | 'water' | 'grave' | 'slum' | 'cave' | 'other';

/** What an exit's symbol says, which also picks its colour. */
export type ExitSort = 'lock' | 'door' | 'hidden' | 'trap' | 'words' | 'gated' | 'guard' | 'timed' | 'toll' | 'spell';

export interface Palette {
  bg: string;
  dot: string;
  /** Room outlines. */
  line: string;
  /** Exit lines. */
  path: string;
  ink: string;
  /** Behind text drawn on the map. */
  halo: string;
  sel: string;
  badge: string;
  oneway: string;
  portal: string;
  missing: string;
  plain: string;
  biome: Record<Biome, string>;
  exit: Record<ExitSort, string>;
  /** A stamp being placed: new rooms and exits, rooms joined, exits in the way, exits replaced, cells left alone. */
  stamp: { fresh: string; join: string; clash: string; replace: string; kept: string };
}

const DAY: Palette = {
  bg: '#f6f7ff', dot: '#dcd8f2', line: '#2e2650', path: '#b4acd8', ink: '#2e2650', halo: '#f6f7ff',
  sel: '#8a6bff', badge: '#ffffff', oneway: '#d22c63', portal: '#8a6bff', missing: '#c0245e', plain: '#ffffff',
  stamp: { fresh: '#1f9d5c', join: '#2f74d0', clash: '#d98a00', replace: '#d9472b', kept: '#8f89ad' },
  biome: {
    street: '#efeafc', shop: '#ff9ac8', house: '#ffd9c4', park: '#c9f2dc', forest: '#93dfb8', farm: '#ffe9a3',
    water: '#a3d9ff', grave: '#d9ccff', slum: '#ffc1a3', cave: '#d8d3ea', other: '#ffffff',
  },
  exit: {
    lock: '#b07a00', door: '#9a5a1c', hidden: '#7d76a3', trap: '#d22c63', words: '#7a55ff',
    gated: '#0f948a', guard: '#d4581c', timed: '#19679f', toll: '#b07a00', spell: '#b0216a',
  },
};

const NIGHT: Palette = {
  bg: '#110f26', dot: '#262150', line: '#0b0a1a', path: '#5d5597', ink: '#f2efff', halo: '#110f26',
  sel: '#ffe27a', badge: '#1f1b3d', oneway: '#ff8dc4', portal: '#b8a5ff', missing: '#ff9fc1', plain: '#4a4380',
  stamp: { fresh: '#5ee3a0', join: '#7fb6ff', clash: '#ffc94d', replace: '#ff8f70', kept: '#8a84b0' },
  biome: {
    street: '#5a5296', shop: '#ff8dc4', house: '#e9a487', park: '#7cd9ad', forest: '#46b184', farm: '#e3c461',
    water: '#58a9e6', grave: '#9f8cf2', slum: '#e8865d', cave: '#77709f', other: '#4a4380',
  },
  exit: {
    lock: '#ffd25e', door: '#ffb98a', hidden: '#9790c4', trap: '#ff8dc4', words: '#c4b0ff',
    gated: '#5fe0d6', guard: '#ffae88', timed: '#7fcbff', toll: '#ffd25e', spell: '#ffb6d9',
  },
};

/** The palette of the theme showing now. */
export function palette(): Palette {
  const scheme = getComputedStyle(document.documentElement).getPropertyValue('--scheme').trim();
  return scheme === 'night' ? NIGHT : DAY;
}

export const BIOMES: Record<Biome, string> = {
  street: 'Streets and squares',
  shop: 'Shops and inns',
  house: 'Buildings',
  park: 'Parks and paths',
  forest: 'Forest',
  farm: 'Farms and fields',
  water: 'Water',
  grave: 'Graves and crypts',
  slum: 'Slums',
  cave: 'Caves and tunnels',
  other: 'Other',
};

// the first that fits wins: a slum street is a slum, a forest path a forest
const WORDS: [Biome, RegExp][] = [
  ['grave', /\b(graves?|graveyard|cemetery|crypts?|tombs?|mausoleum|catacombs?|barrows?|ossuary|sepulchre|burial)\b/i],
  ['slum', /\b(slums?|gutters?|shanty|hovels?|ghetto)\b/i],
  ['water', /\b(river|lake|sea|ocean|pond|stream|waterfall|falls|marsh|swamp|bog|shore|beach|coast|docks?|pier|harbou?r|wharf|canal|creek|lagoon|bay|underwater|water|rapids|ford)\b/i],
  ['cave', /\b(caves?|caverns?|tunnels?|mines?|grotto|underground|dungeons?|sewers?|labyrinth|passages?|burrows?|pits?|chasm|lair|den|hollow)\b/i],
  ['forest', /\b(forest|woods?|woodland|grove|thicket|jungle|copse|darkwood|trees?)\b/i],
  ['farm', /\b(farms?|farmland|fields?|ranch|pastures?|barn|stables?|mill|orchard|vineyard|crops?|wheat|corn|paddock|meadows?)\b/i],
  ['shop', /\b(shop|store|market|bazaar|smithy|blacksmith|armou?ry|tavern|inn|pub|bank|guild|emporium|outfitters?|apothecary|bakery|merchant|trader|vendor|stall)\b/i],
  ['house', /\b(house|home|hut|cottage|cabin|building|tower|keep|castle|manor|mansion|palace|hall|temple|church|chapel|shrine|cathedral|library|office|kitchen|cellar|attic|bedroom|chamber|room|corridor|hallway|stairs?|stairwell|throne|barracks|prison|jail|vault|study|foyer|lobby|gallery)\b/i],
  ['park', /\b(park|gardens?|glade|clearing|lawn|hedges?|maze|hills?|valley|plains?|grassland|heath|moor|trail|path)\b/i],
  ['street', /\b(street|road|avenue|lane|way|boulevard|plaza|square|gates?|bridge|walls?|courtyard|crossroads|intersection|alley(way)?)\b/i],
];

/** What a room looks like it is, guessed from its name (a shop by its number). */
export function biomeOf(r: Pick<MapRoom, 'name' | 'room_type' | 'shop'>): Biome {
  if (r.room_type === 1 || r.shop) return 'shop';
  for (const [b, re] of WORDS) if (re.test(r.name)) return b;
  return 'other';
}

const SWATCH = ['#ff9ac8', '#9fd8ff', '#ffd36e', '#8fe6c1', '#c4b0ff', '#ffb08a', '#7fe3da', '#f6a3ff', '#b6e36b', '#ffc4d8', '#a6b4ff'];
const SWATCH_NIGHT = ['#d9679b', '#4f9fd6', '#d6a93f', '#47b98e', '#9a83e6', '#d8835c', '#3fb3aa', '#c56fd0', '#86b33c', '#d98fa9', '#7787db'];

/** A colour for any number (an area's control room, a monster group), the same each time. */
export function swatch(n: number, p: Palette): string {
  const list = p === NIGHT ? SWATCH_NIGHT : SWATCH;
  return list[Math.abs(Math.imul(n, 2654435761) >>> 0) % list.length]!;
}

/** Mixes two #rrggbb colours. */
export function mix(a: string, b: string, t: number): string {
  const pa = parseInt(a.slice(1), 16);
  const pb = parseInt(b.slice(1), 16);
  const ch = (s: number) => Math.round(((pa >> s) & 255) * (1 - t) + ((pb >> s) & 255) * t);
  return `rgb(${ch(16)},${ch(8)},${ch(0)})`;
}

export type ColourBy = 'place' | 'area' | 'monsters' | 'light' | 'edits';

export const COLOUR_BY: { id: ColourBy; label: string }[] = [
  { id: 'place', label: 'Place' },
  { id: 'area', label: 'Area' },
  { id: 'monsters', label: 'Monsters' },
  { id: 'light', label: 'Light' },
  { id: 'edits', label: 'My edits' },
];

/** A room's group in the legend for a colouring, and its colour. */
export interface Tint {
  key: string;
  label: string;
  colour: string;
}


export function tintOf(r: MapRoom, by: ColourBy, p: Palette, biome: Biome, edited: boolean): Tint {
  switch (by) {
    case 'place':
      return { key: biome, label: BIOMES[biome], colour: p.biome[biome] };
    case 'area':
      return r.control
        ? { key: `a${r.control}`, label: `Control room ${r.control}`, colour: swatch(r.control, p) }
        : { key: 'none', label: 'No control room', colour: p.plain };
    case 'monsters':
      return r.mon_type
        ? { key: `m${r.mon_type}`, label: monsterGroup(r.mon_type), colour: swatch(r.mon_type * 7 + 3, p) }
        : { key: 'none', label: 'Lair group 0', colour: p.plain };
    case 'light': {
      // the game's own bands: blinding, daylight, regular, dimly lit, barely visible, very dark, pitch black
      const band = LIGHT_WORDS.findIndex(([from]) => r.light >= from);
      const colour =
        band < 2 ? mix(p.plain, p === NIGHT ? '#e3c461' : '#ffd25e', band ? 0.45 : 0.85) : band === 2 ? p.plain : mix(p.plain, p === NIGHT ? '#05040d' : '#3a2f7a', [0.3, 0.5, 0.68, 0.85][band - 3]!);
      return { key: `l${band}`, label: LIGHT_WORDS[band]![1], colour };
    }
    case 'edits':
      return edited ? { key: 'edited', label: 'Changed since publishing', colour: p === NIGHT ? '#e3c461' : '#ffd25e' } : { key: 'none', label: 'Not changed', colour: p.plain };
  }
}

const GROUPS = ['Lair', 'Wanderer', 'NPC', 'Living', 'Random', 'Guard'];

function monsterGroup(n: number): string {
  if (n >= 6 && n <= 35) return `Monster group ${n - 5}`;
  return GROUPS[n] ?? ['Arena', 'Angel', 'Quest', 'Other'][n - 36] ?? `Monster type ${n}`;
}

// ---- exits ---------------------------------------------------------------------------

export interface ExitLook {
  sort: ExitSort | null;
  /** The symbol drawn on its line. */
  icon?: IconName;
  /** A door's bars: one, or two when a key locks it. */
  bars?: number;
  dashed?: boolean;
}

/** How an exit is drawn, by its type (`Type n`) and parameters. */
export function exitLook(e: Pick<MapExit, 'kind' | 'para4'>): ExitLook {
  switch (e.kind) {
    case 1:
      return { sort: 'spell', icon: 'sparkles' };
    case 2:
      return { sort: 'lock', icon: 'lock' };
    case 3:
      return { sort: 'lock', icon: 'key' };
    case 4:
      return { sort: 'toll', icon: 'coins' };
    case 5:
    case 12:
      return { sort: 'words', icon: e.kind === 5 ? 'hand' : 'radio' };
    case 6:
      return { sort: 'hidden', icon: 'eye-off', dashed: true };
    case 7:
    case 11:
      return { sort: 'door', bars: e.para4 ? 2 : 1 };
    case 9:
    case 24:
      return { sort: 'trap', icon: 'zap' };
    case 10:
      return { sort: 'words', icon: 'message' };
    case 13:
    case 14:
    case 15:
    case 20:
    case 23:
      return { sort: 'gated', icon: 'user-check' };
    case 16:
      return { sort: 'timed', icon: 'clock' };
    case 17:
      return { sort: 'toll', icon: 'ticket' };
    case 18:
      return { sort: 'gated', icon: 'users' };
    case 19:
      return { sort: 'guard', icon: 'shield' };
    case 21:
      return { sort: 'timed', icon: 'hourglass' };
    case 22:
      return { sort: 'spell', icon: 'wand' };
    default:
      return { sort: null };
  }
}

/** The symbols of the legend: what each kind of exit looks like on the map. */
export const EXIT_LEGEND: { label: string; look: ExitLook }[] = [
  { label: 'Door, gate', look: { sort: 'door', bars: 1 } },
  { label: 'Door with a key', look: { sort: 'door', bars: 2 } },
  { label: 'Key', look: { sort: 'lock', icon: 'lock' } },
  { label: 'Item needed', look: { sort: 'lock', icon: 'key' } },
  { label: 'Hidden', look: { sort: 'hidden', icon: 'eye-off', dashed: true } },
  { label: 'Trap', look: { sort: 'trap', icon: 'zap' } },
  { label: 'Block guard', look: { sort: 'guard', icon: 'shield' } },
  { label: 'Toll', look: { sort: 'toll', icon: 'coins' } },
  { label: 'Ticket', look: { sort: 'toll', icon: 'ticket' } },
  { label: 'Timed', look: { sort: 'timed', icon: 'clock' } },
  { label: 'Delay', look: { sort: 'timed', icon: 'hourglass' } },
  { label: 'Spell', look: { sort: 'spell', icon: 'sparkles' } },
  { label: 'Cast', look: { sort: 'spell', icon: 'wand' } },
  { label: 'Action', look: { sort: 'words', icon: 'hand' } },
  { label: 'Remote action', look: { sort: 'words', icon: 'radio' } },
  { label: 'Text', look: { sort: 'words', icon: 'message' } },
  { label: 'Class, race, level, alignment, ability', look: { sort: 'gated', icon: 'user-check' } },
  { label: 'User count', look: { sort: 'gated', icon: 'users' } },
];

/** The room marks of the legend. */
export const ROOM_LEGEND: { label: string; icon: IconName }[] = [
  { label: 'Shop', icon: 'store' },
  { label: 'Arena', icon: 'swords' },
  { label: 'Lair', icon: 'crown' },
  { label: 'Permanent NPC', icon: 'user-round' },
  { label: 'Command text or room spell', icon: 'sparkles' },
];

// ---- icons, as canvas paths ----------------------------------------------------------

export type IconName = keyof typeof ICONS;

const paths = new Map<IconName, Path2D[]>();

/** A Lucide icon's strokes, on a 24 by 24 box. */
export function iconPaths(name: IconName): Path2D[] {
  let out = paths.get(name);
  if (out) return out;
  out = [];
  const num = (v: string | undefined) => Number(v ?? 0);
  for (const [tag, a] of ICONS[name] as IconNode) {
    const p = new Path2D();
    switch (tag) {
      case 'path':
        out.push(new Path2D(String(a.d)));
        continue;
      case 'circle':
        p.arc(num(a.cx), num(a.cy), num(a.r), 0, Math.PI * 2);
        break;
      case 'ellipse':
        p.ellipse(num(a.cx), num(a.cy), num(a.rx), num(a.ry), 0, 0, Math.PI * 2);
        break;
      case 'rect':
        p.roundRect(num(a.x), num(a.y), num(a.width), num(a.height), num(a.rx));
        break;
      case 'line':
        p.moveTo(num(a.x1), num(a.y1));
        p.lineTo(num(a.x2), num(a.y2));
        break;
      case 'polyline':
      case 'polygon': {
        const pts = String(a.points).trim().split(/[\s,]+/).map(Number);
        for (let i = 0; i + 1 < pts.length; i += 2) {
          if (i) p.lineTo(pts[i]!, pts[i + 1]!);
          else p.moveTo(pts[i]!, pts[i + 1]!);
        }
        if (tag === 'polygon') p.closePath();
        break;
      }
      default:
        continue;
    }
    out.push(p);
  }
  paths.set(name, out);
  return out;
}
