// Daydream's web UI: a list of a table's records on the left and its editor on the right,
// laid out as Nightmare Redux's forms are (layouts.ts). Every save goes to the server,
// which writes it through the same import an NMR .mdb goes through.

import { ABILITIES } from './abilities';
import { renderAnsi, titleOf, toShown, toWire } from './ansi';
import { api, type Cell, type ColumnInfo, type Row, type TableInfo } from './api';
import { columnsShown, layoutFor, type Field, type Layout, type Pick } from './layouts';
import { listsPage } from './lists';
import { mapPage } from './map';
import { toolsPage } from './tools';
import { parseStats } from './users';

/** The editors' tables, in NMR's menu order. */
const ORDER = ['Items', 'Monsters', 'Spells', 'Classes', 'Races', 'Shops', 'Rooms', 'Messages', 'Textblocks', 'Actions', 'Users', 'BankBooks', 'Gangs'];
/** The most rows the list shows at once; the filter narrows the rest. */
const LIST_LIMIT = 500;

interface State {
  tables: TableInfo[];
  table: TableInfo | null;
  layout: Layout;
  rows: [Cell[], Cell][];
  filter: string;
  row: Row | null;
  /** What the editor's inputs hold, as typed, by column. */
  edits: Map<string, string>;
  tab: number;
  /** Names of other tables' records, for reference fields. */
  names: Map<string, Map<number, string>>;
}

const state: State = {
  tables: [],
  table: null,
  layout: [],
  rows: [],
  filter: '',
  row: null,
  edits: new Map(),
  tab: 0,
  names: new Map(),
};

const $ = <T extends HTMLElement>(sel: string) => document.querySelector(sel) as T;

function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Partial<Record<string, string>> = {},
  ...children: (Node | string | null)[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v === undefined) continue;
    if (k === 'class') el.className = v;
    else el.setAttribute(k, v);
  }
  for (const c of children) if (c !== null) el.append(c);
  return el;
}

const show = (v: Cell) => (v === null ? '' : toShown(String(v)));
const keyText = (key: Cell[]) => key.map(show).join('/');
const sameKey = (a: Cell[], b: Cell[]) => JSON.stringify(a) === JSON.stringify(b);

function status(text: string, kind: 'ok' | 'error' | 'info' = 'info') {
  const el = $('#status');
  el.textContent = text;
  el.className = kind;
}

function fail(e: unknown) {
  status(e instanceof Error ? e.message : String(e), 'error');
}

// ---- dirty state ----------------------------------------------------------

function column(name: string): ColumnInfo | undefined {
  return state.table?.columns.find((c) => c.name === name);
}

function changed(): Record<string, Cell> {
  const out: Record<string, Cell> = {};
  if (!state.row) return out;
  for (const [col, typed] of state.edits) {
    if (typed !== show(state.row.values[col] ?? null)) out[col] = toWire(typed);
  }
  return out;
}

const dirty = () => Object.keys(changed()).length > 0;

function confirmLeave(): boolean {
  return !dirty() || confirm('Discard the changes to this record?');
}

function refreshDirty() {
  const d = changed();
  for (const el of document.querySelectorAll<HTMLElement>('[data-col]')) {
    el.classList.toggle('changed', el.dataset.col! in d);
  }
  for (const b of document.querySelectorAll<HTMLButtonElement>('.needs-dirty')) b.disabled = !dirty();
  const n = Object.keys(d).length;
  $('#dirty').textContent = n ? `${n} unsaved change${n === 1 ? '' : 's'}` : '';
}

// ---- reference names ------------------------------------------------------

async function names(table: string): Promise<Map<number, string>> {
  let m = state.names.get(table);
  if (m) return m;
  m = new Map();
  state.names.set(table, m);
  try {
    const list = await api.rows(table);
    for (const [key, title] of list.rows) {
      if (typeof key[0] === 'number') m.set(key[0], show(title));
    }
  } catch {
    // a realm without the table: numbers stay bare
  }
  return m;
}

