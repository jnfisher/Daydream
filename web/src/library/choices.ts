// What a number can be picked from by name: a table's records (a room's on one map, a text
// block's first parts), or the abilities; and the key of the record a number names.

import { ABILITIES } from '../abilities';
import { titleOf } from '../ansi';
import { rowsOf } from '../shell/state.svelte';
import type { Choice } from './sections';

export { refKey } from './problems';

/** A table's records as choices, by number and name (the rooms of `map`). */
export async function recordChoices(table: string, map = 0): Promise<Choice[]> {
  const rows = await rowsOf(table);
  const out: Choice[] = [];
  for (const [key, title] of rows) {
    const name = title === null ? '' : titleOf(String(title));
    if (table === 'Rooms') {
      if (Number(key[0]) === map) out.push({ value: Number(key[1]), label: name });
    } else if (table === 'Textblocks') {
      if (Number(key[1]) === 0) out.push({ value: Number(key[0]), label: name });
    } else if (typeof key[0] === 'number') out.push({ value: key[0], label: name });
  }
  return out;
}

/** The abilities, by number, name and what they do. */
export const abilityChoices = (): Choice[] =>
  Object.entries(ABILITIES)
    .filter(([v]) => v !== '0')
    .map(([v, [name, note]]) => ({ value: Number(v), label: name, sub: note }));
