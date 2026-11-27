// Who else uses each message a record names (GET /api/messages/users): asked for by number
// for the record, read in batches (a card's message fields all at once), and again after a
// write. Asked for inside a component's markup or a derived value, the answer comes in when
// the read does.

import { api, type Cell, type SharedWith } from '../api';
import { keyLabel } from '../shell/routes';

/** By the record asking, then by message. */
const known = $state<Record<string, Record<number, SharedWith>>>({});
/** Asked for and not yet read, by the record asking. */
const wanted = new Map<string, { table: string; key: Cell[]; numbers: Set<number> }>();
/** Being read now: `record\u0000number`. */
const asking = new Set<string>();
let stale = new Set<string>();
let flushing = false;
/** Bumped by a write, so what was read before is asked for again where it is shown. */
let gen = $state(0);

const forget = () => {
  stale = new Set(Object.entries(known).flatMap(([who, by]) => Object.keys(by).map((n) => `${who}\u0000${n}`)));
  gen++;
};
window.addEventListener('dd:wrote', forget);
window.addEventListener('dd:realm', forget);

/** Reads every number asked for so far, 64 at a time, once the code asking is done. */
function flush() {
  if (flushing) return;
  flushing = true;
  queueMicrotask(async () => {
    while (wanted.size) {
      const [who, w] = wanted.entries().next().value!;
      const numbers = [...w.numbers].slice(0, 64);
      for (const n of numbers) {
        w.numbers.delete(n);
        asking.add(`${who}\u0000${n}`);
      }
      if (!w.numbers.size) wanted.delete(who);
      try {
        const r = await api.messageUsers(numbers, w.table, w.key);
        const by = (known[who] ??= {});
        for (const n of numbers) {
          by[n] = r.users[String(n)] ?? { others: 0, tables: {}, some: [] };
          stale.delete(`${who}\u0000${n}`);
        }
      } catch {
        // who else uses a message is extra: a field without it is still the field
      } finally {
        for (const n of numbers) asking.delete(`${who}\u0000${n}`);
      }
    }
    flushing = false;
  });
}

/** The records besides `table`'s record `key` that use message `n`, undefined while read. */
export function sharedWith(n: number, table: string, key: Cell[]): SharedWith | undefined {
  void gen;
  if (!(n > 0)) return { others: 0, tables: {}, some: [] };
  const who = `${table}\u0000${keyLabel(key)}`;
  const id = `${who}\u0000${n}`;
  const have = known[who]?.[n];
  if ((have === undefined || stale.has(id)) && !asking.has(id)) {
    let w = wanted.get(who);
    if (!w) wanted.set(who, (w = { table, key, numbers: new Set() }));
    w.numbers.add(n);
    flush();
  }
  return have;
}
