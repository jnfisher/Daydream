// The World map, drawn on a canvas. Rooms are rounded squares on a grid, placed by layout.ts
// and tinted by what the map is coloured by; an exit between neighbouring rooms is a line, with
// its symbol on it when it is more than a plain way through, and an arrow when it is one-way. An
// exit to a room drawn somewhere else, to another map or to a room the map does not have is a
// short stub in its direction ending in a chip; up and down exits are triangles on the room.
// Names are drawn once per run of rooms that share one, more of them as you zoom in. Over them,
// dotted wires: the links no exit draws (a script's teleport, a spell's, a remote action, a
// death, hang-up or control room; wires.ts), for the selected rooms or for every room.
//
// The camera holds still: new rooms from the server keep the view where it was. In Build mode
// a room can be carried to a free cell; the page pins it there. A free cell or a join mark beside
// the selected room dragged out is a corridor that way, or a join onto the room it is let go
// over (build.ts); with Shift, a click digs a line to the cell. A tool (a stamp held, the Select
// tool) has the pointer first while it is on, and draws over the map; the tools' code loads with
// their panels, the first time each is used.
//
// A map drawn in levels (layout.ts) shows one level at a time, with the level below faint under
// it: selecting a room on another level shows that level, and an exit or wire to one ends in a
// chip saying which way.

import { alongWay, lineTo, reachJoin, reachLine, type Reach } from './build';
import { cellKey, OPPOSITE, STEP, type Layout, layOut, layOutLevels, type Levels, type Spot } from './layout';
import { biomeOf, exitLook, iconPaths, palette, tintOf, type Biome, type ColourBy, type ExitLook, type IconName, type Palette, type Tint } from './look';
import type { MapExit, MapRoom } from './model';
import type { Wire, WireKind } from './wires';

/** A grid cell, in pixels at 100%. */
export const S = 28;
export const MIN_ZOOM = 0.12;
export const MAX_ZOOM = 4;
/** A room's half width, in cells. */
const HALF = 0.34;
/** Where a stub's chip sits, in cells from its room's middle. */
const CHIP = 0.46;
/** How far a chip moves to the side when a room is in the next cell, in cells. */
const ASIDE = 0.2;
/** Where a map change's portal sits when the next cell is free, in cells from its room's middle,
 * and the least zoom portals show at. */
const PORTAL = 0.62;
const PORTALS = 0.6;
/** The least zoom Build mode's free cells and join marks show at. */
const GHOSTS = 0.4;
/** What a fit leaves clear for the controls over the map, in pixels: the map's name and its colours along the top, the hint along the bottom. */
const TOP = 100;
const BOTTOM = 64;
const SIDE = 48;
const TAU = Math.PI * 2;

export interface Layers {
  /** Place names. */
  labels: boolean;
  /** Dots between the cells. */
  grid: boolean;
  /** Exit symbols and room marks. */
  symbols: boolean;
  /** The builder's notes (drawn by the World's layer). */
  notes: boolean;
}

export interface Camera {
  x: number;
  y: number;
  z: number;
}

/** An exit drawn as a line between neighbouring rooms (a two-way pair once). */
interface Edge {
  a: MapRoom;
  b: MapRoom;
  oneWay: boolean;
  look: ExitLook;
}

/** An exit whose room is not beside it. */
export interface Stub {
  room: MapRoom;
  exit: MapExit;
  /** Where it leads: `here` a room of this map drawn elsewhere, `map` another map, `missing` a room this map does not have. */
  sort: 'here' | 'map' | 'missing';
  label: string;
  look: ExitLook;
  /** In levels, a room on another level: how many levels up (or, below 0, down). */
  rise?: number;
}

export interface Hit {
  room: MapRoom | null;
  stub: Stub | null;
  /** A wire, or the chip at its end on another map; and how many wires share that chip. */
  wire?: Wire;
  wires?: number;
  /** Build mode: a way from the selected room to dig (`to` null), or to join to the room drawn there. */
  ghost?: Ghost;
  /** Something of the layer drawn with the map (a note): its id. */
  note?: string;
}

/** Build mode: a way from the selected room with no exit, into a free cell or a room drawn there. */
export interface Ghost {
  dir: number;
  /** The cell. */
  x: number;
  y: number;
  /** The room drawn there, or null: the cell is free. */
  to: number | null;
}

export interface LegendEntry extends Tint {
  count: number;
}

export interface SceneEvents {
  /** A room clicked, or empty space (null). */
  pick(room: MapRoom | null): void;
  /** A stub or an up or down mark clicked. */
  follow(stub: Stub): void;
  /** A wire clicked. */
  wire?(w: Wire): void;
  /** The wheel (or the zoom out key) went on zooming out with all the map small in view: all
   * the maps' turn. */
  out?(): void;
  /** What the pointer is over, for the tooltip (null: nothing), at the pointer's place on the canvas. */
  hover(hit: Hit | null, x: number, y: number): void;
  /** The camera moved. */
  moved(cam: Camera): void;
  /** Build mode: a free cell or a join mark clicked. */
  build(ghost: Ghost): void;
  /** Build mode: a room carried to a free cell. */
  move(room: number, x: number, y: number): void;
  /** Build mode: a corridor or a join reached out of the selected room, to make. */
  reach(r: Reach): void;
  /** Build mode: what a reach being dragged out (or a Shift line) would make, as it changes (null: none). */
  reaching(r: Reach | null): void;
  /** In levels, the level shown changed, or the levels the map has. */
  levelled?(): void;
  /** Something of the layer drawn with the map (a note) clicked: its id. */
  note?(id: string): void;
}

/** How a tool draws on the map: the canvas, its colours, and how big things are at this zoom. */
export interface Pen {
  ctx: CanvasRenderingContext2D;
  pal: Palette;
  /** The canvas's size, in CSS pixels. */
  W: number;
  H: number;
  z: number;
  /** A cell's width, a room's half width and its corners' radius, and an exit line's width. */
  cell: number;
  half: number;
  rad: number;
  lw: number;
  /** Whether a room is in view (or nearly). */
  inView(n: number): boolean;
  /** What is drawn (in levels, the level shown's): a new one when rooms move. */
  layout: Layout;
  /** A room's cell, if it is drawn (in levels, on the level shown). */
  at(n: number): Spot | undefined;
  /** A cell's middle on the canvas. */
  sx(gx: number): number;
  sy(gy: number): number;
}

/** A box on the canvas: x, y, width and height. */
export type Box = [number, number, number, number];

/** How a tool draws on the overview: a cell's size `s`, in pixels, and where cell (0, 0) is. */
export interface MiniPen {
  ctx: CanvasRenderingContext2D;
  pal: Palette;
  s: number;
  ox: number;
  oy: number;
  dpr: number;
  /** A room's cell, if it is drawn (in levels, on the level shown). */
  at(n: number): Spot | undefined;
}

/**
 * Drawn with the map whatever tool is on (the builder's notes): under the exits and the rooms,
 * over them, and on the overview. Unlike a tool's marks, what it draws over the rooms can be
 * pointed at and clicked, with no tool on.
 */
export interface MapLayer {
  /** Draws under the exits and the rooms. */
  back?(p: Pen): void;
  /** Draws over the rooms, keeping clear of the boxes `taken` (the selected room's name) and
   * adding its own, which the place names then keep clear of. */
  front?(p: Pen, taken: Box[]): void;
  mini?(p: MiniPen): void;
  /** What of it is at (x, y) on the canvas, as it was last drawn: an id (null: nothing). */
  hit?(x: number, y: number): string | null;
}

/**
 * A tool on the map (a stamp held, the Select tool). While one is on it has a press and the
 * pointer's moves first, a click is its own (never a room's or a stub's), Build mode's marks
 * hide and a double click does nothing; and it draws over the map. Space held down, and the
 * middle button, still pan the map.
 */
export interface MapTool {
  /** Names the cursor: the canvas's `data-tool`. */
  readonly cursor: string;
  /** A press with the main button or a finger. Answers whether the tool has it; if not, a drag pans the map. */
  down?(x: number, y: number, e: PointerEvent): boolean;
  /** The pointer moved, in a press the tool has (`held`) or with none. Answers whether the tool used it: then nothing is hovered. */
  move?(x: number, y: number, e: PointerEvent, held: boolean): boolean;
  /** The press the tool has ended: let go of, or cancelled (a second finger, or the browser). */
  up?(x: number, y: number, e: PointerEvent, cancelled: boolean): void;
  /** A press the tool did not have ended without moving the map. */
  click?(x: number, y: number, e: PointerEvent): void;
  /** The pointer left the map. */
  leave?(): void;
  /** Draws over the rooms and the exits, under the marks on them. */
  under?(p: Pen): void;
  /** Draws over everything on the map. */
  over?(p: Pen): void;
  /** Draws over the overview. */
  mini?(p: MiniPen): void;
}

/** Where a wire is drawn on the canvas: from (ax, ay) to (bx, by), bowed through (cx, cy), its
 * line from `t0` to `t1` of the way (clear of what is at each end); and the chip of an end on
 * another map (or a room the map lacks), beside the room at the other end. */
interface WirePath {
  w: Wire;
  /** How many wires it stands for: a room's of one kind to (or from) one other map, or to rooms the map lacks, share one chip. */
  n: number;
  ax: number;
  ay: number;
  bx: number;
  by: number;
  cx: number;
  cy: number;
  t0: number;
  t1: number;
  chip: { x: number; y: number; r: number; label: string; missing: boolean } | null;
}

/** The ways a wire's chip may sit beside its room, in turn: SE, SW, NE, NW, E, S, W, N. */
const CHIP_WAYS = [6, 7, 4, 5, 2, 1, 3, 0];
/** A fan of wires (of one kind, out of a room or into one) wider than this is drawn lighter, and without halos. */
const SCATTER = 12;
/** The symbol on each kind of wire. */
const WIRE_ICON: Record<WireKind, IconName> = { teleport: 'wand', spell: 'sparkles', remote: 'radio', death: 'skull', hangup: 'log-out', control: 'crosshair' };
/** An arrowhead at (x, y) pointing at angle `a`, `s` long, added to `p`. */
const arrowTo = (p: Path2D, x: number, y: number, a: number, s: number) => {
  p.moveTo(x + Math.cos(a) * s, y + Math.sin(a) * s);
  p.lineTo(x + Math.cos(a + 2.5) * s, y + Math.sin(a + 2.5) * s);
  p.lineTo(x + Math.cos(a - 2.5) * s, y + Math.sin(a - 2.5) * s);
  p.closePath();
};
/** A point `t` of the way along the curve from (ax, ay) through (cx, cy) to (bx, by). */
const along = (p: WirePath, t: number): [number, number] => {
  const u = 1 - t;
  return [u * u * p.ax + 2 * u * t * p.cx + t * t * p.bx, u * u * p.ay + 2 * u * t * p.cy + t * t * p.by];
};

/** A room being carried, in Build mode: where it is held, and the cell under it. */
interface Carry {
  room: number;
  /** The pointer, on the canvas. */
  px: number;
  py: number;
  /** The cell under the pointer, and whether the room can be put down there. */
  x: number;
  y: number;
  free: boolean;
}

/** A press on the map: where it began, the camera then, whether it has moved the map, the room it may carry or the handle it may drag out (Build mode), and the tool that has it. */
interface Press {
  x: number;
  y: number;
  cx: number;
  cy: number;
  moved: boolean;
  room: number | null;
  ghost: Ghost | null;
  touch: boolean;
  at: number;
  tool: MapTool | null;
}

const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
/** A map change beside its room (up and down ones are marks on the room). */
const portal = (s: Stub) => s.sort === 'map' && s.exit.dir < 8;
const ease = (t: number) => 1 - Math.pow(1 - t, 3);

export class Scene {
  readonly cam: Camera = { x: 0, y: 0, z: 1 };
  rooms: MapRoom[] = [];
  by = new Map<number, MapRoom>();
  /** What is drawn: the map's layout, or in levels the level shown's. */
  layout: Layout | null = null;
  /** The map drawn in levels (null: flat): each level's layout, and where each room is. */
  stack: Levels | null = null;
  /** In levels, the level shown: 0 is the start room's. */
  level = 0;
  /** The start of the layout: room 1, or the lowest room. */
  start = 1;
  selected: number | null = null;

