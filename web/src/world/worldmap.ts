// The world view: every map as a disc, sized by its rooms, and the map change exits between
// them as lines. A force layout places them: each line pulls its two maps to a length that
// clears both discs, every two maps push apart, a disc a line passes over is pushed off it, and
// a little pull keeps them together. Then no two discs overlap and no line passes under a disc.
// It runs from a few starts, keeps the one whose lines cross least, and turns it to fill the
// box. The starts are fixed, so the same world is always laid out the same way.
//
// Pure: no DOM, so `node --test` runs its tests (worldmap.test.ts).

import type { MapLink } from './model.ts';

export type { MapLink };

export interface WorldNode {
  map: number;
  rooms: number;
  x: number;
  y: number;
  r: number;
  /** A map exits lead to that the world has no rooms on (an add-on's, say). */
  missing: boolean;
}

/** Two maps joined, `a` the lower number: the exits from `a` to `b`, and from `b` to `a`. */
export interface WorldEdge {
  a: number;
  b: number;
  ab: MapLink | null;
  ba: MapLink | null;
}

export interface WorldLayout {
  nodes: WorldNode[];
  edges: WorldEdge[];
  /** The box the discs fill. */
  bounds: { x0: number; y0: number; x1: number; y1: number };
}

/** The smallest disc and how much the largest map adds to it. */
const [R0, R1] = [18, 34];
/** The length a line between two maps keeps beyond their discs, the gap any two discs keep, and
 * the room a line leaves beside a disc it passes. */
const [REACH, GAP, BESIDE] = [70, 18, 8];
const ROUNDS = 400;
/** How many starts a world of so many maps gets. */
const STARTS = 6;
const MANY = 60;

/** Every map with rooms (`[map, rooms]`) and every map change between two maps, laid out for a
 * box `aspect` times as wide as it is high. */
export function worldLayout(maps: readonly (readonly [number, number])[], links: readonly MapLink[], aspect = 1): WorldLayout {
  const most = Math.max(1, ...maps.map(([, n]) => n));
  const byMap = new Map<number, WorldNode>();
  for (const [map, rooms] of maps) byMap.set(map, { map, rooms, x: 0, y: 0, r: R0 + R1 * Math.sqrt(rooms / most), missing: false });
  const edges = new Map<string, WorldEdge>();
  for (const l of links) {
    if (l.from === l.to) continue;
    for (const m of [l.from, l.to]) if (!byMap.has(m)) byMap.set(m, { map: m, rooms: 0, x: 0, y: 0, r: R0 * 0.8, missing: true });
    const [a, b] = l.from < l.to ? [l.from, l.to] : [l.to, l.from];
    const key = `${a}>${b}`;
    const e = edges.get(key) ?? { a, b, ab: null, ba: null };
    if (l.from === a) e.ab = l;
    else e.ba = l;
    edges.set(key, e);
  }
  const nodes = [...byMap.values()].sort((p, q) => p.map - q.map);
  const list = [...edges.values()].sort((p, q) => p.a - q.a || p.b - q.b);
  const at = new Map(nodes.map((p, i) => [p.map, i]));
  const joined: Join[] = list.map((e) => [at.get(e.a)!, at.get(e.b)!, 1 + Math.log2(1 + (e.ab?.exits ?? 0) + (e.ba?.exits ?? 0)) / 4]);
  let best: { xy: [number, number][]; crossed: number; under: number; area: number } | null = null;
  for (let start = 0; start < (nodes.length > MANY ? 1 : STARTS); start++) {
    place(nodes, joined, start);
    const b = boundsOf(nodes);
    const tried = { xy: nodes.map((p): [number, number] => [p.x, p.y]), crossed: crossings(nodes, joined), under: lineUnder(nodes, joined), area: (b.x1 - b.x0) * (b.y1 - b.y0) };
    if (!best || tried.under < best.under || (tried.under === best.under && (tried.crossed < best.crossed || (tried.crossed === best.crossed && tried.area < best.area)))) best = tried;
  }
  if (best) nodes.forEach((p, i) => ([p.x, p.y] = best.xy[i]!));
  turnToFit(nodes, Math.min(3, Math.max(1 / 3, aspect || 1)));
  return { nodes, edges: list, bounds: boundsOf(nodes) };
}

/** Two discs a line joins (by index), and how hard it pulls. */
type Join = [number, number, number];

function boundsOf(nodes: readonly WorldNode[]) {
  const b = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
  for (const n of nodes) {
    b.x0 = Math.min(b.x0, n.x - n.r);
    b.y0 = Math.min(b.y0, n.y - n.r);
    b.x1 = Math.max(b.x1, n.x + n.r);
    b.y1 = Math.max(b.y1, n.y + n.r);
  }
  return nodes.length ? b : { x0: 0, y0: 0, x1: 0, y1: 0 };
}

