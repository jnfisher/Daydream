// How the map looks: its colours in each theme and map style, what a room is coloured by, and
// the symbol each kind of exit carries, so colour is never the only sign.

import { ICONS, type IconNode } from './icons.ts';
import type { MapExit, MapRoom } from './model.ts';
import type { WireKind } from './wires.ts';
import { LIGHT_WORDS } from './light.ts';

// ---- colours -------------------------------------------------------------------------

export type Biome = 'street' | 'shop' | 'house' | 'park' | 'forest' | 'farm' | 'water' | 'grave' | 'slum' | 'cave' | 'other';

/** What an exit's symbol says, which also picks its colour. */
export type ExitSort = 'lock' | 'door' | 'hidden' | 'trap' | 'words' | 'action' | 'gated' | 'guard' | 'timed' | 'toll' | 'spell';

export interface Palette {
  /** A dark ground: what is drawn on it is light. */
  dark: boolean;
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
  /** Words on `sel` (the selected room's name). */
  selInk: string;
  /** The ground of chips and marks, which `ink` is drawn on. */
  badge: string;
  oneway: string;
  portal: string;
  missing: string;
  plain: string;
  biome: Record<Biome, string>;
  exit: Record<ExitSort, string>;
  /** The links no exit draws (wires.ts): a script's teleport, a spell's, a remote action, a death, hang-up and control room. */
  wire: Record<WireKind, string>;
  /** A stamp being placed: new rooms and exits, rooms joined, exits in the way, exits replaced, cells left alone. */
  stamp: { fresh: string; join: string; clash: string; replace: string; kept: string };
  /** Colours for any number (an area's control room, a monster group). */
  swatches: readonly string[];
  /** Picked out (a room changed since publishing, bright light), and darkness (pitch black). */
  warm: string;
  deep: string;
  /** How rooms are drawn: their corners' rounding (1 as round as they go, 0 square), and the grid
   * between them (dots at its corners, lines along it, or none). */
  round: number;
  grid: 'dots' | 'lines' | 'none';
  /** The font of names on the map, and of the selected room's name and the captions. */
  font: string;
  ui: string;
  /** Rooms coloured by place are coloured by their ways up and down instead, as NMR's map does. */
  upDown?: boolean;
}

const FREDOKA = 'Fredoka, Nunito, sans-serif';
const NUNITO = 'Nunito, sans-serif';
const MONO = 'ui-monospace, "Cascadia Mono", Consolas, monospace';
const TAHOMA = 'Tahoma, Verdana, "Segoe UI", sans-serif';

const DAY: Palette = {
  dark: false, bg: '#f6f7ff', dot: '#dcd8f2', line: '#2e2650', path: '#b4acd8', ink: '#2e2650', halo: '#f6f7ff',
  sel: '#8a6bff', selInk: '#ffffff', badge: '#ffffff', oneway: '#d22c63', portal: '#8a6bff', missing: '#c0245e', plain: '#ffffff',
  stamp: { fresh: '#1f9d5c', join: '#2f74d0', clash: '#d98a00', replace: '#d9472b', kept: '#8f89ad' },
  biome: {
    street: '#efeafc', shop: '#ff9ac8', house: '#ffd9c4', park: '#c9f2dc', forest: '#93dfb8', farm: '#ffe9a3',
    water: '#a3d9ff', grave: '#d9ccff', slum: '#ffc1a3', cave: '#d8d3ea', other: '#ffffff',
  },
  exit: {
    lock: '#b07a00', door: '#9a5a1c', hidden: '#7d76a3', trap: '#d22c63', words: '#7a55ff', action: '#7a55ff',
    gated: '#0f948a', guard: '#d4581c', timed: '#19679f', toll: '#b07a00', spell: '#b0216a',
  },
  wire: { teleport: '#a82ad1', spell: '#c2186b', remote: '#c26a00', death: '#c0245e', hangup: '#2d6fb8', control: '#0f8a7e' },
  swatches: ['#ff9ac8', '#9fd8ff', '#ffd36e', '#8fe6c1', '#c4b0ff', '#ffb08a', '#7fe3da', '#f6a3ff', '#b6e36b', '#ffc4d8', '#a6b4ff'],
  warm: '#ffd25e', deep: '#3a2f7a', round: 1, grid: 'dots', font: FREDOKA, ui: NUNITO,
};

