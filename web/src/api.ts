// The server's editor API (crates/daydream-server/src/tables.rs).

export type Cell = number | string | null;

export interface ColumnInfo {
  name: string;
  type: 'long' | 'double' | 'text' | 'memo';
  size: number | null;
}

export interface TableInfo {
  name: string;
  /** The index columns, in key order. */
  key: string[];
  /** The column a row list shows beside the key. */
  title: string | null;
  columns: ColumnInfo[];
}

export interface RowList {
  table: string;
  key: string[];
  /** Each row's key and title. */
  rows: [Cell[], Cell][];
}

export interface Row {
  table: string;
  key: Cell[];
  values: Record<string, Cell>;
}

export interface Report {
  table: string;
  inserted: number;
  updated: number;
  unchanged: number;
  skipped: number;
  deleted: number;
  errors: string[];
}

export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}

async function call<T>(method: string, url: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method,
    headers: body === undefined ? {} : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (res.status === 401) {
    // the server has an access token, and this browser's sign-in is gone
    location.href = '/login';
  }
  const text = await res.text();
  let json: unknown = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    // not JSON: an error page
  }
  if (!res.ok) {
    const msg = (json as { error?: string } | null)?.error ?? (text || res.statusText);
    throw new ApiError(res.status, msg);
  }
  // the frame keeps its changes list and history current
  if (method !== 'GET') window.dispatchEvent(new CustomEvent('dd:wrote'));
  return json as T;
}

/** One record changed since the realm was last read from its game or published (`/api/changes`). */
export interface Change {
  stem: string;
  /** Its record id in the `.db` file. */
  id: number;
  /** The record type (`items`), or null for a file Daydream does not decode. */
  table: string | null;
  how: 'added' | 'changed' | 'deleted';
  /** The record for a person: `item 12`, `room 1/5`. */
  what: string;
  name: string | null;
  /** The members a changed record changed. */
  members: string[];
  /** The same, as the editors label their columns (`Max Hit`); null where no editor shows it. */
  labels: string[] | null;
  /** What a renumbered record was. */
  was: string | null;
  /** The editors' table and key for it, or null. */
  nmr: string | null;
  key: Cell[] | null;
}

export interface ChangeList {
  /** When the realm was read or last published (seconds since 1970); null when not known. */
  since: number | null;
  changes: Change[];
}

/** One save in the realm's history (`/api/history`). */
export interface Step {
  step: number;
  /** Seconds since 1970. */
  at: number;
  label: string;
  records: number;
  state: 'done' | 'undone' | 'too_big';
}

export interface History {
  steps: Step[];
  /** The steps the next undo and redo take. */
  undo: Step | null;
  redo: Step | null;
}

/** One step of publishing, as the plan lists it and a job runs it. */
export interface PublishStep {
  id: string;
  title: string;
  detail: string;
  /** `yours` is for the sysop to do (start the game again). */
  state: 'waiting' | 'running' | 'done' | 'failed' | 'skipped' | 'yours';
  log: string[];
}

export type PublishWayId = 'update' | 'now';

/** A way of publishing into the open world's game (`/api/publish`). */
export interface PublishWay {
  id: PublishWayId;
  title: string;
  summary: string;
  button: string;
  ready: boolean;
  /** Why it cannot be taken, when it cannot. */
  why: string | null;
  /** The changes the update file cannot carry (`item 12: ...`). */
  not_carried: string[];
  steps: PublishStep[];
}

export interface PublishCheck {
  level: 'ok' | 'note' | 'warn' | 'block';
  text: string;
}

/** A publish under way, or the last one. */
export interface PublishJob {
  id: number;
  way: PublishWayId;
  engine: string;
  game: string;
  state: 'running' | 'done' | 'failed' | 'cancelled';
  started: number;
  finished: number | null;
  steps: PublishStep[];
  /** Edits that could not be written: they stay among the changes. */
  skipped: string[];
  error: string | null;
  result: string | null;
}

export interface PublishPlan {
  engine: 'worldgroup' | 'majorbbs' | 'silvermere' | 'files';
  /** The game, as a sysop calls it ("Worldgroup"). */
  name: string;
  game: string | null;
  location: string | null;
  running: { running: boolean | null; by?: string | null; error?: string; elsewhere: string[] };
  changes: number;
  since: number | null;
  ways: PublishWay[];
  /** The way to offer first. */
  way: PublishWayId | null;
  checks: PublishCheck[];
  job: PublishJob | null;
}

const t = (table: string) => `/api/tables/${encodeURIComponent(table)}`;
const k = (key: Cell[]) => `?key=${encodeURIComponent(JSON.stringify(key))}`;

export const api = {
  realm: () => call<{ dir: string }>('GET', '/api/realm'),
  tables: () => call<{ tables: TableInfo[] }>('GET', '/api/tables').then((r) => r.tables),
  rows: (table: string) => call<RowList>('GET', `${t(table)}/rows`),
  /** Each row's key, title and the values of `columns`. */
  rowsWith: (table: string, columns: string[]) =>
    call<{ rows: [Cell[], Cell, Cell[]][] }>('GET', `${t(table)}/rows?columns=${encodeURIComponent(columns.join(','))}`),
  row: (table: string, key: Cell[]) => call<Row>('GET', `${t(table)}/row${k(key)}`),
  save: (table: string, key: Cell[], values: Record<string, Cell>) =>
    call<Report>('PUT', `${t(table)}/row${k(key)}`, { values }),
  insert: (table: string, values: Record<string, Cell>) =>
    call<Report>('POST', `${t(table)}/row`, { values }),
  remove: (table: string, key: Cell[]) => call<Report>('DELETE', `${t(table)}/row${k(key)}`),
  changes: () => call<ChangeList>('GET', '/api/changes'),
  publishPlan: () => call<PublishPlan>('GET', '/api/publish'),
  publish: (way: PublishWayId) => call<PublishJob>('POST', '/api/publish', { way }),
  publishJob: () => call<PublishJob | null>('GET', '/api/publish/job'),
  cancelPublish: () => call<{ cancelled: boolean }>('POST', '/api/publish/cancel', {}),
  /** Puts records back as they were when the realm was last read or published. */
  revert: (body: { changes: { stem: string; id: number }[] } | { all: true }) =>
    call<{ reverted: number; failed: { stem: string; id: number; why: string }[] }>('POST', '/api/changes/revert', body),
  history: (limit = 20) => call<History>('GET', `/api/history?limit=${limit}`),
  undo: (step?: number) => call<Step>('POST', '/api/undo', step === undefined ? {} : { step }),
  redo: (step?: number) => call<Step>('POST', '/api/redo', step === undefined ? {} : { step }),
  /** One of NMR's fixes (`daydream_nmr::presets::Preset`), written straight away. */
  preset: (body: Record<string, unknown>) => call<{ changes?: unknown[] }>('POST', '/api/tools/preset', { ...body, dry_run: false }),
};
