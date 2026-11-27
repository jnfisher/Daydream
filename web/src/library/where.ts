// Where a record is used, in words: the rows that name it (GET /api/references) grouped by
// how they name it, each group said as a person would say it ("Dropped by", "Sold in").
//
// Pure: no DOM, so `node --test` runs its tests (where.test.ts).

export type Cell = number | string | null;

/** A row that names the record, as the server answers. */
export interface Reference {
  table: string;
  key: Cell[];
  title: Cell;
  /** The column that names it (`Item Number 3`); null for a room that spawns a monster. */
  column: string | null;
  /** How, for a person. */
  by: string;
}

export interface References {
  references: Reference[];
  /** More rows name it than came back. */
  more: boolean;
}

/** A row that names it, once however many of its columns do. */
export interface Named {
  table: string;
  key: Cell[];
  title: string;
  /** How it names it: the columns, or how a room spawns it. */
  by: string[];
}

export interface Group {
  /** What the rows are to the record (`Dropped by`). */
  says: string;
  /** The table the rows are of. */
  table: string;
  rows: Named[];
}

/** A column's family: its last number written `#` (`Item Number 3` is `Item Number #`). */
export function family(column: string): string {
  return column.replace(/(\d+)(?!.*\d)/, '#');
}

/**
 * How each table's columns name a record, by `TABLE FAMILY` (`spawn` for a room that spawns a
 * monster, `exit` for an exit's parameter), in the order the groups are listed. `{n}` is how
 * many rows, with their kind (`3 monsters`).
 */
const SAYS: [string, string][] = [
  ['Rooms spawn', 'Spawns in {n}'],
  ['Rooms Perm NPC', 'Always in {n}'],
  ['Shops Item #', 'Sold in {n}'],
  ['Monsters Weapon Number', 'Wielded by {n}'],
  ['Monsters Item Number #', 'Dropped by {n}'],
  ['Rooms Room Item #', 'Lying in {n}'],
  ['Rooms Hidden Item #', 'Hidden in {n}'],
  ['Rooms Placed Item #', 'Placed in {n}'],
  ['Rooms Exit #', 'Exits lead here from {n}'],
  ['Rooms Control Room', 'Caps the monsters of {n}'],
  ['Rooms Death Room', 'The dead of {n} come here'],
  ['Rooms Exit Room', 'The exit room of {n}'],
  ['Rooms Shop Number', 'The shop of {n}'],
  ['Rooms Spell', 'Cast in {n}'],
  ['Rooms Command Text', 'The commands of {n}'],
  ['Rooms exit', 'Used by exits of {n}'],
  ['Items Hit Msg', 'The hit message of {n}'],
  ['Items Miss Msg', 'The miss message of {n}'],
  ['Items Distruct Msg', 'The breaking message of {n}'],
  ['Items Read Msg', 'Read on {n}'],
  ['Items Negate #', 'Negated by {n}'],
  ['Items Class #', '{n} only it may use'],
  ['Items Race #', '{n} only it may use'],
  ['Monsters Attack Hit Msg #', 'Attack messages of {n}'],
  ['Monsters Attack Dodge Msg #', 'Attack messages of {n}'],
  ['Monsters Attack Miss Msg #', 'Attack messages of {n}'],
  ['Monsters Attack Hit Spell #', 'Cast on a hit by {n}'],
  ['Monsters Spell Number #', 'Cast between rounds by {n}'],
  ['Monsters Death Spell', 'Cast at the deaths of {n}'],
  ['Monsters Create Spell', 'Cast at the coming of {n}'],
  ['Monsters Move Msg', 'The moves of {n}'],
  ['Monsters Death Msg', 'The deaths of {n}'],
  ['Monsters Desc Txt', 'The names of {n}'],
  ['Monsters Greet Txt', 'The greeting of {n}'],
  ['Monsters Talk Txt', 'The talk of {n}'],
  ['Textblocks Link To', 'Linked to from {n}'],
  ['Spells Cast MSG A', 'The cast message of {n}'],
  ['Spells Cast MSG B', 'The cast message of {n}'],
  ['Shops Class Limit', '{n} only it may use'],
  ['BankBooks Shop #', '{n} kept here'],
  ['Users Weapon', 'Readied by {n}'],
  ['Users Item #', 'Carried by {n}'],
  ['Users Worn #', 'Worn by {n}'],
  ['Users Key #', 'A key of {n}'],
  ['Users Spellbook #', 'In the spellbooks of {n}'],
  ['Users Spell Cast #', 'On {n} now'],
  ['Users Race', '{n}'],
  ['Users Class', '{n}'],
];

const ORDER = new Map(SAYS.map(([k], i) => [k, i]));
const PHRASE = new Map(SAYS);

/** The group a reference goes in: how its table's column names the record. */
function groupKey(r: Reference): string {
  if (r.column === null) return `${r.table} spawn`;
  if (r.table === 'Rooms' && /^Para\d \d$/.test(r.column)) return 'Rooms exit';
  return `${r.table} ${family(r.column)}`;
}

const keyText = (key: Cell[]) => key.map((k) => String(k ?? '')).join('/');

/** How a table's records are counted: `1 monster`, `3 monsters`. */
export type Counted = (table: string, n: number) => string;

/**
 * The references in groups, in a fixed order (then the ones no phrase covers, by table), each
 * row once with every way it names the record.
 */
export function groupReferences(refs: readonly Reference[], counted: Counted): Group[] {
  const groups = new Map<string, { says: string; table: string; order: number; rows: Map<string, Named> }>();
  for (const r of refs) {
    const gk = groupKey(r);
    const says = PHRASE.get(gk) ?? `{n}, by ${r.column === null ? 'how they spawn' : family(r.column).replace(/ ?#/, '')}`;
    // groups that say the same of the same table are one (a monster's three attack messages)
    const id = `${r.table}\u0000${says}`;
    let g = groups.get(id);
    if (!g) groups.set(id, (g = { says, table: r.table, order: ORDER.get(gk) ?? SAYS.length, rows: new Map() }));
    const rk = keyText(r.key);
    const named = g.rows.get(rk);
    if (named) {
      if (!named.by.includes(r.by)) named.by.push(r.by);
    } else {
      g.rows.set(rk, { table: r.table, key: r.key, title: r.title === null ? '' : String(r.title), by: [r.by] });
    }
  }
  return [...groups.values()]
    .sort((a, b) => a.order - b.order || a.table.localeCompare(b.table) || a.says.localeCompare(b.says))
    .map(({ says, table, rows }) => ({ says: says.replace('{n}', counted(table, rows.size)), table, rows: [...rows.values()] }));
}
