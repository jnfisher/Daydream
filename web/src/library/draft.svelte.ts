// A record being edited on its card: the row as saved, what each input holds as typed, and
// what the record was when the world was last published. The card's parts share one.

import { toShown, toWire } from '../ansi';
import type { Cell, ColumnInfo, Row, TableInfo } from '../api';

/** A value as an input shows it: escapes as ␛. */
export const shown = (v: Cell | undefined) => (v === null || v === undefined ? '' : toShown(String(v)));

export class Draft {
  readonly table: TableInfo;
  /** The record as the realm has it. */
  row = $state<Row | null>(null);
  /** What each input holds, as typed, by column. */
  edits = $state<Record<string, string>>({});
  /** Grid slots opened to fill, by grid template and number (`Ability #:3`). */
  opened = $state<Record<string, boolean>>({});
  private readonly columns: Map<string, ColumnInfo>;

  constructor(table: TableInfo) {
    this.table = table;
    this.columns = new Map(table.columns.map((c) => [c.name, c]));
  }

  load(row: Row) {
    this.row = row;
    this.edits = Object.fromEntries(Object.entries(row.values).map(([k, v]) => [k, shown(v)]));
    this.opened = {};
  }

  column(col: string): ColumnInfo | undefined {
    return this.columns.get(col);
  }

  has(col: string): boolean {
    return this.columns.has(col);
  }

  isNumber(col: string): boolean {
    const t = this.columns.get(col)?.type;
    return t === 'long' || t === 'double';
  }

  isKey(col: string): boolean {
    return this.table.key.includes(col);
  }

  /** The input's text. */
  typed(col: string): string {
    return this.edits[col] ?? '';
  }

  set(col: string, text: string) {
    this.edits[col] = text;
  }

  /** The value as the record would hold it: a number column's as a number (0 when blank). */
  get = (col: string): Cell => {
    const t = this.edits[col];
    if (t === undefined) return null;
    if (this.isNumber(col)) {
      const n = Number(t);
      return t.trim() === '' ? 0 : Number.isFinite(n) ? n : t;
    }
    return toWire(t);
  };

  /** The value as saved. */
  saved(col: string): Cell {
    return this.row?.values[col] ?? null;
  }

  /** Whether the input holds something other than the saved value. */
  edited(col: string): boolean {
    return !!this.row && col in this.edits && this.typed(col) !== shown(this.row.values[col]);
  }

  /** The edited columns, as the server takes them. */
  changed(): Record<string, Cell> {
    const out: Record<string, Cell> = {};
    if (!this.row) return out;
    for (const [col, typed] of Object.entries(this.edits)) {
      if (typed !== shown(this.row.values[col])) out[col] = toWire(typed);
    }
    return out;
  }

  get dirty(): boolean {
    return Object.keys(this.changed()).length > 0;
  }

  /** What a column was at the last publish, when it changed since (undefined when not). */
  was(col: string): Cell | undefined {
    const p = this.row?.published;
    return p && 'was' in p ? p.was[col] : undefined;
  }

  /** The record is new since the last publish. */
  get added(): boolean {
    const p = this.row?.published;
    return !!p && 'added' in p;
  }

  /** How many of its columns changed since the last publish. */
  get unpublished(): number {
    const p = this.row?.published;
    return p && 'was' in p ? Object.keys(p.was).length : 0;
  }

  /** Puts the inputs back to the saved record. */
  discard() {
    if (this.row) this.load(this.row);
  }
}
