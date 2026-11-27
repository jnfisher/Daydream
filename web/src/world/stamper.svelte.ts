// A stamp held over the map: its shape, size, turn and mirror, where it is aimed, and what
// putting it down there would do, drawn see-through over the map with each cell and link tinted
// by what it would do. A click puts it down as one save, which one undo takes back with the
// places its rooms are drawn in. Its code loads with the drawer, the first time it opens.

import { api } from '../api';
import { sfx } from '../shell/prefs.svelte';
import { localStep, realmChanged } from '../shell/state.svelte';
import type { Host } from './host';
import { cellsOf, STEP, type Spot } from './layout';
import { KEY_DIR } from './model';
import { kindOf, saved } from './saved.svelte';
import type { MapTool, Pen } from './scene';
import { brushOf, named, placed, request, stamped, STAMPS, summary, turned, UPRIGHT, type LinkSort, type Placed, type StampKind, type StampOptions, type Turn } from './stamps';

const TAU = Math.PI * 2;

export class Stamper implements MapTool {
  readonly cursor = 'stamp';
  readonly host: Host;
  /** The stamp held, its sizes, its roll, how it is turned, and whether it joins the rooms it touches. */
  kind = $state.raw<StampKind | null>(null);
  sizes = $state.raw<number[]>([]);
  seed = $state(1);
  way = $state.raw<Turn>(UPRIGHT);
  seam = $state(false);
  /** Leave the rooms already there alone, and replace the exits in the way: switched on, or held down (Shift, Alt). */
  keep = $state(false);
  overwrite = $state(false);
  mods = $state.raw({ shift: false, alt: false });
  /** The cell the held stamp's first cell is over. */
  aim = $state.raw<[number, number] | null>(null);
  /** The held stamp as it would go down where it is aimed, and the room its new rooms copy. */
  ghost = $state.raw<Placed | null>(null);
  brush = $state<number | null>(null);
  /** The cell the pointer last aimed it at: a second tap there puts it down. */
  private aimed: string | null = null;
  private readonly close: () => void;

  constructor(host: Host, close: () => void) {
    this.host = host;
    this.close = close;
  }

  /** What the held stamp would do where it is aimed, for the drawer. */
  said = $derived.by(() => {
    const g = this.ghost;
    if (!g) return '';
    const makes = g.rooms.length || g.links.some((l) => l.sort === 'make' || l.sort === 'replace');
    if (!makes) return g.count.left ? 'Here: nothing, the exits in its way are left (Alt replaces them).' : 'Here: nothing, it is all there already.';
    return `Here: ${summary(this.host.map, g)}.`;
  });

  /** The drawer opened (the stamps saved from the map read afresh) or put away (and the stamp held with it). */
  opened(on: boolean): void {
    if (on) void saved.load();
    else {
      this.pick(null);
      this.keep = this.overwrite = false;
    }
    this.host.say(on ? (this.host.coarse ? 'Stamps: pick a shape, then tap the map where it goes.' : 'Stamps: pick a shape (1 to 9), then click the map to put it down.') : 'Stamps put away.', true);
  }

  /** Picks a stamp up (or puts the one held away). */
  pick(k: StampKind | null): void {
    const h = this.host;
    const sc = h.scene;
    this.kind = k;
    this.aimed = null;
    if (k) {
      this.sizes = k.sizes.map((z) => z.start);
      this.way = UPRIGHT;
      this.seed = Math.floor(Math.random() * 2 ** 31);
      this.aim ??= this.startAim();
      // too far out to aim at a cell: come in close where it aims
      if (this.aim && sc.cam.z < 0.35) sc.lookAt(this.aim[0], this.aim[1], 0.75, this.openMiddle());
      h.say(
        h.coarse
          ? `${k.name}: tap the map to aim it, and tap it again to put it down.`
          : `${k.name}: click the map to put it down. R turns it, X and Y mirror it${k.sizes.length ? ', [ and ] size it' : ''}.`,
        true,
      );
    } else {
      this.aim = null;
      this.mods = { shift: false, alt: false };
    }
    sc.setTool(k ? this : null);
    this.reghost();
  }

