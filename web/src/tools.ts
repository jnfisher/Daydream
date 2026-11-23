// NMR's database tools: the Database Importer, the Database Deleter and Merge Users, laid
// out as NMR's forms lay them out, over the server's /api/tools routes.

import { ABILITIES } from './abilities';
import { toShown, toWire } from './ansi';
import { api, type Report, type TableInfo } from './api';
import { massRoomEditor } from './massroom';
import { indexChanger } from './indexchanger';
import { recordNumberChanger } from './renumber';

/** The tables NMR's importer and deleter list, in its order. */
const TABLES = ['Items', 'Monsters', 'Spells', 'Shops', 'Races', 'Classes', 'Rooms', 'Actions', 'Messages', 'Textblocks'];

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

const input = (props: Record<string, string | undefined>) => h('input', props);

async function post(url: string, body: BodyInit, json = false): Promise<any> {
  const res = await fetch(url, {
    method: 'POST',
    body,
    headers: json ? { 'content-type': 'application/json' } : {},
  });
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

function reportTable(reports: Report[], dryRun: boolean): HTMLElement {
  const box = h('div', { class: 'tool-result' });
  box.append(h('p', {}, dryRun ? 'Preview: nothing was written.' : 'Done.'));
  const table = h('table', { class: 'grid' });
  table.append(
    h('thead', {}, h('tr', {}, ...['Table', 'Inserted', 'Updated', 'Unchanged', 'Skipped', 'Deleted', 'Errors'].map((c) => h('th', {}, c)))),
  );
  const body = h('tbody');
  for (const r of reports) {
    body.append(
      h('tr', {}, ...[r.table, r.inserted, r.updated, r.unchanged, r.skipped ?? 0, r.deleted, r.errors.length].map((v) => h('td', {}, String(v)))),
    );
  }
  table.append(body);
  box.append(table);
  const errors = reports.flatMap((r) => r.errors);
  if (errors.length) box.append(h('pre', { class: 'errors' }, errors.join('\n')));
  return box;
}

function failure(e: unknown): HTMLElement {
  return h('p', { class: 'error' }, e instanceof Error ? e.message : String(e));
}

/** A range row's inputs: All, and Map (rooms), From and To. */
interface RangeInputs {
  table: string;
  use: HTMLInputElement;
  all: HTMLInputElement;
  map: HTMLInputElement | null;
  from: HTMLInputElement;
  to: HTMLInputElement;
}

function rangeOf(r: RangeInputs): { table: string; map?: number; from: number; to: number } | null {
  if (r.all.checked) return null;
  const n = (i: HTMLInputElement) => Number(i.value.trim() || 0);
  return { table: r.table, map: r.map ? n(r.map) : undefined, from: n(r.from), to: n(r.to) };
}

function rangeRow(table: string, withUse: boolean): [HTMLTableRowElement, RangeInputs] {
  const use = input({ type: 'checkbox', checked: '' });
  const all = input({ type: 'checkbox', checked: '' });
  const num = () => input({ type: 'text', inputmode: 'numeric', class: 'number', disabled: '' });
  const map = table === 'Rooms' ? num() : null;
  const from = num();
  const to = num();
  if (table === 'Actions') all.disabled = true; // actions are named, not numbered
  all.addEventListener('change', () => {
    for (const i of [map, from, to]) if (i) i.disabled = all.checked;
  });
  const tr = h(
    'tr',
    {},
    h('td', {}, withUse ? h('label', {}, use, ' ', table) : table),
    h('td', {}, h('label', {}, all, ' All')),
    h('td', {}, map ?? ''),
    h('td', {}, from),
    h('td', {}, to),
  );
  return [tr, { table, use, all, map, from, to }];
}

function rangeHead(first: string): HTMLElement {
  return h('thead', {}, h('tr', {}, ...[first, 'Range', 'Map', 'From', 'To'].map((c) => h('th', {}, c))));
}

function importer(changed: () => void): HTMLElement {
  const card = h('section', { class: 'tool' }, h('h2', {}, 'Database Importer'));
  card.append(
    h('p', { class: 'hint' }, 'Import an NMR export (.mdb) into this realm. Stop the realm first. Preview First shows what would change without writing.'),
  );
  const file = input({ type: 'file', accept: '.mdb,.json' });
  card.append(h('label', { class: 'file' }, 'Import file: ', file));

  const rows: RangeInputs[] = [];
  const table = h('table', { class: 'grid' });
  table.append(rangeHead('Databases to Import'));
  const body = h('tbody');
  for (const t of TABLES) {
    const [tr, r] = rangeRow(t, true);
    rows.push(r);
    body.append(tr);
  }
  table.append(body);
  const allBtn = h('button', { type: 'button' }, 'All');
  const noneBtn = h('button', { type: 'button' }, 'None');
  allBtn.addEventListener('click', () => rows.forEach((r) => (r.use.checked = true)));
  noneBtn.addEventListener('click', () => rows.forEach((r) => (r.use.checked = false)));
  card.append(table, h('div', { class: 'toolbar' }, allBtn, noneBtn));

  const radio = (name: string, value: string, label: string, checked = false) =>
    h('label', {}, input({ type: 'radio', name, value, checked: checked ? '' : undefined }), ' ', label);
  card.append(
    h('fieldset', {}, h('legend', {}, 'Existing Records'), radio('existing', 'update', 'Update', true), radio('existing', 'skip', 'Skip')),
    h(
      'fieldset',
      {},
      h('legend', {}, 'Rooms'),
      radio('rooms', 'all', 'Everything', true),
      radio('rooms', 'only_items', 'Only Items'),
      radio('rooms', 'not_items', 'Not Items'),
    ),
  );
  const extra = input({ type: 'checkbox', checked: '' });
  card.append(h('label', { class: 'check' }, extra, ' Delete text block parts past the last part the file has (NMR always does)'));

  const preview = h('button', { type: 'button' }, 'Preview First');
  const go = h('button', { type: 'button', class: 'primary' }, 'Import');
  const out = h('div');
  card.append(h('div', { class: 'toolbar' }, preview, go), out);

  const run = async (dryRun: boolean) => {
    const f = file.files?.[0];
    if (!f) {
      out.replaceChildren(failure('Pick the file to import.'));
      return;
    }
    const picked = (name: string) => (card.querySelector(`input[name=${name}]:checked`) as HTMLInputElement).value;
    const options = {
      tables: rows.filter((r) => r.use.checked).map((r) => r.table),
      ranges: rows.filter((r) => r.use.checked).map(rangeOf).filter((r) => r !== null),
      skip_existing: picked('existing') === 'skip',
      room_parts: picked('rooms'),
      extra_parts: extra.checked,
      dry_run: dryRun,
    };
    if (!dryRun && !confirm(`Import ${f.name} into this realm?`)) return;
    out.replaceChildren(h('p', {}, `${dryRun ? 'Previewing' : 'Importing'} ${f.name}…`));
    try {
      const res = await post(`/api/tools/import?options=${encodeURIComponent(JSON.stringify(options))}`, f);
      out.replaceChildren(reportTable(res.reports, res.dry_run));
      if (!dryRun) changed();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));
  return card;
}

function deleter(changed: () => void): HTMLElement {
  const card = h('section', { class: 'tool' }, h('h2', {}, 'Database Deleter'));
  card.append(h('p', { class: 'hint' }, 'Delete every record of a table, or a range of them. This cannot be undone; Preview shows how many would go.'));
  const select = h('select', {}, ...TABLES.map((t) => h('option', { value: t }, t)));
  const holder = h('tbody');
  let row = rangeRow(TABLES[0]!, false);
  holder.append(row[0]);
  select.addEventListener('change', () => {
    row = rangeRow(select.value, false);
    holder.replaceChildren(row[0]);
  });
  const table = h('table', { class: 'grid' }, rangeHead('Table'), holder);
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'danger' }, 'Delete Selected');
  const out = h('div');
  card.append(h('label', {}, 'Delete from: ', select), table, h('div', { class: 'toolbar' }, preview, go), out);
  const run = async (dryRun: boolean) => {
    const r = row[1];
    const range = rangeOf(r);
    const what = range ? `${r.table} ${range.map !== undefined ? `map ${range.map}, ` : ''}${range.from} to ${range.to}` : `every ${r.table} record`;
    if (!dryRun && !confirm(`Delete ${what} from the realm? This cannot be undone.`)) return;
    try {
      const res = await post('/api/tools/delete', JSON.stringify({ table: r.table, range, all: range === null, dry_run: dryRun }), true);
      out.replaceChildren(h('p', {}, `${res.report.deleted} ${r.table} record${res.report.deleted === 1 ? '' : 's'} ${dryRun ? 'would be ' : ''}deleted.`));
      if (!dryRun) changed();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));
  return card;
}