const NIGHT: Palette = {
  dark: true, bg: '#110f26', dot: '#262150', line: '#0b0a1a', path: '#5d5597', ink: '#f2efff', halo: '#110f26',
  sel: '#ffe27a', selInk: '#1a1636', badge: '#1f1b3d', oneway: '#ff8dc4', portal: '#b8a5ff', missing: '#ff9fc1', plain: '#4a4380',
  stamp: { fresh: '#5ee3a0', join: '#7fb6ff', clash: '#ffc94d', replace: '#ff8f70', kept: '#8a84b0' },
  biome: {
    street: '#5a5296', shop: '#ff8dc4', house: '#e9a487', park: '#7cd9ad', forest: '#46b184', farm: '#e3c461',
    water: '#58a9e6', grave: '#9f8cf2', slum: '#e8865d', cave: '#77709f', other: '#4a4380',
  },
  exit: {
    lock: '#ffd25e', door: '#ffb98a', hidden: '#9790c4', trap: '#ff8dc4', words: '#c4b0ff', action: '#c4b0ff',
    gated: '#5fe0d6', guard: '#ffae88', timed: '#7fcbff', toll: '#ffd25e', spell: '#ffb6d9',
  },
  wire: { teleport: '#e7a6ff', spell: '#ff9ccf', remote: '#ffc46b', death: '#ff8fa8', hangup: '#8ec5ff', control: '#6fe6d8' },
  swatches: ['#d9679b', '#4f9fd6', '#d6a93f', '#47b98e', '#9a83e6', '#d8835c', '#3fb3aa', '#c56fd0', '#86b33c', '#d98fa9', '#7787db'],
  warm: '#e3c461', deep: '#05040d', round: 1, grid: 'dots', font: FREDOKA, ui: NUNITO,
};

// The 16 colours of an ANSI terminal on black, with NMR's line colours in them.
const TERMINAL: Palette = {
  dark: true, bg: '#000000', dot: '#262626', line: '#000000', path: '#aaaaaa', ink: '#ffffff', halo: '#000000',
  sel: '#ffff55', selInk: '#000000', badge: '#000000', oneway: '#ff5555', portal: '#ff55ff', missing: '#ff5555', plain: '#aaaaaa',
  stamp: { fresh: '#55ff55', join: '#55ffff', clash: '#ffff55', replace: '#ff5555', kept: '#555555' },
  biome: {
    street: '#aaaaaa', shop: '#ff55ff', house: '#aa5500', park: '#55ff55', forest: '#00aa00', farm: '#ffff55',
    water: '#5555ff', grave: '#aa00aa', slum: '#ff5555', cave: '#555555', other: '#aaaaaa',
  },
  exit: {
    lock: '#55ff55', door: '#5555ff', hidden: '#aa00aa', trap: '#ff5555', words: '#ffff55', action: '#55ffff',
    gated: '#00aaaa', guard: '#aa5500', timed: '#00aa00', toll: '#55ff55', spell: '#ff55ff',
  },
  wire: { teleport: '#ff55ff', spell: '#55ffff', remote: '#ffff55', death: '#ff5555', hangup: '#5555ff', control: '#00aaaa' },
  swatches: ['#ff5555', '#55ff55', '#ffff55', '#5555ff', '#ff55ff', '#55ffff', '#aa5500', '#00aa00', '#aa00aa', '#00aaaa', '#aaaaaa'],
  warm: '#ffff55', deep: '#000000', round: 0, grid: 'dots', font: MONO, ui: MONO,
};

