// The Select tool: rooms picked on the map with a box, a lasso, the brush or the wand (the rooms
// joined to one in its colour), and their fields set together as one save; and Paint, which puts
// light, monsters or a control room straight onto the rooms a stroke goes over (only those
// picked, when some are), a save for each stroke. Its code loads with its panel the first time
// the tool is taken up; the rooms picked wait while it is put away.

import { api } from '../api';
import { MONSTER_GROUP, ROOM_TYPE } from '../layouts';
import { read, sfx, write } from '../shell/prefs.svelte';
import { app } from '../shell/state.svelte';
import type { Host } from './host';
import { cellKey } from './layout';
import { lightWord } from './light';
import type { MapRoom } from './model';
import {
  after,
  BRUSH_SIZES,
  changing,
  clickHow,
  holds,
  howOf,
  inside,
  onRooms,
  PAINT_COLOUR,
  PAINT_FIELDS,
  PICK_FIELDS,
  PICK_TOOLS,
  regionOf,
  valuesOf,
  withValues,
  type PaintField,
  type PickField,
  type PickHow,
  type PickMode,
  type PickTool,
} from './picks';
import type { MapTool, MiniPen, Pen, Scene } from './scene';

const TAU = Math.PI * 2;

/**
 * A pick being drawn and how it picks: the box's corners or the lasso's points (in cells, so a
 * zoom meanwhile keeps them where they were drawn), or the rooms the brush has been over; and
 * where the press began on the canvas, and whether it has moved since.
 */
interface Sweep {
  tool: PickTool;
  how: PickHow;
  pts: [number, number][];
  rooms: Set<number>;
  x: number;
  y: number;
  moved: boolean;
  /** Paint's pipette: the press takes the value of the room it is let go over. */
  take?: boolean;
  /** Paint: the colour each room of the stroke takes, as the map is coloured. */
  tints?: Map<number, string>;
}

const toolFrom = (v: string): PickTool => PICK_TOOLS.find((t) => t.id === v)?.id ?? 'box';
const brushFrom = (v: string): number => (BRUSH_SIZES.includes(Number(v)) ? Number(v) : 3);
const fieldOf = (id: PickField['id']): PickField => PICK_FIELDS.find((f) => f.id === id)!;
const group = (g: number) => MONSTER_GROUP[g] ?? `Mon Type ${g}`;

export class Selector implements MapTool {
  readonly host: Host;
  /** The way it picks (kept for next time). */
  tool = $state<PickTool>(toolFrom(read('dd-map-pick', 'box')));
  /** What a pick does with no key held (Shift adds and Alt takes away, for a moment). */
  mode = $state<PickMode>('set');
  /** How many rooms across the brush takes (kept for next time). */
  brush = $state(brushFrom(read('dd-map-brush', '3')));
  /** The rooms picked, on this map. */
  picked = $state.raw<ReadonlySet<number>>(new Set());
  /** Whether the tool is on. */
  on = $state(false);
  /** Counts the times the map's rooms changed, so what is worked out from them follows. */
  private seen = $state(0);
  private sweep: Sweep | null = null;
  /** The rooms drawn picked while a pick is drawn: as they would be once it is let go. */
  private preview: { of: unknown; rooms: ReadonlySet<number> } | null = null;
  /** Where the brush is over the map, and whether it takes rooms away (Alt). */
  private brushAt: { x: number; y: number; drop: boolean } | null = null;
  /** The rooms the wand would take from the room under the pointer. */
  private wandAt: { n: number; of: MapRoom[]; rooms: ReadonlySet<number> } | null = null;
  /** Space held down: a drag moves the map. */
  private panKey = false;
  /** The field Paint puts on rooms. */
  paintField = $state<PaintField>('light');
  /** What Paint puts in each field's columns, as typed (null while a box is empty). */
  paints = $state<Record<PaintField, (number | null)[]>>({ light: [0], monsters: [0, 0, 0, 0], control: [0] });
  /** The pipette out: a click takes a room's value instead of painting (Alt does it with a mouse). */
  dropper = $state(false);
  /** Alt held down: Paint takes a room's value instead. */
  private alt = false;
  /** Whether Paint has taken its first values (from the room selected). */
  private primed = false;
  private readonly close: () => void;