function mergeUsers(): HTMLElement {
  const card = h('section', { class: 'tool' }, h('h2', {}, 'Merge Users'));
  card.append(h('p', { class: 'hint' }, 'Copy the users of another user file (WCCUSER2.DAT, or a Daydream .db) into this realm, byte for byte. Users the realm already has are skipped.'));
  const file = input({ type: 'file', accept: '.dat,.db' });
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, 'Merge');
  const out = h('div');
  card.append(h('label', { class: 'file' }, 'Users to merge in: ', file), h('div', { class: 'toolbar' }, preview, go), out);
  const run = async (dryRun: boolean) => {
    const f = file.files?.[0];
    if (!f) {
      out.replaceChildren(failure('Pick the user file.'));
      return;
    }
    if (!dryRun && !confirm(`Merge the users of ${f.name} into this realm?`)) return;
    try {
      const res = await post(`/api/tools/merge-users?dry_run=${dryRun}`, f);
      const lines = [`${res.copied} users ${dryRun ? 'would be ' : ''}copied, ${res.skipped.length} skipped (already in the realm).`];
      out.replaceChildren(h('p', {}, lines[0]!));
      if (res.skipped.length) out.append(h('pre', { class: 'errors' }, res.skipped.join('\n')));
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));
  return card;
}

