// NMR's Record Number Changer: records of one table move to new numbers, and everything
// that names them follows (other tables' fields, abilities, text block commands). Or, with
// Change Map Number, a map's rooms move to another map. Each run previews first.

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

/** NMR's list, with the API's names. */
const KINDS: [string, string][] = [
  ['classes', 'Classes'],
  ['items', 'Items'],
  ['messages', 'Messages'],
  ['monsters', 'Monsters'],
  ['shops', 'Shops'],
  ['spells', 'Spells'],
  ['races', 'Races'],
  ['rooms', 'Rooms'],
  ['textblocks', 'Textblocks'],
  ['map', 'Change Map Number'],
];

interface Move {
  from: number;
  to: number;
}

const box = (label: string) => h('input', { type: 'text', inputmode: 'numeric', class: 'number', 'aria-label': label });
const int = (i: HTMLInputElement) => (/^\s*\d+\s*$/.test(i.value) ? Number(i.value) : NaN);

export function recordNumberChanger(
  post: (url: string, body: string) => Promise<any>,
  result: (res: any) => HTMLElement,
  failure: (e: unknown) => HTMLElement,
  changed: () => void,
): HTMLElement {
  const kind = h('select', { 'aria-label': 'Records' }, ...KINDS.map(([v, l]) => h('option', { value: v }, l)));
  kind.value = 'items';
  const map = box('Map');
  const fromStart = box('Change from start');
  const fromEnd = box('Change from end');
  const toStart = box('Change to start');
  const toEnd = h('input', { type: 'text', class: 'number', readonly: '', 'aria-label': 'Change to end' });
  const calc = () => {
    const [a, b, c] = [int(fromStart), int(fromEnd), int(toStart)];
    toEnd.value = [a, b, c].some(Number.isNaN) || b < a ? 'INVALID' : String(c + (b - a));
  };
  for (const i of [fromStart, fromEnd, toStart]) i.addEventListener('input', calc);
  const note = h('p', { class: 'hint' });
  const list = h('select', { size: '12', class: 'change-list', 'aria-label': 'Change list' });
  const moves: Move[] = [];
  const records = h('select', { size: '12', class: 'change-list', 'aria-label': 'Records in the realm' });
  const out = h('div');

  // Change Map Number's own form
  const mapFrom = box('Change map');
  const mapTo = box('To map');
  const boxes = (
    [
      ['exits', "Change 'Map Change' exit types on rooms to new map."],
      ['normal', "If the new map on a map change exit now matches the rooms' map, change the exit type to normal."],
      ['textblocks', 'Change Teleport commands in textblocks to new map.'],
      ['spells', 'Change Teleport abilities in spells to new map.'],
    ] as const
  ).map(([k, label]) => {
    const c = h('input', { type: 'checkbox' });
    c.checked = true;
    return [k, c, h('label', {}, c, ' ', label)] as const;
  });
  const mapForm = h(
    'div',
    { class: 'renumber-map' },
    h('div', { class: 'fix-inputs' }, h('label', {}, 'Change Map: ', mapFrom), h('label', {}, 'To Map: ', mapTo)),
    ...boxes.map(([, , l]) => h('div', {}, l)),
  );

  const redraw = () => {
    list.replaceChildren(...moves.map((m, i) => h('option', { value: String(i) }, `${m.from} -> ${m.to}`)));
    const locked = moves.length > 0;
    kind.disabled = locked;
    map.readOnly = locked;
  };

  const add = (a: number, b: number, c: number): boolean => {
    for (const m of moves) {
      const clash =
        (m.from >= a && m.from <= b) || (m.to >= a && m.to <= b) || (m.from >= c && m.from <= c + (b - a)) || (m.to >= c && m.to <= c + (b - a));
      if (clash) {
        note.textContent = `Adding this range would include record numbers you've already added (record ${m.from} is set to be changed to ${m.to}).`;
        return false;
      }
    }
    for (let x = a; x <= b; x++) moves.push({ from: x, to: c + (x - a) });
    note.textContent = '';
    redraw();
    toStart.value = String(c + (b - a) + 1);
    calc();
    return true;
  };
  const addButton = h('button', { type: 'button' }, 'Add →');
  addButton.addEventListener('click', () => {
    calc();
    if (kind.value === 'rooms' && !(int(map) > 0)) {
      note.textContent = 'Please enter a map number first.';
      return;
    }
    if (toEnd.value === 'INVALID') {
      note.textContent = 'Give a start and end to change (end not below start) and a start to change to.';
      return;
    }
    add(int(fromStart), int(fromEnd), int(toStart));
  });
  const removeButton = h('button', { type: 'button' }, 'Remove');
  removeButton.addEventListener('click', () => {
    const i = list.selectedIndex;
    if (i < 0) return;
    moves.splice(i, 1);
    redraw();
  });
  const clearButton = h('button', { type: 'button' }, 'Clear');
  clearButton.addEventListener('click', () => {
    moves.length = 0;
    redraw();
  });

  // NMR's List Records: the realm's numbers in runs; clicking one fills it in
  const listButton = h('button', { type: 'button' }, 'List Records');
  const addAll = h('button', { type: 'button' }, 'Add All');
  const tableName = () => KINDS.find(([v]) => v === kind.value)![1];
  listButton.addEventListener('click', async () => {
    records.replaceChildren(h('option', {}, 'Building…'));
    try {
      const res = await fetch(`/api/tables/${tableName()}/rows`);
      const body = await res.json();
      if (!res.ok) throw new Error(body?.error ?? res.statusText);
      const want = int(map);
      const nums = [
        ...new Set<number>(
          (body.rows as [number[], string][])
            .filter(([key]) => kind.value !== 'rooms' || key[0] === want)
            .map(([key]) => (kind.value === 'rooms' ? key[1] : key[0])!),
        ),
      ].sort((x, y) => x - y);
      const runs: [number, number][] = [];
      for (const x of nums) {
        const last = runs[runs.length - 1];
        if (last && last[1] + 1 === x) last[1] = x;
        else runs.push([x, x]);
      }
      records.replaceChildren(
        ...runs.map(([a, b]) => {
          const o = h('option', {}, a === b ? `${a}` : `${a}-${b}`);
          Object.assign(o.dataset, { from: String(a), to: String(b) });
          return o;
        }),
      );
      if (!runs.length) records.replaceChildren(h('option', {}, kind.value === 'rooms' ? `No rooms on map ${want || '?'}.` : 'No records.'));
    } catch (e) {
      records.replaceChildren(h('option', {}, `Could not list: ${(e as Error).message}`));
    }
  });
  records.addEventListener('change', () => {
    const o = records.selectedOptions[0];
    if (!o?.dataset.from) return;
    fromStart.value = o.dataset.from;
    fromEnd.value = o.dataset.to!;
    calc();
  });
  addAll.addEventListener('click', () => {
    if (!(int(toStart) > 0)) {
      note.textContent = 'Give the number to start changing to first.';
      return;
    }
    for (const o of records.options) {
      if (!o.dataset.from) continue;
      if (!add(Number(o.dataset.from), Number(o.dataset.to), int(toStart))) break;
    }
  });

  const recordsForm = h(
    'div',
    { class: 'index-changer' },
    h(
      'div',
      {},
      h('div', { class: 'fix-inputs' }, h('label', {}, 'Map: ', map)),
      h('h3', {}, 'Change This …'),
      h('div', { class: 'fix-inputs' }, h('label', {}, 'Start ', fromStart), h('label', {}, 'End ', fromEnd)),
      h('h3', {}, 'To This …'),
      h('div', { class: 'fix-inputs' }, h('label', {}, 'Start ', toStart), h('label', {}, 'End ', toEnd)),
      h('div', { class: 'toolbar' }, addButton),
      note,
      h('div', { class: 'toolbar' }, listButton, addAll),
      records,
    ),
    h('div', {}, h('h3', {}, 'Change list'), list, h('div', { class: 'toolbar' }, removeButton, clearButton)),
  );
  const showForm = () => {
    const isMap = kind.value === 'map';
    recordsForm.hidden = isMap;
    mapForm.hidden = !isMap;
    (map.parentElement as HTMLElement).hidden = kind.value !== 'rooms';
    records.replaceChildren();
    out.replaceChildren();
  };
  kind.addEventListener('change', showForm);

  const request = (): object => {
    if (kind.value === 'map') {
      const body: Record<string, unknown> = { kind: 'map', from: int(mapFrom), to: int(mapTo) };
      for (const [k, c] of boxes) body[k] = c.checked;
      return body;
    }
    return { kind: 'records', table: kind.value, map: kind.value === 'rooms' ? int(map) : undefined, moves };
  };
  const run = async (dryRun: boolean) => {
    if (!dryRun && !confirm('Change the record numbers, and everything that names them? Preview first to see the changes.')) return;
    out.replaceChildren(h('p', {}, dryRun ? 'Previewing…' : 'Changing…'));
    try {
      const res = await post('/api/tools/renumber', JSON.stringify({ ...request(), dry_run: dryRun }));
      const parts: HTMLElement[] = res.tables.map((t: any) => h('div', {}, h('h3', {}, t.table), result(t)));
      out.replaceChildren(...(parts.length ? parts : [h('p', {}, 'Nothing would change.')]));
      if (!dryRun) {
        changed();
        moves.length = 0;
        redraw();
      }
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, 'Start');
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));

  calc();
  const card = h(
    'section',
    { class: 'tool' },
    h('h2', {}, 'Record Number Changer'),
    h(
      'p',
      { class: 'hint' },
      'Moves records to new numbers, and changes every place that names them: other records, abilities and text block commands, as NMR does. New numbers must be free. Rooms move within one map; Change Map Number moves a whole map.',
    ),
    h('div', { class: 'toolbar' }, h('label', {}, 'Records: ', kind)),
    recordsForm,
    mapForm,
    h('div', { class: 'toolbar' }, preview, go),
    out,
  );
  showForm();
  return card;
}