  private colourBy: ColourBy = 'place';
  private edited = new Set<number>();
  /** Legend keys picked out: the other rooms fade. Empty: none fade. */
  private spot = new Set<string>();
  private layers: Layers = { labels: true, grid: true, symbols: true, notes: true };
  /** How much of the canvas's right side the room card covers. */
  private cover = 0;
  /** How much of its top the tools cover and of its bottom a panel (the walk's): rooms are
   * brought into view between them. */
  private over = 0;
  private under = 0;
  /** How far the wheel has gone on zooming out past the whole map, and when it last did. */
  private beyond = 0;
  private beyondAt = 0;
  /** Build mode: the ways the selected room can be dug or joined are drawn. */
  private building = false;
  /** The room being named: its name pill is left to the field over it. */
  private naming: number | null = null;
  /** Where rooms moved by hand are drawn (with their levels, in levels). */
  private pins: ReadonlyMap<number, Spot> = new Map();
  /** Whether the map is drawn in levels. */
  private levelsOn = false;
  /** The room being carried to another cell. */
  private carry: Carry | null = null;
  /** The way of a free cell or join mark being dragged out of the selected room, and what it would make. */
  private reaching: number | null = null;
  private reach: Reach | null = null;
  /** Shift held down, and the line from the selected room to the cell under the pointer it would dig. */
  private shift = false;
  private shiftReach: Reach | null = null;
  /** Where the pointer is over the canvas (null: off it). */
  private pointer: [number, number] | null = null;
  /** The tool on the map, if one is on. */
  private tool: MapTool | null = null;
  /** The layer drawn with the map (the builder's notes), if there is one. */
  private layer: MapLayer | null = null;
  /** The wires drawn, the map they are drawn on, and whether they are every room's (fainter). */
  private wires: Wire[] = [];
  private wireMap = 0;
  private wiresAll = false;
  /** The wires' paths as worked out for these wires, rooms and zoom, and where cell (0, 0) was then: panning only moves them. */
  private wireCache: { wires: Wire[]; layout: Layout; z: number; ox: number; oy: number; paths: WirePath[] } | null = null;
  /** Space held down: a drag pans the map, even with a tool on. */
  private panKey = false;