const OPS: [string, string][] = [['add', '+'], ['sub', '-'], ['mul', '*'], ['div', '/'], ['percent', '%'], ['set', '=']];
const CMPS: [string, string][] = [['eq', '='], ['le', '<='], ['ge', '>='], ['ne', '<>']];

const options = (pairs: [string, string][]) => pairs.map(([v, label]) => h('option', { value: v }, label));
const numberInput = (value = '') => input({ type: 'text', inputmode: 'decimal', class: 'number', value });

/** The numeric columns a modifier may change or test, with each numbered family (`Item Drop % 0`,
 *  `Item Drop % 1`, ...) offered whole as well, as `Item Drop % *`. */
function numericColumns(t: TableInfo): [string, string][] {
  const keys = new Set(t.key);
  const names = t.columns.filter((c) => (c.type === 'long' || c.type === 'double') && !keys.has(c.name)).map((c) => c.name);
  const families = new Map<string, number>();
  for (const n of names) {
    const m = /^(.*\D) ?\d+$/.exec(n);
    if (m) families.set(m[1]!, (families.get(m[1]!) ?? 0) + 1);
  }
  const out: [string, string][] = [];
  for (const [f, count] of families) if (count > 1) out.push([`${f}*`, `${f.trimEnd()} (every one)`]);
  return [...names.map((n): [string, string] => [n, n]), ...out];
}

/** The columns a field choice stands for. */
function columnsOf(t: TableInfo, choice: string): string[] {
  if (!choice.endsWith('*')) return [choice];
  const stem = choice.slice(0, -1);
  return t.columns.map((c) => c.name).filter((n) => n.startsWith(stem) && /^\s?\d+$/.test(n.slice(stem.length)));
}

const abilityOptions = () =>
  options(
    Object.entries(ABILITIES)
      .filter(([n]) => n !== '0')
      .map(([n, [name]]): [string, string] => [n, `${n}: ${name}`]),
  );

