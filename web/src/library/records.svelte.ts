// Single records a card reads beside its own (the messages an attack names, its weapon, a
// text block): each read the first time it is asked for, then again after a write when it is
// asked for again. Asked for inside a component's markup or a derived value, the answer
// comes in when the read does.

import { api, ApiError, type Cell } from '../api';
import { keyLabel } from '../shell/routes';
import type { Lines } from './combat';

type Values = Record<string, Cell>;

const loaded = $state<Record<string, Values | null>>({});
const asking = new Set<string>();
let stale = new Set<string>();

const forget = () => (stale = new Set(Object.keys(loaded)));
window.addEventListener('dd:wrote', forget);
window.addEventListener('dd:realm', forget);

/** A record's values, null when the world has no such record, undefined while it is read. */
export function recordOf(table: string, key: Cell[]): Values | null | undefined {
  const id = `${table}\u0000${keyLabel(key)}`;
  if ((!(id in loaded) || stale.has(id)) && !asking.has(id)) {
    asking.add(id);
    stale.delete(id);
    api
      .row(table, key)
      .then((r) => (loaded[id] = r.values))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 404) loaded[id] = null;
      })
      .finally(() => asking.delete(id));
  }
  return loaded[id];
}

const text = (v: Cell | undefined) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);

/** A message's three lines: null for 0 or a message the world does not have, undefined while
 * it is read. */
export function linesOf(n: number): Lines | null | undefined {
  if (!n) return null;
  const r = recordOf('Messages', [n]);
  return r ? [text(r['Line 1']), text(r['Line 2']), text(r['Line 3'])] : r;
}

/** A text block's text (its first part): null for 0 or one the world does not have. */
export function blockOf(n: number): { text: string; link: number } | null | undefined {
  if (!n) return null;
  const r = recordOf('Textblocks', [n, 0]);
  return r ? { text: text(r['Data']), link: Number(r['Link To']) || 0 } : r;
}
