// NMR's Monster Group/Index Changer: a list of group/index -> group/index changes, built
// a range at a time, run over the monsters (their Group and Index) and then the rooms
// (their Mon Type and Min-Max Index range). As NMR's, the rooms' ranges are listed so a
// range can be picked whole: a room's range moves as one, so its indexes must move together.

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

interface Change {
  from_group: number;
  from_index: number;
  to_group: number;
  to_index: number;
}

const groupName = (g: number) => MONSTER_GROUP[g] ?? `group ${g}`;
const groupSelect = () =>
  h('select', {}, ...Object.entries(MONSTER_GROUP).map(([v, label]) => h('option', { value: v }, label)));
const box = (value = '') => h('input', { type: 'text', inputmode: 'numeric', class: 'number', value });
const int = (i: HTMLInputElement) => (/^\s*\d+\s*$/.test(i.value) ? Number(i.value) : NaN);

export function indexChanger(
  post: (url: string, body: string) => Promise<any>,
  result: (res: any) => HTMLElement,
  failure: (e: unknown) => HTMLElement,
  changed: () => void,
): HTMLElement {
  const fromGroup = groupSelect();
  const toGroup = groupSelect();
  const fromStart = box();
  const fromEnd = box();
  const toStart = box();
  const toEnd = h('input', { type: 'text', class: 'number', readonly: '', 'aria-label': 'To end' });
  const calc = () => {
    const [a, b, c] = [int(fromStart), int(fromEnd), int(toStart)];
    toEnd.value = [a, b, c].some(Number.isNaN) || b < a ? 'INVALID' : String(c + (b - a));
  };
  for (const i of [fromStart, fromEnd, toStart]) i.addEventListener('input', calc);
  const add = h('button', { type: 'button' }, 'Add →');
  const remove = h('button', { type: 'button' }, 'Remove Line');
  const clear = h('button', { type: 'button' }, 'Clear');
  const list = h('select', { size: '12', class: 'change-list', 'aria-label': 'Change list' });
  const changes: Change[] = [];
  const note = h('p', { class: 'hint' });
  const runs = h('select', { size: '12', class: 'change-list', 'aria-label': 'Indexes from the realm' });
  const byRooms = h('button', { type: 'button' }, 'List Indexes by Rooms');
  const byMonsters = h('button', { type: 'button' }, 'List Indexes by Monsters');
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, 'Start');
  const out = h('div');

  const redraw = () => {
    list.replaceChildren(
      ...changes.map((c, i) =>
        h('option', { value: String(i) }, `${groupName(c.from_group)}/${c.from_index} -> ${groupName(c.to_group)}/${c.to_index}`),
      ),
    );
  };

  add.addEventListener('click', () => {
    calc();
    const [a, b, c] = [int(fromStart), int(fromEnd), int(toStart)];
    if (toEnd.value === 'INVALID') {
      note.textContent = 'Give a start and end to change (end not below start) and a start to change to.';
      return;
    }
    const g = Number(fromGroup.value);
    const clash = changes.find((x) => x.from_group === g && x.from_index >= a && x.from_index <= b);
    if (clash) {
      note.textContent = "Adding this range would include 'change from' indexes you've already added.";
      return;
    }
    for (let x = a; x <= b; x++) {
      changes.push({ from_group: g, from_index: x, to_group: Number(toGroup.value), to_index: c + (x - a) });
    }
    note.textContent = '';
    redraw();
    // as NMR's: the next range starts after this one, and the next listed range is picked
    toStart.value = String(c + (b - a) + 1);
    if (runs.selectedIndex >= 0 && runs.selectedIndex + 1 < runs.options.length) {
      runs.selectedIndex += 1;
      takeRun();
    } else calc();
  });
  remove.addEventListener('click', () => {
    const i = list.selectedIndex;
    if (i < 0) return;
    changes.splice(i, 1);
    redraw();
    if (changes.length) list.selectedIndex = Math.max(0, i - 1);
  });
  clear.addEventListener('click', () => {
    changes.length = 0;
    redraw();
  });

  // the realm's indexes: clicking one fills in the range to change
  function takeRun() {
    const o = runs.selectedOptions[0];
    if (!o) return;
    fromGroup.value = o.dataset.group!;
    fromStart.value = o.dataset.from!;
    fromEnd.value = o.dataset.to!;
    calc();
  }
  runs.addEventListener('change', takeRun);
  const fill = async (url: string, lines: (body: any) => HTMLOptionElement[]) => {
    runs.replaceChildren(h('option', {}, 'Building…'));
    try {
      const res = await fetch(url);
      const body = await res.json();
      if (!res.ok) throw new Error(body?.error ?? res.statusText);
      runs.replaceChildren(...lines(body));
    } catch (e) {
      runs.replaceChildren(h('option', {}, `Could not list: ${(e as Error).message}`));
    }
  };
  const option = (group: number, from: number, to: number, what: string) => {
    const range = from === to ? `${from}` : `${from}-${to}`;
    const o = h('option', {}, `${groupName(group)}/${range}: ${what}`);
    Object.assign(o.dataset, { group: String(group), from: String(from), to: String(to) });
    return o;
  };
  byRooms.addEventListener('click', () =>
    void fill('/api/reports/room-indexes', (b) =>
      b.runs.map((r: any) => option(r.group, r.from, r.to, r.rooms.map(([m, n]: number[]) => `${m}/${n}`).join(', '))),
    ),
  );
  byMonsters.addEventListener('click', () =>
    void fill('/api/reports/monster-groups', (b) =>
      b.groups.map((g: any) => option(g.group, g.index, g.index, g.monsters.map(([n, name]: [number, string]) => `${name}(${n})`).join(', '))),
    ),
  );

  const run = async (dryRun: boolean) => {
    if (!changes.length) {
      out.replaceChildren(h('p', { class: 'error' }, 'Add a change first.'));
      return;
    }
    if (!dryRun && !confirm('Change the monster groups and indexes, and the rooms that use them? Preview first to see them.')) return;
    out.replaceChildren(h('p', {}, dryRun ? 'Previewing…' : 'Changing…'));
    try {
      const parts: HTMLElement[] = [];
      // NMR's order: the monsters, then the rooms
      for (const [table, title] of [
        ['monsters', 'Monsters'],
        ['rooms', 'Rooms'],
      ] as const) {
        const res = await post('/api/tools/preset', JSON.stringify({ preset: 'change_monster_indexes', table, changes, dry_run: dryRun }));
        parts.push(h('h3', {}, title), result(res));
      }
      out.replaceChildren(...parts);
      if (!dryRun) changed();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));

  calc();
  return h(
    'section',
    { class: 'tool' },
    h('h2', {}, 'Monster Group/Index Changer'),
    h(
      'p',
      { class: 'hint' },
      "Moves monsters to new groups and indexes, and the rooms whose Monster Type and index range hold them. A room's range moves as one: change the indexes a room's range covers together (List Indexes by Rooms, then click a line to fill it in).",
    ),
    h(
      'div',
      { class: 'index-changer' },
      h(
        'div',
        {},
        h('h3', {}, 'Change this …'),
        h('div', { class: 'fix-inputs' }, h('label', {}, 'Group ', fromGroup), h('label', {}, 'Start ', fromStart), h('label', {}, 'End ', fromEnd)),
        h('h3', {}, 'To this …'),
        h('div', { class: 'fix-inputs' }, h('label', {}, 'Group ', toGroup), h('label', {}, 'Start ', toStart), h('label', {}, 'End ', toEnd)),
        h('div', { class: 'toolbar' }, add),
        note,
        h('div', { class: 'toolbar' }, byRooms, byMonsters),
        runs,
      ),
      h('div', {}, h('h3', {}, 'Change list'), list, h('div', { class: 'toolbar' }, remove, clear, preview, go)),
    ),
    out,
  );
}