function pickName(pick: Pick | undefined, value: string): string {
  if (!pick || value.trim() === '') return '';
  const n = Number(value);
  if ('abilities' in pick) return n === 0 ? '' : (ABILITIES[n]?.[0] ?? 'unknown ability');
  if ('ref' in pick) return n === 0 ? '' : (state.names.get(pick.ref)?.get(n) ?? 'none such');
  return '';
}

// ---- rendering --------------------------------------------------------------

function renderNav() {
  const nav = $('#tables');
  nav.replaceChildren();
  const ordered = [...state.tables].sort(
    (a, b) => (ORDER.indexOf(a.name) + 1 || 99) - (ORDER.indexOf(b.name) + 1 || 99),
  );
  for (const t of ordered) {
    const a = h('a', { href: `#/${encodeURIComponent(t.name)}` }, t.name);
    if (t === state.table && !onPage()) a.classList.add('active');
    nav.append(a);
  }
  const map = h('a', { href: '#/map', class: 'tools-link' }, 'Map');
  const lists = h('a', { href: '#/lists' }, 'Lists');
  const tools = h('a', { href: '#/tools' }, 'Tools');
  if (onPage() === 'map') map.classList.add('active');
  if (onPage() === 'lists') lists.classList.add('active');
  if (onPage() === 'tools') tools.classList.add('active');
  nav.append(map, lists, tools);
}

type Page = 'tools' | 'lists' | 'map';
const PAGES: Page[] = ['tools', 'lists', 'map'];

/** The page the hash names other than an editor: Tools, Lists or Map. */
const onPage = (): Page | null => PAGES.find((p) => location.hash.startsWith(`#/${p}`)) ?? null;

let theMap: ReturnType<typeof mapPage> | null = null;
let lastPage: Page | null = null;

function showPage(which: Page | null) {
  ($('main') as HTMLElement).hidden = which !== null;
  for (const id of PAGES) {
    const page = $(`#${id}`);
    page.hidden = which !== id;
    if (id === 'map' && which === 'map') {
      if (!theMap) {
        theMap = mapPage();
        page.append(theMap.el);
      }
      // coming back from elsewhere, the rooms may have been edited: load them afresh
      void theMap.show(lastPage !== 'map');
      continue;
    }
    if (which === id && !page.childElementCount) {
      page.append(
        id === 'lists'
          ? listsPage()
          : toolsPage(() => {
              // the realm changed under the editors: reload what they show
              state.table = null;
              state.row = null;
              state.names.clear();
            }),
      );
    }
  }
  lastPage = which;
}

function renderList() {
  const list = $('#rows');
  list.replaceChildren();
  const needle = state.filter.trim().toLowerCase();
  const matches = state.rows.filter(
    ([key, title]) => !needle || `${keyText(key)} ${show(title)}`.toLowerCase().includes(needle),
  );
  for (const [key, title] of matches.slice(0, LIST_LIMIT)) {
    const li = h('li', {}, h('span', { class: 'key' }, keyText(key)), ' ', show(title).trimEnd());
    if (state.row && sameKey(state.row.key, key)) li.classList.add('selected');
    li.addEventListener('click', () => select(key));
    list.append(li);
  }
  $('#count').textContent =
    matches.length > LIST_LIMIT
      ? `${LIST_LIMIT} of ${matches.length}; filter to narrow`
      : `${matches.length} of ${state.rows.length}`;
}

function input(field: Field, info: ColumnInfo | undefined): HTMLElement {
  const col = field.col;
  const value = state.edits.get(col) ?? '';
  const isKey = state.table?.key.includes(col) ?? false;
  const pick = field.pick;
  let control: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
  if (pick && 'options' in pick) {
    const select = h('select', { 'data-col': col });
    const options = { ...pick.options };
    if (value !== '' && !(Number(value) in options)) options[Number(value)] = `${value} (unknown)`;
    for (const [n, label] of Object.entries(options)) {
      select.append(h('option', { value: n }, `${n}: ${label}`));
    }
    select.value = value;
    control = select;
  } else if (info?.type === 'memo') {
    control = h('textarea', { 'data-col': col, rows: '8', spellcheck: 'false' });
    control.value = value;
  } else {
    control = h('input', {
      'data-col': col,
      type: 'text',
      spellcheck: 'false',
      inputmode: info?.type === 'text' ? undefined : 'numeric',
      maxlength: info?.size ? String(info.size) : undefined,
    });
    control.value = value;
    if (info?.type !== 'text') control.classList.add('number');
  }
  if (isKey) {
    control.setAttribute('disabled', '');
    control.title = 'The record’s key: insert a new record to use another';
  }
  const name = h('span', { class: 'pick-name' }, pickName(pick, value));
  if (pick && 'abilities' in pick) name.title = ABILITIES[Number(value)]?.[1] ?? '';
  control.addEventListener('input', () => {
    state.edits.set(col, control.value);
    name.textContent = pickName(pick, control.value);
    refreshDirty();
  });
  const wrap = h('span', { class: 'control' }, control);
  if (pick && !('options' in pick)) wrap.append(name);
  return wrap;
}

