// What is wrong with a record on its card, for a person, each with its fix when there is a
// sure one: a value its field cannot hold (checked as it is typed), a number that names a
// record the world does not have, an empty slot that still holds an old entry's numbers, and
// text with stray spaces at its end. A fix is the values to type into the card, so it is one
// Save and one undo.
//
// Pure: no DOM, so `node --test` runs its tests (problems.test.ts); its imports name their
// files whole, as node needs.

import { ESC_SHOWN } from '../ansi.ts';
import type { Finding } from '../api.ts';
import type { Grid } from '../layouts.ts';
import type { Ref } from './script.ts';
import { slotCol, slotsInUse, type Cell, type Section } from './sections.ts';

export type Get = (col: string) => Cell;

/** What a column holds, as the server describes it. */
export interface ColumnFacts {
  type: 'long' | 'double' | 'text' | 'memo';
  size: number | null;
  /** The whole numbers it holds, when its element limits them. */
  range?: [number, number] | null;
}

export interface Problem {
  /** An error the save or the game trips on, a warning worth a look, or something to tidy. */
  level: 'error' | 'warn' | 'tidy';
  /** What is wrong, as a sentence. */
  text: string;
  /** The columns it is about. */
  cols: string[];
  /** Its fix: the button's words and the values to type. */
  fix?: { label: string; set: Record<string, string> };
  /** What it says with records as chips, when it names some (a check across the world's). */
  bits?: (string | { ref: Ref })[];
  /** A word spelling can be told to leave alone. */
  accept?: string;
}

const fmt = (n: number) => n.toLocaleString('en-US');
const num = (v: Cell | undefined) => Number(v) || 0;

// ---- checks as you type -------------------------------------------------------------------

/** 0x80..0x9F of Windows-1252, the code page MajorMUD's text goes through. */
const HIGH_1252 = '€\u0081‚ƒ„…†‡ˆ‰Š‹Œ\u008DŽ\u008F\u0090‘’“”•–—˜™š›œ\u009DžŸ';

/** A character's Windows-1252 byte, or null when it has none. */
export function byteOf(ch: string): number | null {
  const c = ch.codePointAt(0)!;
  if (c <= 0x7f || (c >= 0xa0 && c <= 0xff)) return c;
  const i = HIGH_1252.indexOf(ch);
  return i >= 0 ? 0x80 + i : null;
}

const quoted = (chars: string[]) =>
  [...new Set(chars)]
    .slice(0, 3)
    .map((c) => `“${c}”`)
    .join(' ');

/**
 * Why `typed` will not do for a column, as a short sentence, or null when it will: a number
 * column takes a number, and a whole one in its range when its element limits it; text takes
 * the characters Windows-1252 has, and a text block drops those past ß.
 */
export function checkValue(c: ColumnFacts, typed: string, textblock = false): string | null {
  if (c.type === 'long' || c.type === 'double') {
    const t = typed.trim();
    if (t === '') return null;
    const n = Number(t);
    if (!Number.isFinite(n)) return 'Not a number.';
    if (!c.range) return null;
    if (!Number.isInteger(n)) return 'Takes whole numbers only.';
    const [lo, hi] = c.range;
    if (n < lo || n > hi) return `Takes ${fmt(lo)} to ${fmt(hi)}.`;
    return null;
  }
  const chars = [...typed];
  const none = chars.filter((ch) => byteOf(ch) === null);
  if (none.length) return `The game can't store ${quoted(none)}.`;
  if (textblock) {
    const dropped = chars.filter((ch) => byteOf(ch)! > 0xdf);
    if (dropped.length) return `A text block drops ${quoted(dropped)} (it keeps characters up to ß).`;
  }
  if (c.size && chars.length > c.size) return `Takes ${fmt(c.size)} characters at most.`;
  return null;
}

/** `typed` without the characters a column cannot store (␛ stays: it is ESC). */
export function storable(typed: string, textblock = false): string {
  return [...typed]
    .filter((ch) => {
      if (ch === ESC_SHOWN) return true;
      const b = byteOf(ch);
      return b !== null && (!textblock || b <= 0xdf);
    })
    .join('');
}

/** The value in a column's range nearest to `n`. */
export function clampTo(range: [number, number], n: number): number {
  return Math.min(range[1], Math.max(range[0], Math.round(n)));
}

// ---- references ---------------------------------------------------------------------------