function universalModifier(tables: TableInfo[], changed: () => void): HTMLElement {
  const card = h('section', { class: 'tool' }, h('h2', {}, 'Universal Modifier'));
  card.append(
    h(
      'p',
      { class: 'hint' },
      'Change a field (or an ability) in many records at once. Preview lists every value that would change; Start writes them.',
    ),
  );
  const editable = tables.filter((t) => TABLES.includes(t.name) && t.name !== 'Actions');
  const editor = h('select', {}, ...editable.map((t) => h('option', { value: t.name }, t.name)));
  const top = h('div', { class: 'toolbar' }, h('label', {}, 'Editor: ', editor));
  const rangeBody = h('tbody');
  const actionBox = h('div');
  const onlyIfBox = h('div');
  const out = h('div');
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, 'Start');
  card.append(
    top,
    h('table', { class: 'grid' }, rangeHead('Table'), rangeBody),
    h('fieldset', {}, h('legend', {}, 'Modifier'), actionBox),
    h('fieldset', {}, h('legend', {}, 'Only Perform Action If:'), onlyIfBox),
    h('div', { class: 'toolbar' }, preview, go),
    out,
  );

  let build: () => object = () => ({});

  const setUp = () => {
    const t = editable.find((x) => x.name === editor.value)!;
    const hasAbilities = t.columns.some((c) => c.name === 'Ability 0');
    const [tr, range] = rangeRow(t.name, false);
    rangeBody.replaceChildren(tr);

    // the action: a field or an ability, the arithmetic, the value and the limit
    const kinds: [string, string][] = [['math', 'Field']];
    if (hasAbilities) kinds.push(['change_ability', 'Ability value'], ['give_ability', 'Give ability'], ['take_ability', 'Take ability']);
    const kind = h('select', {}, ...options(kinds));
    const field = h('select', {}, ...options(numericColumns(t)));
    const ability = h('select', {}, ...abilityOptions());
    const op = h('select', {}, ...options(OPS));
    const value = numberInput('0');
    const limitCmp = h('select', {}, h('option', { value: '' }, '(no limit)'), ...options(CMPS.filter(([c]) => c === 'le' || c === 'ge')));
    const limit = numberInput();
    const clear = input({ type: 'checkbox' });
    const fieldLabel = h('label', {}, 'Field ', field);
    const abilityLabel = h('label', {}, 'Ability ', ability);
    const opLabel = h('label', {}, 'Modifier ', op);
    const valueLabel = h('label', {}, 'Value ', value);
    const limitLabel = h('label', {}, 'Limit to ', limitCmp, limit);
    const clearLabel = h('label', {}, clear, ' Clear its value too');
    const show = () => {
      const k = kind.value;
      fieldLabel.hidden = k !== 'math';
      abilityLabel.hidden = k === 'math';
      opLabel.hidden = limitLabel.hidden = k === 'give_ability' || k === 'take_ability';
      valueLabel.hidden = k === 'take_ability';
      clearLabel.hidden = k !== 'take_ability';
    };
    kind.addEventListener('change', show);
    actionBox.replaceChildren(h('label', {}, kind), fieldLabel, abilityLabel, opLabel, valueLabel, limitLabel, clearLabel);
    show();

    // three "only if" boxes, as NMR has
    const conditionKinds: [string, string][] = [['', '(none)'], ['column', 'Field']];
    if (hasAbilities) conditionKinds.push(['has_ability', 'Has ability'], ['lacks_ability', 'Lacks ability']);
    const plainColumns = numericColumns(t).filter(([v]) => !v.endsWith('*'));
    const conditions = [0, 1, 2].map(() => {
      const k = h('select', {}, ...options(conditionKinds));
      const col = h('select', {}, ...options(plainColumns));
      const ab = h('select', {}, ...abilityOptions());
      const cmp = h('select', {}, ...options(CMPS));
      const v = numberInput('0');
      const mustBe = h('span', {}, ' must be ');
      const sync = () => {
        col.hidden = k.value !== 'column';
        ab.hidden = !k.value.endsWith('ability');
        cmp.hidden = v.hidden = mustBe.hidden = k.value === '';
      };
      k.addEventListener('change', sync);
      sync();
      return { row: h('div', { class: 'toolbar' }, k, col, ab, mustBe, cmp, v), k, col, ab, cmp, v };
    });
    onlyIfBox.replaceChildren(...conditions.map((c) => c.row));

    build = () => {
      const n = (i: HTMLInputElement) => Number(i.value.trim() || 0);
      const lim = limitCmp.value ? [limitCmp.value, n(limit)] : null;
      const k = kind.value;
      const action =
        k === 'math'
          ? { kind: k, columns: columnsOf(t, field.value), op: op.value, value: n(value), limit: lim }
          : k === 'change_ability'
            ? { kind: k, ability: Number(ability.value), op: op.value, value: n(value), limit: lim }
            : k === 'give_ability'
              ? { kind: k, ability: Number(ability.value), value: n(value) }
              : { kind: k, ability: Number(ability.value), clear_value: clear.checked };
      const only_if = conditions
        .filter((c) => c.k.value)
        .map((c) =>
          c.k.value === 'column'
            ? { kind: 'column', column: c.col.value, cmp: c.cmp.value, value: n(c.v) }
            : { kind: c.k.value, ability: Number(c.ab.value), cmp: c.cmp.value, value: n(c.v) },
        );
      return { table: t.name, action, range: rangeOf(range), only_if };
    };
  };
  editor.addEventListener('change', () => {
    setUp();
    out.replaceChildren();
  });
  setUp();

  const run = async (dryRun: boolean) => {
    const body = build();
    if (!dryRun && !confirm(`Change ${editor.value} records as set? Preview first to see every change.`)) return;
    out.replaceChildren(h('p', {}, dryRun ? 'Previewing…' : 'Changing…'));
    try {
      const res = await post('/api/tools/modify', JSON.stringify({ ...body, dry_run: dryRun }), true);
      out.replaceChildren(modifyResult(res));
      if (!dryRun) changed();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));
  return card;
}