/** The order a start puts the discs around its circle: map order first, then fixed shuffles. */
function startOrder(n: number, start: number): number[] {
  const order = Array.from({ length: n }, (_, i) => i);
  if (!start) return order;
  // mulberry32, from a fixed seed
  let s = Math.imul(start, 2654435761) >>> 0;
  const rnd = () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let x = Math.imul(s ^ (s >>> 15), s | 1);
    x ^= x + Math.imul(x ^ (x >>> 7), x | 61);
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296;
  };
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(rnd() * (i + 1));
    [order[i], order[j]] = [order[j]!, order[i]!];
  }
  return order;
}

/** Where a line from p to q passes a disc o: how far along (0 to 1, else not beside it), and
 * the way from the line to the disc's middle. */
function beside(p: WorldNode, q: WorldNode, o: WorldNode): { t: number; vx: number; vy: number; off: number } | null {
  const [dx, dy] = [q.x - p.x, q.y - p.y];
  const d2 = dx * dx + dy * dy;
  if (d2 < 1e-4) return null;
  const t = ((o.x - p.x) * dx + (o.y - p.y) * dy) / d2;
  if (t <= 0 || t >= 1) return null;
  let [vx, vy] = [o.x - (p.x + dx * t), o.y - (p.y + dy * t)];
  let off = Math.hypot(vx, vy);
  // right on the line: off to its left
  if (off < 0.01) [vx, vy, off] = [-dy / Math.sqrt(d2), dx / Math.sqrt(d2), 1];
  return { t, vx: vx / off, vy: vy / off, off };
}

/** The force layout from one start, in place. */
function place(nodes: WorldNode[], joined: readonly Join[], start: number) {
  const n = nodes.length;
  if (!n) return;
  // a circle roomy enough for the discs
  const around = nodes.reduce((s, p) => s + 2 * p.r + GAP, 0);
  const radius = n === 1 ? 0 : Math.max(around / (2 * Math.PI), 60);
  startOrder(n, start).forEach((k, i) => {
    const a = (2 * Math.PI * i) / n - Math.PI / 2;
    [nodes[k]!.x, nodes[k]!.y] = [radius * Math.cos(a), radius * Math.sin(a)];
  });
  const push = 6000;
  for (let round = 0; round < ROUNDS; round++) {
    const heat = 1 - round / ROUNDS;
    const fx = new Float64Array(n);
    const fy = new Float64Array(n);
    for (let i = 0; i < n; i++)
      for (let j = i + 1; j < n; j++) {
        const [p, q] = [nodes[i]!, nodes[j]!];
        let [dx, dy] = [p.x - q.x, p.y - q.y];
        let d = Math.sqrt(dx * dx + dy * dy);
        // two in one place part along a fixed slant
        if (d < 0.01) [dx, dy, d] = [0.6, 0.8, 1];
        const clear = Math.max(1, d - p.r - q.r);
        const f = push / (clear * clear) + (clear < GAP ? (GAP - clear) * 0.5 : 0);
        fx[i]! += (dx / d) * f;
        fy[i]! += (dy / d) * f;
        fx[j]! -= (dx / d) * f;
        fy[j]! -= (dy / d) * f;
      }
    for (const [i, j, w] of joined) {
      const [p, q] = [nodes[i]!, nodes[j]!];
      const [dx, dy] = [q.x - p.x, q.y - p.y];
      const d = Math.max(0.01, Math.sqrt(dx * dx + dy * dy));
      const f = (d - (p.r + q.r + REACH)) * 0.06 * w;
      fx[i]! += (dx / d) * f;
      fy[i]! += (dy / d) * f;
      fx[j]! -= (dx / d) * f;
      fy[j]! -= (dy / d) * f;
      // a disc the line passes over moves off it, and the line the other way: a line under a
      // disc would read as two lines through that map
      for (let k = 0; k < n; k++) {
        if (k === i || k === j) continue;
        const o = nodes[k]!;
        const t = ((o.x - p.x) * dx + (o.y - p.y) * dy) / (d * d);
        if (t <= 0 || t >= 1) continue;
        let [vx, vy] = [o.x - (p.x + dx * t), o.y - (p.y + dy * t)];
        let off = Math.sqrt(vx * vx + vy * vy);
        const short = o.r + BESIDE * 1.2 - off;
        if (short <= 0) continue;
        if (off < 0.01) [vx, vy, off] = [-dy / d, dx / d, 1];
        const g = (short * 0.4) / off;
        fx[k]! += vx * g;
        fy[k]! += vy * g;
        fx[i]! -= vx * g * (1 - t);
        fy[i]! -= vy * g * (1 - t);
        fx[j]! -= vx * g * t;
        fy[j]! -= vy * g * t;
      }
    }
    const most = 4 + 40 * heat;
    for (let i = 0; i < n; i++) {
      const p = nodes[i]!;
      // a little pull to the middle keeps maps no line joins from drifting off
      let [mx, my] = [fx[i]! - p.x * 0.01, fy[i]! - p.y * 0.01];
      const m = Math.sqrt(mx * mx + my * my);
      if (m > most) [mx, my] = [(mx / m) * most, (my / m) * most];
      p.x += mx;
      p.y += my;
    }
  }
  // then no two discs overlap, and no line passes under a disc
  for (let pass = 0; pass < 200; pass++) {
    let moved = false;
    for (const [i, j] of joined) {
      const [p, q] = [nodes[i]!, nodes[j]!];
      for (let k = 0; k < n; k++) {
        if (k === i || k === j) continue;
        const o = nodes[k]!;
        const s = beside(p, q, o);
        const short = s ? o.r + BESIDE - s.off : 0;
        if (!s || short <= 0) continue;
        moved = true;
        const g = short * 0.5;
        o.x += s.vx * g;
        o.y += s.vy * g;
        p.x -= s.vx * g * (1 - s.t);
        p.y -= s.vy * g * (1 - s.t);
        q.x -= s.vx * g * s.t;
        q.y -= s.vy * g * s.t;
      }
    }
    for (let i = 0; i < n; i++)
      for (let j = i + 1; j < n; j++) {
        const [p, q] = [nodes[i]!, nodes[j]!];
        let [dx, dy] = [q.x - p.x, q.y - p.y];
        let d = Math.hypot(dx, dy);
        if (d < 0.01) [dx, dy, d] = [0.6, 0.8, 1];
        const over = p.r + q.r + GAP - d;
        if (over <= 0) continue;
        moved = true;
        const [ux, uy] = [(dx / d) * (over / 2), (dy / d) * (over / 2)];
        [p.x, p.y, q.x, q.y] = [p.x - ux, p.y - uy, q.x + ux, q.y + uy];
      }
    if (!moved) break;
  }
}

