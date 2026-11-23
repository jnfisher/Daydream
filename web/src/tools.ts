// NMR's database tools: the Database Importer, the Database Deleter and Merge Users, laid
// out as NMR's forms lay them out, over the server's /api/tools routes.

import type { Report } from './api';

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

/** The tools page; `changed` runs after a write, so the editors reload what they show. */
export function toolsPage(changed: () => void): HTMLElement {
  return h('div', { class: 'tools' }, importer(changed), deleter(changed), mergeUsers());
}
