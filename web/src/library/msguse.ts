// How a message reads for each record that uses it: the lines each use reads, for whom,
// filled in as the game fills them (the record's own name, and a sample player, damage and
// direction for the rest), with the uses gathered by what they read. The server says what
// each use reads (`GET /api/references` for a message, `messages::reads`).
//
// Pure: no DOM, so `node --test` runs its tests (msguse.test.ts).

import { fmtMsg, upperFirst, type Lines } from './combat.ts';
import type { Cell, MessageRead, Reference } from './where.ts';

/** What fills a blank with the name of the record using the message. */
export const ITS_NAME = 'its name';

/** What a preview fills each other blank with. */
const SAMPLE: Record<string, string> = {
  "the player's name": 'Rowan',
  'the damage': '12',
  'his or her': 'her',
  'he or she': 'she',
  "its weapon's miss verb": 'swings',
  "its weapon's name": 'club',
  'where it came from (“the north”)': 'the north',
  'the caster': 'Rowan',
  'the target': 'Aric',
  'the amount': '12',
  'a star (*)': '*',
};

/** Say what fills the blanks of the previews, for the person reading them. */
export const SAMPLES_SAID = 'Rowan stands in for the player, 12 for the damage, the north for where a monster came from.';

/** A line as one use reads it: each blank filled with what the game passes. */
export function readAs(line: string, fills: readonly string[], name: string): string {
  return fmtMsg(
    line,
    fills.map((f) => (f === ITS_NAME ? name : (SAMPLE[f] ?? `‹${f}›`))),
  );
}

/** One record that uses the message in one way. */
export interface User {
  table: string;
  key: Cell[];
  title: string;
  /** The columns that name it this way (`Attack Hit Msg 0`), or a script's lines. */
  by: string[];
}

/** The uses of a message that read it the same way. */
export interface UseGroup {
  table: string;
  /** What they read, line by line, the attack's slot left out (`an attack hits`). */
  reads: MessageRead[] | null;
  users: User[];
}

/** `attack 2 hits` as `an attack hits`, so the uses of each attack slot read as one. */
const anySlot = (when: string) => when.replace(/\battack \d+\b/g, 'an attack');

const sameKey = (a: readonly Cell[], b: readonly Cell[]) => a.length === b.length && a.every((v, i) => v === b[i]);

/**
 * The uses gathered by table and what they read, the ones the game reads line by line first;
 * a record once in each, with every column that names the message that way.
 */
export function groupUses(refs: readonly Reference[]): UseGroup[] {
  const groups = new Map<string, UseGroup>();
  for (const r of refs) {
    const reads = r.reads ? r.reads.map((x) => ({ ...x, when: anySlot(x.when) })) : null;
    const id = `${r.table}\u0000${JSON.stringify(reads)}`;
    let g = groups.get(id);
    if (!g) groups.set(id, (g = { table: r.table, reads, users: [] }));
    const u = g.users.find((x) => sameKey(x.key, r.key));
    if (u) {
      if (!u.by.includes(r.by)) u.by.push(r.by);
    } else g.users.push({ table: r.table, key: r.key, title: r.title === null ? '' : String(r.title), by: [r.by] });
  }
  return [...groups.values()].sort((a, b) => Number(a.reads === null) - Number(b.reads === null));
}

/** When a group's lines are read, for a person: `When an attack hits` or `When it dies`. */
export function whenSaid(reads: readonly MessageRead[]): string {
  const whens = [...new Set(reads.map((r) => r.when))];
  const said = whens.join(', or ');
  return said ? `When ${said}` : '';
}

/** A line the game reads, filled in for a use: who reads it, and what. */
export interface FilledLine {
  line: number;
  to: string;
  when: string;
  text: string;
}

/** A monster's attack line: the game sends it with its first letter upper-cased. */
const COMBAT = /\b(an attack|attack \d+)\b/;

/** The lines a use reads, filled in for it, from the message's lines as they are now. */
export function filledFor(lines: Lines, reads: readonly MessageRead[], name: string): FilledLine[] {
  return reads.map((r) => {
    const text = readAs(lines[r.line - 1] ?? '', r.fills, name || 'it');
    return { line: r.line, to: r.to, when: r.when, text: COMBAT.test(r.when) ? upperFirst(text) : text };
  });
}

/** Whether what a use reads holds its own name (else every use reads the same words). */
export const namesIt = (reads: readonly MessageRead[]) => reads.some((r) => r.fills.includes(ITS_NAME));

/** How many records of each table, in words, the most first: `12 monsters and 1 item`. */
export function countedTables(by: Readonly<Record<string, number>>, counted: (table: string, n: number) => string): string {
  const parts = Object.entries(by)
    .filter(([, n]) => n > 0)
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([t, n]) => counted(t, n));
  return parts.length > 1 ? `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}` : (parts[0] ?? '');
}

/** The records, counted by table in words: `12 monsters and 1 item`. */
export function countedUsers(users: readonly { table: string }[], counted: (table: string, n: number) => string): string {
  const by: Record<string, number> = {};
  for (const u of users) by[u.table] = (by[u.table] ?? 0) + 1;
  return countedTables(by, counted);
}
