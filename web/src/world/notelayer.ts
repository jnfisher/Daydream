// The builder's notes drawn with the map (notes.ts), as the scene's layer: regions under the
// rooms, their names, labels and the notes' marks over them, and what of them the pointer is over.

import type { IconNode } from './icons';
import type { Layout, Spot } from './layout';
import { iconPaths, type Palette } from './look';
import type { MapNote, NoteColour } from './model';
import { colourOf, type Corner, outline, toDo } from './notes';
import type { Box, MapLayer, MiniPen, Pen } from './scene';

/** Traces loops of corners on the canvas, `inset` pixels inside their cells, with rounded corners. */
function trace(ctx: CanvasRenderingContext2D, loops: Corner[][], x0: number, y0: number, cell: number, inset: number) {
  const r = Math.min(10, cell * 0.28);
  for (const loop of loops) {
    const n = loop.length;
    if (n < 3) continue;
    const pts = loop.map((c, i): [number, number] => {
      const p = loop[(i + n - 1) % n]!;
      const q = loop[(i + 1) % n]!;
      // each side's cells are on its right: (dx, dy) turned a quarter clockwise is (−dy, dx)
      const [ax, ay] = [Math.sign(c[0] - p[0]), Math.sign(c[1] - p[1])];
      const [bx, by] = [Math.sign(q[0] - c[0]), Math.sign(q[1] - c[1])];
      return [x0 + (c[0] / 2) * cell - inset * (ay + by), y0 + (c[1] / 2) * cell + inset * (ax + bx)];
    });
    const last = pts[n - 1]!;
    ctx.moveTo((last[0] + pts[0]![0]) / 2, (last[1] + pts[0]![1]) / 2);
    for (let i = 0; i < n; i++) {
      const c = pts[i]!;
      const q = pts[(i + 1) % n]!;
      ctx.arcTo(c[0], c[1], q[0], q[1], r);
    }
    ctx.closePath();
  }
}

// Lucide's sticky-note, square and square-check (v1.71.0, ISC licence)
const STICKY: IconNode = [['path', { d: 'M21 9a2.4 2.4 0 0 0-.706-1.706l-3.588-3.588A2.4 2.4 0 0 0 15 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2z' }], ['path', { d: 'M15 3v5a1 1 0 0 0 1 1h5' }]];
const BOX: IconNode = [['rect', { width: '18', height: '18', x: '3', y: '3', rx: '2' }]];
const TICKED: IconNode = [['rect', { width: '18', height: '18', x: '3', y: '3', rx: '2' }], ['path', { d: 'm16 9-5.5 5.5L8 12' }]];

/** How far a label's tail reaches down to its room. */
const TAIL = 5;

/** The ink on a note's pastel: dark in both themes, as the pastels are light in both. */
const ON_PASTEL = '#241d45';

/** A colour's three tones from the app's tokens: the pastel, its soft ground, and its ink. */
interface Hue {
  base: string;
  soft: string;
  ink: string;
}

/** Where something of the layer was drawn, to point at: a circle, or a box. */
type Spot2 = { id: string; x: number; y: number; r: number } | { id: string; box: Box };

/** A region's outline, worked out for one layout. */
interface Shape {
  layout: Layout;
  rooms: readonly number[];
  loops: Corner[][];
  /** Its top row's cells and its bottom row's, left to right (the first few): where its name can go. */
  top: Spot[];
  bottom: Spot[];
  /** Its cells' bounds. */
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** The notes drawn with the map: the scene's layer. */
export class NoteLayer implements MapLayer {
  notes: readonly MapNote[] = [];
  /** The note the pointer is over, and the one being written: drawn hot. */
  hot: string | null = null;
  open: string | null = null;
  private spots: Spot2[] = [];
  private shapes = new WeakMap<MapNote, Shape>();
  private hues = new Map<NoteColour, Hue>();
  private huesFor: Palette | null = null;

  private hue(c: NoteColour, pal: Palette): Hue {
    if (pal !== this.huesFor) {
      this.hues.clear();
      this.huesFor = pal;
    }
    let h = this.hues.get(c);
    if (!h) {
      const css = getComputedStyle(document.documentElement);
      const v = (name: string) => css.getPropertyValue(name).trim() || '#c4b0ff';
      h = { base: v(`--${c}`), soft: v(`--${c}-2`), ink: v(`--${c}-ink`) };
      this.hues.set(c, h);
    }
    return h;
  }

  /** A region's outline on the level shown (none: none of its rooms is there). */
  private shape(n: MapNote, p: Pen): Shape | null {
    let s = this.shapes.get(n);
    if (!s || s.layout !== p.layout || s.rooms !== n.rooms) {
      const cells = n.rooms.map((r) => p.at(r)).filter((c): c is Spot => !!c);
      let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
      for (const c of cells) [x0, y0, x1, y1] = [Math.min(x0, c.x), Math.min(y0, c.y), Math.max(x1, c.x), Math.max(y1, c.y)];
      const row = (y: number) => cells.filter((c) => c.y === y).sort((a, b) => a.x - b.x).slice(0, 8);
      s = { layout: p.layout, rooms: n.rooms, loops: outline(cells), top: row(y0), bottom: row(y1), x0, y0, x1, y1 };
      this.shapes.set(n, s);
    }
    return s.top.length ? s : null;
  }

