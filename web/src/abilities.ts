// MajorMUD ability numbers, names and notes, from Nightmare Redux's ability.mdb, kept in
// crates/daydream-nmr/src/fields.json (shared with the server).

import FIELDS from '../../crates/daydream-nmr/src/fields.json';

export const ABILITIES: Record<number, [string, string]> = Object.fromEntries(
  Object.entries(FIELDS.abilities).map(([n, a]) => [n, [a.name, a.note]]),
);