  constructor(host: Host, close: () => void) {
    this.host = host;
    this.close = close;
  }

  get cursor(): string {
    return this.tool === 'paint' && (this.dropper || this.alt) ? 'dropper' : this.tool;
  }

  /** The field Paint puts on rooms, with its columns. */
  get paintDef(): PickField {
    return fieldOf(this.paintField);
  }

  private get scene(): Scene {
    return this.host.scene;
  }

  /** The rooms picked, as the map has them, in room order. */
  rooms = $derived.by((): MapRoom[] => {
    void this.seen;
    const by = this.host.scene.by;
    if (!this.picked.size) return [];
    return [...this.picked]
      .map((n) => by.get(n))
      .filter((r): r is MapRoom => !!r)
      .sort((a, b) => a.room - b.room);
  });

  /** The whole numbers each Rooms column holds. */
  ranges = $derived(Object.fromEntries((app.tables.find((t) => t.name === 'Rooms')?.columns ?? []).map((c) => [c.name, c.range ?? null])) as Record<string, [number, number] | null>);

  /** The tool taken up or put away; the rooms picked wait for it. */
  turn(on: boolean): void {
    this.on = on;
    this.sweep = this.preview = this.brushAt = this.wandAt = null;
    this.dropper = this.alt = false;
    if (!on) this.setPanKey(false);
    this.scene.setTool(on ? this : null);
    if (on && this.tool === 'paint') this.readyPaint(true);
    else if (on) this.host.say(this.host.coarse ? 'Select: pick rooms with a finger, then set their fields.' : 'Select: drag a box around rooms (1 to 5 work other ways), then set their fields.', true);
    else this.host.say('Select put away.', true);
  }

  setTool(t: PickTool): void {
    this.tool = t;
    write('dd-map-pick', t);
    this.sweep = this.preview = this.brushAt = this.wandAt = null;
    this.dropper = false;
    if (this.on) {
      if (t === 'paint') this.readyPaint(true);
      this.scene.setTool(this);
    }
  }

  /** Paints another field: the map is coloured by it, so the paint shows. */
  setPaintField(id: PaintField): void {
    this.paintField = id;
    this.dropper = false;
    this.recursor();
    if (this.on && this.tool === 'paint') this.readyPaint(true);
  }

  /** Paint's pipette out (the next click takes a room's value) or put back. */
  setDropper(on: boolean): void {
    this.dropper = on;
    this.brushAt = null;
    this.recursor();
    this.scene.draw();
    if (on) this.host.say(`${this.host.coarse ? 'Tap' : 'Click'} a room to take its ${this.paintDef.label.toLowerCase()}.`, true);
  }

  /**
   * Paint taken up: the map coloured by the field it puts on rooms (`say`: and why), and the
   * first time, the paint taken from the room selected.
   */
  private readyPaint(say = false) {
    if (!this.primed) {
      this.primed = true;
      const r = this.host.selected ?? this.scene.by.get(this.scene.start);
      if (r) for (const id of PAINT_FIELDS) this.paints[id] = valuesOf(r, fieldOf(id));
    }
    const by = PAINT_COLOUR[this.paintField];
    this.host.colour(by);
    if (say) this.host.say(`Paint: the map is coloured by ${by} while you paint ${this.paintDef.label.toLowerCase()}, so the paint shows.`, true);
  }

  /** The canvas's cursor follows the pipette. */
  private recursor() {
    if (this.on) this.host.canvas.dataset.tool = this.cursor;
  }

  setMode(m: PickMode): void {
    this.mode = m;
  }

