// Single records a card reads beside its own (the messages an attack names, its weapon, a
// text block): each read the first time it is asked for, then again after a write when it is
// asked for again. Asked for inside a component's markup or a derived value, the answer
// comes in when the read does.

import { api, ApiError, type Cell } from '../api';
import { keyLabel } from '../shell/routes';
import type { Lines } from './combat';

type Values = Record<string, Cell>;

const loaded = $state<Record<string, Values | null>>({});
const reading = new Map<string, Promise<Values | null>>();
let stale = new Set<string>();

const forget = () => (stale = new Set(Object.keys(loaded)));
window.addEventListener('dd:wrote', forget);
window.addEventListener('dd:realm', forget);

/** A record's values when they are read, null when the world has no such record: read once
 * for this and `recordOf` both, and again after a write. */
export function readRecord(table: string, key: Cell[]): Promise<Values | null> {
  const id = `${table}\u0000${keyLabel(key)}`;
  if (id in loaded && !stale.has(id)) return Promise.resolve(loaded[id]!);
  let read = reading.get(id);
  if (!read) {
    stale.delete(id);
    read = api
      .rowIfAny(table, key)
      .then(
        (r) => (loaded[id] = r.values),
        (e) => {
          if (e instanceof ApiError && e.status === 404) return (loaded[id] = null);
          throw e;
        },
      )
      .finally(() => reading.delete(id));
    reading.set(id, read);
  }
  return read;
}

/** A record's values, null when the world has no such record, undefined while it is read. */
export function recordOf(table: string, key: Cell[]): Values | null | undefined {
  const id = `${table}\u0000${keyLabel(key)}`;
  if (!(id in loaded) || stale.has(id)) readRecord(table, key).catch(() => {});
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
