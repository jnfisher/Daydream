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
  return json as T;
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
  /** One of NMR's fixes (`daydream_nmr::presets::Preset`), written straight away. */
  preset: (body: Record<string, unknown>) => call<{ changes?: unknown[] }>('POST', '/api/tools/preset', { ...body, dry_run: false }),
};
