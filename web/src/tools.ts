// Today's forms for the Workshop's tools not rebuilt yet: the Database Importer, Merge Users,
// the Export to MMUD Explorer (mme.ts), writing back to the WCC files, to and from a silvermere
// database, the Record Number Changer, the Monster Group/Index Changer, the Mass Room Editor and
// NMR's lists, over the server's /api/tools, /api/mme and /api/reports routes. The Workshop
// shows each in its tool's page (workshop/Legacy.svelte); each goes as its tool is rebuilt.

import { toShown } from './ansi';
import { type Report, type TableInfo } from './api';
import { games } from './games';
import { indexChanger } from './indexchanger';
import { controlRooms, findItem, limitedItems, monsterGroups, npcRooms } from './lists';
import { massRoomEditor } from './massroom';
import { mmeExport } from './mme';
import { recordNumberChanger } from './renumber';

/** The tables NMR's importer lists, in its order. */
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
  // the frame keeps its changes list and history current
  window.dispatchEvent(new CustomEvent('dd:wrote'));
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

/** Where the WCC folder was last written, so the card remembers it. */
const WCC_KEY = 'daydream.wcc';
const remembered = (): string => {
  try {
    return localStorage.getItem(WCC_KEY) ?? '';
  } catch {
    return '';
  }
};
const remember = (dir: string) => {
  try {
    localStorage.setItem(WCC_KEY, dir);
  } catch {
    // private window: nothing to remember
  }
};

interface WrittenFile {
  stem: string;
  dest: string;
  action: 'unchanged' | 'updated' | 'rebuilt';
  records: number | null;
  /** Only the realm's edits went in, onto the file as the game has it. */
  merged: boolean;
  /** Edits left out (the game has made them impossible meanwhile), and why. */
  skipped: string[];
}

/** Writes the realm back into its Btrieve WCC*.DAT files (`daydream write-dat`). */
export function writeDat(): HTMLElement {
  const card = h('section', { class: 'tool' }, h('h2', {}, 'Write to WCC files'));
  card.append(
    h(
      'p',
      { class: 'hint' },
      "Write this realm's edits into the WCC*.DAT files it was converted from, for Worldgroup and NMR. Only the edits go in, onto the files as the game has them now: each changed member onto the game's record, so nothing the game did meanwhile (floors, shop stock, players) is undone. Files with no edits are left alone, edits that keep every key are written in place, and files with records added, deleted or renumbered are rebuilt. Stop the game first: Daydream refuses while it sees the game running on the folder.",
    ),
  );
  const wcc = input({ type: 'text', placeholder: 'C:\\WGSERV' });
  // the open game's folder, when it is a WCC game: then the field may stay empty
  let gameDir: string | null = null;
  games
    .list()
    .then((l) => {
      if (l.current?.kind === 'wcc') {
        gameDir = l.current.location ?? null;
        wcc.placeholder = `${gameDir} (the open game's)`;
      } else if (l.current?.kind === 'db' && l.current.copy) {
        card.append(h('p', { class: 'hint' }, "The open game keeps its world in .db files, not WCC files: publish it from the Publish page, which writes them with the game stopped."));
      } else if (l.current?.kind === 'db' && l.current.live) {
        card.append(h('p', { class: 'hint' }, "The open game's own .db files are being edited: every save is already in the game, so there is nothing to write back."));
      } else {
        wcc.value = remembered();
      }
      if (!l.custom) {
        out.disabled = true;
        out.placeholder = 'not with an access token (start Daydream with --allow-any-source)';
      }
    })
    .catch(() => {
      wcc.value = remembered();
    });
  const out = input({ type: 'text', placeholder: 'leave empty to write over the WCC folder' });
  const backup = input({ type: 'checkbox', checked: '' });
  const force = input({ type: 'checkbox' });
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, 'Write');
  const result = h('div');
  card.append(
    h('label', { class: 'path' }, 'WCC folder (on the machine Daydream runs on) ', wcc),
    h('label', { class: 'path' }, 'Write to another folder instead ', out),
    h('label', { class: 'check' }, backup, ' Back up each file before changing it'),
    h('label', { class: 'check' }, force, ' A realm edited before Daydream recorded edits is written whole: write it even over files the game changed since it was converted'),
    h('div', { class: 'toolbar' }, preview, go),
    result,
  );
  const run = async (dryRun: boolean) => {
    const typed = wcc.value.trim();
    const dir = typed || gameDir;
    if (!dir) {
      result.replaceChildren(failure('Give the WCC folder.'));
      return;
    }
    if (!dryRun && !confirm(`Write the realm into the WCC files${out.value.trim() ? ` in ${out.value.trim()}` : ` of ${dir}`}? Stop the game first.`)) return;
    try {
      const res = await post(
        '/api/tools/write-dat',
        JSON.stringify({ wcc: typed || null, out: out.value.trim() || null, backup: backup.checked, force: force.checked, dry_run: dryRun }),
        true,
      );
      if (typed) remember(typed);
      const files = res.files as WrittenFile[];
      const changed = files.filter((f) => f.action !== 'unchanged');
      const would = dryRun ? 'would be ' : '';
      const line = (f: WrittenFile) =>
        `${f.dest}: ${f.action === 'updated' ? `${f.records} records ${would}updated in place` : `${would}rebuilt with ${f.records} records`}${f.merged ? '' : ' (written whole)'}`;
      result.replaceChildren(
        h('p', {}, changed.length ? `${changed.length} of ${files.length} files ${dryRun ? 'would change' : 'written'}.` : `All ${files.length} files hold the realm's edits: nothing to write.`),
      );
      if (changed.length) result.append(h('pre', {}, changed.map(line).join('\n')));
      const skipped = files.flatMap((f) => f.skipped ?? []);
      if (skipped.length) {
        result.append(
          h('p', {}, `${skipped.length} edit${skipped.length === 1 ? '' : 's'} ${dryRun ? 'would be' : 'were'} left out, and kept for the next write:`),
          h('pre', {}, skipped.join('\n')),
        );
      }
      if (res.backup) result.append(h('p', { class: 'hint' }, `The files as they were are in ${res.backup}.`));
    } catch (e) {
      result.replaceChildren(failure(e));
    }
  };
  preview.addEventListener('click', () => void run(true));
  go.addEventListener('click', () => void run(false));
  return card;
}

