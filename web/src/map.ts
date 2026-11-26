// A map of rooms, drawn as NMR's Map form draws it: a 41 by 41 grid with the start room in
// the middle, the rooms reached from it placed by a breadth-first walk of their exits (N is
// one row up, NE one row up and one column right, and so on), and each exit's line coloured
// by its type. Up and down exits are not placed; they show as marks on the room. Clicking a
// room centres the map on it; its editor is a link away.

import { EXITS as DIRS, EXIT_TYPE } from './layouts';
import { recordHref } from './shell/routes';

const SIZE = 41;
const CELL = 22;
/** Grid steps of directions 0 to 7, as [row, column]. */
const STEP: [number, number][] = [
  [-1, 0],
  [1, 0],
  [0, 1],
  [0, -1],
  [-1, 1],
  [-1, -1],
  [1, 1],
  [1, -1],
];

interface Exit {
  dir: number;
  to: number;
  kind: number;
  para1: number;
  para2: number;
}
interface Room {
  room: number;
  name: string;
  exits: Exit[];
  shop: number;
  spell: number;
  command: number;
  npc: number;
  control: number;
  death: number;
  exit_room: number;
}
interface MapAnswer {
  map: number;
  maps: [number, number][];
  rooms: Room[];
}

/** NMR's line colours, by exit type (frmMap.MapExits). */
function lineColour(kind: number): string {
  switch (kind) {
    case 2:
    case 3:
    case 4:
      return '#4ee04e';
    case 5:
    case 12:
      return '#4ee0e0';
    case 6:
      return '#a020a0';
    case 7:
    case 11:
      return '#6a8cff';
    case 8:
      return '#ff5cff';
    case 9:
    case 24:
      return '#ff5050';
    case 10:
      return '#e8d020';
    case 13:
    case 14:
    case 15:
    case 20:
    case 23:
      return '#2040b0';
    case 16:
      return '#20a020';
    default:
      return '#909090';
  }
}

const LEGEND: [string, number][] = [
  ['Normal', 0],
  ['Key, item, toll', 2],
  ['Action, remote action', 5],
  ['Hidden', 6],
  ['Door, gate', 7],
  ['Map change', 8],
  ['Trap, spell trap', 9],
  ['Text', 10],
  ['Class, race, level, alignment, ability', 13],
  ['Timed', 16],
];

function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Record<string, string | undefined> = {},
  ...children: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v === undefined) continue;
    if (k === 'class') el.className = v;
    else el.setAttribute(k, v);
  }
  el.append(...children);
  return el;
}