/** A value as a record reads it: text up to its NUL, with ESC shown. */
const shownCell = (v: unknown) => (typeof v === 'string' ? toShown(v.split('\0')[0]!) : String(v));

function modifyResult(res: any): HTMLElement {
  const box = h('div', { class: 'tool-result' });
  const would = res.dry_run ? 'would change' : 'changed';
  box.append(
    h(
      'p',
      {},
      `${res.records} record${res.records === 1 ? '' : 's'} ${would} (${res.changed} value${res.changed === 1 ? '' : 's'}); ` +
        `${res.unchanged} already as set; ${res.skipped} left out by its conditions.` +
        (res.dry_run ? ' Preview: nothing was written.' : ''),
    ),
  );
  if (res.notes?.length) box.append(h('p', {}, 'Worth a look (not changed):'), h('pre', { class: 'errors' }, res.notes.join('\n')));
  if (res.report?.errors?.length) box.append(h('pre', { class: 'errors' }, res.report.errors.join('\n')));
  if (res.changes.length) {
    const body = h('tbody');
    for (const c of res.changes) {
      body.append(h('tr', {}, ...[c.key.join(' / '), c.column, shownCell(c.before), shownCell(c.after)].map((v) => h('td', {}, v))));
    }
    box.append(h('table', { class: 'grid changes' }, h('thead', {}, h('tr', {}, ...['Record', 'Field', 'Before', 'After'].map((c) => h('th', {}, c)))), body));
    if (res.changes.length < res.changed) box.append(h('p', { class: 'hint' }, `The first ${res.changes.length} changes are listed.`));
  }
  return box;
}

/** One fix: its title, what it does, its inputs, and the request it sends. */
interface Fix {
  title: string;
  hint: string;
  inputs: (HTMLElement | string)[];
  /** The route, and its body without `dry_run`; a thrown Error is shown as the answer. */
  request: () => [string, object];
}

const pick = (values: [string, string][], selected?: string) => {
  const s = h('select', {}, ...options(values));
  if (selected) s.value = selected;
  return s;
};
const check = (label: string, on = true) => {
  const i = input({ type: 'checkbox', checked: on ? '' : undefined });
  return [i, h('label', {}, i, ' ', label)] as const;
};
const num = (i: HTMLInputElement) => Number(i.value.trim() || 0);