  /** Picks up a stamp saved from the map, by its id. */
  pickSaved(id: string): void {
    const s = saved.list.find((o) => o.id === id);
    if (s) this.pick(kindOf(s));
  }

  /** Deletes the saved stamp held (Ctrl+Z brings it back). */
  async drop(): Promise<void> {
    const s = saved.list.find((o) => o.id === this.kind?.saved);
    if (s && (await saved.remove(s))) this.pick(null);
  }

  size(i: number, by: number): void {
    const z = this.kind?.sizes[i];
    if (!z) return;
    this.sizes = this.sizes.map((v, j) => (j === i ? Math.max(z.min, Math.min(z.max, v + by)) : v));
    this.reghost();
  }

  /** A quarter turn clockwise, as the map shows it. */
  rotate(): void {
    this.way = { ...this.way, turns: (this.way.turns + 1) % 4 };
    this.reghost();
  }

  /** Mirrors the stamp as the map shows it: once turned a quarter, across is up and down before the turn. */
  flip(axis: 'x' | 'y'): void {
    const w = this.way;
    this.way = (axis === 'x') === (w.turns % 2 === 0) ? { ...w, flipX: !w.flipX } : { ...w, flipY: !w.flipY };
    this.reghost();
  }

  roll(): void {
    this.seed = Math.floor(Math.random() * 2 ** 31);
    this.reghost();
  }

  setSeam(on: boolean): void {
    this.seam = on;
    this.reghost();
  }

  setKeep(on: boolean): void {
    this.keep = on;
    this.reghost();
  }

  setOverwrite(on: boolean): void {
    this.overwrite = on;
    this.reghost();
  }

  /** The map's rooms or places changed (`moved`: it is another map). */
  changed(moved: boolean): void {
    if (moved) this.aim = null;
    this.reghost();
  }

  /** The drawer's keys: 1 to 9 pick a stamp, and the one held is moved, turned, sized and put down. Answers whether the key was one. */
  keys(e: KeyboardEvent): boolean {
    const k = this.kind;
    const key = e.key.toLowerCase();
    const d = KEY_DIR[e.code] ?? KEY_DIR[e.key];
    if (e.key === 'Escape') {
      if (k) this.pick(null);
      else this.close();
    } else if (!e.shiftKey && /^Digit[1-9]$/.test(e.code)) {
      const next = STAMPS[Number(e.code.slice(5)) - 1] ?? null;
      this.pick(next && next.id !== k?.id ? next : null);
    } else if (!k) return false;
    else if (d !== undefined && !e.shiftKey) {
      const [dx, dy] = STEP[d]!;
      const a = this.aim ? ([this.aim[0] + dx, this.aim[1] + dy] as [number, number]) : this.startAim();
      this.aim = a;
      if (a) this.host.scene.showCell(a[0], a[1]);
      this.reghost();
    } else if (e.key === 'Enter') {
      if (this.aim) this.put(this.aim[0], this.aim[1], { shift: e.shiftKey, alt: false });
    } else if (key === 'r') this.rotate();
    else if (key === 'x') this.flip('x');
    else if (key === 'y') this.flip('y');
    else if (e.key === '[') this.size(0, -1);
    else if (e.key === ']') this.size(0, 1);
    else if (e.key === '{') this.size(1, -1);
    else if (e.key === '}') this.size(1, 1);
    else if (e.key === ' ' && k.random) this.roll();
    else if (key === 'j') this.setSeam(!this.seam);
    else return false;
    e.preventDefault();
    return true;
  }

  /** Shift and Alt held down change what the held stamp would do: shown as they change. */
  held(e: KeyboardEvent): void {
    if (!this.kind) return;
    if (e.key === 'Alt') e.preventDefault();
    if (e.shiftKey === this.mods.shift && e.altKey === this.mods.alt) return;
    this.mods = { shift: e.shiftKey, alt: e.altKey };
    this.reghost();
  }

  /** The window lost the keyboard: keys held down are let go of. */
  letGo(): void {
    if (!this.mods.shift && !this.mods.alt) return;
    this.mods = { shift: false, alt: false };
    this.reghost();
  }

  private options(m = this.mods): StampOptions {
    return { keep: m.shift || this.keep, overwrite: m.alt || this.overwrite, seam: this.seam };
  }