function renderField(field: Field): HTMLElement {
  const info = column(field.col);
  if (!info) return h('span');
  const label = field.label ?? field.col;
  return h(
    'label',
    { class: field.wide ? 'field wide' : 'field', title: field.col },
    h('span', { class: 'label' }, label),
    input(field, info),
  );
}

function renderTab(tabIndex: number): HTMLElement {
  const tab = state.layout[tabIndex];
  const body = h('div', { class: 'tab-body' });
  if (!tab) return body;
  if (tab.fields) {
    const grid = h('div', { class: 'fields' });
    for (const field of tab.fields) grid.append(renderField(field));
    body.append(grid);
  }
  if (tab.preview) {
    const box = h('pre', { class: 'terminal', 'aria-label': 'Preview' });
    const draw = () => {
      box.replaceChildren();
      tab.preview!.forEach((col, i) => {
        if (i) box.append('\n');
        box.append(renderAnsi(state.edits.get(col) ?? ''));
      });
    };
    draw();
    body.addEventListener('input', draw);
    body.append(h('div', { class: 'preview-label' }, 'As players see it'), box);
  }
  if (tab.matrix) {
    const m = tab.matrix;
    const table = h('table', { class: 'grid matrix' });
    const head = h('tr', {}, h('th'));
    for (const label of m.headers) head.append(h('th', {}, label));
    table.append(h('thead', {}, head));
    const tbody = h('tbody');
    for (const row of m.rows) {
      const tr = h('tr', {}, h('td', { class: 'n' }, row.label));
      for (const col of row.cols) {
        const info = column(col);
        tr.append(h('td', { title: col }, info ? input({ col }, info) : ''));
      }
      tbody.append(tr);
    }
    table.append(tbody);
    body.append(table);
  }
  if (tab.grid) {
    const g = tab.grid;
    if (g.across) {
      const table = h('table', { class: 'grid across' });
      const head = h('tr', {}, h('th'));
      for (let n = g.from; n <= g.to; n++) head.append(h('th', {}, `${g.across} ${n - g.from + 1}`));
      table.append(h('thead', {}, head));
      const tbody = h('tbody');
      for (const c of g.columns) {
        const tr = h('tr', {}, h('td', { class: 'n' }, c.label ?? c.template));
        for (let n = g.from; n <= g.to; n++) {
          const col = c.template.replace('#', String(n));
          const info = column(col);
          tr.append(h('td', { title: col }, info ? input({ col, pick: c.pick }, info) : ''));
        }
        tbody.append(tr);
      }
      table.append(tbody);
      body.append(table);
      return body;
    }
    const table = h('table', { class: 'grid' });
    const head = h('tr', {}, h('th', {}, '#'));
    for (const c of g.columns) head.append(h('th', {}, c.label ?? c.template));
    table.append(h('thead', {}, head));
    const tbody = h('tbody');
    for (let n = g.from; n <= g.to; n++) {
      const tr = h('tr', {}, h('td', { class: 'n' }, g.names?.[n - g.from] ?? String(n)));
      for (const c of g.columns) {
        const col = c.template.replace('#', String(n));
        const info = column(col);
        tr.append(h('td', {}, info ? input({ col, pick: c.pick }, info) : ''));
      }
      tbody.append(tr);
    }
    table.append(tbody);
    body.append(table);
  }
  return body;
}