/** How many times two lines cross (lines from one map never do). */
function crossings(nodes: readonly WorldNode[], joined: readonly (readonly [number, number, ...unknown[]])[]): number {
  const side = (a: WorldNode, b: WorldNode, c: WorldNode) => Math.sign((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x));
  let n = 0;
  for (let s = 0; s < joined.length; s++)
    for (let u = s + 1; u < joined.length; u++) {
      const [i, j] = joined[s]!;
      const [k, l] = joined[u]!;
      if (i === k || i === l || j === k || j === l) continue;
      const [a, b, c, d] = [nodes[i]!, nodes[j]!, nodes[k]!, nodes[l]!];
      if (side(a, b, c) !== side(a, b, d) && side(c, d, a) !== side(c, d, b)) n++;
    }
  return n;
}

/** How many times a line passes under a disc not its own, or too close beside it. */
export function lineUnder(nodes: readonly WorldNode[], joined: readonly (readonly [number, number, ...unknown[]])[], room = BESIDE / 2): number {
  let n = 0;
  for (const [i, j] of joined)
    for (let k = 0; k < nodes.length; k++) {
      if (k === i || k === j) continue;
      const s = beside(nodes[i]!, nodes[j]!, nodes[k]!);
      if (s && s.off < nodes[k]!.r + room) n++;
    }
  return n;
}

/** Turns the layout about its middle to the angle that lets it show largest in the box. */
function turnToFit(nodes: WorldNode[], aspect: number) {
  if (nodes.length < 2) return;
  const b0 = boundsOf(nodes);
  const [cx, cy] = [(b0.x0 + b0.x1) / 2, (b0.y0 + b0.y1) / 2];
  const fit = (a: number) => {
    const [c, s] = [Math.cos(a), Math.sin(a)];
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    for (const p of nodes) {
      const [x, y] = [(p.x - cx) * c - (p.y - cy) * s, (p.x - cx) * s + (p.y - cy) * c];
      [x0, y0, x1, y1] = [Math.min(x0, x - p.r), Math.min(y0, y - p.r), Math.max(x1, x + p.r), Math.max(y1, y + p.r)];
    }
    // the scale it shows at in a box `aspect` wide and 1 high
    return Math.min(aspect / (x1 - x0), 1 / (y1 - y0));
  };
  let [best, bestFit] = [0, fit(0)];
  // a turn has to earn its keep: the layout as placed, unless turning shows it larger
  for (let deg = 15; deg < 180; deg += 15) {
    const f = fit((deg * Math.PI) / 180);
    if (f > bestFit * 1.04) [best, bestFit] = [deg, f];
  }
  if (!best) return;
  const a = (best * Math.PI) / 180;
  const [c, s] = [Math.cos(a), Math.sin(a)];
  for (const p of nodes) [p.x, p.y] = [cx + (p.x - cx) * c - (p.y - cy) * s, cy + (p.x - cx) * s + (p.y - cy) * c];
}

/** The map nearest a point, within its disc (and a little more for a finger), or null. */
export function mapAt(l: WorldLayout, x: number, y: number, slack = 0): WorldNode | null {
  let best: WorldNode | null = null;
  let bestD = Infinity;
  for (const n of l.nodes) {
    const d = Math.hypot(n.x - x, n.y - y) - n.r;
    if (d <= slack && d < bestD) [best, bestD] = [n, d];
  }
  return best;
}
