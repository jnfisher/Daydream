// NMR's Paste Character (frmUser's PasteCharacter): a capture of the game's "stat" output,
// or MMUD Explorer's "Copy Only Stats", read into the user editor's fields.

/** NMR's user tools, as a user's card offers them (UserTools.svelte). */
export type UserTool = 'stats' | 'items' | 'spells' | 'import' | 'export' | 'revive' | 'copy';

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

// ---- NMR's Paste Items/Keys/Worn/$$ and Paste Spellbook (frmUser's PasteItems and
// PasteSpells): a capture of the game's inventory or spellbook, read into the user's lists.

/** The characters NMR keeps from an inventory capture (`TestPasteChar`). */
const PASTE_CHAR = /[a-z()\-_,: '".`0-9]/i;

/** Where NMR puts an item named with its place: worn slots 0 to 14 (a second ring in 10),
 *  and 15 for the weapon. */
const EQUIP: Record<string, number> = {
  HEAD: 0, EARS: 1, NECK: 2, BACK: 3, TORSO: 4, ARMS: 5, WRIST: 6, WAIST: 7,
  HANDS: 8, FINGER: 9, LEGS: 11, FEET: 12, WORN: 13, 'OFF-HAND': 14,
  WEAPONHAND: 15, TWOHANDED: 15,
};

const COINS: [string, string][] = [
  ['platinumpieces', 'Platinum'],
  ['goldcrowns', 'Gold'],
  ['silvernobles', 'Silver'],
  ['copperfarthings', 'Copper'],
];

export interface PastedInventory {
  /** Item names as NMR compares them (spaces dropped), each with the amount before it
   *  ('' for none), in the capture's order. */
  items: { name: string; amount: string }[];
  /** The 16 places' item names ('' for none). */
  equipped: string[];
  /** Coin columns and their amounts. */
  coins: Record<string, string>;
}

/** Reads an inventory capture as NMR's PasteItems does, character by character: spaces
 *  dropped, a name ends at `,` or `.` (or at "is equipped with:", "are carrying",
 *  "following keys:" or "wealth:"), and `name (Place)` puts it in that place. */
export function parseInventory(text: string): PastedInventory {
  const out: PastedInventory = { items: [], equipped: Array(16).fill(''), coins: {} };
  let t = '';
  let amount = '';
  let paren = -1;
  const store = () => {
    const rest = t.slice(amount.length);
    const coin = COINS.find(([c]) => c === rest);
    if (coin) out.coins[coin[1]] = amount;
    else if (rest.startsWith('runiccoin')) out.coins['Runic'] = amount;
    else if (amount !== '' && t !== '') out.items.push({ name: rest, amount });
    else out.items.push({ name: t, amount: '' });
    clear();
  };
  const clear = () => {
    t = '';
    amount = '';
    paren = -1;
  };
  for (let p = 0; p < text.length; p++) {
    const c = text[p]!;
    if (!PASTE_CHAR.test(c)) continue;
    t = (t + c).replace(/ /g, '');
    const marker = ['isequippedwith:', 'arecarrying', 'followingkeys:', 'wealth:'].find((m) => t.toLowerCase().includes(m));
    if (marker) {
      t = t.slice(0, t.length - marker.length);
      store();
      continue;
    }
    // an amount comes before the name, not within it
    if (/[0-9]/.test(c) && t.length === amount.length + 1) amount += c;
    if (c === ',' || c === '.') {
      t = t.slice(0, -1);
      store();
      continue;
    }
    if (c === '(') paren = t.length;
    else if (c === ')') {
      if (paren >= 0) {
        const place = EQUIP[t.slice(paren, t.length - 1).toUpperCase()];
        const name = t.slice(0, paren - 1);
        if (place === 9 && out.equipped[9] !== '') out.equipped[10] = name;
        else if (place !== undefined) out.equipped[place] = name;
      }
      clear();
      continue;
    }
    if (p === text.length - 1) store();
  }
  return out;
}

/** An item or spell as the paste matches it: its number, name, and for items type and uses. */
export interface Named {
  number: number;
  name: string;
  type?: number;
  uses?: number;
}

/** Names the paste matched to more than one record: NMR asks which; the first is taken
 *  until another is chosen. */
export interface Ambiguous {
  name: string;
  numbers: number[];
}

const num = (v: Values, c: string) => Number(v[c] ?? 0) || 0;

/** NMR's PasteItems after the reading: with `clear`, items, keys and worn items are
 *  cleared first (NMR's Clear buttons: item uses -1, key uses -2); else what is found is
 *  added, and the worn list is cleared only when the paste names a place. Items are
 *  matched by name (spaces dropped, case kept) in `items`' order, the first of a name
 *  winning; a worn item or weapon is also put in the inventory. Answers the columns to
 *  set and the names more than one item has. */
export function placeInventory(p: PastedInventory, items: Named[], current: Values, clear: boolean): { values: Record<string, string>; ambiguous: Ambiguous[] } {
  const item = range(100).map((i) => num(current, `Item ${i}`));
  const itemUses = range(100).map((i) => num(current, `Item ${i} Uses`));
  const key = range(50).map((i) => num(current, `Key ${i}`));
  const keyUses = range(50).map((i) => num(current, `Key ${i} Uses`));
  let worn = range(20).map((i) => num(current, `Worn ${i}`));
  let weapon = num(current, 'Weapon');
  const equipped = [...p.equipped];
  if (clear) {
    item.fill(0), itemUses.fill(-1), key.fill(0), keyUses.fill(-2);
    worn = worn.map(() => 0);
    weapon = 0;
  } else if (equipped.some((e) => e !== '')) {
    worn = worn.map(() => 0);
    weapon = 0;
  }
  const names = p.items.map((i) => i.name);
  const found: [string, number[]][] = names.map(() => ['', []]);
  let curItem = 0;
  let curKey = 0;
  let curWorn = 0;
  const addItem = (it: Named) => {
    while (curItem < 100 && item[curItem] !== 0) curItem++;
    if (curItem >= 100) return;
    item[curItem] = it.number;
    if ((it.uses ?? 0) > 0) itemUses[curItem] = it.uses!;
    curItem++;
  };
  for (const it of items) {
    const name = it.name.split('\0')[0]!.replace(/ /g, '');
    if (name === '') continue;
    equipped.forEach((e, x) => {
      if (e !== name) return;
      if (x === 15) weapon = it.number;
      else if (curWorn < 20) worn[curWorn++] = it.number;
      addItem(it);
      equipped[x] = '';
    });
    names.forEach((n, x) => {
      if (n !== name) return;
      const times = Math.max(1, Math.trunc(Number(p.items[x]!.amount) || 0));
      for (let y = 0; y < times; y++) {
        if (it.type === 7) {
          while (curKey < 50 && key[curKey] !== 0) curKey++;
          if (curKey < 50) {
            key[curKey] = it.number;
            keyUses[curKey] = it.uses ?? 0;
            curKey++;
          }
        } else addItem(it);
      }
      found[x] = [name, [it.number]];
      names[x] = '';
    });
    const dup = found.find(([n, nums]) => n === name && nums[0] !== it.number);
    if (dup) dup[1].push(it.number);
  }
  const values: Record<string, string> = {};
  range(100).forEach((i) => ((values[`Item ${i}`] = String(item[i])), (values[`Item ${i} Uses`] = String(itemUses[i]))));
  range(50).forEach((i) => ((values[`Key ${i}`] = String(key[i])), (values[`Key ${i} Uses`] = String(keyUses[i]))));
  range(20).forEach((i) => (values[`Worn ${i}`] = String(worn[i])));
  values['Weapon'] = String(weapon);
  // NMR's CheckWornItems: a worn item or weapon not in the inventory comes off
  const has = (n: number) => item.includes(n);
  range(20).forEach((i) => worn[i]! > 0 && !has(worn[i]!) && (values[`Worn ${i}`] = '0'));
  if (weapon > 0 && !has(weapon)) values['Weapon'] = '0';
  Object.assign(values, p.coins);
  const ambiguous = found.filter(([, nums]) => nums.length > 1).map(([name, numbers]) => ({ name, numbers }));
  return { values, ambiguous };
}

/** NMR's choice between items of one name: every slot holding `from` (inventory, keys, worn,
 *  weapon) takes `to`, with `to`'s uses. */
export function chooseItem(values: Record<string, string>, from: number, to: Named) {
  for (const i of range(100)) if (values[`Item ${i}`] === String(from)) (values[`Item ${i}`] = String(to.number)), (values[`Item ${i} Uses`] = String(to.uses ?? 0));
  for (const i of range(50)) if (values[`Key ${i}`] === String(from)) (values[`Key ${i}`] = String(to.number)), (values[`Key ${i} Uses`] = String(to.uses ?? 0));
  for (const i of range(20)) if (values[`Worn ${i}`] === String(from)) values[`Worn ${i}`] = String(to.number);
  if (values['Weapon'] === String(from)) values['Weapon'] = String(to.number);
}

/** Reads a spellbook capture as NMR's PasteSpells does: each line past its first 17
 *  characters is a spell's name (what follows a header line's header counts as a line). */
export function parseSpellbook(text: string): string[] {
  const out: string[] = [];
  for (let line of text.split(/\r|\n/)) {
    // a header starts the line over
    for (const h of ['you have the following spells:', 'level mana short spell name']) {
      const at = line.toLowerCase().lastIndexOf(h);
      if (at >= 0) line = line.slice(at + h.length);
    }
    if (line.length > 17) out.push(line.slice(17));
  }
  return out;
}

/** NMR's PasteSpells after the reading: with `clear` the spellbook is emptied first; each
 *  name takes the first empty slot, the first spell of that name (in `spells`' order)
 *  winning. */
export function placeSpells(names: string[], spells: Named[], current: Values, clear: boolean): { values: Record<string, string>; ambiguous: Ambiguous[] } {
  const book = range(100).map((i) => (clear ? 0 : num(current, `Spellbook ${i}`)));
  const want = [...names];
  const found: [string, number[]][] = want.map(() => ['', []]);
  let cur = 0;
  for (const s of spells) {
    const name = s.name.split('\0')[0]!;
    if (name === '') continue;
    want.forEach((n, x) => {
      if (n !== name) return;
      while (cur < 100 && book[cur] !== 0) cur++;
      if (cur >= 100) return;
      book[cur++] = s.number;
      found[x] = [name, [s.number]];
      want[x] = '';
    });
    const dup = found.find(([n, nums]) => n === name && nums[0] !== s.number);
    if (dup) dup[1].push(s.number);
  }
  const values: Record<string, string> = {};
  range(100).forEach((i) => (values[`Spellbook ${i}`] = String(book[i])));
  const ambiguous = found.filter(([, nums]) => nums.length > 1).map(([name, numbers]) => ({ name, numbers }));
  return { values, ambiguous };
}