const MAP_CHANGE = 8;

/** A record's key as its table keys it: a room on its map, a text block's first part. */
export function refKey(table: string, n: number, map = 0): Cell[] {
  if (table === 'Rooms') return [map, n];
  if (table === 'Textblocks') return [n, 0];
  return [n];
}

/** What says which table a column names (fields.json, through layouts.ts). */
export interface Targets {
  targetOf: (table: string, col: string) => string | undefined;
  /** The table an exit's parameter names for the exit's type. */
  exitLinks: (type: number, para: string) => string | undefined;
}

/**
 * The record a column's number names: its table and key, or null when it names none (0, or a
 * column that names no record). A room's exit names a room on its own map, or on the map a
 * Map Change exit's Para1 names; an exit's parameters name what the exit's type says.
 */
export function refOf(table: string, col: string, get: Get, t: Targets): { table: string; key: Cell[] } | null {
  const n = num(get(col));
  if (n <= 0) return null;
  const own = num(get('Map Number'));
  if (table === 'Rooms') {
    const exit = /^Exit (\d+)$/.exec(col);
    if (exit) {
      const map = num(get(`Type ${exit[1]}`)) === MAP_CHANGE ? num(get(`Para1 ${exit[1]}`)) : own;
      return { table: 'Rooms', key: [map, n] };
    }
    const para = /^(Para\d) (\d+)$/.exec(col);
    if (para) {
      const links = t.exitLinks(num(get(`Type ${para[2]}`)), para[1]!);
      return links ? { table: links, key: refKey(links, n, own) } : null;
    }
  }
  const target = t.targetOf(table, col);
  return target ? { table: target, key: refKey(target, n, own) } : null;
}

// ---- leftovers ----------------------------------------------------------------------------

/** Two spaces as a 16-bit number: what old tools left in a slot they emptied. */
export const TWO_SPACES = 0x2020;

/**
 * The empty slots of a grid that still hold something: a slot whose first column is unset
 * (no ability, no item) with another column set. The game skips such a slot, so its values
 * do nothing. A grid of separate lists has no such slots.
 */
export function leftoverSlots(g: Grid, get: Get): number[] {
  if (g.separate) return [];
  const used = new Set(slotsInUse(g, get));
  const out: number[] = [];
  for (let n = g.from; n <= g.to; n++) {
    if (used.has(n)) continue;
    if (g.columns.slice(1).some((c) => num(get(slotCol(c.template, n))) !== 0)) out.push(n);
  }
  return out;
}

/** The values that empty a grid's slots: 0 for each of their columns. */
export function clearing(g: Grid, slots: number[], has: (col: string) => boolean): Record<string, string> {
  const set: Record<string, string> = {};
  for (const n of slots) for (const c of g.columns) if (has(slotCol(c.template, n))) set[slotCol(c.template, n)] = '0';
  return set;
}

/** A grid's leftovers as one problem to tidy. */
export function leftoverProblem(g: Grid, title: string, get: Get, has: (col: string) => boolean): Problem | null {
  const slots = leftoverSlots(g, get);
  if (!slots.length) return null;
  const cols = slots.flatMap((n) => g.columns.map((c) => slotCol(c.template, n)).filter(has));
  const spaces = cols.some((c) => num(get(c)) === TWO_SPACES);
  const what = slots.length === 1 ? `An empty slot of ${title} still holds a value` : `${slots.length} empty slots of ${title} still hold values`;
  return {
    level: 'tidy',
    text: `${what}${spaces ? ` (${TWO_SPACES} is two spaces an old tool left)` : ''}. The game skips ${slots.length === 1 ? 'it' : 'them'}.`,
    cols,
    fix: { label: slots.length === 1 ? 'Clear it' : 'Clear them', set: clearing(g, slots, has) },
  };
}

// ---- text ---------------------------------------------------------------------------------

/** Text that ends in spaces (before its last line break, if it has one), as a problem to
 * tidy. Spaces inside it are left alone: on a coloured line they can be drawn. */
export function trailingSpaces(col: string, label: string, text: string): Problem | null {
  const m = /[ \t]+(\r?\n)?$/.exec(text);
  if (!m) return null;
  const trimmed = text.slice(0, m.index) + (m[1] ?? '');
  return { level: 'tidy', text: `${label} ends in stray spaces.`, cols: [col], fix: { label: 'Trim them', set: { [col]: trimmed } } };
}

