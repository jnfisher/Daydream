// The names of every table's records, for the chips that show what a number points at: read
// once, then again after a write when a chip asks.

import { titleOf } from '../ansi';
import type { Cell } from '../api';
import { keyLabel } from '../shell/routes';
import { rowsOf } from '../shell/state.svelte';

const loaded = $state<Record<string, Map<string, string>>>({});
const asking = new Set<string>();
let stale = new Set<string>();

window.addEventListener('dd:wrote', () => (stale = new Set(Object.keys(loaded))));
window.addEventListener('dd:realm', () => (stale = new Set(Object.keys(loaded))));

/** A table's names by key (`868`, `1/224`), or undefined while they are read. */
export function namesOf(table: string): Map<string, string> | undefined {
  if ((!loaded[table] || stale.has(table)) && !asking.has(table)) {
    asking.add(table);
    stale.delete(table);
    rowsOf(table)
      .then((rows) => {
        loaded[table] = new Map(rows.map(([k, t]) => [keyLabel(k), t === null ? '' : titleOf(String(t))]));
      })
      .catch(() => {})
      .finally(() => asking.delete(table));
  }
  return loaded[table];
}

/** The name of a record, '' when it has none, or null when the table has no such record. */
export function nameOf(table: string, key: Cell[]): string | null | undefined {
  const m = namesOf(table);
  if (!m) return undefined;
  return m.get(keyLabel(key)) ?? null;
}
