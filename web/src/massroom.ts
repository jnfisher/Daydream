// NMR's Mass Room Editor: the fields of its form, grouped as it groups them, set on every
// room of a map from one room number to another. Its values can be taken from a room. It
// runs as a Universal Modifier "set" over the range, so it previews like the other tools.

import { toShown, toWire } from './ansi';
import { api, type Cell, type TableInfo } from './api';
import { EXIT_TYPE, EXITS, MONSTER_GROUP, ROOM_TYPE, type Options } from './layouts';

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

const count = (n: number) => Array.from({ length: n }, (_, i) => i);

/** A field of a group: its column, its label, and its choice list if it has one. */
type Field = [column: string, label: string, options?: Options];

/** NMR's groups, in its form's order. */
const GROUPS: [string, Field[]][] = [
  ['Name', [['Name', 'Name']]],
  ['Description', count(7).map((i): Field => [`Desc ${i}`, `Line ${i + 1}`])],
  ['Ansi Map', [['AnsiMap', 'Ansi Map']]],
  ['Room Type', [['Type', 'Room Type', ROOM_TYPE]]],
  ['Shop #', [['Shop Number', 'Shop #']]],
  ['Gang House #', [['Gang House Number', 'Gang House #']]],
  ['Light', [['Light', 'Light']]],
  ['Attributes', [['Attributes', 'Attributes']]],
  ['Permanent NPC', [['Perm NPC', 'Permanent NPC']]],
  ['Monster Type', [['Mon Type', 'Monster Type', MONSTER_GROUP]]],
  ['Min Index', [['Min Index', 'Min Index']]],
  ['Max Index', [['Max Index', 'Max Index']]],
  ['Max Regen', [['Max Regen', 'Max Regen']]],
  ['Max Area', [['Max Area', 'Max Area']]],
  ['Control Room', [['Control Room', 'Control Room']]],
  ['Death Room', [['Death Room', 'Death Room']]],
  ['Exit Room', [['Exit Room', 'Exit Room']]],
  ['CMD Text', [['Command Text', 'CMD Text']]],
  ['Delay', [['Delay', 'Delay']]],
  ['Room Spell', [['Spell', 'Room Spell']]],
  ['Visible Coins', ['Runic', 'Platinum', 'Gold', 'Silver', 'Copper'].map((c): Field => [c, c])],
  ['Hidden Coins', ['Runic', 'Platinum', 'Gold', 'Silver', 'Copper'].map((c): Field => [`Invis${c}`, c])],
  ...EXITS.map((d, i): [string, Field[]] => [
    `Exit ${d}`,
    [[`Exit ${i}`, 'To Room'], [`Type ${i}`, 'Exit Type', EXIT_TYPE], [`Para1 ${i}`, 'Para1'], [`Para2 ${i}`, 'Para2'], [`Para3 ${i}`, 'Para3'], [`Para4 ${i}`, 'Para4']],
  ]),
  ['Items In Room', count(17).flatMap((i): Field[] => [[`Room Item ${i}`, `Item ${i}`], [`Room Item ${i} USES`, 'Uses'], [`Room Item ${i} QTY`, 'Qty - 1']])],
  ['Hidden Items In Room', count(15).flatMap((i): Field[] => [[`Hidden Item ${i}`, `Item ${i}`], [`Hidden Item ${i} USES`, 'Uses'], [`Hidden Item ${i} QTY`, 'Qty - 1']])],
  ['Placed Items', count(10).map((i): Field => [`Placed Item ${i}`, `Placed ${i}`])],
  ['Current Monsters In Room', count(15).map((i): Field => [`CurrentRoomMon ${i}`, `Monster ${i}`])],
];

interface Input {
  column: string;
  el: HTMLInputElement | HTMLSelectElement;
  text: boolean;
}

