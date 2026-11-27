// The Select tool: rooms picked on the map with a box, a lasso, the brush or the wand (the rooms
// joined to one in its colour), and their fields set together as one save. Its code loads with
// its panel the first time the tool is taken up; the rooms picked wait while it is put away.

import { api } from '../api';
import { read, sfx, write } from '../shell/prefs.svelte';
import { app } from '../shell/state.svelte';
import type { Host } from './host';
import { cellKey } from './layout';
import type { MapRoom } from './model';
import { after, BRUSH_SIZES, clickHow, howOf, inside, mapValue, onRooms, PICK_TOOLS, regionOf, withValues, type PickField, type PickHow, type PickMode, type PickTool } from './picks';
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
}

const toolFrom = (v: string): PickTool => PICK_TOOLS.find((t) => t.id === v)?.id ?? 'box';
const brushFrom = (v: string): number => (BRUSH_SIZES.includes(Number(v)) ? Number(v) : 3);

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
  private readonly close: () => void;

  constructor(host: Host, close: () => void) {
    this.host = host;
    this.close = close;
  }

  get cursor(): string {
    return this.tool;
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
    if (!on) this.setPanKey(false);
    this.scene.setTool(on ? this : null);
    const coarse = this.host.coarse;
    this.host.say(on ? (coarse ? 'Select: pick rooms with a finger, then set their fields.' : 'Select: drag a box around rooms (1 to 4 pick other ways), then set their fields.') : 'Select put away.', true);
  }

  setTool(t: PickTool): void {
    this.tool = t;
    write('dd-map-pick', t);
    this.sweep = this.preview = this.brushAt = this.wandAt = null;
    if (this.on) this.scene.setTool(this);
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
    const same = (r: MapRoom) => values.every(([c, v]) => mapValue(r, c) === (typeof v === 'string' ? v.trim() : v));
    if (picked.every(same)) {
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
    if (this.tool !== 'wand') return `${at}${this.picked.has(r.room) ? ', picked' : ''}`;
    const k = this.regionOf(r.room).length;
    return `${at}. ${k === 1 ? 'Click to pick it: no room joined to it shares its colour.' : `Click to pick the ${k.toLocaleString()} rooms joined to it in its colour.`}`;
  }

  /** The tool's keys: 1 to 4 pick how, [ and ] size the brush, Esc lets go of the pick being drawn, then of the rooms picked, then puts it away. Answers whether the key was one. */
  keys(e: KeyboardEvent): boolean {
    if (e.key === 'Escape') {
      if (this.sweep) {
        this.sweep = this.preview = null;
        this.scene.draw();
      } else if (this.picked.size) this.clear();
      else this.close();
    } else if (!e.shiftKey && /^Digit[1-4]$/.test(e.code)) this.setTool(PICK_TOOLS[Number(e.code.slice(5)) - 1]!.id);
    else if (this.tool === 'brush' && (e.key === '[' || e.key === ']')) this.sizeBrush(e.key === ']' ? 1 : -1);
    else return false;
    e.preventDefault();
    return true;
  }

  /** Space held down moves the map with a drag, and Ctrl+A picks every room, while the tool is on (and the World shows). */
  onKey(e: KeyboardEvent, here: boolean): void {
    // let go of, wherever the key is let go
    if (e.type === 'keyup' && e.code === 'Space') this.setPanKey(false);
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
    if (this.tool !== 'brush' || e.pointerType === 'touch') return false;
    // the brush shows where it would go, and how much it takes
    this.brushAt = { x, y, drop: e.altKey || (this.mode === 'drop' && !e.shiftKey) };
    this.scene.draw();
    return true;
  }

  up(x: number, y: number, e: PointerEvent, cancelled: boolean): void {
    const s = this.sweep;
    this.sweep = null;
    if (e.pointerType === 'touch') this.brushAt = null;
    this.scene.draw();
    if (!s || cancelled) return;
    if (s.tool === 'brush') this.choose([...s.rooms], s.how);
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

  /** The brush drawn from (x0, y0) to (x1, y1) on the canvas: the rooms it passes over join its stroke. */
  private paint(x0: number, y0: number, x1: number, y1: number) {
    const s = this.sweep;
    if (!s) return;
    const steps = Math.max(1, Math.ceil(Math.hypot(x1 - x0, y1 - y0) / (this.reach() * 0.75)));
    for (let i = 1; i <= steps; i++) {
      for (const n of this.brushRooms(x0 + ((x1 - x0) * i) / steps, y0 + ((y1 - y0) * i) / steps)) s.rooms.add(n);
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
    if (!s) return this.picked;
    const key = s.tool === 'brush' ? s.rooms.size : `${s.pts.length},${s.pts.at(-1)!.join(',')}`;
    if (this.preview?.of !== key) {
      const rooms = s.tool === 'brush' ? [...s.rooms] : s.pts.length > 1 ? this.roomsIn(s.tool, s.pts) : [];
      this.preview = { of: key, rooms: after(this.picked, rooms, s.how) };
    }
    return this.preview.rooms;
  }

  // ---- drawing ---------------------------------------------------------------------

  /** The rooms picked (as they would be while a pick is drawn), and those the wand would take from the room under the pointer. */
  under(p: Pen): void {
    const picks = this.shown();
    if (picks.size) this.drawPicks(p, picks, 0.26, false);
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
    const bt = this.tool === 'brush' ? this.brushAt : null;
    if (!bt) return;
    const r = Math.max(6, this.reach());
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
}