  /** The brush a size larger (1) or smaller (-1). */
  sizeBrush(by: number): void {
    const n = BRUSH_SIZES[BRUSH_SIZES.indexOf(this.brush) + by];
    if (n === undefined) return;
    this.brush = n;
    write('dd-map-brush', String(n));
    this.scene.draw();
  }

  setPicked(next: ReadonlySet<number>): void {
    this.picked = next;
    this.preview = null;
    this.scene.draw();
  }

  /** A pick on the map: a box, a lasso, a brush stroke, a click or the wand's region. */
  choose(rooms: number[], how: PickHow): void {
    const was = this.picked;
    const next = after(was, rooms, how);
    if (next.size === was.size && [...next].every((n) => was.has(n))) return;
    this.setPicked(next);
    this.host.tell(next.size ? `${next.size.toLocaleString()} room${next.size === 1 ? '' : 's'} picked.` : 'No rooms picked.');
  }

  all(): void {
    const rooms = this.scene.rooms;
    this.setPicked(new Set(rooms.map((r) => r.room)));
    this.host.say(`All ${rooms.length.toLocaleString()} rooms of map ${this.host.map} picked.`, true);
  }

  clear(): void {
    this.setPicked(new Set());
  }

  /** The rooms picked, in view. */
  show(): void {
    this.scene.frameRooms(this.picked);
  }

  /** Sets a field on the rooms picked: shown at once, written as one save that one undo takes back. */
  set(f: PickField, values: [string, string | number][]): void {
    const h = this.host;
    const picked = this.rooms;
    if (!picked.length) return;
    const rooms = picked.map((r) => r.room);
    const what = f.label.toLowerCase();
    const m = h.map;
    if (picked.every((r) => holds(r, values))) {
      h.say(rooms.length === 1 ? `Room ${m}/${rooms[0]} has that ${what} already.` : `Those rooms have that ${what} already.`);
      return;
    }
    const set = new Set(rooms);
    h.patch(this.scene.rooms.map((r) => (set.has(r.room) ? withValues(r, values) : r)));
    h.send(
      () => api.mapSet({ map: m, rooms, values, what: f.label }),
      (out) =>
        out.changed >= rooms.length || !out.changed
          ? `Set ${what} ${onRooms(m, rooms)}`
          : `Set ${what} on ${out.changed.toLocaleString()} of the ${rooms.length.toLocaleString()} rooms (the others had it already)`,
    );
    sfx('pop');
  }

  /** A room of this map for a person ("Town square 1/245"), or null when the map has no such room. */
  named(n: number): string | null {
    return this.scene.by.has(n) ? this.host.called(n) : null;
  }

  /** A field's values for a person: "-175, very dark", "Wanderer, index 1 to 10, up to 2", "Town square 1/245". */
  words(id: PickField['id'], one: readonly (string | number | null)[]): string {
    const [a, b, c, d] = one.map(Number) as [number, number, number, number];
    switch (id) {
      case 'name':
        return one[0] ? `“${one[0]}”` : 'No name';
      case 'light':
        return `${a}, ${lightWord(a).toLowerCase()}`;
      case 'monsters':
        return c === 0 || d === 0 ? `${group(a)}, none come` : `${group(a)}, index ${b} to ${c}, up to ${d}`;
      case 'control':
        return a ? (this.named(a) ?? `room ${this.host.map}/${a}, not on this map`) : 'None';
      case 'type':
        return ROOM_TYPE[a] ?? `Type ${a}`;
    }
  }

  /** What is wrong with values typed for a field, if anything. */
  problemOf(f: PickField, nums: readonly (number | null | undefined)[]): string {
    if (f.id === 'name') return '';
    for (const [i, c] of f.columns.entries()) {
      const v = nums[i];
      if (typeof v !== 'number' || !Number.isInteger(v)) return `${c} takes a whole number.`;
      const r = this.ranges[c];
      if (r && (v < r[0] || v > r[1])) return `${c} goes from ${r[0]} to ${r[1]}.`;
    }
    const [a, b, c] = nums as number[];
    if (f.id === 'control' && a && this.named(a) === null) return `Map ${this.host.map} has no room ${a}.`;
    if (f.id === 'monsters' && c && b! > c) return 'Min Index is over Max Index.';
    return '';
  }

