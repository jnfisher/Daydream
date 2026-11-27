// The fields a Workshop run may change or test, from a table's columns. Pure.

import type { TableInfo } from '../api';

/** The tables NMR's tools run over, in its order (Actions are named, not numbered). */
export const RUN_TABLES = ['Items', 'Monsters', 'Spells', 'Shops', 'Races', 'Classes', 'Rooms', 'Messages', 'Textblocks'];

/** The numbered families of a table's number columns (`Item Drop % 0` to `9`), by their stem. */
function families(names: string[]): Map<string, number> {
  const out = new Map<string, number>();
  for (const n of names) {
    const m = /^(.*\D) ?\d+$/.exec(n);
    if (m) out.set(m[1]!, (out.get(m[1]!) ?? 0) + 1);
  }
  return out;
}

/**
 * The number columns a run may change or test, as [choice, label]: each on its own, then each
 * numbered family whole (`Item Drop % *`, "Item Drop % (every one)").
 */
export function numberFields(t: TableInfo, wholeFamilies = true): [string, string][] {
  const keys = new Set(t.key);
  const names = t.columns.filter((c) => (c.type === 'long' || c.type === 'double') && !keys.has(c.name)).map((c) => c.name);
  const out: [string, string][] = names.map((n) => [n, n]);
  if (wholeFamilies) for (const [f, count] of families(names)) if (count > 1) out.push([`${f}*`, `${f.trimEnd()} (every one)`]);
  return out;
}

/** The columns a field choice stands for: itself, or its family's. */
export function columnsOf(t: TableInfo, choice: string): string[] {
  if (!choice.endsWith('*')) return [choice];
  const stem = choice.slice(0, -1);
  return t.columns.map((c) => c.name).filter((n) => n.startsWith(stem) && /^\s?\d+$/.test(n.slice(stem.length)));
}

/** A field choice as said: `Item Drop %` for a family. */
export const fieldName = (choice: string) => (choice.endsWith('*') ? choice.slice(0, -1).trimEnd() : choice);

export const hasAbilities = (t: TableInfo) => t.columns.some((c) => c.name === 'Ability 0');

export const hasColumn = (t: TableInfo | undefined, name: string) => !!t?.columns.some((c) => c.name === name);

/** A whole number typed (or NaN). */
export const whole = (s: string) => (/^\s*-?\d+\s*$/.test(s) ? Number(s) : NaN);

/** A number typed, whole or not (or NaN). */
export const decimal = (s: string) => (/^\s*-?(\d+\.?\d*|\.\d+)\s*$/.test(s) ? Number(s) : NaN);

/** Which records a run covers: all, or a run of numbers (on a map, for rooms). */
export interface Span {
  all: boolean;
  map: string;
  from: string;
  to: string;
}

/** A span as a run's `range` (null: all), or why it cannot be one. */
export function rangeOf(table: string, s: Span): { range: { table: string; map?: number; from: number; to: number } | null } | { error: string } {
  if (s.all) return { range: null };
  const [map, from, to] = [whole(s.map), whole(s.from), whole(s.to)];
  if (table === 'Rooms' && !(map > 0)) return { error: 'Give the map the rooms are on.' };
  if (Number.isNaN(from) || Number.isNaN(to)) return { error: 'Give the first and last numbers.' };
  if (to < from) return { error: 'The last number comes before the first.' };
  return { range: { table, ...(table === 'Rooms' ? { map } : {}), from, to } };
}