interface SilvermereReport {
  table: string;
  inserted: number;
  updated: number;
  unchanged: number;
  deleted: number;
}

const SILVERMERE_KEY = 'daydream.silvermere';

/** The realm in a silvermere database (`daydream silvermere-export` and `silvermere-import`). */
export function silvermere(changed: () => void): HTMLElement {
  const card = h('section', { class: 'tool' }, h('h2', {}, 'silvermere'));
  card.append(
    h(
      'p',
      { class: 'hint' },
      "Write this realm's world (rooms, items, monsters, spells, classes, races, shops, messages, text blocks, actions) straight into a silvermere server's database, or read it back. Players are not touched. Stop silvermere first: Daydream takes its writer lock and refuses while it runs.",
    ),
  );
  let saved = '';
  try {
    saved = localStorage.getItem(SILVERMERE_KEY) ?? '';
  } catch {
    // private window
  }
  const db = input({ type: 'text', placeholder: 'C:\\silvermere\\data\\silvermere.db, or postgres://...', value: saved });
  const resetState = input({ type: 'checkbox' });
  const keepExtra = input({ type: 'checkbox' });
  const noRaw = input({ type: 'checkbox' });
  const status = h('button', { type: 'button' }, 'Status');
  const previewOut = h('button', { type: 'button' }, 'Preview export');
  const goOut = h('button', { type: 'button', class: 'primary' }, 'Export to silvermere');
  const previewIn = h('button', { type: 'button' }, 'Preview import');
  const goIn = h('button', { type: 'button' }, 'Import from silvermere');
  const result = h('div');
  card.append(
    h('label', { class: 'path' }, "silvermere's database: its SQLite file (on the machine Daydream runs on) or a URL ", db),
    h('label', { class: 'check' }, resetState, " Export: rewrite the live state silvermere has (room floors, shop stock, monster kills) from the realm"),
    h('label', { class: 'check' }, keepExtra, ' Keep the rows the other side does not have, instead of deleting them'),
    h('label', { class: 'check' }, noRaw, " Export: don't keep the records' bytes (what silvermere does not store is then lost on the way back)"),
    h('div', { class: 'toolbar' }, status, previewOut, goOut, previewIn, goIn),
    result,
  );
  Promise.all([fetch('/api/silvermere').then((r) => r.json()), games.list()])
    .then(([c, l]: [{ database: string | null; custom: boolean }, Awaited<ReturnType<typeof games.list>>]) => {
      if (l.current?.kind === 'silvermere') {
        // the open game is this database: what was typed for another is not meant now
        db.value = '';
        db.placeholder = `${l.current.location} (the open game's)`;
      } else if (c.database) {
        db.placeholder = `${c.database} (the server's)`;
      }
      if (!c.custom) {
        db.value = '';
        if (!c.database) db.placeholder = 'open a silvermere game first (typing a database needs --allow-any-source)';
      }
    })
    .catch(() => {});
  const database = () => {
    const d = db.value.trim();
    try {
      if (d) localStorage.setItem(SILVERMERE_KEY, d);
    } catch {
      // private window
    }
    return d || null;
  };
  const show = (reports: SilvermereReport[], dryRun: boolean, exporting: boolean) => {
    const table = h('table', { class: 'grid' });
    const heads = exporting ? ['Table', 'New', 'Rewritten', 'Deleted'] : ['Table', 'Inserted', 'Updated', 'Unchanged', 'Deleted'];
    table.append(h('thead', {}, h('tr', {}, ...heads.map((c) => h('th', {}, c)))));
    const body = h('tbody');
    for (const r of reports) {
      const cells = exporting ? [r.table, r.inserted, r.updated, r.deleted] : [r.table, r.inserted, r.updated, r.unchanged, r.deleted];
      body.append(h('tr', {}, ...cells.map((v) => h('td', {}, String(v)))));
    }
    table.append(body);
    result.replaceChildren(h('p', {}, dryRun ? 'Preview: nothing was written.' : 'Done.'), table);
  };
  const run = async (exporting: boolean, dryRun: boolean) => {
    const where = db.value.trim() || db.placeholder.replace(/ \(.*\)$/, '') || "the server's silvermere database";
    if (!dryRun) {
      const ask = exporting
        ? `Write the realm's world into ${where}? silvermere must be stopped.`
        : `Rewrite the realm's world from ${where}? Records silvermere does not have are deleted${keepExtra.checked ? ' (kept: you asked to keep them)' : ''}. Stop the realm first.`;
      if (!confirm(ask)) return;
    }
    try {
      const body = exporting
        ? { database: database(), dry_run: dryRun, reset_state: resetState.checked, keep_extra: keepExtra.checked, no_raw: noRaw.checked }
        : { database: database(), dry_run: dryRun, keep_extra: keepExtra.checked };
      const res = await post(exporting ? '/api/silvermere/export' : '/api/silvermere/import', JSON.stringify(body), true);
      show(res.tables as SilvermereReport[], dryRun, exporting);
      if (!exporting && !dryRun) changed();
    } catch (e) {
      result.replaceChildren(failure(e));
    }
  };
  status.addEventListener('click', async () => {
    try {
      const info = await post('/api/silvermere/info', JSON.stringify({ database: database() }), true);
      const lines = (info.tables as [string, number][]).map(([t, n]) => `${t}: ${n} rows`);
      result.replaceChildren(
        h('p', {}, info.running ? `silvermere is running (${info.running}): stop it before exporting or importing.` : 'silvermere is not running.'),
        h('pre', {}, `${info.url}\n${lines.join('\n')}\nrecord bytes from Daydream: ${info.raw ? 'kept' : 'none'}`),
      );
    } catch (e) {
      result.replaceChildren(failure(e));
    }
  });
  previewOut.addEventListener('click', () => void run(true, true));
  goOut.addEventListener('click', () => void run(true, false));
  previewIn.addEventListener('click', () => void run(false, true));
  goIn.addEventListener('click', () => void run(false, false));
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

/** The old forms that do a Workshop tool's work, by its id; `changed` runs after each write. */
export function legacyTool(id: string, tables: TableInfo[], changed: () => void): HTMLElement[] {
  const json = (url: string, body: string) => post(url, body, true);
  switch (id) {
    case 'nmr':
      return [importer(changed)];
    case 'merge':
      return [mergeUsers()];
    case 'wcc':
      return [writeDat()];
    case 'silvermere':
      return [silvermere(changed)];
    case 'mme':
      return [mmeExport(failure)];
    case 'renumber':
      return [recordNumberChanger(json, modifyResult, failure, changed)];
    case 'spawns':
      return [monsterGroups(), indexChanger(json, modifyResult, failure, changed)];
    case 'rooms': {
      const rooms = tables.find((t) => t.name === 'Rooms');
      return rooms ? [massRoomEditor(rooms, json, modifyResult, changed)] : [];
    }
    case 'where':
      return [findItem()];
    case 'limited':
      return [limitedItems()];
    case 'areas':
      return [controlRooms()];
    case 'npcs':
      return [npcRooms()];
    default:
      return [];
  }
}