  private readonly ctx: CanvasRenderingContext2D;
  private readonly mctx: CanvasRenderingContext2D;
  private W = 0;
  private H = 0;
  private dpr = 1;
  private pal: Palette = palette();
  private edges: Edge[] = [];
  private stubs: Stub[] = [];
  private stubsOf = new Map<number, Stub[]>();
  private biomes = new Map<number, Biome>();
  private tints = new Map<number, Tint>();
  /** What the pointer is over. */
  hovered: Hit | null = null;
  private raf = 0;
  private flight = 0;
  /** Where the camera is flying to, while it flies. */
  private goal: Camera | null = null;
  private miniCache: HTMLCanvasElement | null = null;
  private miniAt = { s: 1, ox: 0, oy: 0 };
  private readonly gone: (() => void)[] = [];

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly mini: HTMLCanvasElement,
    private readonly on: SceneEvents,
  ) {
    this.ctx = canvas.getContext('2d')!;
    this.mctx = mini.getContext('2d')!;
    const ro = new ResizeObserver(() => this.resize());
    ro.observe(canvas);
    ro.observe(mini);
    this.gone.push(() => ro.disconnect());
    this.listen();
    this.resize();
  }

  destroy(): void {
    cancelAnimationFrame(this.raf);
    this.land();
    for (const f of this.gone) f();
  }

  // ---- what is drawn ---------------------------------------------------------------

  /**
   * A map's rooms, where those moved by hand are drawn and whether it is drawn in levels (each
   * kept when not given). The camera stays where it is, and in levels so does the level shown,
   * unless `fresh` (another map): then it is the start room's.
   */
  setRooms(rooms: MapRoom[], pins?: ReadonlyMap<number, Spot>, levels?: boolean, fresh = false): void {
    this.rooms = rooms;
    this.by = new Map(rooms.map((r) => [r.room, r]));
    this.start = this.by.has(1) ? 1 : (rooms[0]?.room ?? 1);
    if (pins) this.pins = pins;
    if (levels !== undefined) this.levelsOn = levels;
    if (fresh) this.level = 0;
    this.biomes = new Map(rooms.map((r) => [r.room, biomeOf(r)]));
    if (this.selected !== null && !this.by.has(this.selected)) this.selected = null;
    this.relayout();
    this.retint();
  }

  /** Where rooms moved by hand are drawn, and whether the map is drawn in levels (kept when not given). */
  setPins(pins: ReadonlyMap<number, Spot>, levels?: boolean): void {
    this.pins = pins;
    if (levels !== undefined) this.levelsOn = levels;
    this.relayout();
    this.miniCache = null;
    this.draw();
  }

  private relayout() {
    if (this.levelsOn && this.rooms.length) {
      const st = layOutLevels(this.rooms, this.start, this.pins);
      this.stack = st;
      // the level shown stays, unless no room is on it now: then the selected room's, or the nearest
      if (!st.of.has(this.level)) {
        const sel = this.selected === null ? undefined : st.at.get(this.selected)?.z;
        this.level = sel ?? [...st.of.keys()].reduce((a, b) => (Math.abs(b - this.level) < Math.abs(a - this.level) ? b : a));
      }
      this.layout = st.of.get(this.level)!;
    } else {
      this.stack = null;
      this.level = 0;
      this.layout = this.rooms.length ? layOut(this.rooms, this.start, this.pins) : null;
    }
    this.hovered = null;
    this.carry = null;
    this.canvas.classList.remove('carrying');
    this.sortExits();
    // a reach being dragged out goes on from the rooms as they are now
    if (this.reaching !== null && this.pointer) this.reachTo(...this.pointer);
    this.shiftLine();
    this.on.levelled?.();
  }

  /** In levels, the level a room is on (undefined: drawn flat, or no such room). */
  levelOf(n: number): number | undefined {
    return this.stack?.at.get(n)?.z;
  }

  /** In levels, shows level `z`, if the map has rooms on it. The camera stays where it is, as the
   * rooms up and down lead to are in the same place; with `fit`, unless none of the level's rooms
   * are in view: then all of them are. Answers whether it is shown now. */
  setLevel(z: number, fit = false): boolean {
    const l = this.stack?.of.get(z);
    if (!l) return false;
    if (z === this.level) return true;
    this.level = z;
    this.layout = l;
    this.hovered = null;
    this.carry = null;
    this.canvas.classList.remove('carrying');
    this.dropReach();
    this.sortExits();
    this.shiftLine();
    this.miniCache = null;
    if (fit && !this.someInView()) this.fit();
    this.draw();
    this.on.levelled?.();
    return true;
  }

  /** Whether any room drawn is in view, beside the card. */
  private someInView(): boolean {
    const [x0, y0] = this.toWorld(0, 0);
    const [x1, y1] = this.toWorld(this.W - this.cover, this.H);
    for (const p of this.layout?.at.values() ?? []) {
      const [x, y] = [(p.x + 0.5) * S, (p.y + 0.5) * S];
      if (x >= x0 && x <= x1 && y >= y0 && y <= y1) return true;
    }
    return false;
  }

  /** Every room's place, as pins keep it: with its level, in levels. */
  places(): Map<number, Spot> {
    if (this.stack) return new Map([...this.stack.at].map(([n, s]) => [n, { ...s }]));
    return new Map([...(this.layout?.at ?? [])].map(([n, s]) => [n, { ...s }]));
  }

  /** A cell of the level shown, as a pin keeps it. */
  pinAt(x: number, y: number): Spot {
    return this.stack ? { x, y, z: this.level } : { x, y };
  }

  /** Whether a room is being carried. */
  get carrying(): boolean {
    return !!this.carry;
  }

  /** Puts the room being carried back where it was. */
  dropCarry(): void {
    if (!this.carry) return;
    this.carry = null;
    this.canvas.classList.remove('carrying');
    this.draw();
  }

  /** Whether a free cell or a join mark is being dragged out. */
  get reachingOut(): boolean {
    return this.reaching !== null;
  }

  /** Lets go of a free cell or join mark being dragged out, making nothing. */
  dropReach(): void {
    if (this.reaching === null) return;
    this.reaching = null;
    this.canvas.classList.remove('reaching');
    this.setReach(null);
  }

  /** Shift held down (or let go): in Build mode a click digs a line to the cell under the pointer. */
  setShift(down: boolean): void {
    if (down === this.shift) return;
    this.shift = down;
    this.shiftLine();
  }

  setColour(by: ColourBy): void {
    this.colourBy = by;
    this.spot.clear();
    this.retint();
  }

  setEdited(rooms: Set<number>): void {
    this.edited = rooms;
    if (this.colourBy === 'edits') this.retint();
  }

  setSpot(keys: Iterable<string>): void {
    this.spot = new Set(keys);
    this.miniCache = null;
    this.draw();
  }

  setLayers(l: Layers): void {
    this.layers = { ...l };
    this.draw();
  }

  /** The wires to draw on map `map` (none: the layer is off), and whether they are every room's. */
  setWires(wires: Wire[], map: number, all: boolean): void {
    this.wires = wires;
    this.wireMap = map;
    this.wiresAll = all;
    if (this.hovered?.wire && !wires.includes(this.hovered.wire)) this.hovered = null;
    this.draw();
  }

  /** The room card is open over the right side (or not). Returns how much of the canvas it covers. */
  setCover(open: boolean): number {
    const wide = !matchMedia('(max-width: 760px)').matches;
    this.cover = open && wide ? Math.min(420, this.W - 24) + 12 : 0;
    return this.cover;
  }

  /** A panel is open along the bottom, `bottom` high, under tools `top` high (0 for none): a room
   * brought into view goes between them. */
  setBand(top: number, bottom: number): void {
    this.under = Math.max(0, Math.min(bottom, this.H - 120));
    this.over = bottom ? Math.max(0, Math.min(top, this.H - this.under - 80)) : 0;
  }

  /** The theme changed. */
  retheme(): void {
    this.pal = palette();
    this.retint();
  }

  select(n: number | null): void {
    this.selected = n !== null && this.by.has(n) ? n : null;
    // in levels, the room's level
    const z = this.selected === null ? undefined : this.levelOf(this.selected);
    if (z !== undefined) this.setLevel(z);
    if (this.reaching !== null && this.selected === null) this.dropReach();
    this.shiftLine();
    this.draw();
  }

  /** Build mode on or off. */
  setBuild(on: boolean): void {
    this.building = on;
    if (!on && this.hovered?.ghost) this.hovered = null;
    if (!on) this.dropReach();
    this.shiftLine();
    this.draw();
  }

  /** A tool on the map (or none): it has the pointer first, and draws over the map. */
  setTool(t: MapTool | null): void {
    this.tool = t;
    if (t) {
      this.carry = null;
      this.canvas.classList.remove('carrying');
      if (this.hovered) this.setHover(null, 0, 0);
    }
    this.canvas.dataset.tool = t?.cursor ?? '';
    this.draw();
  }

  /** The layer drawn with the map (or none). */
  setLayer(l: MapLayer | null): void {
    this.layer = l;
    if (this.hovered?.note) this.hovered = null;
    this.draw();
  }

  /** Space held down (or let go): a drag pans the map. */
  setPanKey(down: boolean): void {
    this.panKey = down;
    this.canvas.classList.toggle('pan-key', down);
  }

  /** The key of the colour a room is drawn in: rooms that share it share a place, an area, monsters or light. */
  tintKey(n: number): string | undefined {
    return this.tints.get(n)?.key;
  }

  /** A cell's width on the canvas, at this zoom. */
  get cell(): number {
    return S * this.cam.z;
  }

  /** A point of the canvas, in cells: (2.5, 3) is halfway across cell (2, 3)'s top edge. */
  inCells(px: number, py: number): [number, number] {
    const [wx, wy] = this.toWorld(px, py);
    return [wx / S, wy / S];
  }

  /** A point given in cells, on the canvas. */
  onCanvas(cx: number, cy: number): [number, number] {
    return [(cx * S - this.cam.x) * this.cam.z + this.W / 2, (cy * S - this.cam.y) * this.cam.z + this.H / 2];
  }

  /** The cell under a point of the canvas. */
  cellAt(px: number, py: number): [number, number] {
    const [wx, wy] = this.toWorld(px, py);
    return [Math.floor(wx / S), Math.floor(wy / S)];
  }

  /** Brings a cell into view if it is near the edge or out of sight. */
  showCell(x: number, y: number): void {
    const px = this.sx(x);
    const py = this.sy(y);
    const w = this.W - this.cover;
    const mx = Math.min(140, w * 0.22);
    const my = Math.min(120, this.H * 0.22);
    if (px < mx || px > w - mx || py < my || py > this.H - my) this.fly((x + 0.5) * S + this.cover / 2 / this.cam.z, (y + 0.5) * S, this.cam.z);
    else this.draw();
  }

  /** Flies to zoom `z` with a cell at a point of the canvas: by default the middle of what the card leaves showing. */
  lookAt(x: number, y: number, z: number, at?: [number, number]): void {
    const [px, py] = at ?? [(this.W - this.cover) / 2, this.H / 2];
    this.fly((x + 0.5) * S - (px - this.W / 2) / z, (y + 0.5) * S - (py - this.H / 2) / z, z);
  }

  /** The room being named (or none), whose pill the field takes the place of. */
  setNaming(n: number | null): void {
    this.naming = n;
    this.draw();
  }

  /** Where a room is on the canvas: its middle, and its half width. */
  roomOnScreen(n: number): { x: number; y: number; half: number } | null {
    const p = this.layout?.at.get(n);
    if (!p) return null;
    return { x: this.sx(p.x), y: this.sy(p.y), half: HALF * S * this.cam.z };
  }

  /** Build mode: the ways from the selected room with no exit, each into a free cell or a room drawn there. */
  ghosts(): Ghost[] {
    const l = this.layout;
    if (!this.building || this.selected === null || !l) return [];
    const r = this.by.get(this.selected);
    const p = l.at.get(this.selected);
    if (!r || !p) return [];
    const out: Ghost[] = [];
    for (let d = 0; d < 8; d++) {
      if (r.exits.some((e) => e.dir === d)) continue;
      const [dx, dy] = STEP[d]!;
      const to = l.cell.get(cellKey(p.x + dx, p.y + dy));
      out.push({ dir: d, x: p.x + dx, y: p.y + dy, to: to ?? null });
    }
    return out;
  }

  /** Where a join mark is drawn: between the selected room and the room it would join. */
  private joinSpot(g: Ghost): { x: number; y: number; r: number } {
    const [dx, dy] = STEP[g.dir]!;
    const x = this.sx(g.x - dx / 2);
    const y = this.sy(g.y - dy / 2);
    return { x, y, r: Math.max(6, 0.17 * S * this.cam.z) };
  }

  /** The legend of the colouring: each colour on this map and how many rooms have it, most first. */
  legend(): LegendEntry[] {
    const out = new Map<string, LegendEntry>();
    for (const t of this.tints.values()) {
      const e = out.get(t.key);
      if (e) e.count++;
      else out.set(t.key, { ...t, count: 1 });
    }
    return [...out.values()].sort((a, b) => b.count - a.count || a.label.localeCompare(b.label));
  }

  /** A room's stubs: its exits that are not lines to a neighbour. */
  stubsFrom(n: number): Stub[] {
    return this.stubsOf.get(n) ?? [];
  }

  /** Whether a room was put away from where its exits wanted it, its cell being taken. */
  displaced(n: number): boolean {
    return !!this.layout?.displaced.has(n);
  }

  private sortExits() {
    this.edges = [];
    this.stubs = [];
    this.stubsOf = new Map();
    const at = this.layout?.at;
    if (!at) return;
    const stub = (room: MapRoom, exit: MapExit, sort: Stub['sort'], label: string, rise?: number) => {
      const s: Stub = { room, exit, sort, label, look: exitLook(exit), rise };
      this.stubs.push(s);
      let list = this.stubsOf.get(room.room);
      if (!list) this.stubsOf.set(room.room, (list = []));
      list.push(s);
    };
    for (const r of this.rooms) {
      // in levels, the level shown's rooms
      const p = at.get(r.room);
      if (!p) continue;
      for (const e of r.exits) {
        if (e.dir > 9) continue;
        if (e.kind === 8) {
          stub(r, e, 'map', `M${e.para1}`);
          continue;
        }
        const t = this.by.get(e.to);
        if (!t) {
          stub(r, e, 'missing', String(e.to));
          continue;
        }
        const q = at.get(t.room);
        const step = STEP[e.dir];
        // a remote action is no way through, whatever room it works
        if (!q || !step || q.x !== p.x + step[0] || q.y !== p.y + step[1] || e.kind === 12) {
          const rise = q ? undefined : (this.levelOf(t.room) ?? this.level) - this.level;
          stub(r, e, 'here', rise && e.dir < 8 ? `${rise > 0 ? '▲' : '▼'}${e.to}` : String(e.to), rise || undefined);
          continue;
        }
        const back = t.exits.find((b) => b.to === r.room && b.dir === OPPOSITE[e.dir] && b.kind !== 8);
        const look = exitLook(e);
        if (!back) this.edges.push({ a: r, b: t, oneWay: true, look });
        else if (r.room < t.room) this.edges.push({ a: r, b: t, oneWay: false, look: look.sort ? look : exitLook(back) });
      }
    }
  }

  private retint() {
    this.tints = new Map();
    for (const r of this.rooms) this.tints.set(r.room, this.tintFor(r));
    this.miniCache = null;
    this.draw();
  }

  /** The colour a room is drawn in, as the map is coloured now (a room as an edit would leave it, too). */
  tintFor(r: MapRoom): Tint {
    return tintOf(r, this.colourBy, this.pal, this.biomes.get(r.room) ?? 'other', this.edited.has(r.room));
  }

  private faded(n: number): boolean {
    return this.spot.size > 0 && !this.spot.has(this.tints.get(n)?.key ?? '');
  }

  // ---- the camera ------------------------------------------------------------------

  /** A cell's middle on the canvas. */
  sx(gx: number): number {
    return ((gx + 0.5) * S - this.cam.x) * this.cam.z + this.W / 2;
  }

  sy(gy: number): number {
    return ((gy + 0.5) * S - this.cam.y) * this.cam.z + this.H / 2;
  }

  private toWorld(px: number, py: number): [number, number] {
    return [(px - this.W / 2) / this.cam.z + this.cam.x, (py - this.H / 2) / this.cam.z + this.cam.y];
  }

  /** Moves the camera to (x, y, z), smoothly unless asked not to or the viewer prefers less motion. */
  private fly(x: number, y: number, z: number, animate = true) {
    this.land();
    z = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, z));
    const from = { ...this.cam };
    const far = Math.hypot((x - from.x) * from.z, (y - from.y) * from.z) > Math.max(this.W, this.H) * 3;
    if (!animate || reduced() || far) {
      Object.assign(this.cam, { x, y, z });
      this.moved();
      return;
    }
    this.goal = { x, y, z };
    const t0 = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - t0) / 360);
      const e = ease(t);
      // zoom moves in proportion, so a zoom in feels as quick as a zoom out
      this.cam.z = from.z * Math.pow(z / from.z, e);
      this.cam.x = from.x + (x - from.x) * e;
      this.cam.y = from.y + (y - from.y) * e;
      this.moved();
      if (t < 1) this.flight = requestAnimationFrame(step);
      else this.goal = null;
    };
    this.flight = requestAnimationFrame(step);
  }

  /** Stops a flight where it is. */
  private land() {
    cancelAnimationFrame(this.flight);
    this.goal = null;
  }

  private moved() {
    this.on.moved({ ...this.cam });
    this.draw();
  }

  /** The camera to put a room in the middle of what the card (and a panel along the bottom) leaves showing. */
  private aim(n: number, z: number): [number, number] | null {
    const p = this.layout?.at.get(n);
    if (!p) return null;
    return [(p.x + 0.5) * S + this.cover / 2 / z, (p.y + 0.5) * S + (this.under - this.over) / 2 / z];
  }

  centreOn(n: number, opts: { zoom?: number; animate?: boolean } = {}): void {
    const z = opts.zoom ?? this.cam.z;
    const a = this.aim(n, z);
    if (a) this.fly(a[0], a[1], z, opts.animate);
  }

  /** Brings a room into view if it is near the edge or out of sight, as walking does. */
  keepInView(n: number): void {
    const p = this.layout?.at.get(n);
    if (!p) return;
    const x = this.sx(p.x);
    const y = this.sy(p.y);
    const w = this.W - this.cover;
    const [top, bottom] = [this.over, this.H - this.under];
    const mx = Math.min(140, w * 0.22);
    const my = Math.min(120, (bottom - top) * 0.22);
    if (x < mx || x > w - mx || y < top + my || y > bottom - my) this.centreOn(n);
    else this.draw();
  }

  zoomBy(f: number, at?: [number, number]): void {
    const [px, py] = at ?? [(this.W - this.cover) / 2, this.H / 2];
    // a zoom while one is under way goes on from where that one was going
    const base = this.goal ?? this.cam;
    // zoomed out past the whole map already: all the maps
    if (f < 1 && this.on.out && !this.tool && base.z <= this.outZoom() * 1.001) {
      this.on.out();
      return;
    }
    const z = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, base.z * f));
    // the point under (px, py) stays there
    const bx = (px - this.W / 2) / base.z + base.x;
    const by = (py - this.H / 2) / base.z + base.y;
    this.fly(bx - (px - this.W / 2) / z, by - (py - this.H / 2) / z, z, !at);
  }

  private zoomNow(z: number, px: number, py: number) {
    z = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, z));
    const [bx, by] = this.toWorld(px, py);
    this.cam.z = z;
    this.cam.x = bx - (px - this.W / 2) / z;
    this.cam.y = by - (py - this.H / 2) / z;
    this.moved();
  }

  /** Shows a box of cells, as large as fits between the controls along the top and the bottom. */
  private frame(x0: number, y0: number, x1: number, y1: number, most: number, animate: boolean) {
    const w = Math.max(1, this.W - this.cover - SIDE * 2);
    const h = Math.max(1, this.H - TOP - BOTTOM);
    const z = Math.max(MIN_ZOOM, Math.min(most, w / ((x1 - x0 + 1) * S), h / ((y1 - y0 + 1) * S)));
    // the box's middle goes in the middle of the space the controls leave
    this.fly(((x0 + x1 + 1) / 2) * S + this.cover / 2 / z, ((y0 + y1 + 1) / 2) * S - (TOP - BOTTOM) / 2 / z, z, animate);
  }

  /** Every room in view. */
  fit(animate = true): void {
    const b = this.layout?.bounds;
    if (b) this.frame(b.x0, b.y0, b.x1, b.y1, 1.6, animate);
  }

  /** The zoom below which all the map is small in view: zooming out on from there asks for all
   * the maps. */
  private outZoom(): number {
    const b = this.layout?.bounds;
    if (!b) return MIN_ZOOM;
    const w = Math.max(1, this.W - this.cover - SIDE * 2);
    const h = Math.max(1, this.H - TOP - BOTTOM);
    return Math.max(MIN_ZOOM, Math.min(w / ((b.x1 - b.x0 + 1) * S), h / ((b.y1 - b.y0 + 1) * S)) * 0.6);
  }

  /** Counts a wheel's zoom out past the whole map; once it has gone far enough, all the maps'
   * turn (true: the wheel's zoom is spent). */
  private pastWhole(f: number): boolean {
    if (!this.on.out || this.tool || this.carry || f >= 1) {
      this.beyond = 0;
      return false;
    }
    const now = performance.now();
    if (now - this.beyondAt > 600) this.beyond = 0;
    this.beyondAt = now;
    if (this.cam.z > this.outZoom() * 1.001) {
      this.beyond = 0;
      return false;
    }
    this.beyond -= Math.log(f);
    if (this.beyond < 0.5) return false;
    this.beyond = 0;
    this.on.out();
    return true;
  }

  /** Every room in view again, if the camera was zoomed out past them (back from all the maps). */
  fitIfPast(animate = true): void {
    if (this.cam.z < this.outZoom() * 1.2) this.fit(animate);
  }

  /** The rooms joined to the start room, in view (the islands below may be cut off). */
  fitMain(animate = false): void {
    const l = this.layout;
    if (!l) return;
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    for (const [n, p] of l.at) {
      if (l.island.get(n) !== 0) continue;
      x0 = Math.min(x0, p.x);
      y0 = Math.min(y0, p.y);
      x1 = Math.max(x1, p.x);
      y1 = Math.max(y1, p.y);
    }
    if (x0 <= x1) this.frame(x0, y0, x1, y1, 1.4, animate);
  }

  /** Rooms in view, as large as fits (not closer than a room and its neighbours close up). */
  frameRooms(rooms: Iterable<number>): void {
    const l = this.layout;
    if (!l) return;
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    for (const n of rooms) {
      const p = l.at.get(n);
      if (!p) continue;
      [x0, y0, x1, y1] = [Math.min(x0, p.x), Math.min(y0, p.y), Math.max(x1, p.x), Math.max(y1, p.y)];
    }
    if (x0 <= x1) this.frame(x0, y0, x1, y1, 1.8, true);
  }

  /** A room and its neighbours, close up. */
  frameRoom(n: number): void {
    this.centreOn(n, { zoom: Math.max(this.cam.z, 1.8) });
  }

  setCamera(c: Camera): void {
    this.land();
    Object.assign(this.cam, c);
    this.moved();
  }

  // ---- input -----------------------------------------------------------------------

  /** What is under a point of the canvas. */
  hitAt(px: number, py: number): Hit | null {
    const l = this.layout;
    if (!l) return null;
    const z = this.cam.z;
    // the layer drawn with the map: its marks sit on and beside rooms
    const note = this.tool ? null : (this.layer?.hit?.(px, py) ?? null);
    if (note !== null) return { room: null, stub: null, note };
    // Build mode: the join marks, then the free cells around the selected room
    if (this.building && !this.tool && z >= GHOSTS) {
      const ghosts = this.ghosts();
      for (const g of ghosts) {
        if (g.to === null) continue;
        const c = this.joinSpot(g);
        if (Math.hypot(c.x - px, c.y - py) <= c.r + 4) return { room: this.by.get(g.to) ?? null, stub: null, ghost: g };
      }
      const [wx, wy] = this.toWorld(px, py);
      const [gx, gy] = [Math.floor(wx / S), Math.floor(wy / S)];
      const g = ghosts.find((h) => h.to === null && h.x === gx && h.y === gy);
      if (g) return { room: null, stub: null, ghost: g };
    }
    // chips and marks first: they sit on and beside rooms (portals show sooner)
    if (this.layers.symbols && z >= PORTALS) {
      const [wx, wy] = this.toWorld(px, py);
      const gx = Math.floor(wx / S);
      const gy = Math.floor(wy / S);
      let best: Stub | null = null;
      let bestD = Infinity;
      for (let x = gx - 1; x <= gx + 1; x++) {
        for (let y = gy - 1; y <= gy + 1; y++) {
          const n = l.cell.get(cellKey(x, y));
          if (n === undefined) continue;
          for (const s of this.stubsOf.get(n) ?? []) {
            if (z < 0.9 && !portal(s)) continue;
            const c = this.stubSpot(s, x, y);
            const d = Math.hypot(c.x - px, c.y - py);
            if (d <= c.r + 4 && d < bestD) [best, bestD] = [s, d];
          }
        }
      }
      if (best) return { room: best.room, stub: best };
    }
    // the whole cell around a room counts, so small rooms are easy to hit; a wire over it only
    // off the room's own square
    const [wx, wy] = this.toWorld(px, py);
    const [gx, gy] = [Math.floor(wx / S), Math.floor(wy / S)];
    const n = l.cell.get(cellKey(gx, gy));
    if (this.wires.length && z >= 0.2) {
      const inside = n !== undefined && Math.abs(wx - (gx + 0.5) * S) <= HALF * S && Math.abs(wy - (gy + 0.5) * S) <= HALF * S;
      const w = this.wireAt(px, py, inside);
      if (w) return { room: null, stub: null, wire: w.w, wires: w.n };
    }
    return n === undefined ? null : { room: this.by.get(n)!, stub: null };
  }

  /** Where a stub's chip (or an up or down mark) is drawn, for a room at cell (gx, gy). */
  private stubSpot(s: Stub, gx: number, gy: number): { x: number; y: number; r: number } {
    const z = this.cam.z;
    const x = this.sx(gx);
    const y = this.sy(gy);
    const half = HALF * S * z;
    if (s.exit.dir > 7) {
      // up and down: triangles on the left edge of the room
      return { x: x - half - 1, y: y + (s.exit.dir === 8 ? -half * 0.4 : half * 0.4), r: Math.max(3, half * 0.38) };
    }
    const [dx, dy] = STEP[s.exit.dir]!;
    let [ox, oy] = [dx * CHIP, dy * CHIP];
    // a room in the next cell: the chip keeps to one side of the gap (its right), clear of that
    // room's own chip that way, and of any line from it
    if (this.layout?.cell.has(cellKey(gx + dx, gy + dy))) {
      const len = Math.hypot(dx, dy);
      ox -= (dy / len) * ASIDE;
      oy += (dx / len) * ASIDE;
    } else if (s.sort === 'map') {
      // the next cell free: a map change's portal, larger and further out
      return { x: x + dx * PORTAL * S * z, y: y + dy * PORTAL * S * z, r: Math.max(2.5, Math.min(13, 0.22 * S * z)) };
    }
    return { x: x + ox * S * z, y: y + oy * S * z, r: Math.max(2.5, 0.17 * S * z) };
  }

  private listen() {
    const cv = this.canvas;
    const pointers = new Map<number, [number, number]>();
    let drag: Press | null = null;
    /** Two fingers: how far apart they began, the zoom then, and the place on the map between them, which stays between them. */
    let pinch: { d: number; z: number; at: [number, number] } | null = null;
    const pos = (e: PointerEvent | MouseEvent | WheelEvent): [number, number] => {
      const b = cv.getBoundingClientRect();
      return [e.clientX - b.left, e.clientY - b.top];
    };
    const add = <K extends keyof HTMLElementEventMap>(el: HTMLElement, type: K, f: (e: HTMLElementEventMap[K]) => void, opts?: AddEventListenerOptions) => {
      el.addEventListener(type, f, opts);
      this.gone.push(() => el.removeEventListener(type, f, opts));
    };

    add(cv, 'pointerdown', (e) => {
      if (e.button !== 0 && e.pointerType === 'mouse' && e.button !== 1) return;
      cv.focus({ preventScroll: true });
      cv.setPointerCapture(e.pointerId);
      pointers.set(e.pointerId, pos(e));
      if (pointers.size === 2) {
        const [a, b] = [...pointers.values()] as [[number, number], [number, number]];
        pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]) || 1, z: this.cam.z, at: this.toWorld((a[0] + b[0]) / 2, (a[1] + b[1]) / 2) };
        // a second finger: the press a tool had is let go of, doing nothing
        drag?.tool?.up?.(...pos(e), e, true);
        drag = null;
        return;
      }
      const [x, y] = pos(e);
      this.land();
      const press: Press = { x, y, cx: this.cam.x, cy: this.cam.y, moved: false, room: null, ghost: null, touch: e.pointerType === 'touch', at: performance.now(), tool: null };
      drag = press;
      // a tool has the press first (with Space held down, or the middle button, a drag pans the map)
      const t = this.tool;
      if (t) {
        if (e.button === 0 && !this.panKey && t.down?.(x, y, e)) {
          press.tool = t;
          this.setHover(null, x, y);
          this.draw();
        }
        return;
      }
      // Build mode: a room pressed on is carried, not the map (a finger holds it a moment first)
      const hit = this.building ? this.hitAt(x, y) : null;
      press.room = hit?.room && !hit.stub && !hit.ghost && e.button === 0 ? hit.room.room : null;
      press.ghost = hit?.ghost && e.button === 0 ? hit.ghost : null;
    });
    add(cv, 'pointermove', (e) => {
      const [x, y] = pos(e);
      this.pointer = [x, y];
      if (pointers.has(e.pointerId)) pointers.set(e.pointerId, [x, y]);
      if (pinch && pointers.size === 2) {
        // the fingers zoom, and move the map with them
        const [a, b] = [...pointers.values()] as [[number, number], [number, number]];
        const z = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, (pinch.z * Math.hypot(a[0] - b[0], a[1] - b[1])) / pinch.d));
        this.cam.z = z;
        this.cam.x = pinch.at[0] - ((a[0] + b[0]) / 2 - this.W / 2) / z;
        this.cam.y = pinch.at[1] - ((a[1] + b[1]) / 2 - this.H / 2) / z;
        this.moved();
        return;
      }
      if (drag?.tool) {
        drag.tool.move?.(x, y, e, true);
        return;
      }
      if (!drag && this.tool?.move?.(x, y, e, false)) {
        // the tool shows where it is (the brush, a stamp held): nothing is hovered
        this.setHover(null, x, y);
        return;
      }
      if (this.carry) {
        this.carryTo(x, y);
        return;
      }
      if (this.reaching !== null) {
        this.reachTo(x, y);
        return;
      }
      if (drag) {
        const dx = x - drag.x;
        const dy = y - drag.y;
        if (!drag.moved && Math.abs(dx) + Math.abs(dy) > 4) {
          drag.moved = true;
          // a free cell or a join mark dragged out (a finger holds it a moment first): a corridor, or a join
          if (drag.ghost && this.selected !== null && (!drag.touch || performance.now() - drag.at > 350)) {
            this.reaching = drag.ghost.dir;
            cv.classList.add('reaching');
            this.setHover(null, x, y);
            this.reachTo(x, y);
            return;
          }
          if (drag.room !== null && (!drag.touch || performance.now() - drag.at > 350)) {
            this.carry = { room: drag.room, px: x, py: y, x: 0, y: 0, free: false };
            cv.classList.add('carrying');
            this.setHover(null, x, y);
            this.carryTo(x, y);
            return;
          }
          cv.classList.add('panning');
          this.setHover(null, x, y);
        }
        if (drag.moved) {
          this.cam.x = drag.cx - dx / this.cam.z;
          this.cam.y = drag.cy - dy / this.cam.z;
          this.moved();
        }
        return;
      }
      if (e.pointerType !== 'mouse') return;
      // Shift: the line a click would dig
      this.shift = e.shiftKey;
      if (this.shiftLine()) this.setHover(null, x, y);
      else this.setHover(this.hitAt(x, y), x, y);
    });
    const end = (e: PointerEvent, cancelled: boolean) => {
      pointers.delete(e.pointerId);
      if (pinch) {
        if (pointers.size < 2) pinch = null;
        return;
      }
      const d = drag;
      drag = null;
      cv.classList.remove('panning');
      if (d?.tool) {
        d.tool.up?.(...pos(e), e, cancelled);
        return;
      }
      if (this.reaching !== null) {
        const r = this.reach;
        this.dropReach();
        if (!cancelled && r) this.on.reach(r);
        return;
      }
      const c = this.carry;
      if (c) {
        this.carry = null;
        cv.classList.remove('carrying');
        this.draw();
        if (cancelled) return;
        const home = this.layout?.at.get(c.room);
        if (c.free) this.on.move(c.room, c.x, c.y);
        // put down where it was: a click that wandered
        else if (home && home.x === c.x && home.y === c.y) this.on.pick(this.by.get(c.room) ?? null);
        return;
      }
      if (!d || d.moved || cancelled) return;
      const [x, y] = pos(e);
      if (this.tool) {
        // a click is the tool's (with Space held down: nothing)
        if (!this.panKey && e.button === 0) this.tool.click?.(x, y, e);
        return;
      }
      // Shift: a line dug to the cell
      if (e.shiftKey && this.shiftReach) {
        this.on.reach(this.shiftReach);
        return;
      }
      const hit = this.hitAt(x, y);
      if (hit?.ghost) this.on.build(hit.ghost);
      else if (hit?.stub) this.on.follow(hit.stub);
      else if (hit?.wire) this.on.wire?.(hit.wire);
      else if (hit?.note !== undefined) this.on.note?.(hit.note);
      else this.on.pick(hit?.room ?? null);
    };
    add(cv, 'pointerup', (e) => end(e, false));
    add(cv, 'pointercancel', (e) => end(e, true));
    add(cv, 'pointerleave', (e) => {
      if (drag) return;
      this.pointer = null;
      this.shiftLine();
      this.setHover(null, ...pos(e));
      this.tool?.leave?.();
    });
    add(
      cv,
      'wheel',
      (e) => {
        e.preventDefault();
        const [x, y] = pos(e);
        const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? this.H : 1;
        if (!e.ctrlKey && (e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY) * 0.5)) {
          // a sideways scroll (a trackpad's two fingers, or Shift and the wheel) pans
          const dx = e.shiftKey && !e.deltaX ? e.deltaY : e.deltaX;
          const dy = e.shiftKey && !e.deltaX ? 0 : e.deltaY;
          this.cam.x += (dx * unit) / this.cam.z;
          this.cam.y += (dy * unit) / this.cam.z;
          this.moved();
          return;
        }
        // the wheel zooms where the pointer is; a pinch (Ctrl and the wheel) a little faster
        const f = Math.exp(-e.deltaY * unit * (e.ctrlKey ? 0.01 : 0.0018));
        if (this.pastWhole(f)) return;
        this.zoomNow(this.cam.z * f, x, y);
      },
      { passive: false },
    );
    add(cv, 'dblclick', (e) => {
      if (this.tool) return;
      const [x, y] = pos(e);
      const hit = this.hitAt(x, y);
      if (hit?.ghost) return;
      if (hit?.room && !hit.stub) this.frameRoom(hit.room.room);
      else if (!hit) this.zoomBy(1.8, [x, y]);
    });

    // the overview: click or drag to look somewhere else
    const look = (e: PointerEvent) => {
      const b = this.mini.getBoundingClientRect();
      const m = this.miniAt;
      const px = (e.clientX - b.left) * this.dpr;
      const py = (e.clientY - b.top) * this.dpr;
      this.cam.x = ((px - m.ox) / m.s) * S + this.cover / 2 / this.cam.z;
      this.cam.y = ((py - m.oy) / m.s) * S;
      this.moved();
    };
    add(this.mini, 'pointerdown', (e) => {
      this.land();
      this.mini.setPointerCapture(e.pointerId);
      look(e);
    });
    add(this.mini, 'pointermove', (e) => {
      if (e.buttons) look(e);
    });
  }

  private setHover(hit: Hit | null, x: number, y: number) {
    const was = this.hovered;
    this.hovered = hit;
    if (was?.room !== hit?.room || was?.stub !== hit?.stub || was?.wire !== hit?.wire || was?.ghost?.dir !== hit?.ghost?.dir || was?.note !== hit?.note) this.draw();
    // in Build mode a room can be picked up and carried
    this.canvas.style.cursor = !hit || this.tool ? '' : this.building && hit.room && !hit.stub && !hit.ghost ? 'grab' : 'pointer';
    this.on.hover(hit, x, y);
  }

  /**
   * A free cell or join mark dragged out to (px, py): a room let go over off its way is joined
   * that way; else a corridor as far along its way as the pointer has gone (none short of the
   * first cell).
   */
  private reachTo(px: number, py: number) {
    const l = this.layout;
    const from = this.selected;
    const dir = this.reaching;
    const p = from === null ? undefined : l?.at.get(from);
    if (!l || from === null || dir === null || !p) return this.dropReach();
    const [wx, wy] = this.toWorld(px, py);
    const [cx, cy] = [Math.floor(wx / S), Math.floor(wy / S)];
    const k = alongWay(dir, cx - p.x, cy - p.y);
    const [sx, sy] = STEP[dir] ?? [0, 0];
    const there = l.cell.get(cellKey(cx, cy));
    const onWay = cx === p.x + sx * k && cy === p.y + sy * k;
    this.setReach(there !== undefined && there !== from && !onWay ? reachJoin(this.by, from, dir, there) : k >= 1 ? reachLine(this.by, l, from, dir, k) : null);
  }

  private setReach(r: Reach | null) {
    const had = this.reach;
    this.reach = r;
    if (r || had) {
      this.draw();
      this.on.reaching(r);
    }
  }

  /** Shift held down in Build mode: the line from the selected room to the cell under the pointer, as a click would dig it. Answers whether there is one. */
  private shiftLine(): boolean {
    const l = this.layout;
    const from = this.selected;
    const p = from === null ? undefined : l?.at.get(from);
    let r: Reach | null = null;
    if (this.shift && this.building && !this.tool && !this.carry && this.reaching === null && this.cam.z >= GHOSTS && l && from !== null && p && this.pointer) {
      const [wx, wy] = this.toWorld(...this.pointer);
      const way = lineTo(Math.floor(wx / S) - p.x, Math.floor(wy / S) - p.y);
      if (way) r = reachLine(this.by, l, from, way.dir, way.count);
    }
    const had = this.shiftReach;
    this.shiftReach = r;
    if (r || had) {
      this.draw();
      if (this.reaching === null) this.on.reaching(r);
    }
    return !!r;
  }

  /** The room being carried is held over (px, py): the cell under it, and whether it is free. */
  private carryTo(px: number, py: number) {
    const c = this.carry;
    const l = this.layout;
    if (!c || !l) return;
    const [wx, wy] = this.toWorld(px, py);
    const x = Math.floor(wx / S);
    const y = Math.floor(wy / S);
    const there = l.cell.get(cellKey(x, y));
    Object.assign(c, { px, py, x, y, free: there === undefined });
    this.draw();
  }

  private resize() {
    const dpr = window.devicePixelRatio || 1;
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    if (!w || !h) return;
    this.dpr = dpr;
    this.W = w;
    this.H = h;
    this.canvas.width = Math.round(w * dpr);
    this.canvas.height = Math.round(h * dpr);
    const mw = Math.round(this.mini.clientWidth * dpr);
    const mh = Math.round(this.mini.clientHeight * dpr);
    if (mw && mh && (this.mini.width !== mw || this.mini.height !== mh)) {
      this.mini.width = mw;
      this.mini.height = mh;
      this.miniCache = null;
    }
    this.frameNow();
  }

  // ---- drawing ---------------------------------------------------------------------

  draw(): void {
    if (!this.raf) this.raf = requestAnimationFrame(() => this.frameNow());
  }

  private frameNow() {
    cancelAnimationFrame(this.raf);
    this.raf = 0;
    const { ctx, W, H, dpr, cam } = this;
    if (!W || !H) return;
    const P = this.pal;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = P.bg;
    ctx.fillRect(0, 0, W, H);
    const l = this.layout;
    if (!l) {
      this.drawMini();
      return;
    }
    const z = cam.z;
    const [vx0, vy0] = this.toWorld(0, 0);
    const [vx1, vy1] = this.toWorld(W, H);
    const gx0 = Math.floor(vx0 / S) - 1;
    const gy0 = Math.floor(vy0 / S) - 1;
    const gx1 = Math.ceil(vx1 / S) + 1;
    const gy1 = Math.ceil(vy1 / S) + 1;
    const inView = (n: number) => {
      const p = l.at.get(n);
      return !!p && p.x >= gx0 && p.x <= gx1 && p.y >= gy0 && p.y <= gy1;
    };
    const half = HALF * S * z;
    const rad = Math.min(7, 5 * z);
    const lw = Math.max(1, 2.2 * Math.min(z, 1.5));
    const symbols = this.layers.symbols;
    const pen: Pen = { ctx, pal: P, W, H, z, cell: S * z, half, rad, lw, inView, layout: l, at: (n) => l.at.get(n), sx: (gx) => this.sx(gx), sy: (gy) => this.sy(gy) };

    // dots between the cells
    if (this.layers.grid && z >= 0.45) {
      ctx.fillStyle = P.dot;
      const d = Math.max(1.5, 2.4 * Math.min(z, 1.4));
      for (let gx = gx0; gx <= gx1; gx++) {
        const x = (gx * S - cam.x) * z + W / 2 - d / 2;
        for (let gy = gy0; gy <= gy1; gy++) ctx.fillRect(x, (gy * S - cam.y) * z + H / 2 - d / 2, d, d);
      }
    }

    // in levels, the level below, faint: what lies under this one, and where the ways down lead
    const below = this.stack?.of.get(this.level - 1);
    if (below && z >= 0.2) this.drawBelow(below, gx0, gy0, gx1, gy1, half, rad, lw);

    // the islands' caption: rooms no walk from the start room reaches
    if (l.islands > 1 && !l.pinned.size && z >= 0.3) {
      let top = Infinity;
      let left = Infinity;
      for (const [n, p] of l.at) {
        if (l.island.get(n) === 0) continue;
        top = Math.min(top, p.y);
        left = Math.min(left, p.x);
      }
      if (top >= gy0 && top <= gy1 + 2) {
        ctx.font = `800 ${Math.min(13, 11 * Math.max(z, 0.8))}px Nunito, sans-serif`;
        ctx.textAlign = 'left';
        ctx.textBaseline = 'bottom';
        ctx.fillStyle = P.ink;
        ctx.globalAlpha = 0.55;
        ctx.fillText(`Rooms no walk from room ${this.start} reaches`, this.sx(left) - half, this.sy(top) - half - 8);
        ctx.globalAlpha = 1;
      }
    }

    // the layer drawn with the map, under it (the regions of the builder's notes)
    this.layer?.back?.(pen);

    // exits between neighbours
    const xy = (r: MapRoom): [number, number] => {
      const p = l.at.get(r.room)!;
      return [this.sx(p.x), this.sy(p.y)];
    };
    const shown = this.edges.filter((e) => inView(e.a.room) || inView(e.b.room));
    const dim = (e: Edge) => this.faded(e.a.room) && this.faded(e.b.room);
    ctx.lineCap = 'round';
    for (const fade of [true, false]) {
      ctx.globalAlpha = fade ? 0.2 : 1;
      ctx.strokeStyle = P.path;
      ctx.lineWidth = lw;
      ctx.beginPath();
      for (const e of shown) {
        if (dim(e) !== fade || e.oneWay || (symbols && e.look.sort)) continue;
        const [ax, ay] = xy(e.a);
        const [bx, by] = xy(e.b);
        ctx.moveTo(ax, ay);
        ctx.lineTo(bx, by);
      }
      ctx.stroke();
      for (const e of shown) {
        if (dim(e) !== fade || (!e.oneWay && !(symbols && e.look.sort))) continue;
        const [ax, ay] = xy(e.a);
        const [bx, by] = xy(e.b);
        const sort = symbols ? e.look.sort : null;
        ctx.strokeStyle = sort ? P.exit[sort] : e.oneWay ? P.oneway : P.path;
        ctx.lineWidth = sort ? lw * 1.2 : lw;
        ctx.setLineDash(sort && e.look.dashed ? [lw * 1.4, lw * 2.4] : []);
        ctx.beginPath();
        ctx.moveTo(ax, ay);
        ctx.lineTo(bx, by);
        ctx.stroke();
        ctx.setLineDash([]);
        if (e.oneWay && z >= 0.5) this.arrow((ax + bx) / 2, (ay + by) / 2, Math.atan2(by - ay, bx - ax), 4 * Math.min(z, 1.6) + 2.5, ctx.strokeStyle as string);
      }
    }
    ctx.globalAlpha = 1;

    // stubs: exits to rooms drawn elsewhere, to other maps, or to rooms the map does not have;
    // their chips once there is room for them
    const stubsShown = z >= 0.35 && symbols;
    const chipsShown = z >= 0.9 && symbols;
    if (stubsShown) {
      for (const s of this.stubs) {
        if (s.exit.dir > 7 || !inView(s.room.room)) continue;
        const p = l.at.get(s.room.room)!;
        const c = this.stubSpot(s, p.x, p.y);
        ctx.globalAlpha = this.faded(s.room.room) ? 0.2 : 1;
        ctx.strokeStyle = s.look.sort ? P.exit[s.look.sort] : s.sort === 'map' ? P.portal : s.sort === 'missing' ? P.missing : P.path;
        ctx.lineWidth = lw;
        ctx.setLineDash(s.look.dashed ? [lw * 1.4, lw * 2.4] : []);
        ctx.beginPath();
        ctx.moveTo(this.sx(p.x), this.sy(p.y));
        ctx.lineTo(c.x, c.y);
        ctx.stroke();
        ctx.setLineDash([]);
      }
      ctx.globalAlpha = 1;
    }

    // rooms, one fill per colour (a room being carried is drawn where it is held, below)
    const carry = this.carry;
    const vis = this.rooms.filter((r) => inView(r.room) && r.room !== carry?.room);
    const groups = new Map<string, MapRoom[]>();
    for (const r of vis) {
      const t = this.tints.get(r.room)!;
      const key = (this.faded(r.room) ? '~' : '') + t.colour;
      let g = groups.get(key);
      if (!g) groups.set(key, (g = []));
      g.push(r);
    }
    const square = (r: MapRoom, grow = 0) => {
      const p = l.at.get(r.room)!;
      const x = this.sx(p.x);
      const y = this.sy(p.y);
      if (half < 2) ctx.rect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2);
      else ctx.roundRect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2, rad + grow);
    };
    for (const [key, list] of groups) {
      const fade = key.startsWith('~');
      ctx.globalAlpha = fade ? 0.18 : 1;
      ctx.fillStyle = fade ? key.slice(1) : key;
      ctx.beginPath();
      for (const r of list) square(r);
      ctx.fill();
      if (z >= 0.3) {
        ctx.strokeStyle = P.line;
        ctx.lineWidth = z < 0.55 ? 1 : 1.6;
        ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;

    // Build mode: free cells to dig into around the selected room, and marks to join the rooms
    // drawn next to it (a corridor dragged out of one, or a Shift line, in their place)
    const reach = this.reach ?? this.shiftReach;
    if (reach) this.drawReach(reach, half, rad);
    else if (this.building && !this.tool && z >= GHOSTS && !carry) {
      for (const g of this.ghosts()) {
        const hot = this.hovered?.ghost?.dir === g.dir;
        if (g.to === null) {
          const x = this.sx(g.x);
          const y = this.sy(g.y);
          ctx.fillStyle = P.sel;
          ctx.globalAlpha = hot ? 0.3 : 0.1;
          ctx.beginPath();
          ctx.roundRect(x - half, y - half, half * 2, half * 2, rad);
          ctx.fill();
          ctx.globalAlpha = hot ? 1 : 0.7;
          ctx.strokeStyle = P.sel;
          ctx.lineWidth = hot ? 2.4 : 1.6;
          ctx.setLineDash([4, 3]);
          ctx.stroke();
          ctx.setLineDash([]);
          if (half >= 5) this.icon('plus', x, y, half * 1.1, P.sel, 2.6);
          ctx.globalAlpha = 1;
        } else {
          const c = this.joinSpot(g);
          ctx.fillStyle = hot ? P.sel : P.badge;
          ctx.strokeStyle = P.sel;
          ctx.lineWidth = hot ? 2.4 : 1.6;
          ctx.setLineDash(hot ? [] : [3, 2]);
          ctx.beginPath();
          ctx.arc(c.x, c.y, c.r, 0, TAU);
          ctx.fill();
          ctx.stroke();
          ctx.setLineDash([]);
          if (c.r >= 6) this.icon('link-2', c.x, c.y, c.r * 1.25, hot ? P.badge : P.sel, 2.6);
        }
      }
    }

    // a tool's marks over the rooms (the rooms picked)
    this.tool?.under?.(pen);

    // marks on rooms, and the chips at the stubs' ends
    if (symbols && z >= 0.75) {
      const ic = half * 1.1;
      for (const r of vis) {
        const p = l.at.get(r.room)!;
        const x = this.sx(p.x);
        const y = this.sy(p.y);
        ctx.globalAlpha = this.faded(r.room) ? 0.25 : 1;
        if (r.room_type === 1 || r.shop) this.icon('store', x, y, ic, P.line, 2.2);
        else if (r.room_type === 2 || r.room_type === 5) this.icon('swords', x, y, ic, P.line, 2.2);
        else if (r.room_type === 3) this.icon('crown', x, y, ic, P.line, 2.2);
        if (r.npc) this.badge(x - half, y - half, 'user-round', P.badge, P.ink);
        if (r.command || r.spell) this.badge(x + half, y - half, 'sparkles', '#ffe08a', '#2e2650');
      }
      ctx.globalAlpha = 1;
    }
    if (chipsShown || (symbols && z >= PORTALS)) {
      for (const s of this.stubs) {
        if (!inView(s.room.room) || (!chipsShown && !portal(s))) continue;
        const p = l.at.get(s.room.room)!;
        ctx.globalAlpha = this.faded(s.room.room) ? 0.25 : 1;
        this.drawStub(s, this.stubSpot(s, p.x, p.y), s === this.hovered?.stub);
      }
      ctx.globalAlpha = 1;
    }

    // symbols on exit lines: what it takes to pass
    if (symbols && z >= 0.9) {
      for (const e of shown) {
        if (!e.look.sort || dim(e)) continue;
        const [ax, ay] = xy(e.a);
        const [bx, by] = xy(e.b);
        const mx = (ax + bx) / 2;
        const my = (ay + by) / 2;
        const colour = P.exit[e.look.sort];
        if (e.look.bars) {
          // a door: bars across the line, two when a key locks it
          const a = Math.atan2(by - ay, bx - ax) + Math.PI / 2;
          const len = 0.2 * S * z;
          ctx.strokeStyle = colour;
          ctx.lineWidth = Math.max(2, lw * 1.3);
          ctx.beginPath();
          for (let i = 0; i < e.look.bars; i++) {
            const off = (i - (e.look.bars - 1) / 2) * 0.14 * S * z;
            const ox = Math.cos(a - Math.PI / 2) * off;
            const oy = Math.sin(a - Math.PI / 2) * off;
            ctx.moveTo(mx + ox - Math.cos(a) * len, my + oy - Math.sin(a) * len);
            ctx.lineTo(mx + ox + Math.cos(a) * len, my + oy + Math.sin(a) * len);
          }
          ctx.stroke();
        }
        if (e.look.icon) {
          const br = 6.5 * Math.min(z, 1.7);
          ctx.fillStyle = P.badge;
          ctx.strokeStyle = colour;
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          ctx.arc(mx, my, br, 0, TAU);
          ctx.fill();
          ctx.stroke();
          this.icon(e.look.icon, mx, my, br * 1.3, colour, 2.6);
        }
      }
    }

    if (carry) this.drawCarry(carry, half, rad);

    // room numbers, close up (once the gaps between rooms have room for them)
    if (z >= 2.2) {
      ctx.font = `700 ${Math.min(11, 6.5 * z)}px ui-monospace, "Cascadia Mono", Consolas, monospace`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.lineJoin = 'round';
      ctx.lineWidth = 3;
      ctx.strokeStyle = P.halo;
      ctx.fillStyle = P.ink;
      for (const r of vis) {
        const p = l.at.get(r.room)!;
        const x = this.sx(p.x);
        const y = this.sy(p.y) + half + 3;
        ctx.globalAlpha = this.faded(r.room) ? 0.3 : 1;
        ctx.strokeText(String(r.room), x, y);
        ctx.fillText(String(r.room), x, y);
      }
      ctx.globalAlpha = 1;
    }

    // the wires: links no exit draws
    if (this.wires.length && z >= 0.2 && !carry) this.drawWires(this.wirePaths());

    // where the selected (or hovered) room's far exits lead (a remote action's, its wire)
    const curves: Stub[] = [];
    const far = (s: Stub) => s.sort === 'here' && !(s.exit.kind === 12 && this.wires.length);
    if (this.selected !== null) for (const s of this.stubsOf.get(this.selected) ?? []) if (far(s)) curves.push(s);
    const hs = this.hovered?.stub;
    if (hs && far(hs) && !curves.includes(hs)) curves.push(hs);
    for (const s of curves) {
      const a = l.at.get(s.room.room)!;
      const b = l.at.get(s.exit.to);
      if (!b) continue;
      const [x0, y0, x1, y1] = [this.sx(a.x), this.sy(a.y), this.sx(b.x), this.sy(b.y)];
      const mx = (x0 + x1) / 2 - (y1 - y0) * 0.18;
      const my = (y0 + y1) / 2 + (x1 - x0) * 0.18;
      ctx.strokeStyle = P.portal;
      ctx.globalAlpha = s === hs ? 0.95 : 0.7;
      ctx.lineWidth = Math.max(1.5, lw * 0.9);
      ctx.setLineDash([2, 5]);
      ctx.beginPath();
      ctx.moveTo(x0, y0);
      ctx.quadraticCurveTo(mx, my, x1, y1);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(x1, y1, half + 6, 0, TAU);
      ctx.stroke();
      ctx.globalAlpha = 1;
    }

    // the hovered room
    const hr = this.hovered?.room;
    if (hr && !this.hovered?.stub && inView(hr.room)) {
      ctx.strokeStyle = P.sel;
      ctx.globalAlpha = 0.6;
      ctx.lineWidth = 2;
      ctx.beginPath();
      square(hr, 3);
      ctx.stroke();
      ctx.globalAlpha = 1;
    }

    // the selected room: a ring and its name
    let pill: Box | null = null;
    const sel = this.selected !== null && this.selected !== carry?.room ? this.by.get(this.selected) : undefined;
    if (sel && inView(sel.room)) {
      const p = l.at.get(sel.room)!;
      const x = this.sx(p.x);
      const y = this.sy(p.y);
      ctx.fillStyle = P.sel;
      ctx.globalAlpha = 0.2;
      ctx.beginPath();
      ctx.arc(x, y, half + Math.min(half * 1.3 + 4, 24), 0, TAU);
      ctx.fill();
      ctx.globalAlpha = 1;
      ctx.strokeStyle = P.sel;
      ctx.lineWidth = 3.5;
      ctx.beginPath();
      square(sel, 4.5);
      ctx.stroke();
    }
    if (sel && inView(sel.room) && this.naming !== sel.room) {
      const p = l.at.get(sel.room)!;
      const x = this.sx(p.x);
      const y = this.sy(p.y);
      const label = sel.name.trim() || `Room ${sel.room}`;
      ctx.font = '800 12.5px Nunito, sans-serif';
      const num = String(sel.room);
      const tw = ctx.measureText(label).width;
      ctx.font = '700 11px ui-monospace, "Cascadia Mono", Consolas, monospace';
      const nw = ctx.measureText(num).width;
      const w = tw + nw + 26;
      const py = y - half - 22;
      pill = [x - w / 2, py - 11, w, 22];
      ctx.fillStyle = P.sel;
      ctx.strokeStyle = P.line;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.roundRect(x - w / 2, py - 11, w, 22, 11);
      ctx.fill();
      ctx.stroke();
      const ink = this.pal.sel === '#ffe27a' ? '#1a1636' : '#ffffff';
      ctx.fillStyle = ink;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      ctx.font = '800 12.5px Nunito, sans-serif';
      ctx.fillText(label, x - w / 2 + 10, py + 0.5);
      ctx.globalAlpha = 0.75;
      ctx.font = '700 11px ui-monospace, "Cascadia Mono", Consolas, monospace';
      ctx.fillText(num, x - w / 2 + 16 + tw, py + 1);
      ctx.globalAlpha = 1;
    }

    // the layer drawn with the map, over it (the builder's notes), clear of the selected room's name
    const placed: Box[] = pill ? [pill] : [];
    this.layer?.front?.(pen, placed);

    // names: one per run of rooms that share one, more as you zoom in
    if (this.layers.labels && z >= 0.12) {
      const least = z < 0.32 ? 60 : z < 0.55 ? 18 : z < 1 ? 6 : z < 1.6 ? 3 : z < 2.6 ? 2 : 1;
      // the selected room's pill already names it
      const near = sel ? l.at.get(sel.room) : undefined;
      const said = sel?.name.trim();
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.lineJoin = 'round';
      for (const lb of l.labels) {
        if (lb.size < least) break;
        if (lb.x < gx0 || lb.x > gx1 || lb.y < gy0 || lb.y > gy1) continue;
        const n = l.cell.get(cellKey(lb.x, lb.y));
        if (n !== undefined && this.faded(n)) continue;
        if (near && lb.text === said && Math.abs(lb.x - near.x) <= 2 && Math.abs(lb.y - near.y) <= 2) continue;
        const fs = lb.size >= 60 ? 17 : lb.size >= 18 ? 14.5 : 12.5;
        ctx.font = `${lb.size >= 18 ? 600 : 500} ${fs}px Fredoka, Nunito, sans-serif`;
        const x = this.sx(lb.x);
        const y = this.sy(lb.y) - (z >= 1 ? half + 10 : 0);
        const w = ctx.measureText(lb.text).width + 8;
        const h = fs + 6;
        const box: Box = [x - w / 2, y - h / 2, w, h];
        if (placed.some((b) => box[0] < b[0] + b[2] && b[0] < box[0] + box[2] && box[1] < b[1] + b[3] && b[1] < box[1] + box[3])) continue;
        placed.push(box);
        ctx.lineWidth = 4.5;
        ctx.strokeStyle = P.halo;
        ctx.strokeText(lb.text, x, y);
        ctx.fillStyle = P.ink;
        ctx.fillText(lb.text, x, y);
      }
    }

    // a tool's marks over everything (a pick being drawn, the brush, a stamp held)
    this.tool?.over?.(pen);

    this.drawMini();
  }

  /** The level below the one shown, faint and flat: its rooms in view and the exits between them. */
  private drawBelow(b: Layout, gx0: number, gy0: number, gx1: number, gy1: number, half: number, rad: number, lw: number) {
    const { ctx } = this;
    const P = this.pal;
    const near = (x: number, y: number) => x >= gx0 - 1 && x <= gx1 + 1 && y >= gy0 - 1 && y <= gy1 + 1;
    ctx.save();
    ctx.lineCap = 'round';
    ctx.strokeStyle = P.path;
    ctx.globalAlpha = 0.16;
    ctx.lineWidth = lw;
    ctx.beginPath();
    const rooms: [number, number][] = [];
    for (const [n, p] of b.at) {
      if (!near(p.x, p.y)) continue;
      rooms.push([this.sx(p.x), this.sy(p.y)]);
      // (a pair of exits each way draws one line: one path, stroked once)
      for (const e of this.by.get(n)?.exits ?? []) {
        const step = STEP[e.dir];
        if (!step || e.kind === 8 || e.kind === 12) continue;
        const [qx, qy] = [p.x + step[0], p.y + step[1]];
        if (b.cell.get(cellKey(qx, qy)) !== e.to) continue;
        ctx.moveTo(this.sx(p.x), this.sy(p.y));
        ctx.lineTo(this.sx(qx), this.sy(qy));
      }
    }
    ctx.stroke();
    ctx.beginPath();
    for (const [x, y] of rooms) {
      if (half < 2) ctx.rect(x - half, y - half, half * 2, half * 2);
      else ctx.roundRect(x - half, y - half, half * 2, half * 2, rad);
    }
    ctx.fillStyle = P.path;
    ctx.globalAlpha = 0.1;
    ctx.fill();
    if (this.cam.z >= 0.3) {
      ctx.globalAlpha = 0.3;
      ctx.lineWidth = 1.2;
      ctx.setLineDash([3, 3]);
      ctx.stroke();
    }
    ctx.restore();
  }

  /** A corridor or join reached out of the selected room: its new rooms dashed with their numbers, the rooms it joins lit, and where it stops short. */
  private drawReach(r: Reach, half: number, rad: number) {
    const { ctx } = this;
    const P = this.pal;
    const l = this.layout;
    const p = l?.at.get(r.from);
    if (!l || !p) return;
    const [dx, dy] = STEP[r.dir] ?? [0, 0];
    const box = (x: number, y: number, grow = 0) => {
      ctx.beginPath();
      ctx.roundRect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2, rad + grow);
    };
    ctx.save();
    ctx.lineCap = 'round';
    if (r.kind === 'join') {
      const q = l.at.get(r.to);
      if (!q) return ctx.restore();
      const [bx, by] = [this.sx(q.x), this.sy(q.y)];
      ctx.strokeStyle = P.sel;
      ctx.lineWidth = 2.4;
      ctx.setLineDash([6, 4]);
      ctx.beginPath();
      ctx.moveTo(this.sx(p.x) + dx * half, this.sy(p.y) + dy * half);
      ctx.lineTo(bx, by);
      ctx.stroke();
      ctx.setLineDash([]);
      box(bx, by, 3);
      ctx.lineWidth = 3;
      ctx.stroke();
      ctx.restore();
      return;
    }
    // the ways: from one room's side to the next's, faint where they are there already, dashed where one-way
    let [x0, y0] = [this.sx(p.x), this.sy(p.y)];
    r.links.forEach((k, i) => {
      const c = r.cells[i]!;
      const [x1, y1] = [this.sx(c.x), this.sy(c.y)];
      ctx.strokeStyle = P.sel;
      ctx.globalAlpha = k.out ? 1 : 0.5;
      ctx.lineWidth = k.out ? 2.6 : 1.6;
      ctx.setLineDash(k.out && k.back === 'taken' ? [5, 3] : []);
      ctx.beginPath();
      ctx.moveTo(x0 + dx * half, y0 + dy * half);
      ctx.lineTo(x1 - dx * half, y1 - dy * half);
      ctx.stroke();
      [x0, y0] = [x1, y1];
    });
    ctx.setLineDash([]);
    ctx.globalAlpha = 1;
    const numbers = half >= 12;
    if (numbers) {
      ctx.font = `700 ${Math.min(11, half * 0.62)}px ui-monospace, "Cascadia Mono", Consolas, monospace`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
    }
    r.cells.forEach((c, i) => {
      const [x, y] = [this.sx(c.x), this.sy(c.y)];
      if (c.made) {
        // a new room: dashed, faintly filled, with its number
        ctx.fillStyle = P.sel;
        ctx.globalAlpha = 0.18;
        box(x, y);
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.strokeStyle = P.sel;
        ctx.lineWidth = 2;
        ctx.setLineDash([4, 3]);
        ctx.stroke();
        ctx.setLineDash([]);
        if (numbers) {
          ctx.fillStyle = P.ink;
          ctx.fillText(String(c.room), x, y + 0.5);
        }
      } else if (r.links[i]?.out) {
        // a room it joins lights up
        box(x, y, 3);
        ctx.strokeStyle = P.sel;
        ctx.lineWidth = 3;
        ctx.stroke();
      }
    });
    if (r.stop) {
      // where it stops short: an exit that way to somewhere else
      const [x, y] = [this.sx(r.stop.x), this.sy(r.stop.y)];
      const s = Math.max(5, half * 0.45);
      ctx.fillStyle = P.missing;
      ctx.beginPath();
      ctx.arc(x, y, s, 0, TAU);
      ctx.fill();
      ctx.strokeStyle = P.badge;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(x - s * 0.45, y - s * 0.45);
      ctx.lineTo(x + s * 0.45, y + s * 0.45);
      ctx.moveTo(x + s * 0.45, y - s * 0.45);
      ctx.lineTo(x - s * 0.45, y + s * 0.45);
      ctx.stroke();
    }
    ctx.restore();
  }

  /** A room being carried: its cell left as an outline, the cell under it, and the room itself. */
  private drawCarry(c: Carry, half: number, rad: number) {
    const { ctx } = this;
    const P = this.pal;
    const from = this.layout?.at.get(c.room);
    const r = this.by.get(c.room);
    if (!from || !r) return;
    const box = (x: number, y: number, grow = 0) => {
      ctx.beginPath();
      ctx.roundRect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2, rad + grow);
    };
    ctx.setLineDash([4, 3]);
    ctx.lineWidth = 1.6;
    ctx.strokeStyle = P.line;
    ctx.globalAlpha = 0.6;
    box(this.sx(from.x), this.sy(from.y));
    ctx.stroke();
    // where it would go: free, or taken by another room
    const tx = this.sx(c.x);
    const ty = this.sy(c.y);
    const home = c.x === from.x && c.y === from.y;
    if (!home) {
      ctx.globalAlpha = 1;
      ctx.strokeStyle = c.free ? P.sel : P.missing;
      ctx.lineWidth = 2.4;
      box(tx, ty, 3);
      ctx.stroke();
    }
    ctx.setLineDash([]);
    // the room, lifted, under the pointer
    const grow = Math.max(2, half * 0.15);
    ctx.shadowColor = 'rgba(40, 20, 70, 0.3)';
    ctx.shadowBlur = 12;
    ctx.shadowOffsetY = 5;
    ctx.fillStyle = this.tints.get(c.room)?.colour ?? P.bg;
    box(c.px, c.py, grow);
    ctx.fill();
    ctx.shadowColor = 'transparent';
    ctx.strokeStyle = P.sel;
    ctx.lineWidth = 3;
    ctx.stroke();
    const icon: IconName | null = r.room_type === 1 || r.shop ? 'store' : r.room_type === 2 || r.room_type === 5 ? 'swords' : r.room_type === 3 ? 'crown' : null;
    if (icon && half >= 5) this.icon(icon, c.px, c.py, (half + grow) * 1.1, P.line, 2.2);
  }

  /** Where each wire is drawn, offset by (dx, dy) since the map was last panned. */
  private wirePaths(): { paths: WirePath[]; dx: number; dy: number } {
    const l = this.layout;
    if (!l || !this.wires.length) return { paths: [], dx: 0, dy: 0 };
    const z = this.cam.z;
    const [ox, oy] = [this.sx(0), this.sy(0)];
    const c = this.wireCache;
    if (c && c.wires === this.wires && c.layout === l && c.z === z) return { paths: c.paths, dx: ox - c.ox, dy: oy - c.oy };
    const half = HALF * S * z;
    const cr = Math.max(3, Math.min(11, 0.2 * S * z));
    // the chips beside a room sit the ways it has neither a room next door nor an exit, from its
    // bottom right (its name is above it); then round it
    const slots = new Map<number, number>();
    const chipBy = (n: number, p: Spot): [number, number] => {
      const k = slots.get(n) ?? 0;
      slots.set(n, k + 1);
      const exits = this.by.get(n)?.exits ?? [];
      const free = CHIP_WAYS.filter((d) => !l.cell.has(cellKey(p.x + STEP[d]![0], p.y + STEP[d]![1])) && !exits.some((e) => e.dir === d));
      const d = free[k];
      const a = d === undefined ? Math.PI / 4 + k * 0.62 : Math.atan2(STEP[d]![1], STEP[d]![0]);
      return [this.sx(p.x) + Math.cos(a) * 0.95 * S * z, this.sy(p.y) + Math.sin(a) * 0.95 * S * z];
    };
    const out: WirePath[] = [];
    /** The paths to chips, by the room, the kind and the other map. */
    const shared = new Map<string, WirePath>();
    /** Where the far end of a wire is, on its chip: its map, in levels how many levels up or down, or ✕ for a room the map does not have. */
    const away = (e: { map: number; room: number }): string => {
      if (e.map !== this.wireMap) return String(e.map);
      const rise = (this.levelOf(e.room) ?? this.level) - this.level;
      return !this.by.has(e.room) ? '✕' : rise > 0 ? `▲${rise}` : `▼${-rise}`;
    };
    for (const w of this.wires) {
      const pa = w.from.map === this.wireMap ? l.at.get(w.from.room) : undefined;
      const pb = w.to.map === this.wireMap ? l.at.get(w.to.room) : undefined;
      if (!pa && !pb) continue;
      const key = pa && pb ? '' : pa ? `${w.kind}>${w.from.room}>${away(w.to)}` : `${w.kind}<${w.to.room}<${away(w.from)}`;
      const was = key ? shared.get(key) : undefined;
      if (was) {
        was.n++;
        continue;
      }
      let [ax, ay, bx, by] = [0, 0, 0, 0];
      let [ra, rb] = [half + 2, half + 3];
      let chip: WirePath['chip'] = null;
      if (pa) [ax, ay] = [this.sx(pa.x), this.sy(pa.y)];
      if (pb) {
        [bx, by] = [this.sx(pb.x), this.sy(pb.y)];
        if (w.exit !== undefined) {
          // the exit it works: midway to the next cell, or the up or down mark
          const step = STEP[w.exit];
          if (step) [bx, by] = [bx + step[0] * 0.5 * S * z, by + step[1] * 0.5 * S * z];
          else [bx, by] = [bx - half - 1, by + (w.exit === 8 ? -half : half) * 0.4];
          rb = 3;
        }
      }
      if (!pa) {
        [ax, ay] = chipBy(w.to.room, pb!);
        chip = { x: ax, y: ay, r: cr, label: away(w.from), missing: false };
        ra = cr + 1;
      } else if (!pb) {
        [bx, by] = chipBy(w.from.room, pa);
        const label = away(w.to);
        chip = { x: bx, y: by, r: cr, label, missing: label === '✕' };
        rb = cr + 2;
      }
      const len = Math.hypot(bx - ax, by - ay);
      if (len < 1) continue;
      // bowed to its left, so a wire each way is two
      const k = Math.min(0.22 * len + 6, 90);
      const p: WirePath = { w, n: 1, ax, ay, bx, by, cx: (ax + bx) / 2 + ((by - ay) / len) * k, cy: (ay + by) / 2 - ((bx - ax) / len) * k, t0: 0, t1: 1, chip };
      if (key) shared.set(key, p);
      // clear of what is at each end, with room for the arrowhead
      const head = 3 + 2.2 * Math.min(z, 1.6);
      const clear = (t: number, x: number, y: number, r: number) => {
        const [px, py] = along(p, t);
        return Math.hypot(px - x, py - y) >= r;
      };
      while (p.t0 < 0.5 && !clear(p.t0, ax, ay, ra)) p.t0 += 1 / 64;
      while (p.t1 > 0.5 && !clear(p.t1, bx, by, rb + head)) p.t1 -= 1 / 64;
      out.push(p);
    }
    this.wireCache = { wires: this.wires, layout: l, z, ox, oy, paths: out };
    return { paths: out, dx: 0, dy: 0 };
  }

  /** The line of a wire's path, from its `t0` to its `t1`, added to `to`. */
  private wireLine(p: WirePath, to: CanvasPath) {
    const { t0, t1 } = p;
    const [x0, y0] = along(p, t0);
    const [x1, y1] = along(p, t1);
    // the curve's control point between the two (de Casteljau)
    const [u0, u1] = [1 - t0, 1 - t1];
    const m = (a: number, c: number, b: number) => u1 * u0 * a + (u1 * t0 + t1 * u0) * c + t1 * t0 * b;
    to.moveTo(x0, y0);
    to.quadraticCurveTo(m(p.ax, p.cx, p.bx), m(p.ay, p.cy, p.by), x1, y1);
  }

  /** The wires: dotted, each with its arrowhead and, where there is room, its symbol; then the chips at their far ends. */
  private drawWires({ paths, dx, dy }: { paths: WirePath[]; dx: number; dy: number }) {
    const { ctx, pal: P } = this;
    // the paths' own place on the canvas, before the latest pans
    const [W, H] = [this.W - dx, this.H - dy];
    ctx.save();
    ctx.translate(dx, dy);
    const z = this.cam.z;
    const hot = this.hovered?.wire ?? null;
    const near = this.hovered?.room?.room ?? this.selected;
    const width = Math.max(1.6, 2 * Math.min(z, 1.4));
    const touches = (w: Wire) => near !== null && ((w.from.map === this.wireMap && w.from.room === near) || (w.to.map === this.wireMap && w.to.room === near));
    const strong = (w: Wire) => !this.wiresAll || w === hot || touches(w);
    const seen = paths.filter((p) => Math.max(p.ax, p.bx, p.cx) > -20 - dx && Math.min(p.ax, p.bx, p.cx) < W + 20 && Math.max(p.ay, p.by, p.cy) > -20 - dy && Math.min(p.ay, p.by, p.cy) < H + 20);
    // fainter first, so the strong ones lie on top
    seen.sort((a, b) => Number(strong(a.w)) - Number(strong(b.w)) || Number(a.w === hot) - Number(b.w === hot));
    // the fans of one kind out of a room or into one: how many, and the longest
    const length = (p: WirePath) => Math.hypot(p.bx - p.ax, p.by - p.ay);
    const ends = (p: WirePath) => [`${p.w.kind}>${p.w.from.map}/${p.w.from.room}`, `${p.w.kind}<${p.w.to.map}/${p.w.to.room}`];
    const fans = new Map<string, { n: number; longest: WirePath; most: number }>();
    for (const p of seen) {
      const d = length(p);
      for (const k of ends(p)) {
        const f = fans.get(k);
        if (!f) fans.set(k, { n: 1, longest: p, most: d });
        else {
          f.n++;
          if (d > f.most) [f.longest, f.most] = [p, d];
        }
      }
    }
    /** A wire of a fan so wide (a room scattering players over many) that one more says little. */
    const scatter = (p: WirePath) => ends(p).some((k) => fans.get(k)!.n > SCATTER);
    ctx.lineCap = 'round';
    const head = 3 + 2.2 * Math.min(z, 1.6);
    // the wires in layers, each stroked at once kind by kind (a map full of them draws quickly)
    const layer = (list: WirePath[], alpha: number, lw: number, halo: boolean) => {
      if (!list.length) return;
      if (halo) {
        // under the dots, to keep them clear of the rooms they cross
        const h = new Path2D();
        for (const p of list) this.wireLine(p, h);
        ctx.globalAlpha = 0.75;
        ctx.strokeStyle = P.halo;
        ctx.lineWidth = lw + 3;
        ctx.stroke(h);
      }
      const kinds = new Map<WireKind, [Path2D, Path2D]>();
      for (const p of list) {
        let k = kinds.get(p.w.kind);
        if (!k) kinds.set(p.w.kind, (k = [new Path2D(), new Path2D()]));
        this.wireLine(p, k[0]);
        const [ex, ey] = along(p, p.t1);
        const [qx, qy] = along(p, Math.max(p.t0, p.t1 - 0.02));
        arrowTo(k[1], ex, ey, Math.atan2(ey - qy, ex - qx), head);
      }
      ctx.globalAlpha = alpha;
      ctx.lineWidth = lw;
      ctx.setLineDash([0.1, lw * 2.2]);
      for (const [kind, [line, heads]] of kinds) {
        ctx.strokeStyle = ctx.fillStyle = P.wire[kind];
        ctx.stroke(line);
        ctx.fill(heads);
      }
      ctx.setLineDash([]);
    };
    // the faint; one of many (a room rolled from a range, or of a wide scatter), lighter; the
    // strong; and the one pointed at
    const drawn = seen.filter((p) => p.t1 > p.t0 && p.w !== hot);
    const many = (p: WirePath) => !!p.w.among || scatter(p);
    layer(drawn.filter((p) => !strong(p.w)), 0.45, width, false);
    layer(drawn.filter((p) => strong(p.w) && many(p)), 0.6, width, false);
    layer(drawn.filter((p) => strong(p.w) && !many(p)), 1, width, true);
    layer(seen.filter((p) => p.t1 > p.t0 && p.w === hot), 1, width * 1.6, true);
    // each one's symbol, midway, where it is long enough to have one; a fan has a single symbol,
    // on its longest
    const br = 6 * Math.min(Math.max(z, 0.8), 1.6);
    const marked = (p: WirePath) => p.w === hot || ends(p).every((k) => fans.get(k)!.n <= 3 || fans.get(k)!.longest === p);
    for (const p of seen) {
      const s = strong(p.w);
      if (z < 0.55 || (!s && z < 1) || length(p) < br * 7 || !marked(p)) continue;
      const [mx, my] = along(p, 0.5);
      const colour = P.wire[p.w.kind];
      ctx.globalAlpha = s ? 1 : 0.6;
      ctx.fillStyle = P.badge;
      ctx.strokeStyle = colour;
      ctx.lineWidth = p.w === hot ? 2.4 : 1.5;
      ctx.beginPath();
      ctx.arc(mx, my, br, 0, TAU);
      ctx.fill();
      ctx.stroke();
      this.icon(WIRE_ICON[p.w.kind], mx, my, br * 1.3, colour, 2.4);
    }
    // the chips: another map's number on the wire's colour, or a cross for a room the map lacks
    for (const p of seen) {
      const c = p.chip;
      if (!c) continue;
      const colour = P.wire[p.w.kind];
      ctx.globalAlpha = strong(p.w) ? 1 : 0.6;
      ctx.font = `800 ${Math.min(11, c.r * 1.05)}px ui-monospace, "Cascadia Mono", Consolas, monospace`;
      const text = c.r >= 5;
      const w = text ? Math.max(c.r * 2, ctx.measureText(c.label).width + 6) : c.r * 2;
      ctx.fillStyle = c.missing ? P.badge : colour;
      ctx.strokeStyle = c.missing ? P.missing : p.w === hot ? P.sel : P.line;
      ctx.lineWidth = p.w === hot ? 2.4 : 1.3;
      ctx.beginPath();
      ctx.roundRect(c.x - w / 2, c.y - c.r, w, c.r * 2, c.r);
      ctx.fill();
      ctx.stroke();
      if (text) {
        ctx.fillStyle = c.missing ? P.missing : P.badge;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(c.label, c.x, c.y + 0.5);
      }
    }
    ctx.restore();
  }

  /** The wire under a point: a chip at its end, else its line (not where a room is drawn under it). */
  private wireAt(px: number, py: number, overRoom: boolean): WirePath | null {
    let best: WirePath | null = null;
    let bestD = Infinity;
    const { paths, dx, dy } = this.wirePaths();
    [px, py] = [px - dx, py - dy];
    for (const p of paths) {
      const c = p.chip;
      if (!c) continue;
      const d = Math.hypot(c.x - px, c.y - py) - c.r;
      if (d <= 4 && d < bestD) [best, bestD] = [p, d];
    }
    if (best || overRoom) return best;
    const reach = 6;
    for (const p of paths) {
      if (p.t1 <= p.t0) continue;
      if (px < Math.min(p.ax, p.bx, p.cx) - reach || px > Math.max(p.ax, p.bx, p.cx) + reach || py < Math.min(p.ay, p.by, p.cy) - reach || py > Math.max(p.ay, p.by, p.cy) + reach) continue;
      let [x0, y0] = along(p, p.t0);
      for (let i = 1; i <= 16; i++) {
        const [x1, y1] = along(p, p.t0 + ((p.t1 - p.t0) * i) / 16);
        const [dx, dy] = [x1 - x0, y1 - y0];
        const t = Math.max(0, Math.min(1, ((px - x0) * dx + (py - y0) * dy) / (dx * dx + dy * dy || 1)));
        const d = Math.hypot(x0 + t * dx - px, y0 + t * dy - py);
        if (d <= reach && d < bestD) [best, bestD] = [p, d];
        [x0, y0] = [x1, y1];
      }
    }
    return best;
  }

  private drawStub(s: Stub, c: { x: number; y: number; r: number }, hot: boolean) {
    const { ctx, pal: P } = this;
    const z = this.cam.z;
    if (s.exit.dir > 7) {
      // up or down: a triangle pointing its way
      const up = s.exit.dir === 8;
      const k = c.r;
      ctx.fillStyle = s.sort === 'map' ? P.portal : s.sort === 'missing' ? P.missing : P.ink;
      ctx.strokeStyle = P.halo;
      ctx.lineWidth = 1.2;
      ctx.beginPath();
      ctx.moveTo(c.x - k, c.y + (up ? k * 0.6 : -k * 0.6));
      ctx.lineTo(c.x + k, c.y + (up ? k * 0.6 : -k * 0.6));
      ctx.lineTo(c.x, c.y + (up ? -k * 0.85 : k * 0.85));
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
      if (hot) {
        ctx.strokeStyle = P.sel;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(c.x, c.y, k + 3, 0, TAU);
        ctx.stroke();
      }
      return;
    }
    if (s.sort === 'map') {
      // a portal: its map's number on a disc (a pill for a long number), once there is room
      const label = String(s.exit.para1);
      const text = c.r >= 5.5;
      ctx.font = `800 ${Math.min(12, c.r * 1.1)}px ui-monospace, "Cascadia Mono", Consolas, monospace`;
      const w = text ? Math.max(c.r * 2, ctx.measureText(label).width + 6) : c.r * 2;
      ctx.fillStyle = P.portal;
      ctx.strokeStyle = P.line;
      ctx.lineWidth = 1.4;
      ctx.beginPath();
      ctx.roundRect(c.x - w / 2, c.y - c.r, w, c.r * 2, c.r);
      ctx.fill();
      ctx.stroke();
      if (hot) {
        ctx.strokeStyle = P.sel;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.roundRect(c.x - w / 2 - 3, c.y - c.r - 3, w + 6, c.r * 2 + 6, c.r + 3);
        ctx.stroke();
      }
      if (text) {
        ctx.fillStyle = P.badge;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(label, c.x, c.y + 0.5);
      }
      return;
    }
    const fill = P.badge;
    const ring = s.sort === 'missing' ? P.missing : s.look.sort ? P.exit[s.look.sort] : P.path;
    const ink = s.sort === 'missing' ? P.missing : P.ink;
    const text = z >= 2;
    ctx.font = `800 ${Math.min(10.5, 6.4 * z)}px ui-monospace, "Cascadia Mono", Consolas, monospace`;
    const w = text ? Math.max(c.r * 2, ctx.measureText(s.label).width + 8) : c.r * 2;
    const h = text ? Math.max(c.r * 2, 7.5 * Math.min(z, 1.6) + 4) : c.r * 2;
    ctx.fillStyle = fill;
    ctx.strokeStyle = hot ? P.sel : ring;
    ctx.lineWidth = hot ? 2.2 : 1.4;
    ctx.beginPath();
    ctx.roundRect(c.x - w / 2, c.y - h / 2, w, h, h / 2);
    ctx.fill();
    ctx.stroke();
    if (text) {
      ctx.fillStyle = ink;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(s.label, c.x, c.y + 0.5);
    }
  }

  /** An arrowhead at (x, y), pointing at angle `a`. */
  arrow(x: number, y: number, a: number, s: number, colour: string): void {
    const ctx = this.ctx;
    ctx.fillStyle = colour;
    ctx.beginPath();
    ctx.moveTo(x + Math.cos(a) * s, y + Math.sin(a) * s);
    ctx.lineTo(x + Math.cos(a + 2.5) * s, y + Math.sin(a + 2.5) * s);
    ctx.lineTo(x + Math.cos(a - 2.5) * s, y + Math.sin(a - 2.5) * s);
    ctx.closePath();
    ctx.fill();
  }

  private icon(name: IconName, x: number, y: number, size: number, colour: string, width = 2) {
    const ctx = this.ctx;
    const k = size / 24;
    ctx.save();
    ctx.translate(x - size / 2, y - size / 2);
    ctx.scale(k, k);
    ctx.strokeStyle = colour;
    ctx.lineWidth = width;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    for (const p of iconPaths(name)) ctx.stroke(p);
    ctx.restore();
  }

  private badge(x: number, y: number, name: IconName, fill: string, ink: string) {
    const ctx = this.ctx;
    const r = Math.min(8.5, 4.6 * this.cam.z);
    ctx.fillStyle = fill;
    ctx.strokeStyle = this.pal.line;
    ctx.lineWidth = 1.3;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, TAU);
    ctx.fill();
    ctx.stroke();
    if (r > 5.5) this.icon(name, x, y, r * 1.25, ink, 2.6);
  }

  // ---- the overview ----------------------------------------------------------------

  private drawMini() {
    const { mini, mctx, pal: P } = this;
    const mw = mini.width;
    const mh = mini.height;
    if (!mw || !mh) return;
    mctx.setTransform(1, 0, 0, 1, 0, 0);
    const l = this.layout;
    if (!l) {
      mctx.fillStyle = P.bg;
      mctx.fillRect(0, 0, mw, mh);
      return;
    }
    const b = l.bounds;
    const bw = b.x1 - b.x0 + 1;
    const bh = b.y1 - b.y0 + 1;
    const s = Math.min(mw / (bw + 2), mh / (bh + 2));
    const ox = (mw - bw * s) / 2 - b.x0 * s;
    const oy = (mh - bh * s) / 2 - b.y0 * s;
    this.miniAt = { s, ox, oy };
    if (!this.miniCache) {
      const c = document.createElement('canvas');
      c.width = mw;
      c.height = mh;
      const g = c.getContext('2d')!;
      g.fillStyle = P.bg;
      g.fillRect(0, 0, mw, mh);
      const d = Math.max(1.2 * this.dpr, s * 0.8);
      for (const r of this.rooms) {
        // in levels, the level shown's
        const p = l.at.get(r.room);
        if (!p) continue;
        const t = this.tints.get(r.room)!;
        g.globalAlpha = this.faded(r.room) ? 0.25 : 1;
        g.fillStyle = t.colour === P.plain ? P.path : t.colour;
        g.fillRect(ox + p.x * s, oy + p.y * s, d, d);
      }
      this.miniCache = c;
    }
    mctx.drawImage(this.miniCache, 0, 0);
    const mp: MiniPen = { ctx: mctx, pal: P, s, ox, oy, dpr: this.dpr, at: (n) => l.at.get(n) };
    this.layer?.mini?.(mp);
    this.tool?.mini?.(mp);
    // what the map shows now
    const [vx0, vy0] = this.toWorld(0, 0);
    const [vx1, vy1] = this.toWorld(this.W - this.cover, this.H);
    mctx.strokeStyle = P.sel;
    mctx.lineWidth = 2 * this.dpr;
    mctx.strokeRect(ox + (vx0 / S) * s, oy + (vy0 / S) * s, ((vx1 - vx0) / S) * s, ((vy1 - vy0) / S) * s);
  }
}