  /** What Paint puts on rooms: each of its field's columns, and its value. */
  private paintValues(): [string, number][] {
    const nums = this.paints[this.paintField];
    return this.paintDef.columns.map((c, i) => [c, nums[i] ?? 0]);
  }

  /** A stroke of Paint let go of: the rooms it went over take the paint, shown at once and written as one save that one undo takes back. */
  private apply(stroke: ReadonlySet<number>): void {
    const h = this.host;
    if (!stroke.size) {
      if (this.picked.size) h.say(`Paint stays inside the rooms picked: ${h.coarse ? 'Clear' : 'Esc'} lets go of them.`);
      return;
    }
    const f = this.paintDef;
    const values = this.paintValues();
    const rooms = changing(this.scene.by, stroke, values);
    const what = `${f.label.toLowerCase()} (${this.words(f.id, values.map(([, v]) => v))})`;
    if (!rooms.length) {
      h.say(stroke.size === 1 ? `${h.called([...stroke][0]!)} has that ${f.label.toLowerCase()} already.` : `Those rooms have that ${f.label.toLowerCase()} already.`);
      return;
    }
    const m = h.map;
    const set = new Set(rooms);
    h.patch(this.scene.rooms.map((r) => (set.has(r.room) ? withValues(r, values) : r)));
    h.send(
      // the history's line: "Set Light -175 on 12 rooms of map 1"
      () => api.mapSet({ map: m, rooms, values, what: f.columns.length === 1 ? `${f.columns[0]} ${values[0]![1]}` : f.label }),
      (out) => {
        const n = out.changed && out.changed < rooms.length ? out.changed : rooms.length;
        const had = stroke.size - n;
        return `Painted ${what} ${n === rooms.length ? onRooms(m, rooms) : `on ${n.toLocaleString()} rooms`}${had ? ` (${had.toLocaleString()} had it already)` : ''}`;
      },
    );
    sfx('pop');
  }

  /** Paint's pipette: the paint takes the value of the room at (x, y) on the canvas. */
  private takeAt(x: number, y: number): void {
    // a click beside the rooms leaves the pipette out
    const r = this.scene.hitAt(x, y)?.room;
    if (!r) return;
    if (this.dropper) this.setDropper(false);
    const f = this.paintDef;
    const v = valuesOf(r, f);
    this.paints[this.paintField] = v;
    this.host.say(`Took ${f.label.toLowerCase()} (${this.words(f.id, v)}) from ${this.host.called(r.room)}.`);
    sfx('toggle');
  }

  /** The map's rooms changed (`moved`: it is another map): rooms gone are no longer picked. */
  changed(moved: boolean): void {
    this.seen++;
    this.wandAt = null;
    const by = this.scene.by;
    const p = this.picked;
    if (moved) {
      if (p.size) this.setPicked(new Set());
    } else if ([...p].some((n) => !by.has(n))) this.setPicked(new Set([...p].filter((n) => by.has(n))));
  }

  /** The tooltip's line for a room under the pointer. */
  about(r: MapRoom): string {
    const at = `Room ${this.host.map}/${r.room}`;
    if (this.tool === 'paint') {
      const f = this.paintDef;
      return `${at}: ${f.label.toLowerCase()} ${this.words(f.id, valuesOf(r, f))}.${this.dropper || this.alt ? ` ${this.host.coarse ? 'Tap' : 'Click'} to take it.` : ''}`;
    }
    if (this.tool !== 'wand') return `${at}${this.picked.has(r.room) ? ', picked' : ''}`;
    const k = this.regionOf(r.room).length;
    return `${at}. ${k === 1 ? 'Click to pick it: no room joined to it shares its colour.' : `Click to pick the ${k.toLocaleString()} rooms joined to it in its colour.`}`;
  }

