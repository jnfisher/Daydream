// The Library's list of a table's records: each record's number, name and the values its
// card lists, how a search finds one, and the key a new record is offered.
//
// Pure: no DOM, so `node --test` runs its tests (list.test.ts).

export type Cell = number | string | null;

export interface ListRow {
  key: Cell[];
  /** The key as people write it: `868`, `1/224`. */
  label: string;
  name: string;
  /** The values of the columns the list asked for, by column. */
  vals: Record<string, Cell>;
  get: (col: string) => Cell;
}

export const keyLabel = (key: Cell[]) => key.map((k) => String(k ?? '')).join('/');

/** A row of the list: `cols[i]` is `values[i]`. */
export function listRow(key: Cell[], name: string, cols: readonly string[], values: readonly Cell[]): ListRow {
  const vals: Record<string, Cell> = {};
  cols.forEach((c, i) => (vals[c] = values[i] ?? null));
  return { key, label: keyLabel(key), name, vals, get: (col) => vals[col] ?? null };
}

/**
 * Whether a search finds a record: its number starting with what was typed (a room's number
 * on its map too: `224` finds `1/224`), or its name holding it, in any case.
 */
export function found(r: ListRow, search: string): boolean {
  const q = search.trim().toLowerCase();
  if (!q) return true;
  return r.label.startsWith(q) || String(r.key.at(-1) ?? '').startsWith(q) || r.name.toLowerCase().includes(q);
}

/**
 * The key offered to a new record: the number after the highest. A room's is the next on
 * the map of `near` (the record open, or the first); a text block's is a new block's first
 * part. A key of text has nothing to offer.
 */
export function nextKey(table: string, numeric: readonly boolean[], rows: readonly { key: Cell[] }[], near?: Cell[] | null): Cell[] {
  const n = numeric.length;
  if (!numeric.every(Boolean)) return numeric.map((num) => (num ? 0 : ''));
  const max = (at: number, match: (k: Cell[]) => boolean) => rows.reduce((m, r) => (match(r.key) ? Math.max(m, Number(r.key[at]) || 0) : m), 0);
  if (n === 1) return [max(0, () => true) + 1];
  if (table === 'Textblocks') return [max(0, () => true) + 1, ...Array<number>(n - 1).fill(0)];
  const head = (near ?? rows[0]?.key ?? []).slice(0, n - 1);
  while (head.length < n - 1) head.push(1);
  return [...head, max(n - 1, (k) => head.every((h, i) => k[i] === h)) + 1];
}

/** What is wrong with a key typed for a new record, or '' when nothing is. */
export function keyProblem(parts: readonly string[], numeric: readonly boolean[], taken: (label: string) => boolean, what: string): string {
  for (const [i, p] of parts.entries()) {
    if (!p.trim()) return 'Fill in every part of the key.';
    if (numeric[i] && !/^\d+$/.test(p.trim())) return `${p.trim()} is not a whole number.`;
  }
  const label = keyLabel(parts.map((p, i) => (numeric[i] ? Number(p) : p.trim())));
  if (taken(label)) return `${what} ${label} is there already.`;
  return '';
}