function renderEditor() {
  const ed = $('#editor');
  ed.replaceChildren();
  const t = state.table;
  if (!t) return;
  if (!state.row) {
    ed.append(h('p', { class: 'empty' }, 'Pick a record on the left, or insert a new one.'));
    ed.append(toolbar(t));
    return;
  }
  const title = t.title ? titleOf(show(state.row.values[t.title] ?? null)) : '';
  ed.append(h('h2', {}, `${t.name} ${keyText(state.row.key)}`, title ? ` — ${title}` : ''));
  ed.append(toolbar(t));
  const tabs = h('div', { class: 'tabs', role: 'tablist' });
  state.layout.forEach((tab, i) => {
    const b = h('button', { role: 'tab', type: 'button' }, tab.title);
    if (i === state.tab) b.classList.add('active');
    b.addEventListener('click', () => {
      state.tab = i;
      renderEditor();
    });
    tabs.append(b);
  });
  ed.append(tabs, renderTab(state.tab));
  refreshDirty();
}

function toolbar(t: TableInfo): HTMLElement {
  const bar = h('div', { class: 'toolbar' });
  const button = (label: string, title: string, run: () => void, cls = '') => {
    const b = h('button', { type: 'button', title, class: cls }, label);
    b.addEventListener('click', run);
    bar.append(b);
    return b;
  };
  if (state.row) {
    button('Save', 'Write the changes to the realm (Ctrl+S)', () => void save(), 'needs-dirty primary');
    button('Discard', 'Undo the unsaved changes', discard, 'needs-dirty');
  }
  button('Insert', state.row ? `Copy this record to a new ${t.name} record` : `Add a new ${t.name} record`, () => void insert());
  if (state.row) button('Delete', 'Remove this record from the realm', () => void remove(), 'danger');
  if (state.row && t.name === 'Rooms') {
    const [map, room] = state.row.key;
    bar.append(h('a', { href: `#/map/${map}/${room}`, class: 'button-link' }, 'Map'));
  }
  if (state.row && t.name === 'Users') {
    button('Revive', 'NMR\'s Revive Character: HP and Mana to full, then lives, spells cast and where to move them', () => void revive());
    button('Paste Stats', 'NMR\'s Paste Character: read a capture of the game\'s stat output into the fields', () => void pasteStats());
  }
  bar.append(h('span', { id: 'dirty' }));
  return bar;
}

// ---- actions -----------------------------------------------------------------

function loadEdits(row: Row) {
  state.edits = new Map(Object.entries(row.values).map(([k, v]) => [k, show(v)]));
}

async function openTable(name: string, key?: Cell[]) {
  const t = state.tables.find((x) => x.name === name);
  if (!t) return;
  if (t !== state.table) {
    state.table = t;
    const layout = layoutFor(t);
    const rest = t.columns.map((c) => c.name).filter((c) => !columnsShown(layout).has(c));
    state.layout = rest.length ? [...layout, { title: 'More', fields: rest.map((col) => ({ col })) }] : layout;
    state.tab = 0;
    state.row = null;
    state.filter = '';
    ($('#filter') as HTMLInputElement).value = '';
    renderNav();
    status(`Loading ${t.name}…`);
    const list = await api.rows(t.name);
    state.rows = list.rows;
    status(`${t.name}: ${list.rows.length} records`);
    // names the editor's reference fields show
    const refs = new Set<string>();
    for (const tab of state.layout) {
      for (const f of tab.fields ?? []) if (f.pick && 'ref' in f.pick) refs.add(f.pick.ref);
      for (const c of tab.grid?.columns ?? []) if (c.pick && 'ref' in c.pick) refs.add(c.pick.ref);
    }
    await Promise.all([...refs].map(names));
  }
  if (key) {
    try {
      state.row = await api.row(t.name, key);
      loadEdits(state.row);
    } catch (e) {
      state.row = null;
      fail(e);
    }
  }
  renderList();
  renderEditor();
}