  /**
   * The tool's keys: 1 to 5 pick how, [ and ] size the brush, Esc lets go of the pick or stroke
   * being drawn, then of the pipette, then of the rooms picked, then puts it away. Answers whether
   * the key was one.
   */
  keys(e: KeyboardEvent): boolean {
    if (e.key === 'Escape') {
      if (this.sweep) {
        this.sweep = this.preview = null;
        this.scene.draw();
      } else if (this.dropper) this.setDropper(false);
      else if (this.picked.size) this.clear();
      else this.close();
    } else if (!e.shiftKey && /^Digit[1-5]$/.test(e.code)) this.setTool(PICK_TOOLS[Number(e.code.slice(5)) - 1]!.id);
    else if ((this.tool === 'brush' || this.tool === 'paint') && (e.key === '[' || e.key === ']')) this.sizeBrush(e.key === ']' ? 1 : -1);
    else return false;
    e.preventDefault();
    return true;
  }

  /** Space held down moves the map with a drag, Alt takes Paint's pipette, and Ctrl+A picks every room, while the tool is on (and the World shows). */
  onKey(e: KeyboardEvent, here: boolean): void {
    // let go of, wherever the key is let go
    if (e.type === 'keyup' && e.code === 'Space') this.setPanKey(false);
    if (e.key === 'Alt') this.setAlt(e.type === 'keydown' && this.on && here);
    if (!this.on || !here || e.type !== 'keydown') return;
    const t = e.target;
    if (t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
    if (e.code === 'Space') {
      // a button keeps its Space
      if (t instanceof HTMLElement && /^(BUTTON|A)$/.test(t.tagName)) return;
      e.preventDefault();
      this.setPanKey(true);
    } else if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'a') {
      e.preventDefault();
      this.all();
    }
  }

  /** The window lost the keyboard: keys held down are let go of. */
  letGo(): void {
    this.setPanKey(false);
    this.setAlt(false);
  }

  /** Alt held down (or let go): Paint takes a room's value instead, and the brush makes way for the pipette. */
  private setAlt(down: boolean) {
    if (down === this.alt) return;
    this.alt = down;
    if (this.tool !== 'paint') return;
    this.recursor();
    if (down && this.brushAt) {
      this.brushAt = null;
      this.scene.draw();
    }
  }

  private setPanKey(down: boolean) {
    if (down === this.panKey) return;
    this.panKey = down;
    this.scene.setPanKey(down);
  }

  /** The wand's region from room `n`: the rooms joined to it through rooms of its colour. */
  private regionOf(n: number): number[] {
    return regionOf(this.scene.by, n, (m) => this.scene.tintKey(m));
  }

  // ---- on the map --------------------------------------------------------------------

  down(x: number, y: number, e: PointerEvent): boolean {
    const tool = this.tool;
    // a drag with the wand moves the map; a click picks
    if (tool === 'wand') return false;
    if (tool === 'paint') {
      const take = this.dropper || e.altKey;
      const problem = take ? '' : this.problemOf(this.paintDef, this.paints[this.paintField]);
      if (problem) {
        // held, so the press does nothing
        this.host.say(problem);
        sfx('nope');
        return true;
      }
      this.sweep = { tool, how: 'add', pts: [this.scene.inCells(x, y)], rooms: new Set(), x, y, moved: false, take, tints: new Map() };
      this.preview = null;
      if (!take) {
        this.brushAt = { x, y, drop: false };
        this.paint(x, y, x, y);
      }
      this.scene.draw();
      return true;
    }
    const plain = this.mode === 'set' && tool === 'brush' ? 'add' : this.mode;
    const s: Sweep = { tool, how: howOf(e, plain), pts: [this.scene.inCells(x, y)], rooms: new Set(), x, y, moved: false };
    this.sweep = s;
    this.preview = null;
    if (tool === 'brush') {
      this.brushAt = { x, y, drop: s.how === 'drop' };
      this.paint(x, y, x, y);
    }
    return true;
  }