// ---- in words -----------------------------------------------------------------------------

/**
 * What a card calls each column it shows, for its problems: a field's NMR label, an attack's
 * column by its slot (`Attack 2 Hit Msg`); a column it does not name keeps its own name.
 */
export function labelsOf(sections: Section[]): Map<string, string> {
  const out = new Map<string, string>();
  for (const s of sections) {
    for (const f of s.fields ?? []) out.set(f.col, f.label || f.col);
    const g = s.grid;
    if (g?.across)
      for (let n = g.from; n <= g.to; n++)
        for (const c of g.columns) out.set(slotCol(c.template, n), `${g.across} ${n - g.from + 1} ${c.label ?? c.template.replace(' #', '')}`);
  }
  return out;
}

// ---- checks across the world --------------------------------------------------------------

/** `by` in the case `like` is written in: `Exchange` for `Exhange`, `EXCHANGE` for `EXHANGE`. */
export function caseLike(by: string, like: string): string {
  if (like.length > 1 && like === like.toUpperCase()) return by.toUpperCase();
  if (like[0] && like[0] === like[0].toUpperCase() && like[0] !== like[0].toLowerCase()) return by[0]!.toUpperCase() + by.slice(1);
  return by;
}

/** A colour code, as stored or as an input shows it. */
const CODE = /(?:\u241b|\x1b)\[[0-9;]*[A-Za-z]/g;

/** `text` with each whole `word` (any case) made `by`, in the case it was written in: a word
 * just after a colour code (`␛[0mexhange`) is a word all the same. */
export function replaceWord(text: string, word: string, by: string): string {
  const esc = word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`(?<![A-Za-z])${esc}(?![A-Za-z])`, 'gi');
  const fix = (s: string) => s.replace(re, (m) => caseLike(by, m));
  let out = '';
  let at = 0;
  for (const m of text.matchAll(CODE)) {
    out += fix(text.slice(at, m.index)) + m[0];
    at = m.index + m[0].length;
  }
  return out + fix(text.slice(at));
}

const sameKey = (a: readonly Cell[], b: readonly Cell[]) => a.length === b.length && a.every((x, i) => Number(x) === Number(b[i]));

/** A record a finding is about, as a chip. */
function chipOf(about: Finding['about']): { ref: Ref } {
  const t = about.table as Ref['t'];
  return { ref: t === 'Rooms' ? { t, n: about.key[1]!, map: about.key[0]! } : { t, n: about.key[0]! } };
}

/**
 * The checks across the world that concern a record (the server's), as its problems: what
 * players meet is a warning, the rest is to tidy. One about the record names its field and
 * its fix (the game's own words for a blank message, a likely typo put right); one that only
 * rests on it starts with the record it is about. What was checked and holds is no problem.
 */
export function foundProblems(found: Finding[], table: string, key: readonly Cell[], get: Get): Problem[] {
  const out: Problem[] = [];
  for (const f of found) {
    if (f.level === 'ok') continue;
    const own = f.about.table === table && sameKey(f.about.key, key);
    const p: Problem = {
      level: f.level === 'warn' ? 'warn' : 'tidy',
      text: f.text,
      cols: own && f.column ? [f.column] : [],
      bits: own ? f.bits : [chipOf(f.about), ': ', ...f.bits],
    };
    if (own && Object.keys(f.fix).length) p.fix = { label: "Use the game's own words", set: f.fix };
    if (own && f.word && f.column && f.suggest[0]) {
      const was = String(get(f.column) ?? '');
      const fixed = replaceWord(was, f.word, f.suggest[0]);
      if (fixed !== was) p.fix = { label: `Make it “${f.suggest[0]}”`, set: { [f.column]: fixed } };
      p.accept = f.word;
    }
    out.push(p);
  }
  return out;
}

// ---- in order ------------------------------------------------------------------------------

const ORDER = { error: 0, warn: 1, tidy: 2 } as const;

/** Problems, the worst first. */
export function byLevel(ps: Problem[]): Problem[] {
  return [...ps].sort((a, b) => ORDER[a.level] - ORDER[b.level]);
}

/** Every fix of a kind, as one set of values. */
export function allFixes(ps: Problem[]): Record<string, string> {
  return Object.assign({}, ...ps.map((p) => p.fix?.set ?? {}));
}