// White lines on blue paper, on a grid.
const BLUEPRINT: Palette = {
  dark: true, bg: '#1d3f8f', dot: '#ffffff', line: '#e8f0ff', path: '#9fbaf2', ink: '#ffffff', halo: '#1d3f8f',
  sel: '#ffe27a', selInk: '#1a1636', badge: '#1d3f8f', oneway: '#ff9ac8', portal: '#ffffff', missing: '#ff9ac8', plain: '#24499f',
  stamp: { fresh: '#7df0c5', join: '#bfe0ff', clash: '#ffe27a', replace: '#ff9f8a', kept: '#9fbaf2' },
  biome: {
    street: '#22489c', shop: '#3a64cc', house: '#2d55b3', park: '#22679f', forest: '#1d6b8c', farm: '#3d5ab8',
    water: '#1857c4', grave: '#4549ad', slum: '#3a479e', cave: '#253d80', other: '#24499f',
  },
  exit: {
    lock: '#ffe27a', door: '#ffffff', hidden: '#9fbaf2', trap: '#ff9ac8', words: '#d9ccff', action: '#c4f0ff',
    gated: '#7df0c5', guard: '#ffb08a', timed: '#9fe3ff', toll: '#ffe27a', spell: '#ffc4e8',
  },
  wire: { teleport: '#e7c6ff', spell: '#ffb3d9', remote: '#ffd58a', death: '#ff9fb5', hangup: '#bfe0ff', control: '#7df0c5' },
  swatches: ['#7fb2ff', '#7df0c5', '#ffe27a', '#c4b0ff', '#ff9ac8', '#9fe3ff', '#b6e36b', '#ffb08a', '#e8f0ff', '#a6b4ff', '#f6a3ff'],
  warm: '#ffe27a', deep: '#0b1d4a', round: 0.3, grid: 'lines', font: NUNITO, ui: NUNITO,
};

// Nightmare Redux's map: grey rooms on black, each exit's line in its type's colour, and the rooms
// with ways up and down picked out (frmMap, frmMapLegend).
const CLASSIC: Palette = {
  dark: true, bg: '#000000', dot: '#000000', line: '#000000', path: '#909090', ink: '#e0e0e0', halo: '#000000',
  sel: '#ffffff', selInk: '#000000', badge: '#000000', oneway: '#ff5050', portal: '#ff5cff', missing: '#ff2020', plain: '#c0c0c0',
  stamp: { fresh: '#4ee04e', join: '#4ee0e0', clash: '#e8d020', replace: '#ff5050', kept: '#606060' },
  biome: {
    street: '#c0c0c0', shop: '#c0c0c0', house: '#c0c0c0', park: '#c0c0c0', forest: '#c0c0c0', farm: '#c0c0c0',
    water: '#c0c0c0', grave: '#c0c0c0', slum: '#c0c0c0', cave: '#c0c0c0', other: '#c0c0c0',
  },
  exit: {
    lock: '#4ee04e', door: '#6a8cff', hidden: '#a020a0', trap: '#ff5050', words: '#e8d020', action: '#4ee0e0',
    gated: '#3a5ad0', guard: '#909090', timed: '#20a020', toll: '#4ee04e', spell: '#909090',
  },
  wire: { teleport: '#ff5cff', spell: '#ff5cff', remote: '#4ee0e0', death: '#ff5050', hangup: '#6a8cff', control: '#4ee04e' },
  swatches: ['#ff5050', '#4ee04e', '#e8d020', '#6a8cff', '#ff5cff', '#4ee0e0', '#c08040', '#a0a0ff', '#80c080', '#e0a0a0', '#a0e0e0'],
  warm: '#ffff00', deep: '#202020', round: 0, grid: 'none', font: TAHOMA, ui: TAHOMA, upDown: true,
};

/** How the map is drawn: in the theme's own colours, or in a style of its own. */
export type MapStyle = 'theme' | 'daydream' | 'starlight' | 'terminal' | 'blueprint' | 'classic';