  /** Whether a region's cells are in view (or nearly). */
  private seen(s: Shape, p: Pen): boolean {
    const h = p.cell;
    return p.sx(s.x1) + h >= 0 && p.sx(s.x0) - h <= p.W && p.sy(s.y1) + h >= 0 && p.sy(s.y0) - h * 2 <= p.H;
  }

  back(p: Pen): void {
    const { ctx } = p;
    for (const n of this.notes) {
      if (n.kind !== 'region') continue;
      const s = this.shape(n, p);
      if (!s || !this.seen(s, p)) continue;
      const hue = this.hue(colourOf(n), p.pal);
      const hot = this.hot === n.id || this.open === n.id;
      ctx.beginPath();
      trace(ctx, s.loops, p.sx(0), p.sy(0), p.cell, Math.max(1.5, p.cell * 0.06));
      ctx.fillStyle = hue.base;
      ctx.globalAlpha = hot ? 0.26 : 0.16;
      ctx.fill();
      ctx.globalAlpha = 0.95;
      ctx.strokeStyle = hot ? p.pal.sel : hue.base;
      ctx.lineWidth = hot ? 2.6 : 2;
      ctx.setLineDash(hot ? [] : [7, 4]);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.globalAlpha = 1;
    }
  }

  front(p: Pen, taken: Box[]): void {
    const { ctx, z } = p;
    this.spots = [];
    const clash = (b: Box) => taken.find((t) => b[0] < t[0] + t[2] && t[0] < b[0] + b[2] && b[1] < t[1] + t[3] && t[1] < b[1] + b[3]);
    /** Moves a box up, clear of what is in the way (a few times at most); answers whether it moved. */
    const lift = (b: Box): boolean => {
      let moved = false;
      for (let i = 0, hit = clash(b); hit && i < 4; i++, hit = clash(b)) {
        b[1] = hit[1] - 3 - b[3];
        moved = true;
      }
      return moved;
    };
    ctx.textBaseline = 'middle';
    // labels, each over the room it is by: placed first, clear of the selected room's name and of each other
    const fs = z < 0.6 ? 11 : z < 1.6 ? 12.5 : 14;
    const labelFont = `700 ${fs}px Fredoka, Nunito, sans-serif`;
    const labels: { n: MapNote; box: Box; tail: boolean }[] = [];
    if (z >= 0.25) {
      ctx.font = labelFont;
      const h = fs + 9;
      for (const n of this.notes) {
        if (n.kind !== 'label') continue;
        const c = p.at(n.rooms[0]!);
        if (!c || !p.inView(n.rooms[0]!)) continue;
        const w = Math.min(ctx.measureText(n.text).width, 260) + 16;
        // with its tail down to the room, unless it had to move
        const box: Box = [p.sx(c.x) - w / 2, p.sy(c.y) - p.half - 3 - TAIL - h, w, h + TAIL];
        const tail = !lift(box);
        if (!tail) box[1] += TAIL;
        box[3] = h;
        taken.push(box);
        labels.push({ n, box, tail });
      }
    }
    // regions' names, on a tab along the top of each, else along its bottom, clear of the labels
    const tabFont = `700 ${z < 0.7 ? 11 : 12.5}px Fredoka, Nunito, sans-serif`;
    const tabs: { n: MapNote; box: Box; below: boolean }[] = [];
    if (z >= 0.3) {
      ctx.font = tabFont;
      const h = z < 0.7 ? 16 : 19;
      const inset = Math.max(1.5, p.cell * 0.06);
      for (const n of this.notes) {
        if (n.kind !== 'region') continue;
        const s = this.shape(n, p);
        if (!s || !this.seen(s, p)) continue;
        const w = ctx.measureText(n.text).width + 14;
        const left = (c: Spot) => p.sx(c.x) - p.cell / 2 + inset;
        const tries: { box: Box; below: boolean }[] = [
          ...s.top.map((c) => ({ box: [left(c), p.sy(c.y) - p.cell / 2 + inset - h + 1, w, h] as Box, below: false })),
          ...s.bottom.map((c) => ({ box: [left(c), p.sy(c.y) + p.cell / 2 - inset - 1, w, h] as Box, below: true })),
        ];
        // nowhere clear: over its top left corner, moved up out of the way
        const tab = tries.find((t) => !clash(t.box)) ?? tries[0]!;
        lift(tab.box);
        taken.push(tab.box);
        tabs.push({ n, ...tab });
      }
    }
    // drawn under the labels, as the labels are drawn under the notes' marks
    ctx.font = tabFont;
    ctx.textAlign = 'left';
    for (const { n, box, below } of tabs) {
      const [x, y, w, h] = box;
      const hue = this.hue(colourOf(n), p.pal);
      ctx.fillStyle = hue.base;
      ctx.strokeStyle = this.hot === n.id || this.open === n.id ? p.pal.sel : hue.base;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.roundRect(x, y, w, h, below ? [2, 8, 8, 8] : [8, 8, 8, 2]);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = ON_PASTEL;
      ctx.fillText(n.text, x + 7, y + h / 2 + 0.5);
      this.spots.push({ id: n.id, box });
    }
    ctx.font = labelFont;
    ctx.textAlign = 'center';
    for (const { n, box, tail } of labels) {
      const [x0, y, w, h] = box;
      const x = x0 + w / 2;
      const hue = this.hue(colourOf(n), p.pal);
      const hot = this.hot === n.id || this.open === n.id;
      ctx.fillStyle = hue.soft;
      ctx.strokeStyle = hot ? p.pal.sel : hue.base;
      ctx.lineWidth = hot ? 2.6 : 1.8;
      ctx.beginPath();
      ctx.roundRect(x0, y, w, h, h / 2);
      if (tail) {
        // a tail down to the room
        ctx.moveTo(x - 4.5, y + h);
        ctx.lineTo(x, y + h + TAIL);
        ctx.lineTo(x + 4.5, y + h);
      }
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = hue.ink;
      ctx.fillText(n.text, x, y + h / 2 + 1, 260);
      this.spots.push({ id: n.id, box });
    }
    // notes: a mark on the room's bottom right corner
    if (z >= 0.35) {
      const r = Math.max(3.4, Math.min(8.5, 4.6 * z));
      for (const n of this.notes) {
        if (n.kind !== 'note') continue;
        const room = n.rooms[0]!;
        const c = p.at(room);
        if (!c || !p.inView(room)) continue;
        const x = p.sx(c.x) + p.half;
        const y = p.sy(c.y) + p.half;
        const hue = this.hue(colourOf(n), p.pal);
        const hot = this.hot === n.id || this.open === n.id;
        ctx.globalAlpha = n.done ? 0.55 : 1;
        ctx.fillStyle = hue.base;
        ctx.strokeStyle = hot ? p.pal.sel : p.pal.line;
        ctx.lineWidth = hot ? 2.6 : 1.3;
        ctx.beginPath();
        ctx.arc(x, y, hot ? r + 1.5 : r, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        if (r > 5.5) {
          const size = r * 1.25;
          const k = size / 24;
          ctx.save();
          ctx.translate(x - size / 2, y - size / 2);
          ctx.scale(k, k);
          ctx.strokeStyle = ON_PASTEL;
          ctx.lineWidth = 2.6;
          ctx.lineCap = 'round';
          ctx.lineJoin = 'round';
          for (const path of iconPaths(n.todo ? (n.done ? TICKED : BOX) : STICKY)) ctx.stroke(path);
          ctx.restore();
        }
        ctx.globalAlpha = 1;
        this.spots.push({ id: n.id, x, y, r });
      }
    }
  }

  mini(p: MiniPen): void {
    const { ctx, s, ox, oy } = p;
    for (const n of this.notes) {
      if (n.kind !== 'region') continue;
      ctx.fillStyle = this.hue(colourOf(n), p.pal).base;
      ctx.globalAlpha = 0.55;
      ctx.beginPath();
      for (const r of n.rooms) {
        const c = p.at(r);
        if (c) ctx.rect(ox + (c.x - 0.15) * s, oy + (c.y - 0.15) * s, s * 1.1, s * 1.1);
      }
      ctx.fill();
    }
    // the things left to do, so they can be found
    ctx.globalAlpha = 1;
    const d = Math.max(2.5 * p.dpr, s);
    for (const n of this.notes) {
      if (!toDo(n)) continue;
      const c = p.at(n.rooms[0]!);
      if (!c) continue;
      ctx.fillStyle = this.hue(colourOf(n), p.pal).base;
      ctx.strokeStyle = p.pal.line;
      ctx.lineWidth = p.dpr;
      ctx.beginPath();
      ctx.arc(ox + c.x * s + s * 0.4, oy + c.y * s + s * 0.4, d, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
  }

  hit(x: number, y: number): string | null {
    for (let i = this.spots.length - 1; i >= 0; i--) {
      const s = this.spots[i]!;
      if ('box' in s) {
        const [bx, by, bw, bh] = s.box;
        if (x >= bx - 2 && x <= bx + bw + 2 && y >= by - 2 && y <= by + bh + 2) return s.id;
      } else if (Math.hypot(x - s.x, y - s.y) <= s.r + 4) return s.id;
    }
    return null;
  }
}