  /** Where a stamp first aims: the selected room, else the middle of what the map shows. */
  private startAim(): [number, number] | null {
    const sc = this.host.scene;
    const sel = this.host.selected;
    const p = sel ? sc.layout?.at.get(sel.room) : undefined;
    return p ? [p.x, p.y] : sc.cellAt(...this.openMiddle());
  }

  /** The middle of the map that the card and the stamp drawer leave showing, on the canvas. */
  private openMiddle(): [number, number] {
    const cv = this.host.canvas;
    const c = cv.getBoundingClientRect();
    let [x0, x1, y0, y1] = [0, cv.clientWidth - this.host.card, 0, cv.clientHeight];
    const d = cv.parentElement?.querySelector('section.drawer')?.getBoundingClientRect();
    if (d?.width) {
      // a sheet along the bottom (a small screen), else a panel down the left
      if (d.top - c.top > c.height * 0.3) y1 = Math.min(y1, d.top - c.top);
      else x0 = Math.max(x0, d.right - c.left);
    }
    return [(x0 + x1) / 2, (y0 + y1) / 2];
  }

  /** The held stamp at (x, y), as it would go down. */
  private placeAt(x: number, y: number, o: StampOptions): Placed | null {
    const sc = this.host.scene;
    const k = this.kind;
    return sc.layout && k ? placed(sc.by, sc.layout, turned(k.make(this.sizes, this.seed), this.way), x, y, o) : null;
  }

  /** The room new rooms copy when the stamp lands on none: a saved stamp's own (on its map, while it is there), else the room picked, else the start room. (Build mode picks the start room when it opens, so the room picked is often no choice at all.) */
  private fallback(): number {
    const h = this.host;
    const own = this.kind?.brush;
    if (own && own.map === h.map && h.scene.by.has(own.room)) return own.room;
    return h.selected?.room ?? h.scene.start;
  }

  /** The held stamp, drawn where it would go down. */
  private reghost() {
    const sc = this.host.scene;
    const p = this.aim ? this.placeAt(this.aim[0], this.aim[1], this.options()) : null;
    this.ghost = p;
    this.brush = p ? brushOf(p, this.fallback()) : null;
    sc.draw();
  }

  /** Puts the held stamp down with its first cell at (x, y). */
  put(x: number, y: number, m: { shift: boolean; alt: boolean }): void {
    const h = this.host;
    const sc = h.scene;
    const k = this.kind;
    if (!k || !sc.layout) return;
    this.aim = [x, y];
    const o = this.options(m);
    const p = this.placeAt(x, y, o)!;
    if (!p.rooms.length && !p.links.some((l) => l.sort === 'make' || l.sort === 'replace')) {
      sfx('nope');
      h.say(p.count.left ? 'Nothing to put down here: the exits in its way are left (hold Alt to replace them).' : 'Nothing to put down here: it is all there already.');
      this.reghost();
      return;
    }
    const brushNo = brushOf(p, this.fallback());
    const brush = brushNo === null ? undefined : sc.by.get(brushNo);
    if (!brush) {
      sfx('nope');
      h.say('Pick a room first: new rooms copy it.');
      return;
    }
    const m0 = h.map;
    const what = k.saved ? `“${k.name}”` : named(k);
    // where its new rooms are drawn: the first stamp on a map keeps every room where it is drawn
    const before = h.pins;
    const first = before.size === 0;
    const next = new Map(first ? [...sc.layout.at].map(([n, s]) => [n, { ...s }]) : before);
    for (const c of p.cells) if (c.sort === 'new' && c.room !== null) next.set(c.room, { x: c.x, y: c.y });
    const said = `Stamped ${what}: ${summary(m0, p)}${first ? '. The map keeps this layout now: rooms stay where they are drawn' : ''}`;
    h.patch(stamped(sc.rooms, p, brush));
    h.showPins(m0, next);
    sfx('pop');
    const req = request(m0, brush.room, p, o, what);
    // written once the edits before it are in, then the places its rooms are drawn in; Ctrl+Z
    // (or its toast) takes back both at once
    h.send(
      async () => {
        const out = await api.stamp(req);
        await h.savePins(m0, cellsOf(next), true);
        const step = out.step;
        if (step !== null) {
          const back = async (to: ReadonlyMap<number, Spot>, redo: boolean) => {
            h.quiet();
            await (redo ? api.redo(step) : api.undo(step));
            h.showPins(m0, to);
            await h.savePins(m0, cellsOf(to), true);
            realmChanged();
          };
          localStep({ label: `Stamp ${what}`, after: step, undo: () => back(before, false), redo: () => back(next, true) });
        }
        return out;
      },
      said,
      () => h.showPins(m0, before),
    );
  }