/** NMR's Revive Character, asking its questions as NMR does, then writing the user. */
async function revive() {
  const row = state.row;
  if (!row || !state.table) return;
  if (Object.keys(changed()).length) {
    status('Save or discard your changes first.', 'error');
    return;
  }
  const v = (c: string) => Number(row.values[c] ?? 0);
  const lives = v('Lives') < 9 && confirm('Reset Lives to 9?');
  const cast = v('Poison') !== 0 || Array.from({ length: 10 }, (_, i) => v(`Spell Cast ${i}`)).some((x) => x > 0);
  const spells = cast && confirm('Reset Auras (clear the spells cast on them)?');
  let to: unknown = null;
  if (v('Map Number') !== 1 || v('Room Number') !== 2189) {
    const answer = prompt(
      'Move user?\n- Enter 1-19: move them that many rooms back\n- Enter 99: to the Halls of the Dead\n- Enter Map/Room, as 1/2189\n\nEnter 0 or cancel to leave them alone.',
      '0',
    );
    const mr = /^\s*(\d+)\s*\/\s*(\d+)\s*$/.exec(answer ?? '');
    const n = parseInt(answer ?? '', 10) || 0;
    if (mr) to = { room: { map: Number(mr[1]), room: Number(mr[2]) } };
    else if (n === 99) to = 'halls';
    else if (n >= 1 && n <= 19) to = { back: n };
  }
  try {
    await api.preset({ preset: 'revive_character', user: String(row.key[0]), lives, spells, to });
    const fresh = await api.row(state.table.name, row.key);
    state.row = fresh;
    loadEdits(fresh);
    renderEditor();
    status(`Revived ${row.key[0]}.`, 'ok');
  } catch (e) {
    fail(e);
  }
}

/** NMR's Paste Character: the capture goes into the fields, to be saved as any edit. */
async function pasteStats() {
  const text = h('textarea', { rows: '14', cols: '80', spellcheck: 'false', 'aria-label': 'Stat capture' });
  const ok = h('button', { type: 'submit', value: 'ok', class: 'primary' }, 'Paste');
  const cancel = h('button', { type: 'submit', value: 'cancel' }, 'Cancel');
  const form = h(
    'form',
    { method: 'dialog' },
    h('p', {}, "Paste a capture of a character's stat output (or MMUD Explorer's Copy Only Stats). Class, race, level, exp, lives, CP, HP, mana and the six stats are read."),
    text,
    h('div', { class: 'toolbar' }, ok, cancel),
  );
  const dialog = h('dialog', { class: 'paste-dialog' }, form);
  document.body.append(dialog);
  dialog.showModal();
  const answer = await new Promise<string>((done) => dialog.addEventListener('close', () => done(dialog.returnValue), { once: true }));
  dialog.remove();
  if (answer !== 'ok' || text.value.trim().length < 10) return;
  const { values, race, cls } = parseStats(text.value);
  for (const [table, name, col] of [
    ['Races', race, 'Race'],
    ['Classes', cls, 'Class'],
  ] as const) {
    if (!name) continue;
    for (const [n, title] of await names(table)) if (title === name) values[col] = String(n);
  }
  for (const [col, v] of Object.entries(values)) state.edits.set(col, v);
  renderEditor();
  const n = Object.keys(values).length;
  status(n ? `Pasted ${n} fields; save to keep them.` : 'Nothing in the paste was recognised.', n ? 'ok' : 'error');
}

function select(key: Cell[]) {
  if (!state.table || (state.row && sameKey(state.row.key, key))) return;
  if (!confirmLeave()) return;
  location.hash = `#/${encodeURIComponent(state.table.name)}/${encodeURIComponent(JSON.stringify(key))}`;
}

async function save() {
  const t = state.table;
  const row = state.row;
  if (!t || !row) return;
  const values = changed();
  if (!Object.keys(values).length) return;
  try {
    const report = await api.save(t.name, row.key, values);
    state.row = await api.row(t.name, row.key);
    loadEdits(state.row);
    const i = state.rows.findIndex(([k]) => sameKey(k, row.key));
    if (i >= 0 && t.title) {
      const v = state.row.values[t.title] ?? null;
      state.rows[i] = [row.key, typeof v === 'string' ? titleOf(v) : v];
    }
    state.names.delete(t.name);
    renderList();
    renderEditor();
    status(report.updated ? `Saved ${t.name} ${keyText(row.key)}.` : 'Nothing changed in the record.', 'ok');
  } catch (e) {
    fail(e);
  }
}

