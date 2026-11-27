// The change lists of NMR's changers, as the Workshop's forms build them: lines of numbers
// typed as text, turned into the moves a run makes (or why it cannot run yet); the numbers a
// table uses, in runs; and a long text's change cut down to the part that differs. Pure.

import type { IndexChange, Move } from '../api';

/** A line of a change list as typed: a run of numbers (`to` blank for one) and where it starts anew. */
export interface Line {
  from: string;
  to: string;
  start: string;
}

/** A line of the Monster Group/Index Changer: a run of one group's indexes, to another group. */
export interface IndexLine extends Line {
  group: number;
  toGroup: number;
}

/** The most records one run moves. */
export const MOST = 10_000;

const counted = (s: string) => (/^\s*\d+\s*$/.test(s) ? Number(s) : NaN);
const blank = (l: Line) => !l.from.trim() && !l.to.trim() && !l.start.trim();

/** A line's numbers, or why they are not yet a run. `least` is the lowest a number may be. */
function runOf(l: Line, least: number): { from: number; to: number; start: number } | { error: string } {
  const from = counted(l.from);
  const to = l.to.trim() ? counted(l.to) : from;
  const start = counted(l.start);
  if (Number.isNaN(from) || Number.isNaN(to)) return { error: 'Give the numbers to move as whole numbers.' };
  if (to < from) return { error: `${to} comes before ${from}: give the first number, then the last.` };
  if (Number.isNaN(start)) return { error: `Give the new number for ${from === to ? from : `${from} to ${to}`}.` };
  if (from < least || start < least) return { error: least ? `Numbers start at ${least}.` : 'Numbers cannot be below 0.' };
  return { from, to, start };
}

/** The moves a record change list makes, or why it cannot run yet. Blank lines are skipped. */
export function movesOf(lines: Line[]): { moves: Move[] } | { error: string } {
  const moves: Move[] = [];
  for (const l of lines) {
    if (blank(l)) continue;
    const r = runOf(l, 1);
    if ('error' in r) return r;
    if (moves.length + r.to - r.from + 1 > MOST) return { error: `One run moves at most ${MOST.toLocaleString()} records.` };
    for (let x = r.from; x <= r.to; x++) moves.push({ from: x, to: r.start + (x - r.from) });
  }
  if (!moves.length) return { error: 'Give the number to move and its new number.' };
  return { moves };
}

/** The changes a group and index change list makes, or why it cannot run yet. */
export function indexChangesOf(lines: IndexLine[]): { changes: IndexChange[] } | { error: string } {
  const changes: IndexChange[] = [];
  const seen = new Set<string>();
  for (const l of lines) {
    if (blank(l)) continue;
    const r = runOf(l, 0);
    if ('error' in r) return r;
    if (changes.length + r.to - r.from + 1 > MOST) return { error: `One run moves at most ${MOST.toLocaleString()} indexes.` };
    for (let x = r.from; x <= r.to; x++) {
      if (seen.has(`${l.group}/${x}`)) return { error: `Index ${x} of that group is in two lines.` };
      seen.add(`${l.group}/${x}`);
      changes.push({ from_group: l.group, from_index: x, to_group: l.toGroup, to_index: r.start + (x - r.from) });
    }
  }
  if (!changes.length) return { error: 'Give the indexes to move and where they go.' };
  return { changes };
}

/** A line as said: `12–20 → 40–48`, or `868 → 3000`. Empty when it is not a run yet. */
export function lineText(l: Line): string {
  const r = runOf(l, 0);
  if ('error' in r) return '';
  const span = (a: number, b: number) => (a === b ? `${a}` : `${a}–${b}`);
  return `${span(r.from, r.to)} → ${span(r.start, r.start + r.to - r.from)}`;
}

/** Where a line's run ends anew, as the form shows it beside the new start ('' until it can say). */
export function endOf(l: Line): string {
  const r = runOf(l, 0);
  return 'error' in r ? '' : String(r.start + r.to - r.from);
}

/** Numbers in runs of consecutive ones, lowest first: `[[1, 3], [5, 5]]`. */
export function runsOf(numbers: Iterable<number>): [number, number][] {
  const sorted = [...new Set(numbers)].filter(Number.isFinite).sort((a, b) => a - b);
  const runs: [number, number][] = [];
  for (const n of sorted) {
    const last = runs.at(-1);
    if (last && last[1] + 1 === n) last[1] = n;
    else runs.push([n, n]);
  }
  return runs;
}

/**
 * A long text's change, cut down to what differs: the lines that change when it keeps its
 * lines, or else the part between what the two share at each end, with a little either side.
 * Short texts come back whole.
 */
export function textChange(before: string, after: string, room = 80): [string, string] {
  if (before.length <= room && after.length <= room) return [before, after];
  const a = before.split('\n');
  const b = after.split('\n');
  if (a.length === b.length) {
    const at = a.flatMap((line, i) => (line !== b[i] ? [i] : []));
    if (at.length) {
      const pick = (lines: string[]) => at.map((i) => lines[i]!).join('\n');
      return [pick(a), pick(b)];
    }
  }
  let head = 0;
  while (head < before.length && head < after.length && before[head] === after[head]) head++;
  let tail = 0;
  while (tail < before.length - head && tail < after.length - head && before[before.length - 1 - tail] === after[after.length - 1 - tail]) tail++;
  const SIDE = 24;
  const cut = (s: string) =>
    (head > SIDE ? '…' : '') + s.slice(Math.max(0, head - SIDE), s.length - Math.max(0, tail - SIDE)) + (tail > SIDE ? '…' : '');
  return [cut(before), cut(after)];
}