  // ---- on the map --------------------------------------------------------------------

  /** The held stamp follows the pointer (a finger moves it by tapping). */
  move(x: number, y: number, e: PointerEvent): boolean {
    if (e.pointerType !== 'touch') this.aimAt(x, y);
    return true;
  }

  /** A click puts the stamp down; a tap moves it there first, and a second tap puts it down. */
  click(x: number, y: number, e: PointerEvent): void {
    const [cx, cy] = this.host.scene.cellAt(x, y);
    const again = this.aimed === `${cx},${cy}`;
    this.aimAt(x, y);
    if (e.pointerType !== 'touch' || again) this.put(cx, cy, { shift: e.shiftKey, alt: e.altKey });
  }

  /** The held stamp is over the cell at (px, py). */
  private aimAt(px: number, py: number) {
    const [x, y] = this.host.scene.cellAt(px, py);
    const k = `${x},${y}`;
    if (k === this.aimed) return;
    this.aimed = k;
    this.aim = [x, y];
    this.reghost();
  }

  /**
   * The stamp as it would go down, over the map faded back so it shows over any room's colour:
   * its new rooms filled, the rooms it joins ringed, the cells it leaves alone hatched; its
   * links drawn in what they would do (made, there already, left for an exit in the way,
   * replacing one), one-way ones with an arrow, and the new rooms' numbers close up.
   */
  over({ ctx, pal: P, W, H, z, cell, half, rad, lw }: Pen): void {
    const p = this.ghost;
    if (!p) return;
    const sc = this.host.scene;
    ctx.globalAlpha = 0.5;
    ctx.fillStyle = P.bg;
    ctx.fillRect(0, 0, W, H);
    ctx.globalAlpha = 1;
    const colour = (sort: LinkSort) => (sort === 'make' ? P.stamp.fresh : sort === 'have' ? P.stamp.join : sort === 'clash' ? P.stamp.clash : sort === 'replace' ? P.stamp.replace : P.stamp.kept);
    const box = (x: number, y: number, grow = 0) => {
      ctx.beginPath();
      if (half < 2) ctx.rect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2);
      else ctx.roundRect(x - half - grow, y - half - grow, (half + grow) * 2, (half + grow) * 2, rad + grow);
    };
    // the cells first, so the links show over them
    for (const c of p.cells) {
      const x = sc.sx(c.x);
      const y = sc.sy(c.y);
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
        const s = half + 2;
        for (let k = -2 * s; k <= 2 * s; k += Math.max(4, s * 0.45)) {
          ctx.moveTo(x - s + k, y + s);
          ctx.lineTo(x + s + k, y - s);
        }
        ctx.stroke();
        ctx.restore();
      }
    }
    ctx.globalAlpha = 1;
    for (const l of p.links) {
      const [ax, ay, bx, by] = [sc.sx(l.ax), sc.sy(l.ay), sc.sx(l.bx), sc.sy(l.by)];
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
      if (l.oneWay && z >= 0.5) sc.arrow(mx, my, Math.atan2(by - ay, bx - ax), 4 * Math.min(z, 1.6) + 3, c);
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
    if (z < 1.1) return;
    ctx.font = `800 ${Math.min(11.5, 0.3 * cell)}px ui-monospace, "Cascadia Mono", Consolas, monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.lineJoin = 'round';
    ctx.lineWidth = 3;
    for (const c of p.cells) {
      if (c.sort !== 'new' || c.room === null) continue;
      const x = sc.sx(c.x);
      const y = sc.sy(c.y);
      ctx.strokeStyle = P.halo;
      ctx.strokeText(String(c.room), x, y);
      ctx.fillStyle = P.ink;
      ctx.fillText(String(c.room), x, y);
    }
  }
}