  move(x: number, y: number, e: PointerEvent, held: boolean): boolean {
    const s = this.sweep;
    if (held) {
      if (!s) return true;
      if (!s.moved && Math.abs(x - s.x) + Math.abs(y - s.y) > 4) s.moved = true;
      if (s.take) return true;
      const at = this.scene.inCells(x, y);
      if (s.tool === 'box') s.pts = [s.pts[0]!, at];
      else if (s.tool === 'lasso') {
        const last = s.pts.at(-1)!;
        if (Math.hypot(at[0] - last[0], at[1] - last[1]) * this.scene.cell > 3) s.pts.push(at);
      } else {
        const b = this.brushAt;
        this.paint(b?.x ?? x, b?.y ?? y, x, y);
        this.brushAt = { x, y, drop: s.how === 'drop' };
      }
      this.scene.draw();
      return true;
    }
    if (this.tool === 'paint') this.setAlt(e.altKey);
    // the pipette: the room under the pointer shows, with its value
    const brushes = this.tool === 'brush' || (this.tool === 'paint' && !this.dropper && !this.alt);
    if (!brushes || e.pointerType === 'touch') {
      if (this.brushAt) {
        this.brushAt = null;
        this.scene.draw();
      }
      return false;
    }
    // the brush shows where it would go, and how much it takes
    this.brushAt = { x, y, drop: this.tool === 'brush' && (e.altKey || (this.mode === 'drop' && !e.shiftKey)) };
    this.scene.draw();
    return true;
  }

  up(x: number, y: number, e: PointerEvent, cancelled: boolean): void {
    const s = this.sweep;
    this.sweep = null;
    if (e.pointerType === 'touch') this.brushAt = null;
    this.scene.draw();
    if (!s || cancelled) return;
    if (s.tool === 'paint') {
      if (!s.take) this.apply(s.rooms);
      else if (!s.moved) this.takeAt(x, y);
    } else if (s.tool === 'brush') this.choose([...s.rooms], s.how);
    else if (s.moved) this.choose(this.roomsIn(s.tool, s.pts), s.how);
    else this.click(x, y, e);
  }

  /**
   * A click: the wand takes the rooms joined to the room clicked in its colour, the others that
   * room (clickHow). Empty space lets go of every room, unless adding or taking away.
   */
  click(x: number, y: number, m: { shiftKey: boolean; altKey: boolean; ctrlKey: boolean; metaKey: boolean }): void {
    const n = this.scene.hitAt(x, y)?.room?.room;
    if (n === undefined) {
      if (this.mode === 'set' && !m.shiftKey && !m.altKey && !m.ctrlKey && !m.metaKey) this.choose([], 'set');
      return;
    }
    if (this.tool === 'wand') this.choose(this.regionOf(n), howOf(m, this.mode));
    else this.choose([n], clickHow(m, this.mode));
  }

  leave(): void {
    if (!this.brushAt) return;
    this.brushAt = null;
    this.scene.draw();
  }

  /** The rooms whose middles a box (two corners) or a lasso (its points), in cells, holds. */
  private roomsIn(tool: PickTool, pts: [number, number][]): number[] {
    const l = this.scene.layout;
    if (!l || !pts.length) return [];
    let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
    for (const [x, y] of tool === 'lasso' ? pts : [pts[0]!, pts.at(-1)!]) {
      [x0, x1, y0, y1] = [Math.min(x0, x), Math.max(x1, x), Math.min(y0, y), Math.max(y1, y)];
    }
    const out: number[] = [];
    for (const [n, p] of l.at) {
      const [x, y] = [p.x + 0.5, p.y + 0.5];
      if (x < x0 || x > x1 || y < y0 || y > y1) continue;
      if (tool !== 'lasso' || inside(x, y, pts)) out.push(n);
    }
    return out;
  }