function discard() {
  if (!state.row) return;
  loadEdits(state.row);
  renderEditor();
  status('Changes discarded.');
}

/** The key of a new record: the next number, or what the user types for other keys. */
function newKey(t: TableInfo): Cell[] | null {
  const numeric = t.key.length === 1 && t.columns.find((c) => c.name === t.key[0])?.type === 'long';
  if (numeric) {
    const max = state.rows.reduce((m, [k]) => Math.max(m, Number(k[0]) || 0), 0);
    const typed = prompt(`Number of the new ${t.name} record:`, String(max + 1));
    if (typed === null) return null;
    const n = Number(typed.trim());
    if (typed.trim() === '' || !Number.isInteger(n) || n <= 0) {
      status(`${typed} is not a record number`, 'error');
      return null;
    }
    return [n];
  }
  const typed = prompt(`${t.key.join(', ')} of the new ${t.name} record (separate with /):`);
  if (typed === null) return null;
  const parts = typed.split('/').map((s) => s.trim());
  if (parts.length !== t.key.length) {
    status(`A ${t.name} key is ${t.key.length} value(s): ${t.key.join(', ')}`, 'error');
    return null;
  }
  return parts.map((p, i) => (t.columns.find((c) => c.name === t.key[i])?.type === 'text' ? p : Number(p)));
}

/** A new record, as NMR's Insert makes it: the current record (saved first) under a new key. */
async function insert() {
  const t = state.table;
  if (!t) return;
  if (dirty()) await save();
  if (dirty()) return;
  const key = newKey(t);
  if (!key) return;
  const values: Record<string, Cell> = {};
  if (state.row) for (const [col, typed] of state.edits) values[col] = toWire(typed);
  t.key.forEach((c, i) => (values[c] = key[i] ?? null));
  try {
    await api.insert(t.name, values);
    const list = await api.rows(t.name);
    state.rows = list.rows;
    state.row = null;
    state.names.delete(t.name);
    status(`Inserted ${t.name} ${keyText(key)}.`, 'ok');
    location.hash = `#/${encodeURIComponent(t.name)}/${encodeURIComponent(JSON.stringify(key))}`;
  } catch (e) {
    fail(e);
  }
}

async function remove() {
  const t = state.table;
  const row = state.row;
  if (!t || !row) return;
  if (!confirm(`Delete ${t.name} ${keyText(row.key)} from the realm? This cannot be undone.`)) return;
  try {
    await api.remove(t.name, row.key);
    state.rows = state.rows.filter(([k]) => !sameKey(k, row.key));
    state.row = null;
    state.names.delete(t.name);
    status(`Deleted ${t.name} ${keyText(row.key)}.`, 'ok');
    location.hash = `#/${encodeURIComponent(t.name)}`;
    renderList();
    renderEditor();
  } catch (e) {
    fail(e);
  }
}

// ---- routing -----------------------------------------------------------------

async function route() {
  showPage(onPage());
  if (onPage()) {
    renderNav();
    return;
  }
  const [, table, key] = location.hash.split('/');
  const name = decodeURIComponent(table ?? '') || state.tables.find((t) => t.name === ORDER[0])?.name;
  if (!name) return;
  let parsed: Cell[] | undefined;
  try {
    parsed = key ? (JSON.parse(decodeURIComponent(key)) as Cell[]) : undefined;
  } catch {
    parsed = undefined;
  }
  if (!parsed && state.table?.name === name) {
    state.row = null;
    renderList();
    renderEditor();
    return;
  }
  await openTable(name, parsed);
}

async function main() {
  $('#filter').addEventListener('input', (e) => {
    state.filter = (e.target as HTMLInputElement).value;
    renderList();
  });
  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      void save();
    }
  });
  window.addEventListener('beforeunload', (e) => {
    if (dirty()) e.preventDefault();
  });
  window.addEventListener('hashchange', () => void route().catch(fail));
  try {
    const realm = await api.realm();
    $('#realm').textContent = realm.dir;
    state.tables = await api.tables();
    renderNav();
    await route();
  } catch (e) {
    fail(e);
  }
}

void main();
