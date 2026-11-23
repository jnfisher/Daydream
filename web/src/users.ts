// NMR's Paste Character (frmUser's PasteCharacter): a capture of the game's "stat" output,
// or MMUD Explorer's "Copy Only Stats", read into the user editor's fields.

/** NMR's `GetNextNumbers` then VB's `Val`: from `at`, the run of digits and spaces (a `*`
 *  restarts it), its spaces dropped. Empty when there is none. */
export function nextNumber(text: string, at: number): string {
  let start = at;
  let y = at;
  for (; y < text.length; y++) {
    const c = text[y]!;
    if (c === '*') start = y + 1;
    else if (!/[0-9 ]/.test(c)) break;
  }
  if (y <= start) return '';
  const digits = text.slice(start, y).replace(/ /g, '');
  return digits === '' ? '0' : String(Number(digits));
}

/** The text after `label` up to `next` (when it comes within 15 characters, on the same
 *  line) or the line's end, trimmed: NMR's race and class names. */
function nameAfter(text: string, label: string, next: string): string {
  const i = text.indexOf(label);
  if (i < 0) return '';
  const x = i + label.length;
  let y = text.indexOf(next, x);
  if (y > x + 15 || (y >= 0 && text.slice(x, y).trim().includes('\n'))) y = -1;
  if (y < 0) y = text.indexOf('\r', x);
  if (y < 0) y = text.indexOf('\n', x);
  return y > x ? text.slice(x, y).trim() : '';
}

export interface PastedStats {
  /** Column values, as NMR fills its fields. */
  values: Record<string, string>;
  race: string;
  cls: string;
}

/** Reads a stat capture as NMR does: each label's first number, where it is at least 1
 *  (Exp at least 0); each stat sets both its base and current value. */
export function parseStats(text: string): PastedStats {
  const values: Record<string, string> = {};
  const take = (label: string, columns: string[], min = 1) => {
    const i = text.indexOf(label);
    if (i < 0) return;
    const n = nextNumber(text, i + label.length);
    if (n === '' || Number(n) < min) return;
    for (const c of columns) values[c] = n;
  };
  for (const [label, stat] of [
    ['Intellect: ', 'INT'],
    ['Willpower: ', 'WIL'],
    ['Strength: ', 'STR'],
    ['Health: ', 'HEA'],
    ['Agility: ', 'AGL'],
    ['Charm: ', 'CHM'],
  ] as const) {
    take(label, [`Base ${stat}`, stat]);
  }
  const exp = text.indexOf('Exp: ');
  if (exp >= 0) {
    const n = nextNumber(text, exp + 5);
    if (n !== '') {
      // the record holds experience as billions and the rest
      const e = Number(n);
      values['Exp Billions'] = String(Math.floor(e / 1e9));
      values['Exp'] = String(e % 1e9);
    }
  }
  take('Level: ', ['Level']);
  take('Hits: ', ['HP']);
  take('Mana: ', ['Mana']);
  take('Kai: ', ['Mana']);
  take('Lives/CP:', ['Lives']);
  // CP: the number after the slash that follows the first "CP:"
  const cp = text.indexOf('CP:');
  if (cp >= 0) {
    const slash = text.indexOf('/', cp + 3);
    if (slash >= 0 && slash + 1 < text.length) {
      const n = nextNumber(text, slash + 1);
      if (n !== '') values['CP'] = n;
    }
  }
  return { values, race: nameAfter(text, 'Race: ', 'Exp:'), cls: nameAfter(text, 'Class: ', 'Level:') };
}

// ---- NMR's user Export and Import (frmUser's cmdUserExport and ImportFromClipboard) ----
// The text is `Key:value` lines, lists comma separated and pairs `|` separated, so a
// character can move between NMR and Daydream, or between realms.

type Values = Record<string, string | number | null>;

const STATS = ['INT', 'WIL', 'STR', 'HEA', 'AGL', 'CHM'];
const STAT_COLUMNS = [...STATS.map((s) => `Base ${s}`), ...STATS];
const range = (n: number) => Array.from({ length: n }, (_, i) => i);

/** Single values: NMR's key, the column, and which import box it needs. */
const SINGLES: [string, string, ('name' | 'gang' | 'deets' | 'rooms')?][] = [
  ['BBSName', 'BBS Name'],
  ['FirstName', 'First Name', 'name'],
  ['LastName', 'Last Name', 'name'],
  ['Race', 'Race'],
  ['Class', 'Class'],
  ['Experience', ''],
  ['Level', 'Level'],
  ['Title', 'Title'],
  ['EvilPoints', 'Evil Points'],
  ['GangName', 'Gang', 'gang'],
  ['SuicidePassword', 'Suicide Password', 'deets'],
  ['BroadcastChan', 'Broadcast Channel', 'deets'],
  ['bEDITED', 'Edited'],
  ['Runic', 'Runic', 'deets'],
  ['Platinum', 'Platinum', 'deets'],
  ['Gold', 'Gold', 'deets'],
  ['Silver', 'Silver', 'deets'],
  ['Copper', 'Copper', 'deets'],
  ['MaxHP', 'Max HP'],
  ['CurrentHP', 'HP'],
  ['HPRolls', 'HP Rolls'],
  ['MaxMana', 'Max Mana'],
  ['CurrentMana', 'Mana'],
  ['SpellCasting', 'Spellcasting'],
  ['LivesRemaining', 'Lives', 'deets'],
  ['CPRemaining', 'CP'],
  ['Perception', 'Perception'],
  ['Thievery', 'Thievery'],
  ['Traps', 'Traps'],
  ['Picklocks', 'Picklocks'],
  ['Tracking', 'Tracking'],
  ['MartialArts', 'Martial Arts'],
  ['MagicRes', 'Max MR'],
  ['Stealth', 'Stealth'],
  ['CurrentENC', 'Weight'],
  ['MaxENC', 'Max Weight'],
  ['WeaponHand', 'Weapon'],
  ['RoomNum', 'Room Number', 'rooms'],
  ['MapNumber', 'Map Number', 'rooms'],
];

