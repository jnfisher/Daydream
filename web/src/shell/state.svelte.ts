// What the frame knows about the open world, shared by every part of it: where the page is,
// the world and whether its game runs, the changes waiting to publish, the history of saves,
// and the records open in the inspector. Kept current by polling and by every write.

import { api, type Cell, type ChangeList, type History, type TableInfo } from '../api';
import { games, type Current } from '../games';
import { read, sfx, write } from './prefs.svelte';
import { keyLabel, type Route } from './routes';
import { messageOf, toast } from './toasts.svelte';

export interface RecordRef {
  table: string;
  key: Cell[];
  /** Its name, when known (for crumbs and recent lists). */
  name?: string;
}

export const app = $state({
  route: { place: 'home', rest: [] } as Route,
  /** The open world, or null before one is opened. */
  current: null as Current | null,
  /** Whether its game is running; null when Daydream cannot tell. */
  running: null as boolean | null,
  runningBy: null as string | null,
  tables: [] as TableInfo[],
  changes: null as ChangeList | null,
  history: null as History | null,
  paletteOpen: false,
  /** The records open in the inspector, oldest first; the last is shown. */
  inspecting: [] as RecordRef[],
  /** Records opened lately, newest first. */
  recent: [] as RecordRef[],
});

// ---- the changes list and the history --------------------------------------------------

let pending: ReturnType<typeof setTimeout> | null = null;

/** Reads the changes and history again (soon: a burst of writes asks once). */
export function refresh(): void {
  if (pending) clearTimeout(pending);
  pending = setTimeout(() => {
    pending = null;
    void refreshNow();
  }, 120);
}

export async function refreshNow(): Promise<void> {
  if (!app.current) return;
  try {
    const [changes, history] = await Promise.all([api.changes(), api.history(20)]);
    app.changes = changes;
    app.history = history;
  } catch {
    // the realm may be busy writing; the next refresh asks again
  }
}

export async function refreshStatus(): Promise<void> {
  const id = app.current?.id;
  if (!id) {
    app.running = null;
    return;
  }
  try {
    const s = await games.status(id);
    app.running = s.running;
    app.runningBy = s.by ?? null;
  } catch {
    app.running = null;
  }
}

// ---- the record caches the palette searches --------------------------------------------

const rowCache = new Map<string, Promise<[Cell[], Cell][]>>();

/** Every row's key and title of a table, cached until the next write. */
export function rowsOf(table: string): Promise<[Cell[], Cell][]> {
  let rows = rowCache.get(table);
  if (!rows) {
    rows = api.rows(table).then((r) => r.rows);
    rows.catch(() => rowCache.delete(table));
    rowCache.set(table, rows);
  }
  return rows;
}

// ---- writes ---------------------------------------------------------------------------

/** After any write: the lists are read again. */
window.addEventListener('dd:wrote', () => {
  rowCache.clear();
  refresh();
});

/** The realm changed under whatever the page shows (an undo, a take-back): pages reload. */
export function realmChanged(): void {
  rowCache.clear();
  refresh();
  window.dispatchEvent(new CustomEvent('dd:realm'));
}

/** Asked before anything replaces what the editor holds; false keeps it. Set by the editor. */
let guard: () => boolean = () => true;
export const setGuard = (g: () => boolean) => (guard = g);
export const mayLeave = () => guard();

export async function undo(step?: number): Promise<void> {
  if (!mayLeave()) return;
  try {
    const s = await api.undo(step);
    sfx('undo');
    toast(`Undone: ${s.label}`, { action: 'Redo', onAction: () => void redo() });
    realmChanged();
  } catch (e) {
    sfx('nope');
    toast(messageOf(e), { kind: 'error' });
  }
}

export async function redo(step?: number): Promise<void> {
  if (!mayLeave()) return;
  try {
    const s = await api.redo(step);
    sfx('undo');
    toast(`Redone: ${s.label}`, { action: 'Undo', onAction: () => void undo() });
    realmChanged();
  } catch (e) {
    sfx('nope');
    toast(messageOf(e), { kind: 'error' });
  }
}

/** Takes changes back: each record as it was when the world was last read or published. */
export async function takeBack(changes: { stem: string; id: number }[] | 'all', what: string): Promise<void> {
  if (!mayLeave()) return;
  try {
    const r = await api.revert(changes === 'all' ? { all: true } : { changes });
    if (r.failed.length) toast(`${what}: ${r.failed.map((f) => f.why).join('; ')}`, { kind: 'error' });
    else toast(`Took back ${what}`, { action: 'Undo', onAction: () => void undo() });
    realmChanged();
  } catch (e) {
    toast(messageOf(e), { kind: 'error' });
  }
}

// ---- the inspector and the records opened lately ----------------------------------------

const sameRef = (a: RecordRef, b: RecordRef) => a.table === b.table && keyLabel(a.key) === keyLabel(b.key);

/** Opens a record beside whatever the page shows; another open record stays one step back. */
export function inspect(ref: RecordRef, fresh = false): void {
  const top = app.inspecting.at(-1);
  if (top && sameRef(top, ref)) return;
  if (fresh) app.inspecting = [ref];
  else {
    const at = app.inspecting.findIndex((r) => sameRef(r, ref));
    app.inspecting = at >= 0 ? app.inspecting.slice(0, at + 1) : [...app.inspecting, ref].slice(-8);
  }
}

export const closeInspector = () => (app.inspecting = []);

const recentKey = () => `dd-recent:${app.current?.realm_dir ?? ''}`;

export function loadRecent(): void {
  try {
    const v: unknown = JSON.parse(read(recentKey(), '[]'));
    app.recent = Array.isArray(v) ? (v as RecordRef[]).filter((r) => r && typeof r.table === 'string' && Array.isArray(r.key)) : [];
  } catch {
    app.recent = [];
  }
}

/** Notes a record as opened, for Home and the palette. */
export function remember(ref: RecordRef): void {
  const old = app.recent.find((r) => sameRef(r, ref));
  const entry = { ...ref, name: ref.name ?? old?.name };
  app.recent = [entry, ...app.recent.filter((r) => !sameRef(r, ref))].slice(0, 12);
  write(recentKey(), JSON.stringify(app.recent));
}