export const MAP_STYLES: { id: MapStyle; name: string; sub: string }[] = [
  { id: 'theme', name: 'Your theme', sub: 'The colours of the theme you use' },
  { id: 'daydream', name: 'Daydream', sub: 'Pastel paper' },
  { id: 'starlight', name: 'Starlight', sub: 'Indigo night' },
  { id: 'terminal', name: 'Terminal', sub: 'The 16 ANSI colours on black' },
  { id: 'blueprint', name: 'Blueprint', sub: 'White lines on blue paper' },
  { id: 'classic', name: 'Classic NMR', sub: 'Nightmare Redux’s grey rooms and line colours' },
];

const STYLES: Record<Exclude<MapStyle, 'theme'>, Palette> = { daydream: DAY, starlight: NIGHT, terminal: TERMINAL, blueprint: BLUEPRINT, classic: CLASSIC };

/** A map style by its name (as stored), else the theme's. */
export const styleFrom = (v: string): MapStyle => (MAP_STYLES.some((m) => m.id === v) ? (v as MapStyle) : 'theme');

/** A map style's palette: the theme showing now's for `theme`. */
export function palette(style: MapStyle = 'theme'): Palette {
  if (style !== 'theme') return STYLES[style];
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

/** A colour for any number (an area's control room, a monster group), the same each time. */
export function swatch(n: number, p: Palette): string {
  return p.swatches[Math.abs(Math.imul(n, 2654435761) >>> 0) % p.swatches.length]!;
}

/** How light a colour is (#rrggbb, or rgb(r,g,b) as `mix` gives), from 0 to 1, as WCAG reckons it. */
export function luminance(c: string): number {
  const v = c.startsWith('#') ? [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16)) : (c.match(/\d+/g) ?? []).slice(0, 3).map(Number);
  const [r, g, b] = v.map((x) => {
    const s = (x ?? 0) / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** How far apart two colours' lightness is, as WCAG's contrast ratio (1 to 21). */
export function contrast(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
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
    case 'place': {
      if (!p.upDown) return { key: biome, label: BIOMES[biome], colour: p.biome[biome] };
      // NMR's map: green for a way up, yellow for a way down, cyan for both
      const up = r.exits.some((e) => e.dir === 8);
      const down = r.exits.some((e) => e.dir === 9);
      if (up && down) return { key: 'updown', label: 'Ways up and down', colour: '#4ee0e0' };
      if (up) return { key: 'up', label: 'A way up', colour: '#4ee04e' };
      if (down) return { key: 'down', label: 'A way down', colour: '#e8d020' };
      return { key: 'none', label: 'No way up or down', colour: p.plain };
    }
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
      const colour = band < 2 ? mix(p.plain, p.warm, band ? 0.45 : 0.85) : band === 2 ? p.plain : mix(p.plain, p.deep, [0.3, 0.5, 0.68, 0.85][band - 3]!);
      return { key: `l${band}`, label: LIGHT_WORDS[band]![1], colour };
    }
    case 'edits':
      return edited ? { key: 'edited', label: 'Changed since publishing', colour: p.warm } : { key: 'none', label: 'Not changed', colour: p.plain };
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
      return { sort: 'action', icon: e.kind === 5 ? 'hand' : 'radio' };
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
  { label: 'Action', look: { sort: 'action', icon: 'hand' } },
  { label: 'Remote action', look: { sort: 'action', icon: 'radio' } },
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

const paths = new Map<IconName | IconNode, Path2D[]>();

/** A Lucide icon's strokes, on a 24 by 24 box: one of ICONS, or another's data. */
export function iconPaths(icon: IconName | IconNode): Path2D[] {
  let out = paths.get(icon);
  if (out) return out;
  out = [];
  const num = (v: string | undefined) => Number(v ?? 0);
  for (const [tag, a] of (typeof icon === 'string' ? ICONS[icon] : icon) as IconNode) {
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
  paths.set(icon, out);
  return out;
}
