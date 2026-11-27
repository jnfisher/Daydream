// The spreadsheet grid: the columns it starts with, the columns it offers (under the card's
// headings), how its rows sort, and its CSV.
//
// Pure: no DOM, so `node --test` runs its tests (grid.test.ts); its imports name their files
// whole, as node needs.

import { columnsOf, type Cell, type Section } from './sections.ts';

/**
 * The columns a table's grid starts with: its name, then the numbers its list reads, else
 * the card's first single fields, up to `n`. Never the key, which every row shows anyway.
 */
export function defaultColumns(
  sections: readonly Section[],
  all: readonly string[],
  key: readonly string[],
  title: string | null | undefined,
  listCols: readonly string[] | undefined,
  n = 8,
): string[] {
  const out: string[] = [];
  const take = (c: string | undefined) => {
    if (c && out.length < n && all.includes(c) && !key.includes(c) && !out.includes(c)) out.push(c);
  };
  take(title ?? undefined);
  for (const c of listCols ?? []) take(c);
  if (out.length < 3) for (const s of sections) for (const f of s.fields ?? []) [f.col, f.with].forEach(take);
  return out;
}

/** The columns the picker offers, under the card's headings; what they leave out last. */
export function columnGroups(sections: readonly Section[], all: readonly string[], key: readonly string[]): { title: string; cols: string[] }[] {
  const seen = new Set(key);
  const out: { title: string; cols: string[] }[] = [];
  for (const s of sections) {
    const cols = columnsOf(s).filter((c) => all.includes(c) && !seen.has(c));
    cols.forEach((c) => seen.add(c));
    if (cols.length) out.push({ title: s.title, cols });
  }
  const rest = all.filter((c) => !seen.has(c));
  if (rest.length) out.push({ title: 'More fields', cols: rest });
  return out;
}

/** The columns picked, in the order the picker lists them. */
export const inOrder = (picked: ReadonlySet<string>, groups: readonly { cols: string[] }[]) =>
  groups.flatMap((g) => g.cols).filter((c) => picked.has(c));

/** How two cells sort: numbers by value, text alphabetically ignoring case, blanks last. */
export function compareCells(a: Cell | undefined, b: Cell | undefined): number {
  const blankA = a === null || a === undefined || a === '';
  const blankB = b === null || b === undefined || b === '';
  if (blankA || blankB) return blankA === blankB ? 0 : blankA ? 1 : -1;
  if (typeof a === 'number' && typeof b === 'number') return a - b;
  return String(a).localeCompare(String(b), undefined, { sensitivity: 'base', numeric: true });
}

/**
 * One CSV field: quoted when it holds a comma, a quote or a line break. A text a spreadsheet
 * would run as a formula (starting =, +, -, @) starts with ' instead.
 */
export function csvField(v: string | number | null | undefined): string {
  if (v === null || v === undefined) return '';
  if (typeof v === 'number') return String(v);
  const s = /^[=+\-@\t\r]/.test(v) ? `'${v}` : v;
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/** A CSV file: a header line, then a line per row, each ending CRLF, with a byte order mark
 * so spreadsheets read it as UTF-8. */
export function toCsv(header: readonly string[], rows: readonly (readonly (string | number | null | undefined)[])[]): string {
  return `﻿${[header, ...rows].map((r) => `${r.map(csvField).join(',')}\r\n`).join('')}`;
}