/** Lists: NMR's key and, for each entry, its columns. */
const LISTS: [string, number, (i: number) => string[], 'rooms'?][] = [
  ['STATS', 12, (i) => [STAT_COLUMNS[i]!]],
  ['ABILS', 30, (i) => [`Ability ${i}`, `Ability ${i} Value`]],
  ['WORN', 20, (i) => [`Worn ${i}`]],
  ['ITEMS', 100, (i) => [`Item ${i}`, `Item ${i} Uses`]],
  ['KEYS', 50, (i) => [`Key ${i}`, `Key ${i} Uses`]],
  ['SPELLS', 100, (i) => [`Spellbook ${i}`]],
  ['AURAS', 10, (i) => [`Spell Cast ${i}`, `Spell Cast ${i} Value`, `Spell Cast ${i} Rounds`]],
  ['ROOMS', 20, (i) => [`Trail ${i} Map`, `Trail ${i} Room`], 'rooms'],
];

/** NMR's Export, in its order (lists after the singles they follow in NMR's text). */
export function exportUser(v: Values): string {
  const s = (c: string) => String(v[c] ?? 0);
  const exp = BigInt(Math.trunc(Number(v['Exp Billions'] ?? 0))) * 1_000_000_000n + BigInt(Math.trunc(Number(v['Exp'] ?? 0)));
  const lines: string[] = [];
  const list = (key: string) => {
    const [, n, cols] = LISTS.find(([k]) => k === key)!;
    lines.push(`${key}:${range(n).map((i) => cols(i).map(s).join('|')).join(',')}`);
  };
  for (const [key, col] of SINGLES) {
    if (key === 'CurrentENC') list('STATS'), list('ABILS');
    if (key === 'RoomNum') list('WORN'), list('ITEMS'), list('KEYS'), list('SPELLS'), list('AURAS');
    lines.push(`${key}:${key === 'Experience' ? exp.toString() : s(col).split('\0')[0]}`);
  }
  list('ROOMS');
  return lines.join('\r\n');
}

export interface ImportOptions {
  /** "Import Character Name" */
  name: boolean;
  /** "Import Gang Name" */
  gang: boolean;
  /** "Import Lives, Suicide, Channel, Cash" */
  deets: boolean;
  /** "Import Rooms" */
  rooms: boolean;
  clearItems: boolean;
  clearKeys: boolean;
  clearSpells: boolean;
  clearAbils: boolean;
}

/** NMR's defaults: clear items and spells first, nothing else. */
export const IMPORT_DEFAULTS: ImportOptions = {
  name: false, gang: false, deets: false, rooms: false,
  clearItems: true, clearKeys: false, clearSpells: true, clearAbils: false,
};

/** NMR's Import: the columns it sets. BBS Name is never taken (it is the record's key). */
export function importUser(text: string, o: ImportOptions): Record<string, string> {
  const out: Record<string, string> = {};
  const zero = (key: string) => {
    const [, n, cols] = LISTS.find(([k]) => k === key)!;
    for (const i of range(n)) for (const c of cols(i)) out[c] = '0';
  };
  if (o.clearItems) zero('ITEMS'), zero('WORN');
  if (o.clearKeys) zero('KEYS');
  if (o.clearSpells) zero('SPELLS'), zero('AURAS');
  if (o.clearAbils) zero('ABILS');
  const allowed = (need?: string) => !need || o[need as 'name' | 'gang' | 'deets' | 'rooms'];
  for (const line of text.trim().split(/\r?\n/)) {
    const m = /^([^\r\n:]+):(.*)$/.exec(line);
    if (!m) continue;
    const [, key, raw] = m as unknown as [string, string, string];
    const value = raw.trim();
    const single = SINGLES.find(([k]) => k === key);
    if (single && key !== 'BBSName') {
      const [, col, need] = single;
      if (!allowed(need)) continue;
      if (key === 'Experience') {
        const e = /^\d+$/.test(value) ? BigInt(value) : 0n;
        out['Exp Billions'] = (e / 1_000_000_000n).toString();
        out['Exp'] = (e % 1_000_000_000n).toString();
      } else out[col] = value;
      continue;
    }
    const list = LISTS.find(([k]) => k === key);
    if (!list) continue;
    const [, n, cols, need] = list;
    if (!allowed(need)) continue;
    value.split(',').slice(0, n).forEach((entry, i) => {
      const parts = entry.split('|');
      const want = cols(i);
      // as NMR's: a pair (or triple) that is not whole is passed over
      if (want.length > 1 && parts.length !== want.length) return;
      want.forEach((c, j) => (out[c] = parts[j]!.trim()));
    });
  }
  return out;
}