function fixes(): Fix[] {
  const mulBy = numberInput('2');
  const mulCap = numberInput('0');
  const divBy = numberInput('2');
  const bossBy = numberInput('2');
  const date = input({ type: 'date', value: '1990-01-01' });
  const strip = input({ type: 'text', class: 'strip' });
  const padMap = numberInput('1');
  const unpadMap = numberInput('1');
  const mapOrAll = (i: HTMLInputElement) => (i.value.trim() === '' ? null : num(i));

  // Modify Item Drop Percentages
  const dropMode = pick([['set', 'Set to'], ['add', 'Add (+/-)']]);
  const dropValue = numberInput('20');
  const dropCmp = pick(CMPS, 'le');
  const dropLimit = numberInput('20');
  const dropLimited = pick([['include', 'Include limited items'], ['exclude', 'Exclude limited items'], ['only', 'Only limited items']]);
  const [delMaint, delMaintLabel] = check('Exclude weapons and armour deleted at maintenance (ability 119)', true);
  const [regenOn, regenLabel] = check('AND only if monster regen >', true);
  const regen = numberInput('1');
  const types = ['Weapons', 'Armour', 'Keys', 'Containers', 'Scrolls'].map((t) => check(t));

  return [
    {
      title: 'No Limited Items',
      hint: 'Every item\'s Game Limit to 0, except the quest portals (839, 1078, 1326, 1531, 1643, 1686, 1749) and gang items.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'no_limited_items' }],
    },
    {
      title: 'No Level Restrictions',
      hint: 'Takes the level restriction abilities (135 and 136) off every item, values too.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'no_level_restrictions' }],
    },
    {
      title: 'Fix Number of Uses (items)',
      hint: 'Items with 0 uses get -1. Items kept after their uses get a recharge value equal to their uses; those without a recharge ability are listed.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'fix_item_uses' }],
    },
    {
      title: 'Fix Number of Uses on Item Drops',
      hint: 'Each monster drop\'s uses set to the item\'s own uses. Run the item fix first.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'fix_drop_uses' }],
    },
    {
      title: 'Multiply All Exp',
      hint: 'Monster experience times 2 to 20, split into Experience × Exp Multiplier as NMR splits it. A cap of 0 is the most 1.11p holds.',
      inputs: [h('label', {}, 'Times ', mulBy), h('label', {}, 'Cap ', mulCap)],
      request: () => ['/api/tools/preset', { preset: 'multiply_exp', by: num(mulBy), cap: num(mulCap) || null }],
    },
    {
      title: 'Divide All Exp',
      hint: 'Monster experience divided by 2 to 20 (at least 1).',
      inputs: [h('label', {}, 'Divide by ', divBy)],
      request: () => ['/api/tools/preset', { preset: 'divide_exp', by: num(divBy) }],
    },
    {
      title: 'Multiply Boss Exp',
      hint: 'Exp Multiplier times this, for monsters with Game Limit = 1 (which are probably bosses). The Universal Modifier does the same with other settings.',
      inputs: [h('label', {}, 'Times ', bossBy)],
      request: () => [
        '/api/tools/modify',
        {
          table: 'Monsters',
          action: { kind: 'math', columns: ['Exp Multiplier'], op: 'mul', value: num(bossBy), limit: null },
          only_if: [{ kind: 'column', column: 'Game Limit', cmp: 'eq', value: 1 }],
        },
      ],
    },
    {
      title: 'Combine Exp and Exp Multi Fields',
      hint: 'Experience × Exp Multiplier into Experience, multiplier 1 (fixes the experience of monsters killed by damage over time). A multiplier of 0 or less counts as 1.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'combine_exp' }],
    },
    {
      title: 'Set Last Killed Times to Date',
      hint: 'Every monster last killed on this date, at 01:01:02.',
      inputs: [h('label', {}, 'Date ', date)],
      request: () => {
        const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date.value);
        if (!m) throw new Error('Pick a date.');
        return ['/api/tools/preset', { preset: 'set_kill_times', year: Number(m[1]), month: Number(m[2]), day: Number(m[3]) }];
      },
    },
    {
      title: 'Reset Last Killed Times to Zero',
      hint: 'Every monster\'s last killed date and time to 0.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'reset_kill_times' }],
    },
    {
      title: 'Restock All Shops',
      hint: 'Every shop item\'s stock to its max, where its regen number, regen % and regen time are all set. Gang shops are left alone.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'restock_shops' }],
    },
    {
      title: 'Combine Like Items on Floor',
      hint: 'In every room, items with the same number and uses (on the floor, or hidden) are put in one slot.',
      inputs: [],
      request: () => ['/api/tools/preset', { preset: 'combine_room_items' }],
    },
    {
      title: 'Insert Buffer Rooms',
      hint: 'Adds a "Buffer Room" for every room number missing below a map\'s last room, so rooms keep loading (and sweeping) past gaps of 50. Leave the map empty for maps 1 to 18.',
      inputs: [h('label', {}, 'Map ', padMap)],
      request: () => ['/api/tools/preset', { preset: 'insert_buffer_rooms', map: mapOrAll(padMap) }],
    },
    {
      title: 'Delete Buffer Rooms',
      hint: 'Deletes every room whose name begins with "Buffer Room". Leave the map empty for every map.',
      inputs: [h('label', {}, 'Map ', unpadMap)],
      request: () => ['/api/tools/preset', { preset: 'delete_buffer_rooms', map: mapOrAll(unpadMap) }],
    },
    {
      title: 'Strip Characters off Text Blocks',
      hint: 'Takes this text off the end of every text block part that ends with it. Use it for strange characters at the end of every text block. ESC is ␛.',
      inputs: [h('label', {}, 'Text ', strip)],
      request: () => {
        if (!strip.value) throw new Error('Give the text to take off the end.');
        return ['/api/tools/preset', { preset: 'strip_textblocks', text: toWire(strip.value) }];
      },
    },
    {
      title: 'Modify Item Drop Percentages',
      hint: 'Change the drop % of monsters\' item drops, by the kind of item. A drop already at the value is left alone, as NMR leaves it.',
      inputs: [
        h('div', { class: 'toolbar' }, dropMode, dropValue, h('span', {}, ' only if the drop % is '), dropCmp, dropLimit),
        h('div', { class: 'toolbar' }, ...types.map(([, l]) => l)),
        h('div', { class: 'toolbar' }, dropLimited, delMaintLabel),
        h('div', { class: 'toolbar' }, regenLabel, regen),
      ],
      request: () => {
        const [w, a, k, c, sc] = types.map(([i]) => i.checked);
        return [
          '/api/tools/preset',
          {
            preset: 'item_drop_pct',
            set: dropMode.value === 'set',
            value: num(dropValue),
            only_if: [dropCmp.value, num(dropLimit)],
            limited: dropLimited.value,
            exclude_del_at_maint: delMaint.checked,
            min_regen: regenOn.checked ? num(regen) : null,
            weapons: w,
            armour: a,
            keys: k,
            containers: c,
            scrolls: sc,
          },
        ];
      },
    },
  ];
}