function svg(tag: string, attrs: Record<string, string | number>): SVGElement {
  const el = document.createElementNS('http://www.w3.org/2000/svg', tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

/** Where each room lands. Exported for tests of the walk. */
export interface Layout {
  /** Room number by cell (row * SIZE + column). */
  cells: Map<number, number>;
  /** Cell by room number. */
  at: Map<number, number>;
  /** Exits followed to a room that is not on the map. */
  missing: Set<number>;
}

/** Whether a walk goes through an exit of this type. */
function follows(kind: number, hidden: boolean): boolean {
  if (kind === 8 || kind === 12) return false;
  if (kind === 6) return hidden;
  return true;
}

/** NMR's walk: breadth first from `start`, each exit placing its room one step away, if
 *  that cell is on the grid and free and the room is not placed yet. */
export function layOut(rooms: Map<number, Room>, start: number, hidden: boolean): Layout {
  const cells = new Map<number, number>();
  const at = new Map<number, number>();
  const missing = new Set<number>();
  const centre = Math.floor(SIZE / 2) * SIZE + Math.floor(SIZE / 2);
  cells.set(centre, start);
  at.set(start, centre);
  const queue = [start];
  while (queue.length) {
    const n = queue.shift()!;
    const room = rooms.get(n);
    if (!room) continue;
    const cell = at.get(n)!;
    const [r, c] = [Math.floor(cell / SIZE), cell % SIZE];
    for (const e of room.exits) {
      if (e.dir > 7 || !follows(e.kind, hidden) || at.has(e.to)) continue;
      const [dr, dc] = STEP[e.dir];
      const [nr, nc] = [r + dr, c + dc];
      if (nr < 0 || nc < 0 || nr >= SIZE || nc >= SIZE) continue;
      const next = nr * SIZE + nc;
      if (cells.has(next)) continue;
      cells.set(next, e.to);
      at.set(e.to, next);
      if (rooms.has(e.to)) queue.push(e.to);
      else missing.add(e.to);
    }
  }
  return { cells, at, missing };
}

async function fetchMap(map: number): Promise<MapAnswer> {
  const res = await fetch(`/api/map?map=${map}`);
  const body = await res.json().catch(() => null);
  if (!res.ok) throw new Error(body?.error ?? res.statusText);
  return body as MapAnswer;
}

/** The map page; `show` draws the map and room the hash names (`#/map-classic/M/R`). */
export function mapPage(): { el: HTMLElement; show: (fresh?: boolean) => Promise<void> } {
  const el = h('div', { class: 'map-page' });
  const mapSel = h('select', { 'aria-label': 'Map' });
  const roomBox = h('input', { type: 'number', min: '1', 'aria-label': 'Room', class: 'num' });
  const hidden = h('input', { type: 'checkbox' });
  hidden.checked = true;
  const draw = h('button', { type: 'button', class: 'primary' }, 'Map');
  const note = h('span', { class: 'note' });
  // what the last build left to look at; kept across redraws
  const flash = h('p', { class: 'note map-flash' });
  const bar = h(
    'div',
    { class: 'toolbar' },
    h('label', {}, 'Map ', mapSel),
    h('label', {}, 'Start room ', roomBox),
    h('label', {}, hidden, ' Follow hidden exits'),
    draw,
    note,
  );
  const board = h('div', { class: 'map-board' });
  const side = h('aside', { class: 'map-side' });
  const legend = h(
    'ul',
    { class: 'map-legend' },
    ...LEGEND.map(([label, kind]) => {
      const sw = h('span', { class: 'swatch' });
      sw.style.background = lineColour(kind);
      return h('li', {}, sw, label);
    }),
    h('li', {}, h('span', { class: 'swatch start' }), 'Start room'),
    h('li', {}, h('span', { class: 'swatch missing' }), 'Exit to a room not on the map'),
    h('li', {}, h('span', { class: 'mark cmd' }), 'Command text'),
    h('li', {}, h('span', { class: 'mark shop' }), 'Shop'),
    h('li', {}, h('span', { class: 'mark npc' }), 'Permanent NPC'),
    h('li', {}, '▲▼ up and down exits'),
  );
  el.append(
    h('h2', {}, 'Map'),
    bar,
    flash,
    h('div', { class: 'map-body' }, board, h('div', { class: 'map-aside' }, side, legend)),
  );

  let data: MapAnswer | null = null;
  let lastLay: Layout | null = null;
  const go = (map: number, room: number) => {
    location.hash = `#/map-classic/${map}/${room}`;
  };
  draw.addEventListener('click', () => go(Number(mapSel.value), Number(roomBox.value) || 1));
  mapSel.addEventListener('change', () => go(Number(mapSel.value), 1));
  hidden.addEventListener('change', () => void show());

  function info(map: number, r: Room | undefined, n: number) {
    side.replaceChildren();
    if (!r) {
      side.append(h('p', {}, `Room ${map}/${n} is not on the map.`));
      return;
    }
    side.append(
      h('h3', {}, `Room ${map}/${r.room}`),
      h('p', {}, r.name || '(no name)'),
      h('p', {}, h('a', { href: recordHref('Rooms', [map, r.room]) }, 'Open in the room editor')),
    );
    const list = h('ul', { class: 'map-exits' });
    for (const e of r.exits) {
      const target = e.kind === 8 ? `map ${e.para1} room ${e.to}` : `room ${e.to}`;
      const a = h('a', { href: e.kind === 8 ? `#/map-classic/${e.para1}/${e.to}` : `#/map-classic/${map}/${e.to}` }, target);
      const sw = h('span', { class: 'swatch' });
      sw.style.background = lineColour(e.kind);
      list.append(h('li', {}, sw, `${DIRS[e.dir]}: `, a, ` (${EXIT_TYPE[e.kind] ?? `type ${e.kind}`})`));
    }
    side.append(r.exits.length ? list : h('p', {}, 'No exits.'));
    const extras: string[] = [];
    if (r.shop) extras.push(`Shop ${r.shop}`);
    if (r.npc) extras.push(`Permanent NPC ${r.npc}`);
    if (r.spell) extras.push(`Spell ${r.spell}`);
    if (r.command) extras.push(`Command text ${r.command}`);
    if (r.control) extras.push(`Control room ${r.control}`);
    if (r.death) extras.push(`Death room ${r.death}`);
    if (r.exit_room) extras.push(`Exit room ${r.exit_room}`);
    if (extras.length) side.append(h('p', { class: 'note' }, extras.join(' · ')));
    side.append(builder(map, r));
  }

  // NMR's Map Editor: a move where there is no exit makes a new room, or joins an
  // existing one, with an exit each way
  function builder(map: number, r: Room): HTMLElement {
    const panel = h('details', { class: 'map-build' }, h('summary', {}, 'Build (Map Editor)'));
    const form = h('div', { class: 'map-build-form' });
    const pad = h('div', { class: 'map-pad' });
    // keypad order: NW N NE / W . E / SW S SE, then U D
    const order = [5, 0, 4, 3, -1, 2, 7, 1, 6, 8, 9];
    for (const d of order) {
      if (d < 0) {
        pad.append(h('span', {}, String(r.room)));
        continue;
      }
      const e = r.exits.find((x) => x.dir === d);
      const b = h('button', { type: 'button', title: e ? `Go ${DIRS[d]} to ${e.to}` : `No exit ${DIRS[d]}: make or join a room` }, DIRS[d]!);
      if (e) b.classList.add('has-exit');
      b.addEventListener('click', () => {
        if (e) {
          go(e.kind === 8 ? e.para1 : map, e.to);
          return;
        }
        ask(d);
      });
      pad.append(b);
    }
    panel.append(pad, form);

    function ask(d: number) {
      const rooms = data?.rooms ?? [];
      const next = rooms.reduce((m, x) => Math.max(m, x.room), 0) + 1;
      // the room already drawn in that direction, as NMR offers it
      let near = '';
      if (lastLay && d < 8) {
        const cell = lastLay.at.get(r.room);
        if (cell !== undefined) {
          const [dr, dc] = STEP[d]!;
          const there = lastLay.cells.get(cell + dr * SIZE + dc);
          if (there !== undefined) near = String(there);
        }
      }
      const existing = h('input', { type: 'checkbox' });
      const toMap = h('input', { type: 'text', inputmode: 'numeric', class: 'num', value: String(map), 'aria-label': 'Map' });
      const toRoom = h('input', { type: 'text', inputmode: 'numeric', class: 'num', value: String(next), 'aria-label': 'Room' });
      existing.addEventListener('change', () => {
        toRoom.value = existing.checked ? near : String(next);
      });
      const replace = h('input', { type: 'checkbox' });
      const make = h('button', { type: 'button', class: 'primary' }, 'Create');
      const result = h('div');
      make.addEventListener('click', async () => {
        const body = {
          map,
          room: r.room,
          dir: d,
          to_map: Number(toMap.value),
          to_room: Number(toRoom.value),
          existing: existing.checked,
          replace_back: replace.checked,
        };
        result.replaceChildren(h('p', {}, 'Writing…'));
        flash.textContent = '';
        try {
          const res = await fetch('/api/map/dig', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
          const out = await res.json().catch(() => null);
          if (!res.ok) throw new Error(out?.error ?? res.statusText);
          if (out.notes?.length) alertNote(out.notes.join(' '));
          await show(true);
          if (body.to_map === map) go(map, body.to_room);
        } catch (e) {
          result.replaceChildren(h('p', { class: 'error' }, (e as Error).message));
        }
      });
      form.replaceChildren(
        h('p', {}, `No exit ${DIRS[d]} from room ${r.room}. Make a new room there, or join an existing one; an exit is made each way.`),
        h('div', { class: 'fix-inputs' }, h('label', {}, existing, ' Existing room'), h('label', {}, 'Map ', toMap), h('label', {}, 'Room ', toRoom)),
        h('div', {}, h('label', {}, replace, ' Replace its exit back if in use')),
        h('div', { class: 'toolbar' }, make),
        result,
      );
    }
    return panel;
  }

  function alertNote(text: string) {
    flash.textContent = text;
  }

  function render(map: number, start: number) {
    if (!data) return;
    const rooms = new Map(data.rooms.map((r) => [r.room, r]));
    if (!rooms.has(start) && data.rooms.length) start = data.rooms[0].room;
    roomBox.value = String(start);
    const lay = layOut(rooms, start, hidden.checked);
    lastLay = lay;
    const px = SIZE * CELL;
    const s = svg('svg', { viewBox: `0 0 ${px} ${px}`, width: px, height: px, class: 'map-svg' }) as SVGSVGElement;
    const centreOf = (cell: number): [number, number] => [
      (cell % SIZE) * CELL + CELL / 2,
      Math.floor(cell / SIZE) * CELL + CELL / 2,
    ];
    // lines first, under the rooms: each placed room's exits in directions 0-7
    for (const [cell, n] of lay.cells) {
      const r = rooms.get(n);
      if (!r) continue;
      const [x, y] = centreOf(cell);
      for (const e of r.exits) {
        if (e.dir > 7) continue;
        const [dr, dc] = STEP[e.dir];
        const there = lay.at.get(e.to);
        const adjacent = e.kind !== 8 && there === cell + dr * SIZE + dc;
        // a line to the next cell when the exit's room is there, else a stub halfway
        const reach = adjacent ? 0.5 : 0.42;
        const line = svg('line', {
          x1: x,
          y1: y,
          x2: x + dc * CELL * reach,
          y2: y + dr * CELL * reach,
          stroke: lineColour(e.kind),
          'stroke-width': 2,
        });
        if (!adjacent) line.setAttribute('stroke-dasharray', '2 2');
        s.append(line);
      }
    }
    const box = CELL * 0.55;
    for (const [cell, n] of lay.cells) {
      const r = rooms.get(n);
      const [x, y] = centreOf(cell);
      const g = svg('g', { class: 'map-room', 'data-room': n });
      const rect = svg('rect', {
        x: x - box / 2,
        y: y - box / 2,
        width: box,
        height: box,
        class: !r ? 'missing' : n === start ? 'start' : r.command ? 'cmd' : '',
      });
      const title = svg('title', {});
      title.textContent = r ? `${n}: ${r.name}` : `${n}: not on map ${map}`;
      g.append(title, rect);
      if (r?.shop) g.append(svg('circle', { cx: x + box / 2, cy: y - box / 2, r: 2.5, class: 'shop' }));
      if (r?.npc) g.append(svg('circle', { cx: x - box / 2, cy: y - box / 2, r: 2.5, class: 'npc' }));
      if (r?.exits.some((e) => e.dir === 8)) {
        g.append(svg('path', { d: `M${x - 3} ${y - 1}l3 -4l3 4z`, class: 'updown' }));
      }
      if (r?.exits.some((e) => e.dir === 9)) {
        g.append(svg('path', { d: `M${x - 3} ${y + 1}l3 4l3 -4z`, class: 'updown' }));
      }
      g.addEventListener('click', () => {
        info(map, r, n);
        if (r && n !== start) go(map, n);
      });
      s.append(g);
    }
    board.replaceChildren(s);
    // centre the start room in view
    board.scrollLeft = (px - board.clientWidth) / 2;
    board.scrollTop = (px - board.clientHeight) / 2;
    info(map, rooms.get(start), start);
    const placed = [...lay.cells.values()].filter((n) => rooms.has(n)).length;
    note.textContent =
      `${placed} of ${data.rooms.length} rooms on map ${map} drawn from room ${start}` +
      (lay.missing.size ? `; ${lay.missing.size} exits lead to rooms not on the map` : '') +
      '.';
  }

  async function show(fresh = false) {
    const [, , m, r] = location.hash.split('/');
    const map = Number(m) || 1;
    const start = Number(r) || 1;
    try {
      if (fresh || !data || data.map !== map) {
        note.textContent = 'Loading…';
        data = await fetchMap(map);
        mapSel.replaceChildren(
          ...data.maps.map(([n, count]) => h('option', { value: String(n) }, `${n} (${count} rooms)`)),
        );
        if (!data.maps.some(([n]) => n === map)) mapSel.append(h('option', { value: String(map) }, `${map} (none)`));
      }
      mapSel.value = String(map);
      render(map, start);
    } catch (e) {
      note.textContent = `Could not load the map: ${(e as Error).message}`;
    }
  }

  return { el, show };
}
