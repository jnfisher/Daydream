// The World map, drawn on a canvas. Rooms are rounded squares on a grid, placed by layout.ts
// and tinted by what the map is coloured by; an exit between neighbouring rooms is a line, with
// its symbol on it when it is more than a plain way through, and an arrow when it is one-way. An
// exit to a room drawn somewhere else, to another map or to a room the map does not have is a
// short stub in its direction ending in a chip; up and down exits are triangles on the room.
// Names are drawn once per run of rooms that share one, more of them as you zoom in.
//
// The camera holds still: new rooms from the server keep the view where it was. In Build mode
// a room can be carried to a free cell; the page pins it there. A stamp being placed is drawn
// over the map see-through, each cell and link tinted by what putting it down would do.

import { cellKey, OPPOSITE, STEP, type Layout, layOut, type Spot } from './layout';
import { biomeOf, exitLook, iconPaths, palette, tintOf, type Biome, type ColourBy, type ExitLook, type IconName, type Palette, type Tint } from './look';
import type { MapExit, MapRoom } from './model';
import { after, clickHow, howOf, inside, regionOf, type PickHow, type PickMode, type PickTool } from './picks';
import type { LinkSort, Placed } from './stamps';

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
}

export interface Hit {
  room: MapRoom | null;
  stub: Stub | null;
  /** Build mode: a way from the selected room to dig (`to` null), or to join to the room drawn there. */
  ghost?: Ghost;
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
  /** What the pointer is over, for the tooltip (null: nothing), at the pointer's place on the canvas. */
  hover(hit: Hit | null, x: number, y: number): void;
  /** The camera moved. */
  moved(cam: Camera): void;
  /** Build mode: a free cell or a join mark clicked. */
  build(ghost: Ghost): void;
  /** Build mode: a room carried to a free cell. */
  move(room: number, x: number, y: number): void;
  /** A stamp is held: the pointer is over cell (x, y). */
  aim(x: number, y: number): void;
  /** A stamp is held: put it down with its first cell at (x, y). */
  stamp(x: number, y: number, mods: { shift: boolean; alt: boolean }): void;
  /** The Select tool: rooms picked by a box, a lasso, the brush, a click or the wand. */
  choose(rooms: number[], how: PickHow): void;
}

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

const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
const ease = (t: number) => 1 - Math.pow(1 - t, 3);

export class Scene {
  readonly cam: Camera = { x: 0, y: 0, z: 1 };
  rooms: MapRoom[] = [];
  by = new Map<number, MapRoom>();
  layout: Layout | null = null;
  /** The start of the layout: room 1, or the lowest room. */
  start = 1;
  selected: number | null = null;

  private colourBy: ColourBy = 'place';
  private edited = new Set<number>();
  /** Legend keys picked out: the other rooms fade. Empty: none fade. */
  private spot = new Set<string>();
  private layers: Layers = { labels: true, grid: true, symbols: true };
  /** How much of the canvas's right side the room card covers. */
  private cover = 0;
  /** Build mode: the ways the selected room can be dug or joined are drawn. */
  private building = false;
  /** The room being named: its name pill is left to the field over it. */
  private naming: number | null = null;
  /** Where rooms moved by hand are drawn. */
  private pins: ReadonlyMap<number, Spot> = new Map();
  /** The room being carried to another cell. */
  private carry: Carry | null = null;
  /** A stamp is held: a click puts it down, and the pointer moves it. */
  private holding = false;
  /** The stamp held, as it would go down where it is. */
  private stamped: Placed | null = null;
  /** The cell the held stamp was last aimed at. */
  private aimed: string | null = null;
  /** The Select tool, while it is on: a drag picks rooms (a box, a lasso, the brush), a click one, the wand its region. */
  private picking: PickTool | null = null;
  /** What a pick does with no key held (Shift adds and Alt takes away, for a moment). */
  private pickMode: PickMode = 'set';
  /** How many rooms across the brush takes. */
  private brushSize = 3;
  /** The rooms picked, drawn washed over. */
  private picked: ReadonlySet<number> = new Set();
  /**
   * A pick being drawn, and how it picks: the box's corners or the lasso's points (on the map,
   * so a zoom meanwhile keeps them where they were drawn), or the rooms the brush has been over.
   */
  private sweep: { tool: PickTool; how: PickHow; pts: [number, number][]; rooms: Set<number> } | null = null;
  /** The rooms drawn picked while a pick is drawn: as they would be once it is let go. */
  private preview: { of: unknown; rooms: ReadonlySet<number> } | null = null;
  /** Where the brush is over the map, and whether it takes rooms away (Alt). */
  private brushAt: { x: number; y: number; drop: boolean } | null = null;
  /** Space held down: a drag pans the map, even with the Select tool. */
  private panKey = false;
  /** The pick being drawn was let go of (Esc): the press it began with ends doing nothing. */
  private dropped = false;
  /** The rooms the wand would take from the room under the pointer. */
  private wandAt: { n: number; of: MapRoom[]; rooms: ReadonlySet<number> } | null = null;

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
  private hovered: Hit | null = null;
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

