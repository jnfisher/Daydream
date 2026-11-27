// A record card's parts: each a titled group of a table's columns (single fields, a family of
// numbered columns, lines of game text, the rows that name the record), and how a card made
// of them still shows every column. The card of a table groups its columns by what they are
// for (cards.ts); "All fields" shows NMR's own tabs, as sections too.
//
// Pure: no DOM, so `node --test` runs its tests (sections.test.ts); its imports name their
// files whole, as node needs.

import type { Grid, Layout, Matrix, Pick } from '../layouts.ts';

export type Cell = number | string | null;

/** One field of a section. */
export interface FieldSpec {
  col: string;
  /** NMR's label, when it is not the column's name. */
  label?: string;
  /** Shown beside the value. */
  note?: string;
  /** Drawn among its peers' values (cards.ts says who the peers are). */
  strip?: boolean;
  /** Smaller is better (a weapon's speed), as the strip says. */
  lower?: boolean;
  /** A second column edited beside it (a cost and its coin). */
  with?: string;
  pick?: Pick;
}

export interface Section {
  id: string;
  title: string;
  /** What the section is for, in a few words. */
  hint?: string;
  /** In the card's side column (on a wide screen). */
  side?: boolean;
  fields?: FieldSpec[];
  /** A family of numbered columns: the slots in use, and a way to fill the next. */
  grid?: Grid;
  matrix?: Matrix;
  /** Lines of text as a terminal shows them, edited line by line. */
  text?: { cols: string[]; label?: string };
  /** The rows of the world that name the record. */
  where?: boolean;
  /**
   * Drawn by a part of its own (a monster's attacks, how it is wired, where it spawns). Its
   * fields and grid still say which columns it shows.
   */
  custom?: 'attacks' | 'wired' | 'spawns';
  /** Two of its numbers drawn against its peers' (a weapon's Max Hit against its Speed). Both
   * must be among the list's columns. */
  scatter?: { x: string; y: string; xLabel?: string; yLabel?: string; note?: string };
  /** Only for the records this says yes to (an item's fight, for a weapon). */
  when?: (get: (col: string) => Cell) => boolean;
  /** Closed until opened. */
  folded?: boolean;
}

/** A number a picker offers, by name. */
export interface Choice {
  value: number;
  label: string;
  sub?: string;
}

/** A grid's column for slot `n`. */
export const slotCol = (template: string, n: number) => template.replace('#', String(n));

/** Every column a section shows. */
export function columnsOf(s: Section): string[] {
  const out: string[] = [];
  for (const f of s.fields ?? []) {
    out.push(f.col);
    if (f.with) out.push(f.with);
  }
  for (const r of s.matrix?.rows ?? []) out.push(...r.cols);
  if (s.grid) {
    for (let n = s.grid.from; n <= s.grid.to; n++) for (const c of s.grid.columns) out.push(slotCol(c.template, n));
  }
  out.push(...(s.text?.cols ?? []));
  return out;
}

/**
 * The sections a record shows: those it is for, then "More fields" with each column of
 * `columns` none of them shows (in the table's order), so nothing is out of reach. The key's
 * columns are the card's header, not a field.
 */
export function complete(sections: Section[], columns: string[], key: string[], get: (col: string) => Cell): Section[] {
  const shown = sections.filter((s) => !s.when || s.when(get));
  const seen = new Set<string>(key);
  for (const s of shown) for (const c of columnsOf(s)) seen.add(c);
  const rest = columns.filter((c) => !seen.has(c));
  return rest.length
    ? [...shown, { id: 'more', title: 'More fields', hint: 'what the groups above leave out', folded: true, fields: rest.map((col) => ({ col })) }]
    : shown;
}

/** NMR's tabs as sections, for the "All fields" view: every column where NMR's form has it. */
export function fromLayout(layout: Layout, columns: string[], key: string[]): Section[] {
  const out: Section[] = layout.map((tab, i) => ({
    id: `tab${i}`,
    title: tab.title,
    fields: tab.fields?.filter((f) => !key.includes(f.col) && !tab.preview?.includes(f.col)).map((f) => ({ col: f.col, label: f.label, pick: f.pick })),
    grid: tab.grid,
    matrix: tab.matrix,
    text: tab.preview ? { cols: tab.preview } : undefined,
  }));
  const seen = new Set<string>(key);
  for (const s of out) for (const c of columnsOf(s)) seen.add(c);
  const rest = columns.filter((c) => !seen.has(c));
  if (rest.length) out.push({ id: 'more', title: 'More', fields: rest.map((col) => ({ col })) });
  return out;
}

/**
 * The slots of a grid in use: a row whose first column is set (a drop slot with no item is
 * empty, whatever its chance), or, for a grid of separate lists, any of its columns.
 */
export function slotsInUse(g: Grid, get: (col: string) => Cell): number[] {
  const set = (col: string) => {
    const v = get(col);
    return v !== null && v !== 0 && v !== '' && v !== '0';
  };
  const out: number[] = [];
  for (let n = g.from; n <= g.to; n++) {
    const cols = g.columns.map((c) => slotCol(c.template, n));
    if (g.separate ? cols.some(set) : set(cols[0]!)) out.push(n);
  }
  return out;
}

/** The first slot of a grid not in use, or null when every one is. */
export function freeSlot(g: Grid, get: (col: string) => Cell): number | null {
  const used = new Set(slotsInUse(g, get));
  for (let n = g.from; n <= g.to; n++) if (!used.has(n)) return n;
  return null;
}

/** Where a value stands among its peers' (the others, not itself): how many are below it, the
 *  same, and above, and the range they span with it. */
export interface Standing {
  min: number;
  max: number;
  below: number;
  same: number;
  above: number;
}

export function standing(peers: readonly number[], v: number): Standing {
  let min = v;
  let max = v;
  let below = 0;
  let same = 0;
  let above = 0;
  for (const p of peers) {
    if (p < min) min = p;
    if (p > max) max = p;
    if (p < v) below++;
    else if (p > v) above++;
    else same++;
  }
  return { min, max, below, same, above };
}

/** The standing in words: `the highest of 22`, `higher than 14 of the other 21`. */
export function standingWords(s: Standing): string {
  const others = s.below + s.same + s.above;
  if (!others) return 'the only one';
  if (!s.above && !s.same) return `the highest of ${others + 1}`;
  if (!s.below && !s.same) return `the lowest of ${others + 1}`;
  if (!s.below && !s.above) return `the same as all ${others} others`;
  return `higher than ${s.below} of the other ${others}`;
}