function fixesCard(changed: () => void): HTMLElement {
  const card = h('section', { class: 'tool' }, h('h2', {}, 'Fixes'));
  card.append(h('p', { class: 'hint' }, "NMR's one-click fixes from its Items and Monsters menus. Each previews first; Run writes."));
  const choose = h('select', {});
  const list = fixes();
  list.forEach((f, i) => choose.append(h('option', { value: String(i) }, f.title)));
  const body = h('div');
  const out = h('div');
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, 'Run');
  const show = () => {
    const f = list[Number(choose.value)]!;
    body.replaceChildren(h('p', { class: 'hint' }, f.hint), ...(f.inputs.length ? [h('div', { class: 'fix-inputs' }, ...f.inputs)] : []));
    out.replaceChildren();
  };
  choose.addEventListener('change', show);
  show();
  card.append(h('div', { class: 'toolbar' }, h('label', {}, 'Fix: ', choose)), body, h('div', { class: 'toolbar' }, preview, go), out);
  const run = async (dryRun: boolean) => {
    const f = list[Number(choose.value)]!;
    try {
      const [url, req] = f.request();
      if (!dryRun && !confirm(`${f.title}: write the changes? Preview first to see them.`)) return;
      out.replaceChildren(h('p', {}, dryRun ? 'Previewing…' : 'Changing…'));
      const res = await post(url, JSON.stringify({ ...req, dry_run: dryRun }), true);
      out.replaceChildren(modifyResult(res));
      if (!dryRun) changed();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));
  return card;
}

/** The tools page; `changed` runs after a write, so the editors reload what they show. */
export function toolsPage(changed: () => void): HTMLElement {
  const page = h('div', { class: 'tools' }, importer(changed), deleter(changed), mergeUsers());
  api
    .tables()
    .then((tables) => {
      page.append(universalModifier(tables, changed));
      const rooms = tables.find((t) => t.name === 'Rooms');
      if (rooms) page.append(massRoomEditor(rooms, (url, body) => post(url, body, true), modifyResult, changed));
      page.append(fixesCard(changed));
      page.append(indexChanger((url, body) => post(url, body, true), modifyResult, failure, changed));
      page.append(recordNumberChanger((url, body) => post(url, body, true), modifyResult, failure, changed));
    })
    .catch((e) => page.append(failure(e)));
  return page;
}