  /** A map's rooms, and where those moved by hand are drawn (kept when not given). The camera stays where it is. */
  setRooms(rooms: MapRoom[], pins?: ReadonlyMap<number, Spot>): void {
    this.rooms = rooms;
    this.by = new Map(rooms.map((r) => [r.room, r]));
    this.start = this.by.has(1) ? 1 : (rooms[0]?.room ?? 1);
    if (pins) this.pins = pins;
    this.biomes = new Map(rooms.map((r) => [r.room, biomeOf(r)]));
    if (this.selected !== null && !this.by.has(this.selected)) this.selected = null;
    this.relayout();
    this.retint();
  }

  /** Where rooms moved by hand are drawn. */
  setPins(pins: ReadonlyMap<number, Spot>): void {
    this.pins = pins;
    this.relayout();
    this.miniCache = null;
    this.draw();
  }

  private relayout() {
    this.layout = this.rooms.length ? layOut(this.rooms, this.start, this.pins) : null;
    this.hovered = null;
    this.carry = null;
    this.canvas.classList.remove('carrying');
    this.sortExits();
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

  /** The room card is open over the right side (or not). Returns how much of the canvas it covers. */
  setCover(open: boolean): number {
    const wide = !matchMedia('(max-width: 760px)').matches;
    this.cover = open && wide ? Math.min(420, this.W - 24) + 12 : 0;
    return this.cover;
  }

  /** The theme changed. */
  retheme(): void {
    this.pal = palette();
    this.retint();
  }

  select(n: number | null): void {
    this.selected = n !== null && this.by.has(n) ? n : null;
    this.draw();
  }

  /** Build mode on or off. */
  setBuild(on: boolean): void {
    this.building = on;
    if (!on && this.hovered?.ghost) this.hovered = null;
    this.draw();
  }

  /** A stamp held (or put away): a click puts it down, and the pointer aims it. */
  setHolding(on: boolean): void {
    this.holding = on;
    this.aimed = null;
    if (on) {
      this.hovered = null;
      this.carry = null;
      this.canvas.classList.remove('carrying');
    }
    this.canvas.classList.toggle('stamping', on);
    this.draw();
  }

  /** The held stamp as it would go down, drawn see-through (or none). */
  setStamp(p: Placed | null): void {
    this.stamped = p;
    this.draw();
  }

  /** The Select tool on (with its way of picking) or off. */
  setPicking(tool: PickTool | null): void {
    this.picking = tool;
    this.sweep = null;
    this.preview = null;
    this.brushAt = null;
    this.wandAt = null;
    if (tool) {
      this.carry = null;
      this.canvas.classList.remove('carrying');
    }
    this.canvas.classList.toggle('picking', !!tool);
    this.canvas.dataset.tool = tool ?? '';
    this.draw();
  }

  /** What a pick does with no key held. */
  setPickMode(mode: PickMode): void {
    this.pickMode = mode;
  }

  /** How many rooms across the brush takes. */
  setBrushSize(n: number): void {
    this.brushSize = n;
    this.draw();
  }

  /** The rooms picked, drawn washed over. */
  setPicked(rooms: ReadonlySet<number>): void {
    this.picked = rooms;
    this.preview = null;
    this.draw();
  }

  /** Whether a pick is being drawn. */
  get sweeping(): boolean {
    return !!this.sweep;
  }

  /** Lets go of the pick being drawn, picking nothing. */
  cancelSweep(): void {
    if (this.sweep) this.dropped = true;
    this.sweep = null;
    this.preview = null;
    this.draw();
  }

  /** Space held down (or let go): a drag pans the map. */
  setPanKey(down: boolean): void {
    this.panKey = down;
    this.canvas.classList.toggle('pan-key', down);
  }

  /** The wand's region from room `n`: the rooms joined to it through rooms of its colour. */
  regionOf(n: number): number[] {
    return regionOf(this.by, n, (m) => this.tints.get(m)?.key);
  }

  /** The rooms whose middles a box (two corners) or a lasso (its points), on the map, holds. */
  private roomsIn(tool: PickTool, pts: [number, number][]): number[] {
    const l = this.layout;
    if (!l || !pts.length) return [];
    let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
    for (const [x, y] of tool === 'lasso' ? pts : [pts[0]!, pts.at(-1)!]) {
      [x0, x1, y0, y1] = [Math.min(x0, x), Math.max(x1, x), Math.min(y0, y), Math.max(y1, y)];
    }
    const out: number[] = [];
    for (const [n, p] of l.at) {
      const [x, y] = [(p.x + 0.5) * S, (p.y + 0.5) * S];
      if (x < x0 || x > x1 || y < y0 || y > y1) continue;
      if (tool !== 'lasso' || inside(x, y, pts)) out.push(n);
    }
    return out;
  }

  /** The brush drawn from (x0, y0) to (x1, y1) on the canvas: the rooms it passes over join its stroke. */
  private paint(x0: number, y0: number, x1: number, y1: number) {
    const s = this.sweep;
    if (!s) return;
    const steps = Math.max(1, Math.ceil(Math.hypot(x1 - x0, y1 - y0) / (this.brushReach() * 0.75)));
    for (let i = 1; i <= steps; i++) {
      for (const n of this.brushRooms(x0 + ((x1 - x0) * i) / steps, y0 + ((y1 - y0) * i) / steps)) s.rooms.add(n);
    }
  }

  /** The rooms to draw picked: those picked, or while a pick is drawn, as they would be once it is let go. */
  private shownPicks(): ReadonlySet<number> {
    const s = this.sweep;
    if (!s) return this.picked;
    const key = s.tool === 'brush' ? s.rooms.size : s.pts.length === 0 ? 0 : `${s.pts.length},${s.pts.at(-1)!.join(',')}`;
    if (this.preview?.of !== key) {
      const rooms = s.tool === 'brush' ? [...s.rooms] : s.pts.length > 1 ? this.roomsIn(s.tool, s.pts) : [];
      this.preview = { of: key, rooms: after(this.picked, rooms, s.how) };
    }
    return this.preview.rooms;
  }

  /** How far the brush reaches from its middle to the middles of the rooms it takes, on the canvas. */
  private brushReach(): number {
    return Math.max(4, ((this.brushSize - 1) / 2 + 0.55) * S * this.cam.z);
  }

  /** The rooms under the brush at (px, py). */
  private brushRooms(px: number, py: number): number[] {
    const l = this.layout;
    if (!l) return [];
    const r = this.brushReach();
    const [wx0, wy0] = this.toWorld(px - r, py - r);
    const [wx1, wy1] = this.toWorld(px + r, py + r);
    const out: number[] = [];
    for (let gx = Math.floor(wx0 / S); gx <= Math.floor(wx1 / S); gx++) {
      for (let gy = Math.floor(wy0 / S); gy <= Math.floor(wy1 / S); gy++) {
        const n = l.cell.get(cellKey(gx, gy));
        if (n !== undefined && Math.hypot(this.sx(gx) - px, this.sy(gy) - py) <= r) out.push(n);
      }
    }
    return out;
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
    const stub = (room: MapRoom, exit: MapExit, sort: Stub['sort'], label: string) => {
      const s: Stub = { room, exit, sort, label, look: exitLook(exit) };
      this.stubs.push(s);
      let list = this.stubsOf.get(room.room);
      if (!list) this.stubsOf.set(room.room, (list = []));
      list.push(s);
    };
    for (const r of this.rooms) {
      const p = at.get(r.room)!;
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
        const q = at.get(t.room)!;
        const step = STEP[e.dir];
        if (!step || q.x !== p.x + step[0] || q.y !== p.y + step[1]) {
          stub(r, e, 'here', String(e.to));
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
    for (const r of this.rooms) {
      this.tints.set(r.room, tintOf(r, this.colourBy, this.pal, this.biomes.get(r.room) ?? 'other', this.edited.has(r.room)));
    }
    this.miniCache = null;
    this.draw();
  }

  private faded(n: number): boolean {
    return this.spot.size > 0 && !this.spot.has(this.tints.get(n)?.key ?? '');
  }

  // ---- the camera ------------------------------------------------------------------

  /** A cell's middle on the canvas. */
  private sx(gx: number): number {
    return ((gx + 0.5) * S - this.cam.x) * this.cam.z + this.W / 2;
  }

  private sy(gy: number): number {
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

  /** The camera to put a room in the middle of what the card leaves showing. */
  private aim(n: number, z: number): [number, number] | null {
    const p = this.layout?.at.get(n);
    if (!p) return null;
    return [(p.x + 0.5) * S + this.cover / 2 / z, (p.y + 0.5) * S];
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
    const mx = Math.min(140, w * 0.22);
    const my = Math.min(120, this.H * 0.22);
    if (x < mx || x > w - mx || y < my || y > this.H - my) this.centreOn(n);
    else this.draw();
  }

  zoomBy(f: number, at?: [number, number]): void {
    const [px, py] = at ?? [(this.W - this.cover) / 2, this.H / 2];
    // a zoom while one is under way goes on from where that one was going
    const base = this.goal ?? this.cam;
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
    // Build mode: the join marks, then the free cells around the selected room
    if (this.building && !this.picking && z >= GHOSTS) {
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
    // chips and marks first: they sit on and beside rooms
    if (this.layers.symbols && z >= 0.9) {
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
            const c = this.stubSpot(s, x, y);
            const d = Math.hypot(c.x - px, c.y - py);
            if (d <= c.r + 4 && d < bestD) [best, bestD] = [s, d];
          }
        }
      }
      if (best) return { room: best.room, stub: best };
    }
    // the whole cell around a room counts, so small rooms are easy to hit
    const [wx, wy] = this.toWorld(px, py);
    const n = l.cell.get(cellKey(Math.floor(wx / S), Math.floor(wy / S)));
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
    }
    return { x: x + ox * S * z, y: y + oy * S * z, r: Math.max(2.5, 0.17 * S * z) };
  }

  private listen() {
    const cv = this.canvas;
    const pointers = new Map<number, [number, number]>();
    let drag: { x: number; y: number; cx: number; cy: number; moved: boolean; room: number | null; touch: boolean; at: number } | null = null;
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
        drag = null;
        // a second finger: the pick the first began is let go of
        if (this.sweep) {
          this.sweep = null;
          this.brushAt = null;
          this.draw();
        }
        return;
      }
      const [x, y] = pos(e);
      this.land();
      this.dropped = false;
      // the Select tool: a drag draws a box or a lasso or brushes rooms (the wand's drag, Space
      // held down and the middle button pan the map)
      const tool = this.picking;
      if (tool && tool !== 'wand' && e.button === 0 && !this.panKey) {
        const plain = this.pickMode === 'set' && tool === 'brush' ? 'add' : this.pickMode;
        this.sweep = { tool, how: howOf(e, plain), pts: [this.toWorld(x, y)], rooms: new Set() };
        this.preview = null;
        if (tool === 'brush') {
          this.brushAt = { x, y, drop: this.sweep.how === 'drop' };
          this.paint(x, y, x, y);
        }
        this.setHover(null, x, y);
        drag = { x, y, cx: this.cam.x, cy: this.cam.y, moved: false, room: null, touch: e.pointerType === 'touch', at: performance.now() };
        this.draw();
        return;
      }
      // Build mode: a room pressed on is carried, not the map (a finger holds it a moment first)
      const hit = this.building && !this.holding && !tool ? this.hitAt(x, y) : null;
      const room = hit?.room && !hit.stub && !hit.ghost && e.button === 0 ? hit.room.room : null;
      drag = { x, y, cx: this.cam.x, cy: this.cam.y, moved: false, room, touch: e.pointerType === 'touch', at: performance.now() };
    });
    add(cv, 'pointermove', (e) => {
      const [x, y] = pos(e);
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
      const s = this.sweep;
      if (s && drag) {
        if (!drag.moved && Math.abs(x - drag.x) + Math.abs(y - drag.y) > 4) drag.moved = true;
        if (s.tool === 'box') s.pts = [s.pts[0]!, this.toWorld(x, y)];
        else if (s.tool === 'lasso') {
          const last = s.pts.at(-1)!;
          const w = this.toWorld(x, y);
          if (Math.hypot(w[0] - last[0], w[1] - last[1]) * this.cam.z > 3) s.pts.push(w);
        } else {
          const b = this.brushAt;
          this.paint(b?.x ?? x, b?.y ?? y, x, y);
          this.brushAt = { x, y, drop: s.how === 'drop' };
        }
        this.draw();
        return;
      }
      if (this.picking === 'brush' && !drag && e.pointerType !== 'touch') {
        // the brush shows where it would go, and how much it takes
        this.brushAt = { x, y, drop: e.altKey || (this.pickMode === 'drop' && !e.shiftKey) };
        this.setHover(null, x, y);
        this.draw();
        return;
      }
      if (this.carry) {
        this.carryTo(x, y);
        return;
      }
      if (drag) {
        const dx = x - drag.x;
        const dy = y - drag.y;
        if (!drag.moved && Math.abs(dx) + Math.abs(dy) > 4) {
          drag.moved = true;
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
      if (this.holding) {
        // the held stamp follows the pointer (a finger moves it by tapping)
        if (e.pointerType !== 'touch') this.aimAt(x, y);
        return;
      }
      if (e.pointerType === 'mouse') this.setHover(this.hitAt(x, y), x, y);
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
      if (this.dropped) {
        this.dropped = false;
        return;
      }
      const s = this.sweep;
      if (s) {
        this.sweep = null;
        if (e.pointerType === 'touch') this.brushAt = null;
        this.draw();
        if (cancelled) return;
        if (s.tool === 'brush') this.on.choose([...s.rooms], s.how);
        else if (d?.moved) this.on.choose(this.roomsIn(s.tool, s.pts), s.how);
        else this.clickPick(...pos(e), e);
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
      if (this.picking) {
        // the wand (a drag with it pans), or a click with Space held down: nothing
        if (!this.panKey && e.button === 0) this.clickPick(x, y, e);
        return;
      }
      if (this.holding) {
        // a click puts the stamp down; a tap moves it there first, and a second tap puts it down
        const [cx, cy] = this.cellAt(x, y);
        const again = this.aimed === `${cx},${cy}`;
        this.aimAt(x, y);
        if (e.pointerType !== 'touch' || again) this.on.stamp(cx, cy, { shift: e.shiftKey, alt: e.altKey });
        return;
      }
      const hit = this.hitAt(x, y);
      if (hit?.ghost) this.on.build(hit.ghost);
      else if (hit?.stub) this.on.follow(hit.stub);
      else this.on.pick(hit?.room ?? null);
    };
    add(cv, 'pointerup', (e) => end(e, false));
    add(cv, 'pointercancel', (e) => end(e, true));
    add(cv, 'pointerleave', (e) => {
      if (drag) return;
      this.setHover(null, ...pos(e));
      if (this.brushAt) {
        this.brushAt = null;
        this.draw();
      }
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
        this.zoomNow(this.cam.z * Math.exp(-e.deltaY * unit * (e.ctrlKey ? 0.01 : 0.0018)), x, y);
      },
      { passive: false },
    );
    add(cv, 'dblclick', (e) => {
      if (this.holding || this.picking) return;
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

  /**
   * A click with the Select tool: the wand takes the rooms joined to the room clicked in its
   * colour, the others that room (clickHow). Empty space lets go of every room, unless adding
   * or taking away.
   */
  private clickPick(px: number, py: number, m: { shiftKey: boolean; altKey: boolean; ctrlKey: boolean; metaKey: boolean }) {
    const n = this.hitAt(px, py)?.room?.room;
    const tool = this.picking;
    if (n === undefined || !tool) {
      if (this.pickMode === 'set' && !m.shiftKey && !m.altKey && !m.ctrlKey && !m.metaKey) this.on.choose([], 'set');
      return;
    }
    if (tool === 'wand') this.on.choose(this.regionOf(n), howOf(m, this.pickMode));
    else this.on.choose([n], clickHow(m, this.pickMode));
  }

  /** The held stamp is over the cell at (px, py). */
  private aimAt(px: number, py: number) {
    const [x, y] = this.cellAt(px, py);
    const k = `${x},${y}`;
    if (k === this.aimed) return;
    this.aimed = k;
    this.on.aim(x, y);
  }

  private setHover(hit: Hit | null, x: number, y: number) {
    const was = this.hovered;
    this.hovered = hit;
    if (was?.room !== hit?.room || was?.stub !== hit?.stub || was?.ghost?.dir !== hit?.ghost?.dir) this.draw();
    // in Build mode a room can be picked up and carried
    this.canvas.style.cursor = !hit || this.picking ? '' : this.building && hit.room && !hit.stub && !hit.ghost ? 'grab' : 'pointer';
    this.on.hover(hit, x, y);
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

    // dots between the cells
    if (this.layers.grid && z >= 0.45) {
      ctx.fillStyle = P.dot;
      const d = Math.max(1.5, 2.4 * Math.min(z, 1.4));
      for (let gx = gx0; gx <= gx1; gx++) {
        const x = (gx * S - cam.x) * z + W / 2 - d / 2;
        for (let gy = gy0; gy <= gy1; gy++) ctx.fillRect(x, (gy * S - cam.y) * z + H / 2 - d / 2, d, d);
      }
    }

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
    // drawn next to it
    if (this.building && !this.picking && z >= GHOSTS && !carry && !this.holding) {
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

    // the Select tool: the rooms picked (as they would be while a pick is drawn), and those the
    // wand would take from the room under the pointer
    const picks = this.picking || this.picked.size ? this.shownPicks() : null;
    if (picks?.size) this.drawPicks(picks, inView, 0.26, false);
    const wr = this.picking === 'wand' && !this.sweep && !this.hovered?.stub ? this.hovered?.room : undefined;
    if (wr) {
      if (this.wandAt?.n !== wr.room || this.wandAt.of !== this.rooms) this.wandAt = { n: wr.room, of: this.rooms, rooms: new Set(this.regionOf(wr.room)) };
      this.drawPicks(this.wandAt.rooms, inView, 0.12, true);
    }

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
    if (chipsShown) {
      for (const s of this.stubs) {
        if (!inView(s.room.room)) continue;
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

    // where the selected (or hovered) room's far exits lead
    const curves: Stub[] = [];
    if (this.selected !== null) for (const s of this.stubsOf.get(this.selected) ?? []) if (s.sort === 'here') curves.push(s);
    const hs = this.hovered?.stub;
    if (hs && hs.sort === 'here' && !curves.includes(hs)) curves.push(hs);
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
    let pill: [number, number, number, number] | null = null;
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

    // names: one per run of rooms that share one, more as you zoom in
    if (this.layers.labels && z >= 0.12) {
      const least = z < 0.32 ? 60 : z < 0.55 ? 18 : z < 1 ? 6 : z < 1.6 ? 3 : z < 2.6 ? 2 : 1;
      const placed: [number, number, number, number][] = pill ? [pill] : [];
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
        const box: [number, number, number, number] = [x - w / 2, y - h / 2, w, h];
        if (placed.some((b) => box[0] < b[0] + b[2] && b[0] < box[0] + box[2] && box[1] < b[1] + b[3] && b[1] < box[1] + box[3])) continue;
        placed.push(box);
        ctx.lineWidth = 4.5;
        ctx.strokeStyle = P.halo;
        ctx.strokeText(lb.text, x, y);
        ctx.fillStyle = P.ink;
        ctx.fillText(lb.text, x, y);
      }
    }

    // the pick being drawn: the box or the lasso, and the brush
    const sw = this.sweep;
    if (sw && sw.tool !== 'brush' && sw.pts.length > 1) {
      ctx.beginPath();
      if (sw.tool === 'box') {
        const [a, b] = [sw.pts[0]!, sw.pts.at(-1)!];
        const [ax, ay] = [(a[0] - cam.x) * z + W / 2, (a[1] - cam.y) * z + H / 2];
        const [bx, by] = [(b[0] - cam.x) * z + W / 2, (b[1] - cam.y) * z + H / 2];
        ctx.rect(Math.min(ax, bx), Math.min(ay, by), Math.abs(bx - ax), Math.abs(by - ay));
      } else {
        for (const [wx, wy] of sw.pts) ctx.lineTo((wx - cam.x) * z + W / 2, (wy - cam.y) * z + H / 2);
        ctx.closePath();
      }
      ctx.fillStyle = P.sel;
      ctx.globalAlpha = 0.08;
      ctx.fill();
      ctx.globalAlpha = 1;
      ctx.strokeStyle = P.sel;
      ctx.lineWidth = 1.6;
      ctx.lineJoin = 'round';
      ctx.setLineDash([6, 4]);
      ctx.stroke();
      ctx.setLineDash([]);
    }
    const bt = this.picking === 'brush' ? this.brushAt : null;
    if (bt) {
      const r = Math.max(6, this.brushReach());
      ctx.beginPath();
      ctx.arc(bt.x, bt.y, r, 0, TAU);
      ctx.fillStyle = P.sel;
      ctx.globalAlpha = sw ? 0.14 : 0.07;
      ctx.fill();
      ctx.globalAlpha = 1;
      ctx.strokeStyle = P.sel;
      ctx.lineWidth = 2;
      ctx.setLineDash(bt.drop ? [4, 4] : []);
      ctx.stroke();
      ctx.setLineDash([]);
      // Alt takes rooms away: a minus in the middle
      if (bt.drop && r >= 10) {
        ctx.beginPath();
        ctx.moveTo(bt.x - 4, bt.y);
        ctx.lineTo(bt.x + 4, bt.y);
        ctx.stroke();
      }
    }

    // a stamp held: the map behind it faded back, so it shows over any room's colour
    if (this.stamped) {
      ctx.globalAlpha = 0.5;
      ctx.fillStyle = P.bg;
      ctx.fillRect(0, 0, W, H);
      ctx.globalAlpha = 1;
      this.drawStamp(this.stamped, half, rad, lw);
    }

    this.drawMini();
  }

  /**
   * Rooms picked: their cells washed over and the edge of each patch of them drawn, so a patch
   * reads as one place (dashed: the rooms the wand would take).
   */
  private drawPicks(rooms: ReadonlySet<number>, inView: (n: number) => boolean, wash: number, dashed: boolean) {
    const { ctx, cam, W, H } = this;
    const P = this.pal;
    const l = this.layout;
    if (!l) return;
    const z = cam.z;
    const c = S * z;
    const shown: [number, number, number, number][] = [];
    for (const n of rooms) {
      if (!inView(n)) continue;
      const p = l.at.get(n)!;
      shown.push([p.x, p.y, (p.x * S - cam.x) * z + W / 2, (p.y * S - cam.y) * z + H / 2]);
    }
    if (!shown.length) return;
    ctx.fillStyle = P.sel;
    ctx.globalAlpha = wash;
    ctx.beginPath();
    for (const [, , x, y] of shown) ctx.rect(x, y, c, c);
    ctx.fill();
    ctx.globalAlpha = 1;
    // the edges with no room picked beyond them
    const on = (gx: number, gy: number) => {
      const m = l.cell.get(cellKey(gx, gy));
      return m !== undefined && rooms.has(m);
    };
    ctx.beginPath();
    for (const [gx, gy, x, y] of shown) {
      if (!on(gx, gy - 1)) {
        ctx.moveTo(x, y);
        ctx.lineTo(x + c, y);
      }
      if (!on(gx, gy + 1)) {
        ctx.moveTo(x, y + c);
        ctx.lineTo(x + c, y + c);
      }
      if (!on(gx - 1, gy)) {
        ctx.moveTo(x, y);
        ctx.lineTo(x, y + c);
      }
      if (!on(gx + 1, gy)) {
        ctx.moveTo(x + c, y);
        ctx.lineTo(x + c, y + c);
      }
    }
    ctx.strokeStyle = P.sel;
    ctx.lineWidth = z < 0.4 ? 1.5 : 2.2;
    ctx.lineCap = 'square';
    ctx.setLineDash(dashed ? [5, 4] : []);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.lineCap = 'round';
  }

  /**
   * A stamp as it would go down: its new rooms filled, the rooms it joins ringed, the cells it
   * leaves alone hatched; its links drawn in what they would do (made, there already, left for
   * an exit in the way, replacing one), one-way ones with an arrow, and the new rooms' numbers
   * close up.
   */
  private drawStamp(p: Placed, half: number, rad: number, lw: number) {
    const { ctx } = this;
    const P = this.pal;
    const z = this.cam.z;
    const colour = (sort: LinkSort) => (sort === 'make' ? P.stamp.fresh : sort === 'have' ? P.stamp.join : sort === 'clash' ? P.stamp.clash : sort === 'replace' ? P.stamp.replace : P.stamp.kept);
    const box = (x: number, y: number, grow = 0) => {
      ctx.beginPath();
      if (half < 2) ctx.rect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2);
      else ctx.roundRect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2, rad + grow);
    };
    // the cells first, so the links show over them
    for (const c of p.cells) {
      const x = this.sx(c.x);
      const y = this.sy(c.y);
      if (c.sort === 'new') {
        ctx.globalAlpha = 0.42;
        ctx.fillStyle = P.stamp.fresh;
        box(x, y);
        ctx.fill();
        ctx.globalAlpha = 0.95;
        ctx.strokeStyle = P.stamp.fresh;
        ctx.lineWidth = 2;
        ctx.stroke();
      } else if (c.sort === 'merge') {
        ctx.globalAlpha = 0.22;
        ctx.fillStyle = P.stamp.join;
        box(x, y, 3);
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.strokeStyle = P.stamp.join;
        ctx.lineWidth = 2.6;
        ctx.stroke();
      } else {
        // left alone: hatched
        ctx.save();
        box(x, y, 2);
        ctx.globalAlpha = 0.9;
        ctx.strokeStyle = P.stamp.kept;
        ctx.lineWidth = 1.6;
        ctx.setLineDash([3, 3]);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.clip();
        ctx.lineWidth = 1.4;
        ctx.globalAlpha = 0.7;
        ctx.beginPath();
        const h = half + 2;
        for (let k = -2 * h; k <= 2 * h; k += Math.max(4, h * 0.45)) {
          ctx.moveTo(x - h + k, y + h);
          ctx.lineTo(x + h + k, y - h);
        }
        ctx.stroke();
        ctx.restore();
      }
    }
    ctx.globalAlpha = 1;
    for (const l of p.links) {
      const [ax, ay, bx, by] = [this.sx(l.ax), this.sy(l.ay), this.sx(l.bx), this.sy(l.by)];
      const c = colour(l.sort);
      ctx.strokeStyle = c;
      ctx.globalAlpha = l.sort === 'have' || l.sort === 'skip' ? 0.6 : 1;
      ctx.lineWidth = l.sort === 'clash' ? lw * 1.5 : lw * 1.3;
      ctx.setLineDash(l.dir > 7 || l.seam || l.sort === 'skip' ? [lw * 1.6, lw * 2.2] : l.sort === 'clash' ? [lw * 3, lw * 2] : []);
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      if (l.dir > 7) {
        // up and down: a curve, as the far exits are drawn
        ctx.quadraticCurveTo((ax + bx) / 2 - (by - ay) * 0.3, (ay + by) / 2 + (bx - ax) * 0.3, bx, by);
      } else ctx.lineTo(bx, by);
      ctx.stroke();
      ctx.setLineDash([]);
      const [mx, my] = [(ax + bx) / 2, (ay + by) / 2];
      if (l.oneWay && z >= 0.5) this.arrow(mx, my, Math.atan2(by - ay, bx - ax), 4 * Math.min(z, 1.6) + 3, c);
      if (l.sort === 'clash' && z >= 0.55) {
        // an exit in the way: a mark on the link
        const r = Math.max(5, 6 * Math.min(z, 1.6));
        ctx.globalAlpha = 1;
        ctx.fillStyle = c;
        ctx.strokeStyle = P.badge;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.arc(mx, my, r, 0, TAU);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = P.badge;
        ctx.font = `900 ${r * 1.5}px Nunito, sans-serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('!', mx, my + 0.5);
      }
    }
    ctx.globalAlpha = 1;
    // the numbers the new rooms will take
    if (z >= 1.1) {
      ctx.font = `800 ${Math.min(11.5, 0.3 * S * z)}px ui-monospace, "Cascadia Mono", Consolas, monospace`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.lineJoin = 'round';
      ctx.lineWidth = 3;
      for (const c of p.cells) {
        if (c.sort !== 'new' || c.room === null) continue;
        const x = this.sx(c.x);
        const y = this.sy(c.y);
        ctx.strokeStyle = P.halo;
        ctx.strokeText(String(c.room), x, y);
        ctx.fillStyle = P.ink;
        ctx.fillText(String(c.room), x, y);
      }
    }
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
    const fill = s.sort === 'map' ? P.portal : P.badge;
    const ring = s.sort === 'missing' ? P.missing : s.sort === 'map' ? P.line : s.look.sort ? P.exit[s.look.sort] : P.path;
    const ink = s.sort === 'map' ? P.badge : s.sort === 'missing' ? P.missing : P.ink;
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

  private arrow(x: number, y: number, a: number, s: number, colour: string) {
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
        const p = l.at.get(r.room)!;
        const t = this.tints.get(r.room)!;
        g.globalAlpha = this.faded(r.room) ? 0.25 : 1;
        g.fillStyle = t.colour === P.plain ? P.path : t.colour;
        g.fillRect(ox + p.x * s, oy + p.y * s, d, d);
      }
      this.miniCache = c;
    }
    mctx.drawImage(this.miniCache, 0, 0);
    // the rooms picked
    if (this.picked.size) {
      mctx.fillStyle = P.sel;
      const d = Math.max(2 * this.dpr, s);
      for (const n of this.picked) {
        const p = l.at.get(n);
        if (p) mctx.fillRect(ox + p.x * s - (d - s) / 2, oy + p.y * s - (d - s) / 2, d, d);
      }
    }
    // what the map shows now
    const [vx0, vy0] = this.toWorld(0, 0);
    const [vx1, vy1] = this.toWorld(this.W - this.cover, this.H);
    mctx.strokeStyle = P.sel;
    mctx.lineWidth = 2 * this.dpr;
    mctx.strokeRect(ox + (vx0 / S) * s, oy + (vy0 / S) * s, ((vx1 - vx0) / S) * s, ((vy1 - vy0) / S) * s);
  }
}
