// NMR's lists: the Limited Items List, Find an Item, the Control Room List, the Monster
// Group / Index List and the NPC / Room List, over the server's /api/reports routes. Each
// record they name links to its editor.

import { MONSTER_GROUP } from './layouts';

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

async function get(url: string): Promise<any> {
  const res = await fetch(url);
  const text = await res.text();
  let parsed: any = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    // an error page
  }
  if (!res.ok) throw new Error(parsed?.error ?? (text || res.statusText));
  return parsed;
}

/** A link to a record's editor. */
const link = (table: string, key: number[], label: string) =>
  h('a', { href: `#/${table}/${encodeURIComponent(JSON.stringify(key))}` }, label);

const roomLink = (map: number, room: number, label = `${map}/${room}`) => link('Rooms', [map, room], label);

interface Place {
  kind: 'room' | 'hidden' | 'shop' | 'user';
  key: string;
  name: string;
  count: number;
}

function placeLink(p: Place): Node {
  if (p.kind === 'room' || p.kind === 'hidden') {
    const [map, room] = p.key.split('/').map(Number);
    return roomLink(map!, room!, `${p.kind === 'hidden' ? 'Room (hidden)' : 'Room'} ${p.key}`);
  }
  if (p.kind === 'shop') return link('Shops', [Number(p.key)], `Shop ${p.key}`);
  return document.createTextNode(`User ${p.name}`);
}

function grid(head: string[], rows: (Node | string)[][], rowClass?: (i: number) => string | undefined): HTMLElement {
  const body = h('tbody');
  rows.forEach((r, i) => body.append(h('tr', { class: rowClass?.(i) }, ...r.map((c) => h('td', {}, c)))));
  return h('table', { class: 'grid list' }, h('thead', {}, h('tr', {}, ...head.map((c) => h('th', {}, c)))), body);
}

/** Commas between nodes. */
function joined(nodes: Node[]): Node[] {
  return nodes.flatMap((n, i) => (i ? [document.createTextNode(', '), n] : [n]));
}

function card(title: string, hint: string, controls: (Node | string)[], build: () => Promise<HTMLElement>): HTMLElement {
  const out = h('div', { class: 'tool-result' });
  const go = h('button', { type: 'button', class: 'primary' }, 'Build List');
  go.addEventListener('click', async () => {
    out.replaceChildren(h('p', {}, 'Building…'));
    try {
      out.replaceChildren(await build());
    } catch (e) {
      out.replaceChildren(h('p', { class: 'error' }, e instanceof Error ? e.message : String(e)));
    }
  });
  return h('section', { class: 'tool' }, h('h2', {}, title), h('p', { class: 'hint' }, hint), h('div', { class: 'toolbar' }, ...controls, go), out);
}

function scanBoxes(): [HTMLElement[], () => string] {
  const boxes = ['Users', 'Shops', 'Rooms'].map((name) => {
    const i = h('input', { type: 'checkbox', checked: '' });
    return [h('label', {}, i, ` Scan ${name}`), i, name.toLowerCase()] as const;
  });
  return [boxes.map(([l]) => l), () => boxes.map(([, i, n]) => `${n}=${i.checked}`).join('&')];
}

function limitedItems(): HTMLElement {
  const [boxes, query] = scanBoxes();
  return card(
    'Limited Items List',
    'Every limited item, and where its copies are. Items with more copies than their limit are in red.',
    boxes,
    async () => {
      const res = await get(`/api/reports/limited-items?${query()}`);
      const over = res.items.filter((i: any) => i.over).length;
      return h(
        'div',
        {},
        h('p', {}, `${res.items.length} limited items; ${over} over their limit.`),
        grid(
          ['Number', 'Name (limit)', 'In use', 'Where'],
          res.items.map((i: any) => [
            link('Items', [i.number], String(i.number)),
            i.limit > 1 ? `${i.name} (${i.limit})` : i.name,
            String(i.in_use),
            h('span', {}, ...joined(i.places.map((p: Place) => h('span', {}, placeLink(p), ` (${p.count})`)))),
          ]),
          (n) => (res.items[n].over ? 'over' : undefined),
        ),
      );
    },
  );
}

function findItem(): HTMLElement {
  const [boxes, query] = scanBoxes();
  const item = h('input', { type: 'text', inputmode: 'numeric', class: 'number' });
  return card('Find an Item', 'Every room, shop and user that has the item.', [h('label', {}, 'Item to Find: ', item), ...boxes], async () => {
    const n = Number(item.value.trim());
    if (!Number.isInteger(n) || n <= 0) throw new Error('Give the item number.');
    const res = await get(`/api/reports/find-item?item=${n}&${query()}`);
    if (!res.places.length) return h('p', {}, `Item ${n} is nowhere.`);
    return grid(
      ['Where', 'Name', 'Count'],
      res.places.map((p: Place) => [placeLink(p), p.name, String(p.count)]),
    );
  });
}

function controlRooms(): HTMLElement {
  return card('Control Room List', 'Each control room, and the rooms on its map that name it.', [], async () => {
    const res = await get('/api/reports/control-rooms');
    return grid(
      ['Control Room', 'Name', 'References'],
      res.rooms.map((c: any) => [
        roomLink(c.map, c.room),
        c.name || '(no such room)',
        h('span', {}, `${c.rooms.length}: `, ...joined(c.rooms.map((r: number) => roomLink(c.map, r, String(r))))),
      ]),
    );
  });
}

function monsterGroups(): HTMLElement {
  return card('Monster Group / Index List', 'The monsters of each group and index (at most 15 each, as NMR lists).', [], async () => {
    const res = await get('/api/reports/monster-groups');
    return grid(
      ['Group/Index', 'Monsters'],
      res.groups.map((g: any) => [
        `${MONSTER_GROUP[g.group] ?? g.group}/${g.index}`,
        h('span', {}, ...joined(g.monsters.map(([n, name]: [number, string]) => link('Monsters', [n], `${name} (${n})`)))),
      ]),
    );
  });
}

function npcRooms(): HTMLElement {
  return card('NPC / Room List', 'Each room with a permanent NPC, by the monster\'s name.', [], async () => {
    const res = await get('/api/reports/npc-rooms');
    return grid(
      ['Room', 'Room Name', 'Monster'],
      res.rooms.map((r: any) => [roomLink(r.map, r.room), r.room_name, link('Monsters', [r.monster], `${r.monster_name} (${r.monster})`)]),
    );
  });
}

export function listsPage(): HTMLElement {
  return h('div', { class: 'tools' }, limitedItems(), findItem(), controlRooms(), monsterGroups(), npcRooms());
}
