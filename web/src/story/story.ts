// A story as a graph: from a monster that talks, a room with commands or a text block, every
// text block, message and record the game reaches through them, and how: greeted, asked about
// a topic, the script an answer runs, what a step says, gives, takes, needs, runs or moves to.
// Each text block is read as the game reads it where this story reaches it (script.ts), and
// each way on leaves from the row of its node that takes it: a topic, a step.
//
// Then where each goes on the page: a column per step from the start, each node level with
// the rows that lead to it.
//
// Pure: no DOM, so `node --test` runs its tests (story.test.ts).

import { linesOf, type Reading, type Role } from '../library/script.ts';

export type Cell = number | string | null;
export type Values = Record<string, Cell>;
/** A record's values: undefined while it is read, null when the world has none. */
export type Look = (table: string, key: Cell[]) => Values | null | undefined;

export interface StoryNode {
  /** `Textblocks/907/0` */
  id: string;
  table: string;
  key: Cell[];
  /** Steps from the start: its column. */
  depth: number;
  /** A text block: how the game reads it where this story reaches it. */
  reading?: Reading;
  /** A text block a monster shows as an answer: the game then runs its Link To. */
  answer?: boolean;
  state: 'ok' | 'pending' | 'missing';
}

/** How a node leads to the next: a monster talks or chatters, a talk block greets or is asked,
 * an answer then runs a script, a room's commands are typed, or a step's record by its role. */
export type EdgeKind = 'talks' | 'chatter' | 'greets' | 'asks' | 'then' | 'types' | Role;

export interface StoryEdge {
  from: string;
  to: string;
  kind: EdgeKind;
  /** The row of `from` it leaves by: a topic `t2`, a step `1.0` (its line's place, then the
   * step's), or `talks`, `chatter`, `types`, `greet`, `then`. */
  port: string;
}

export interface Story {
  nodes: StoryNode[];
  edges: StoryEdge[];
  /** Records left out past the most a story shows. */
  more: number;
}

const text = (v: Cell | undefined) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);
const num = (v: Cell | undefined) => Number(v) || 0;
export const nodeId = (table: string, key: Cell[]) => `${table}/${key.map((k) => String(k ?? '')).join('/')}`;

/** The key a record's number names. */
function keyFor(table: string, n: number, map?: number): Cell[] {
  if (table === 'Rooms') return [map ?? 0, n];
  if (table === 'Textblocks') return [n, 0];
  return [n];
}

/**
 * The story that starts at a monster (its greeting and topics, and what it says now and then),
 * a room (its commands) or a text block (read as `reading`, and as an answer when `answer`),
 * at most `max` records.
 */
export function storyOf(root: { table: string; key: Cell[]; reading?: Reading; answer?: boolean }, look: Look, max = 60): Story {
  const nodes = new Map<string, StoryNode>();
  const edges = new Map<string, StoryEdge>();
  const queue: StoryNode[] = [];
  let more = 0;

  function add(table: string, key: Cell[], depth: number, how: Pick<StoryNode, 'reading' | 'answer'> = {}): StoryNode | null {
    const id = nodeId(table, key);
    const had = nodes.get(id);
    if (had) {
      // an answer reached again as plain text still runs its link
      if (how.answer && !had.answer) {
        had.answer = true;
        if (had.state === 'ok') queue.push(had);
      }
      return had;
    }
    if (nodes.size >= max) {
      more++;
      return null;
    }
    const v = look(table, key);
    const n: StoryNode = { id, table, key, depth, ...how, state: v === undefined ? 'pending' : v === null ? 'missing' : 'ok' };
    nodes.set(id, n);
    if (n.state === 'ok') queue.push(n);
    return n;
  }

  function link(from: StoryNode, to: StoryNode | null, kind: EdgeKind, port: string) {
    if (!to) return;
    const id = `${from.id}>${to.id}>${kind}>${port}`;
    if (!edges.has(id)) edges.set(id, { from: from.id, to: to.id, kind, port });
  }

  /** The records a block's steps name, each by its role; a block they run, read as it runs. */
  function steps(from: StoryNode, data: string, reading: Reading) {
    linesOf(data, reading).forEach((line, li) => {
      if (line.never) return;
      line.steps.forEach((s, si) => {
        for (const b of s.bits) {
          if (typeof b !== 'object' || !('ref' in b) || b.ref.n <= 0) continue;
          const { t, n, map } = b.ref;
          // a remote action's room is on the player's map: the start's, when it is a room
          const m = t === 'Rooms' && map === undefined ? (root.table === 'Rooms' ? num(root.key[0]) : undefined) : map;
          if (t === 'Rooms' && m === undefined) continue;
          const runs: Reading | undefined = t === 'Textblocks' ? (b.role === 'runs' ? (s.clause.kw === 'random' ? 'chances' : 'script') : 'text') : undefined;
          link(from, add(t, keyFor(t, n, m), from.depth + 1, runs ? { reading: runs } : {}), b.role, `${li}.${si}`);
        }
      });
    });
  }

  const start = add(root.table, root.key, 0, { ...(root.reading ? { reading: root.reading } : {}), ...(root.answer ? { answer: true } : {}) });
  for (let node = queue.shift(); node; node = queue.shift()) {
    const v = look(node.table, node.key);
    if (!v) continue;
    const at = node.depth + 1;
    if (node.table === 'Monsters' && node === start) {
      const greet = num(v['Greet Txt']);
      const talk = num(v['Talk Txt']);
      if (greet > 0) link(node, add('Textblocks', [greet, 0], at, { reading: 'topics' }), 'talks', 'talks');
      if (talk > 0) link(node, add('Textblocks', [talk, 0], at, { reading: 'talk' }), 'chatter', 'chatter');
    } else if (node.table === 'Rooms' && node === start) {
      const cmd = num(v['Command Text']);
      if (cmd > 0) link(node, add('Textblocks', [cmd, 0], at, { reading: 'commands' }), 'types', 'types');
    } else if (node.table === 'Textblocks') {
      const data = text(v['Data']);
      const to = num(v['Link To']);
      if (node.reading === 'topics') {
        if (to > 0) link(node, add('Textblocks', [to, 0], at, { reading: 'text' }), 'greets', 'greet');
        linesOf(data, 'topics').forEach((p, i) => {
          if ((p.block ?? 0) > 0) link(node, add('Textblocks', [p.block!, 0], at, { reading: 'text', answer: true }), 'asks', `t${i}`);
        });
      } else if (node.reading === 'text') {
        if (node.answer && to > 0) link(node, add('Textblocks', [to, 0], at, { reading: 'script' }), 'then', 'then');
      } else if (node.reading === 'script' || node.reading === 'commands' || node.reading === 'chances') {
        steps(node, data, node.reading);
      }
    }
  }
  return { nodes: [...nodes.values()], edges: [...edges.values()], more };
}