  /** The brush drawn from (x0, y0) to (x1, y1) on the canvas: the rooms it passes over join its stroke (Paint's, only rooms picked, when some are). */
  private paint(x0: number, y0: number, x1: number, y1: number) {
    const s = this.sweep;
    if (!s) return;
    const mask = s.tool === 'paint' && this.picked.size ? this.picked : null;
    const steps = Math.max(1, Math.ceil(Math.hypot(x1 - x0, y1 - y0) / (this.reach() * 0.75)));
    for (let i = 1; i <= steps; i++) {
      for (const n of this.brushRooms(x0 + ((x1 - x0) * i) / steps, y0 + ((y1 - y0) * i) / steps)) if (!mask || mask.has(n)) s.rooms.add(n);
    }
  }

  /** How far the brush reaches from its middle to the middles of the rooms it takes, on the canvas. */
  private reach(): number {
    return Math.max(4, ((this.brush - 1) / 2 + 0.55) * this.scene.cell);
  }

  /** The rooms under the brush at (px, py). */
  private brushRooms(px: number, py: number): number[] {
    const sc = this.scene;
    const l = sc.layout;
    if (!l) return [];
    const r = this.reach();
    const [x0, y0] = sc.inCells(px - r, py - r);
    const [x1, y1] = sc.inCells(px + r, py + r);
    const out: number[] = [];
    for (let gx = Math.floor(x0); gx <= Math.floor(x1); gx++) {
      for (let gy = Math.floor(y0); gy <= Math.floor(y1); gy++) {
        const n = l.cell.get(cellKey(gx, gy));
        if (n !== undefined && Math.hypot(sc.sx(gx) - px, sc.sy(gy) - py) <= r) out.push(n);
      }
    }
    return out;
  }

  /** The rooms to draw picked: those picked, or while a pick is drawn, as they would be once it is let go. */
  private shown(): ReadonlySet<number> {
    const s = this.sweep;
    if (!s || s.tool === 'paint') return this.picked;
    const key = s.tool === 'brush' ? s.rooms.size : `${s.pts.length},${s.pts.at(-1)!.join(',')}`;
    if (this.preview?.of !== key) {
      const rooms = s.tool === 'brush' ? [...s.rooms] : s.pts.length > 1 ? this.roomsIn(s.tool, s.pts) : [];
      this.preview = { of: key, rooms: after(this.picked, rooms, s.how) };
    }
    return this.preview.rooms;
  }

  // ---- drawing ---------------------------------------------------------------------

  /**
   * The rooms picked (as they would be while a pick is drawn: with Paint, only their edge, so
   * the colours show), a stroke of Paint in the colours it gives, and the rooms the wand would
   * take from the room under the pointer.
   */
  under(p: Pen): void {
    const picks = this.shown();
    if (picks.size) this.drawPicks(p, picks, this.tool === 'paint' ? 0 : 0.26, false);
    const s = this.sweep;
    if (s?.tool === 'paint' && s.rooms.size) this.drawStroke(p, s.rooms, s.tints!);
    const h = this.scene.hovered;
    const wr = this.tool === 'wand' && !this.sweep && !h?.stub ? h?.room : undefined;
    if (!wr) return;
    const rooms = this.scene.rooms;
    if (this.wandAt?.n !== wr.room || this.wandAt.of !== rooms) this.wandAt = { n: wr.room, of: rooms, rooms: new Set(this.regionOf(wr.room)) };
    this.drawPicks(p, this.wandAt.rooms, 0.12, true);
  }