export function massRoomEditor(rooms: TableInfo, post: (url: string, body: string) => Promise<any>, show: (res: any) => HTMLElement, changed: () => void): HTMLElement {
  const textual = new Set(rooms.columns.filter((c) => c.type === 'text' || c.type === 'memo').map((c) => c.name));
  const num = (value = '') => h('input', { type: 'text', inputmode: 'numeric', class: 'number', value });
  const card = h('section', { class: 'tool' }, h('h2', {}, 'Mass Room Editor'));
  card.append(
    h('p', { class: 'hint' }, 'Set the checked fields on every room of a map from one room to another. Take the values from a room, or type them. Item quantities are the count less one (0 for 1).'),
  );
  const map = num('1');
  const from = num('1');
  const to = num('1');
  const srcMap = num('1');
  const srcRoom = num('1');
  const take = h('button', { type: 'button' }, 'Take Values');
  card.append(
    h('div', { class: 'toolbar' }, h('label', {}, 'Map ', map), h('label', {}, 'From: ', from), h('label', {}, 'To: ', to)),
    h('div', { class: 'toolbar' }, h('span', {}, 'Values from room '), h('label', {}, 'Map ', srcMap), h('label', {}, 'Room # ', srcRoom), take),
  );

  const inputs: Input[] = [];
  const groups: { box: HTMLInputElement; fields: Input[] }[] = [];
  const list = h('div', { class: 'mass-groups' });
  for (const [label, fields] of GROUPS) {
    const box = h('input', { type: 'checkbox' });
    const body = h('div', { class: 'mass-fields', hidden: '' });
    const mine: Input[] = [];
    for (const [column, name, opts] of fields) {
      const text = textual.has(column);
      const el = opts
        ? h('select', {}, ...Object.entries(opts).map(([v, l]) => h('option', { value: v }, l)))
        : h('input', { type: 'text', class: text ? 'text' : 'number', value: text ? '' : '0' });
      const i = { column, el, text };
      mine.push(i);
      inputs.push(i);
      body.append(h('label', {}, `${name} `, el));
    }
    box.addEventListener('change', () => (body.hidden = !box.checked));
    groups.push({ box, fields: mine });
    list.append(h('div', { class: 'mass-group' }, h('label', { class: 'check' }, box, ` ${label}`), body));
  }
  const all = h('button', { type: 'button' }, 'Select All');
  const none = h('button', { type: 'button' }, 'Select None');
  const setAll = (on: boolean) =>
    groups.forEach((g) => {
      g.box.checked = on;
      g.box.dispatchEvent(new Event('change'));
    });
  all.addEventListener('click', () => setAll(true));
  none.addEventListener('click', () => setAll(false));
  card.append(h('div', { class: 'toolbar' }, all, none), list);

  const out = h('div');
  take.addEventListener('click', async () => {
    try {
      const row = await api.row('Rooms', [Number(srcMap.value), Number(srcRoom.value)]);
      for (const i of inputs) {
        const v: Cell = row.values[i.column] ?? (i.text ? '' : 0);
        i.el.value = i.text ? toShown(String(v ?? '')) : String(v ?? 0);
      }
      out.replaceChildren(h('p', {}, `Values taken from room ${srcMap.value}/${srcRoom.value}.`));
    } catch (e) {
      out.replaceChildren(h('p', { class: 'error' }, e instanceof Error ? e.message : String(e)));
    }
  });

  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, 'Go');
  card.append(h('div', { class: 'toolbar' }, preview, go), out);
  const run = async (dryRun: boolean) => {
    const values: [string, Cell][] = [];
    for (const g of groups) {
      if (!g.box.checked) continue;
      for (const i of g.fields) {
        if (i.text) values.push([i.column, toWire(i.el.value)]);
        else {
          const n = Number(i.el.value.trim() || 0);
          if (!Number.isFinite(n)) {
            out.replaceChildren(h('p', { class: 'error' }, `${i.column}: ${i.el.value} is not a number.`));
            return;
          }
          values.push([i.column, n]);
        }
      }
    }
    if (!values.length) {
      out.replaceChildren(h('p', { class: 'error' }, 'Check the fields to set.'));
      return;
    }
    const range = { table: 'Rooms', map: Number(map.value), from: Number(from.value), to: Number(to.value) };
    if (range.from > range.to) {
      out.replaceChildren(h('p', { class: 'error' }, 'Illegal Range Entered!'));
      return;
    }
    if (!dryRun && !confirm(`Set ${values.length} fields on rooms ${range.from} to ${range.to} of map ${range.map}?`)) return;
    out.replaceChildren(h('p', {}, dryRun ? 'Previewing…' : 'Changing…'));
    try {
      const res = await post('/api/tools/modify', JSON.stringify({ table: 'Rooms', action: { kind: 'set', values }, range, only_if: [], dry_run: dryRun }));
      out.replaceChildren(show(res));
      if (!dryRun) changed();
    } catch (e) {
      out.replaceChildren(h('p', { class: 'error' }, e instanceof Error ? e.message : String(e)));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));
  return card;
}
