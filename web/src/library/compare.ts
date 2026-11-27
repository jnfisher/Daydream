// Compare: a record beside others of its table, field by field under its card's headings, the
// differences marked; and the others it starts with, the closest of its peers by the numbers
// that measure them (or, without such numbers, the records beside it in its list).
//
// Pure: no DOM, so `node --test` runs its tests (compare.test.ts); its imports name their
// files whole, as node needs.

import type { Pick } from '../layouts.ts';
import type { Cell, Section } from './sections.ts';

type Get = (col: string) => Cell;

const num = (v: Cell | undefined) => Number(v) || 0;

/**
 * The `n` records of `rows` closest to `mine` by the numbers in `cols`: each column's
 * difference over its spread among them all, added up. Ties keep the rows' order.
 */
export function closest<R extends { get: Get }>(mine: Get, rows: readonly R[], cols: readonly string[], n: number): R[] {
  if (!cols.length) return rows.slice(0, n);
  const spread = cols.map((c) => {
    let lo = num(mine(c));
    let hi = lo;
    for (const r of rows) {
      const v = num(r.get(c));
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
    return hi - lo || 1;
  });
  return rows
    .map((r, i) => ({ r, i, d: cols.reduce((d, c, k) => d + Math.abs(num(r.get(c)) - num(mine(c))) / spread[k]!, 0) }))
    .sort((a, b) => a.d - b.d || a.i - b.i)
    .slice(0, n)
    .map((x) => x.r);
}

/**
 * Up to `n` of `rows` around the one at `at`, nearest first (the next, the one before, the one
 * after that…), in the rows' order; the first `n` when `at` is not among them.
 */
export function around<R>(rows: readonly R[], at: number, n: number): R[] {
  if (at < 0 || at >= rows.length) return rows.slice(0, n);
  const picked: number[] = [];
  for (let d = 1; picked.length < n && (at + d < rows.length || at - d >= 0); d++) {
    for (const i of [at + d, at - d]) if (i >= 0 && i < rows.length && picked.length < n) picked.push(i);
  }
  return picked.sort((a, b) => a - b).map((i) => rows[i]!);
}

/** A field compared: its column, the card's label, and how its values read. */
export interface CompareField {
  col: string;
  label: string;
  pick?: Pick;
}

/**
 * The fields to compare, under the card's headings: each section's single fields (and the
 * column edited beside one, a cost's coin), in the card's order, the key left out. Families of
 * numbered slots are left out too: their slots do not line up from one record to the next.
 */
export function compareFields(sections: readonly Section[], key: readonly string[], has: (col: string) => boolean): { title: string; fields: CompareField[] }[] {
  const seen = new Set(key);
  const out: { title: string; fields: CompareField[] }[] = [];
  for (const s of sections) {
    const fields: CompareField[] = [];
    for (const f of s.fields ?? []) {
      for (const [col, label, pick] of [
        [f.col, f.label || f.col, f.pick],
        [f.with, f.with, undefined],
      ] as const) {
        if (!col || seen.has(col) || !has(col)) continue;
        seen.add(col);
        fields.push({ col, label: label ?? col, pick });
      }
    }
    for (const col of s.text?.cols ?? []) {
      if (seen.has(col) || !has(col)) continue;
      seen.add(col);
      fields.push({ col, label: col });
    }
    if (fields.length) out.push({ title: s.title, fields });
  }
  return out;
}

/** A text as Compare shows it: up to a NUL (what follows is left over), every line, its colour
 * codes left out. */
export function plainText(v: string): string {
  return v
    .split('\0')[0]!
    .replace(/\x1b\[[0-9;]*[A-Za-z]?/g, '')
    .replace(/\r\n?/g, '\n')
    .replace(/\n+$/, '');
}

/** Whether two values are the same, as a record holds them (a number column's 5 and '5'). */
export function same(a: Cell | undefined, b: Cell | undefined): boolean {
  if (typeof a === 'number' || typeof b === 'number') return Number(a ?? 0) === Number(b ?? 0);
  return (a ?? '') === (b ?? '');
}