  /** The pick being drawn: the box or the lasso, and the brush. */
  over({ ctx, pal: P }: Pen): void {
    const sw = this.sweep;
    if (sw && sw.tool !== 'brush' && sw.pts.length > 1) {
      const sc = this.scene;
      ctx.beginPath();
      if (sw.tool === 'box') {
        const [ax, ay] = sc.onCanvas(...sw.pts[0]!);
        const [bx, by] = sc.onCanvas(...sw.pts.at(-1)!);
        ctx.rect(Math.min(ax, bx), Math.min(ay, by), Math.abs(bx - ax), Math.abs(by - ay));
      } else {
        for (const q of sw.pts) ctx.lineTo(...sc.onCanvas(...q));
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
    const bt = this.tool === 'brush' || this.tool === 'paint' ? this.brushAt : null;
    if (!bt) return;
    const r = Math.max(6, this.reach());
    ctx.beginPath();
    ctx.arc(bt.x, bt.y, r, 0, TAU);
    // Paint's brush is filled with its paint
    const paint = this.tool === 'paint' ? this.paintTint() : null;
    ctx.fillStyle = paint ?? P.sel;
    ctx.globalAlpha = paint ? (sw ? 0.55 : 0.4) : sw ? 0.14 : 0.07;
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

  /** The rooms picked, on the overview. */
  mini({ ctx, pal, s, ox, oy, dpr }: MiniPen): void {
    const l = this.scene.layout;
    if (!l || !this.picked.size) return;
    ctx.fillStyle = pal.sel;
    const d = Math.max(2 * dpr, s);
    for (const n of this.picked) {
      const q = l.at.get(n);
      if (q) ctx.fillRect(ox + q.x * s - (d - s) / 2, oy + q.y * s - (d - s) / 2, d, d);
    }
  }

  /** The colour Paint gives a room, as the map is coloured. */
  private paintTint(): string | null {
    const r = this.scene.rooms[0];
    return r ? this.scene.tintFor(withValues(r, this.paintValues())).colour : null;
  }

  /** A stroke of Paint as it goes on: its rooms in the colours they take, and its edge. */
  private drawStroke(p: Pen, rooms: ReadonlySet<number>, tints: Map<number, string>) {
    const { ctx, pal: P, z, half, rad, inView } = p;
    const sc = this.scene;
    const l = sc.layout;
    if (!l) return;
    const values = this.paintValues();
    const groups = new Map<string, number[]>();
    for (const n of rooms) {
      if (!inView(n)) continue;
      let colour = tints.get(n);
      if (colour === undefined) {
        const r = sc.by.get(n);
        if (!r) continue;
        colour = sc.tintFor(withValues(r, values)).colour;
        tints.set(n, colour);
      }
      const g = groups.get(colour);
      if (g) g.push(n);
      else groups.set(colour, [n]);
    }
    for (const [colour, list] of groups) {
      ctx.fillStyle = colour;
      ctx.beginPath();
      for (const n of list) {
        const q = l.at.get(n)!;
        const [x, y] = [sc.sx(q.x), sc.sy(q.y)];
        if (half < 2) ctx.rect(x - half, y - half, half * 2, half * 2);
        else ctx.roundRect(x - half, y - half, half * 2, half * 2, rad);
      }
      ctx.fill();
      if (z >= 0.3) {
        ctx.strokeStyle = P.line;
        ctx.lineWidth = z < 0.55 ? 1 : 1.6;
        ctx.stroke();
      }
    }
    this.drawPicks(p, rooms, 0, false);
  }

  /**
   * Rooms picked: their cells washed over and the edge of each patch of them drawn, so a patch
   * reads as one place (dashed: the rooms the wand would take).
   */
  private drawPicks({ ctx, pal: P, z, cell: c, inView }: Pen, rooms: ReadonlySet<number>, wash: number, dashed: boolean) {
    const sc = this.scene;
    const l = sc.layout;
    if (!l) return;
    const shown: [number, number, number, number][] = [];
    for (const n of rooms) {
      if (!inView(n)) continue;
      const p = l.at.get(n)!;
      shown.push([p.x, p.y, sc.sx(p.x) - c / 2, sc.sy(p.y) - c / 2]);
    }
    if (!shown.length) return;
    if (wash) {
      ctx.fillStyle = P.sel;
      ctx.globalAlpha = wash;
      ctx.beginPath();
      for (const [, , x, y] of shown) ctx.rect(x, y, c, c);
      ctx.fill();
      ctx.globalAlpha = 1;
    }
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
}