// ---- where each node goes ----------------------------------------------------------------

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Layout {
  at: Map<string, Box>;
  width: number;
  height: number;
}

export interface Spacing {
  /** A node's width, and the gaps between columns and between nodes. */
  w: number;
  gapX: number;
  gapY: number;
  pad: number;
  /** How far down a node an arrow comes in: the middle of its title. */
  entry: number;
}

export const SPACING: Spacing = { w: 264, gapX: 76, gapY: 14, pad: 18, entry: 18 };

/**
 * A column per depth; down each column the nodes in the order of the rows that lead to them,
 * each level with its row when the nodes above it allow, the ones led to from the same row
 * centred on it together. `portOf` says how far down its node an edge leaves.
 */
export function layout(story: Story, heightOf: (id: string) => number, portOf: (e: StoryEdge) => number | undefined = () => undefined, s: Spacing = SPACING): Layout {
  const at = new Map<string, Box>();
  const cols = new Map<number, StoryNode[]>();
  for (const n of story.nodes) cols.set(n.depth, [...(cols.get(n.depth) ?? []), n]);
  const into = new Map<string, StoryEdge[]>();
  for (const e of story.edges) into.set(e.to, [...(into.get(e.to) ?? []), e]);
  let height = 0;
  for (const depth of [...cols.keys()].sort((a, b) => a - b)) {
    // the middle of the placed rows that lead to each, null for none
    const order = cols
      .get(depth)!
      .map((n, i) => {
        const ys = (into.get(n.id) ?? []).flatMap((e) => {
          const b = at.get(e.from);
          return b ? [b.y + (portOf(e) ?? s.entry)] : [];
        });
        return { n, i, want: ys.length ? ys.reduce((a, b) => a + b, 0) / ys.length : null };
      })
      .sort((a, b) => (a.want ?? Infinity) - (b.want ?? Infinity) || a.i - b.i);
    let y = s.pad;
    for (let g = 0; g < order.length; ) {
      // the nodes that want the same row, their arrows' ends centred on it together
      let end = g + 1;
      while (end < order.length && order[end]!.want !== null && order[end]!.want === order[g]!.want) end++;
      const group = order.slice(g, end);
      const want = order[g]!.want;
      const above = group.slice(0, -1).reduce((sum, { n }) => sum + heightOf(n.id) + s.gapY, 0);
      y = Math.max(y, want === null ? y : want - s.entry - above / 2);
      for (const { n } of group) {
        const h = heightOf(n.id);
        at.set(n.id, { x: s.pad + depth * (s.w + s.gapX), y, w: s.w, h });
        y += h + s.gapY;
      }
      g = end;
    }
    height = Math.max(height, y - s.gapY + s.pad);
  }
  const deepest = Math.max(-1, ...cols.keys());
  const width = deepest < 0 ? 0 : s.pad * 2 + (deepest + 1) * s.w + deepest * s.gapX;
  return { at, width, height };
}

/** An arrow's path from `down` into box `a` (its row's middle) to the title of box `b`: out of
 * the right side and into the left; to a box no further right, out and back round into its
 * right side. */
export function edgePath(a: Box, down: number, b: Box, entry = SPACING.entry): string {
  const [x1, y1] = [a.x + a.w, a.y + down];
  const y2 = b.y + Math.min(entry, b.h / 2);
  if (b.x > x1) {
    const c = Math.max(28, (b.x - x1) / 2);
    return `M${x1} ${y1} C${x1 + c} ${y1}, ${b.x - c} ${y2}, ${b.x} ${y2}`;
  }
  const x2 = b.x + b.w;
  const out = Math.max(x1, x2) + 40;
  return `M${x1} ${y1} C${out} ${y1}, ${out} ${y2}, ${x2} ${y2}`;
}
