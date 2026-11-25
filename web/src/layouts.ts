// How each editor lays out its table's columns: tabs as NMR's forms have them, fields
// labelled as NMR labels them. A table without a layout here gets `defaultLayout`: one
// tab of its single fields, and a tab per family of numbered columns, shown as a grid.

import type { TableInfo } from './api';
import FIELDS from '../../crates/daydream-nmr/src/fields.json';

/** A choice list: value to label. */
export type Options = Record<number, string>;

/** How a field picks its value besides typing it. */
export type Pick =
  | { options: Options }
  /** A record of another table, by number (0 is none). */
  | { ref: string }
  | { abilities: true };

export interface Field {
  col: string;
  label?: string;
  pick?: Pick;
  /** Wide text spans the row. */
  wide?: boolean;
}

/** Numbered columns as a grid: one row per number, one column per template ('#' is the number). */
export interface Grid {
  columns: { template: string; label?: string; pick?: Pick }[];
  from: number;
  to: number;
  /** One row per template and one column per number, headed with this and the number. */
  across?: string;
  /** Names for the numbers, from `from` on (exit directions). */
  names?: string[];
}

/** Columns that pair up as a small table: a row per label, a cell per header. */
export interface Matrix {
  headers: string[];
  rows: { label: string; cols: string[] }[];
}

export interface Tab {
  title: string;
  fields?: Field[];
  grid?: Grid;
  matrix?: Matrix;
  /** Columns shown below, as a terminal shows them (ANSI colours), one after another. */
  preview?: string[];
}

export type Layout = Tab[];

// NMR's labels for numbers (modMMUDFunctions: GetItemType, GetWornType, ...) and the lists of
// its forms' combo boxes, kept with the server's column notes in crates/daydream-nmr/src/fields.json
export const ITEM_TYPE: Options = FIELDS.lists.ITEM_TYPE;
export const WORN: Options = FIELDS.lists.WORN;
export const WEAPON: Options = FIELDS.lists.WEAPON;
export const CLASS_WEAPON: Options = FIELDS.lists.CLASS_WEAPON;
export const ARMOUR: Options = FIELDS.lists.ARMOUR;
export const COST_TYPE: Options = FIELDS.lists.COST_TYPE;
export const MAGERY: Options = FIELDS.lists.MAGERY;
export const CLASS_ARMOUR: Options = FIELDS.lists.CLASS_ARMOUR;
export const CLASS_WEAPONS: Options = FIELDS.lists.CLASS_WEAPONS;
export const MONSTER_GROUP: Options = FIELDS.lists.MONSTER_GROUP;
export const MONSTER_TYPE: Options = FIELDS.lists.MONSTER_TYPE;
export const ALIGNMENT: Options = FIELDS.lists.ALIGNMENT;
export const GENDER: Options = FIELDS.lists.GENDER;
export const ATTACK_TYPE: Options = FIELDS.lists.ATTACK_TYPE;
export const SPELL_TYPE: Options = FIELDS.lists.SPELL_TYPE;
export const TARGET: Options = FIELDS.lists.TARGET;
export const RESIST_ABILITY: Options = FIELDS.lists.RESIST_ABILITY;
export const ELEMENT: Options = FIELDS.lists.ELEMENT;
export const RESISTS: Options = FIELDS.lists.RESISTS;
export const YES_NO: Options = FIELDS.lists.YES_NO;
export const ROOM_TYPE: Options = FIELDS.lists.ROOM_TYPE;
export const EXIT_TYPE: Options = FIELDS.lists.EXIT_TYPE;
export const SHOP_TYPE: Options = FIELDS.lists.SHOP_TYPE;
/** Exit directions by exit number (GetFriendlyRoomExit). */
export const EXITS = ['N', 'S', 'E', 'W', 'NE', 'NW', 'SE', 'SW', 'U', 'D'];

const f = (col: string, label?: string, pick?: Pick): Field => ({ col, label, pick });
const wide = (col: string, label?: string): Field => ({ col, label, wide: true });
const abilities = (from: number, to: number): Grid => ({
  columns: [
    { template: 'Ability #', label: 'Ability' },
    { template: 'Ability Value #', label: 'Value' },
  ],
  from,
  to,
});

export const LAYOUTS: Record<string, Layout> = {
  // frmItem
  Items: [
    {
      title: 'General',
      fields: [
        f('Number'), f('Name'), f('Type', 'Type'),
        f('Worn On', 'Worn'), f('Armour', 'Armour'),
        f('Weapon', 'Weapon'), f('Cost'),
        f('Cost Type', 'Cost Type'), f('Game Limit'), f('Uses', 'Max Uses'),
        f('Weight'), f('Min Hit'), f('Max Hit'), f('Accuracy'), f('Speed'), f('AC'), f('DR'),
        f('Req Str', 'Req STR'), f('Gettable', 'Getable'), f('Robable'), f('Not Droppable'),
        f('Destroy On Death', 'Destroys On Death'), f('Retain After Uses', 'Retain After Uses Expire'),
      ],
    },
    {
      title: 'Desc/Msg/Cash',
      fields: [
        wide('Desc1', 'Description'), wide('Desc2', ''), wide('Desc3', ''), wide('Desc4', ''),
        wide('Desc5', ''), wide('Desc6', ''),
        f('Hit Msg', 'Hit (msg)'), f('Miss Msg', 'Miss (msg)'),
        f('Read Msg', 'Read (tb)'), f('Distruct Msg', 'Destrct (msg)'),
        f('OpenRunic', 'Runic'), f('OpenPlatinum', 'Platinum'), f('OpenGold', 'Gold'),
        f('OpenSilver', 'Silver'), f('OpenCopper', 'Copper'),
      ],
    },
    {
      title: 'Race/Class/Negate',
      grid: {
        columns: [
          { template: 'Class #', label: 'Class Restrictions' },
          { template: 'Race #', label: 'Race Restrictions' },
          { template: 'Negate #', label: 'Negate Spells' },
        ],
        from: 0,
        to: 9,
      },
    },
    { title: 'Abilities p1', grid: abilities(0, 9) },
    { title: 'Abilities p2', grid: abilities(10, 19) },
  ],

  // frmMonster
  Monsters: [
    {
      title: 'General',
      fields: [
        f('Number'), f('Name'), f('Group', 'Group'), f('Index'),
        f('Type', 'Type'), f('Alignment', 'Alignment'),
        f('Gender', 'Gender'), f('Active'), f('Undead'), f('Experience'),
        f('Exp Multiplier', 'Multiplier'), f('Hit Points', 'Hitpoints'), f('HP Regen'), f('Energy'),
        f('AC'), f('DR'), f('MR', 'M.R.'), f('Follow', 'Follow %'), f('Charm LvL', 'Charm Lvl'),
        f('Charm RES', 'Charm Resist'), f('BS Defense'), f('Game Limit'), f('Regen Time'),
        f('Date Killed', 'Last Killed (date)'), f('Time Killed', 'Last Killed (time)'),
      ],
    },
    {
      title: 'Drop',
      fields: [f('Runic'), f('Platinum'), f('Gold'), f('Silver'), f('Copper')],
      grid: {
        columns: [
          { template: 'Item Number #', label: 'Item' },
          { template: 'Item Uses #', label: 'Uses' },
          { template: 'Item Drop % #', label: '%' },
        ],
        from: 0,
        to: 9,
      },
    },
    {
      title: 'Spells/Attacks',
      grid: {
        columns: [
          { template: 'Attack Type #', label: 'Type' },
          { template: 'Attack Accu/Spell #', label: 'Accu/Spell' },
          { template: 'Attack % #', label: 'Attack%' },
          { template: 'Attack Min Hit/Cast % #', label: 'MinH/Cast%' },
          { template: 'Attack Max Hit/Cast LVL #', label: 'MaxH/CastLvL' },
          { template: 'Attack Energy #', label: 'Energy' },
          { template: 'Attack Hit Msg #', label: 'Hit Msg' },
          { template: 'Attack Dodge Msg #', label: 'Dodge Msg' },
          { template: 'Attack Miss Msg #', label: 'Miss Msg' },
          { template: 'Attack Hit Spell #', label: 'Hit Spell' },
        ],
        from: 0,
        to: 4,
        across: 'Attack',
      },
    },
    {
      title: 'Between-Round Spells',
      grid: {
        columns: [
          { template: 'Spell Number #', label: 'Spell' },
          { template: 'Spell Cast % #', label: '%' },
          { template: 'Spell Cast LVL #', label: 'LvL' },
        ],
        from: 0,
        to: 4,
      },
    },
    {
      title: 'Weapon/Txt/Msg',
      fields: [
        f('Weapon Number', 'Weapon'), f('Death Spell', 'Death Spell'),
        f('Create Spell', 'Create Spell'), f('Move Msg', 'Move Msg'),
        f('Death Msg', 'Death Msg'), f('Greet Txt'), f('Desc Txt'), f('Talk Txt'),
        wide('Desc 1', 'Description'), wide('Desc 2', ''), wide('Desc 3', ''), wide('Desc 4', ''),
      ],
    },
    { title: 'Abilities', grid: abilities(0, 9) },
  ],
  // frmSpell
  Spells: [
    {
      title: 'General',
      fields: [
        f('Number'), f('Name'), f('Short Name'), f('Level', 'Req. Level'), f('Level Cap'),
        f('Magery A', 'Magery'), f('Magery B', 'Magery Level'),
        f('Spell Type', 'Spell Type'), f('Target', 'Target'),
        f('Attack Type', 'Type of Attack'),
        f('Type of Resists', 'Type of Resists'),
        f('Resist Ability', 'Resist Ability'), f('Difficulty'),
        f('Energy'), f('Mana'), f('Min', 'Base Min'), f('Max', 'Base Max'), f('Duration', 'Base Duration'),
        f('Min Increase', 'Min increase'), f('LVLS Min Increase', 'LVLs/Min increase'),
        f('Max Increase', 'Max increase'), f('LVLS Max Increase', 'LVLs/Max increase'),
        f('Dur Increase', 'Dur. increase'), f('LVLS Dur Increase', 'LVLs/Dur. increase'),
        f('MSG Style', 'Norm Msg Style'),
        f('Cast MSG A', 'Cast Msg A'), f('Cast MSG B', 'Cast Msg B'),
        wide('Desc 1', 'Description Line 1'), wide('Desc 2', 'Description Line 2'),
        f('UNDEFINED01', 'unknown 1'), f('UNDEFINED02', 'unknown 2'),
      ],
    },
    { title: 'Abilities', grid: abilities(0, 9) },
  ],
  // frmClass
  Classes: [
    {
      title: 'General',
      fields: [
        f('Number'), f('Name'), f('EXP %', 'Exp %'), f('Min HP', 'HP (min)'), f('Max HP', 'HP (max)'),
        f('Armour', 'Armour'), f('Weapon', 'Weapons'),
        f('Combat'), f('Magic Type', 'Magery'), f('Magic LVL', 'Magery Level'),
        f('Title Text'),
      ],
    },
    { title: 'Abilities', grid: abilities(0, 9) },
  ],
  // frmRace
  Races: [
    {
      title: 'General',
      fields: [f('Number'), f('Name'), f('EXP %', 'Exp %'), f('CP', 'Starting Cp'), f('HP Bonus', 'Hp Bonus')],
      matrix: {
        headers: ['Min', 'Max'],
        rows: [
          { label: 'Strength', cols: ['Min STR', 'Max STR'] },
          { label: 'Agility', cols: ['Min AGL', 'Max AGL'] },
          { label: 'Intellect', cols: ['Min INT', 'Max INT'] },
          { label: 'Health', cols: ['Min HEA', 'Max HEA'] },
          { label: 'Willpower', cols: ['Min WIL', 'Max WIL'] },
          { label: 'Charm', cols: ['Min CHM', 'Max CHM'] },
        ],
      },
    },
    { title: 'Abilities', grid: abilities(0, 9) },
  ],
  // frmShop
  Shops: [
    {
      title: 'General',
      fields: [
        f('Number'), f('Name'), f('Type', 'Type'), f('Min Lvl', 'Min LVL'),
        f('Max Lvl', 'Max LVL'), f('MarkUp', 'Markup %'), f('Class Limit', 'Class'),
        wide('Desc A', 'Description'), wide('Desc B', ''), wide('Desc C', ''),
      ],
    },
    {
      title: 'Items Sold',
      grid: {
        columns: [
          { template: 'Item #', label: 'Item' },
          { template: 'Max #', label: 'Max' },
          { template: 'Normal #', label: 'Normal' },
          { template: 'Regen Time #', label: 'Time' },
          { template: 'Regen Number#', label: 'Rgn#' },
          { template: 'Regen %#', label: 'Rgn%' },
        ],
        from: 0,
        to: 19,
      },
    },
  ],
  // frmRoom
  Rooms: [
    {
      title: 'General',
      fields: [
        f('Map Number'), f('Room Number'), wide('Name'), wide('Desc 0', 'Description'), wide('Desc 1', ''),
        wide('Desc 2', ''), wide('Desc 3', ''), wide('Desc 4', ''), wide('Desc 5', ''), wide('Desc 6', ''),
        f('Type', 'Type'), f('Shop Number', 'Shop'),
        f('Gang House Number', 'Gang House'), f('Light'), f('AnsiMap', 'ANSI Map'), f('Attributes'),
      ],
    },
    {
      title: 'Exits',
      grid: {
        columns: [
          { template: 'Exit #', label: 'To Room' },
          { template: 'Type #', label: 'Type' },
          { template: 'Para1 #', label: 'Para 1' },
          { template: 'Para2 #', label: 'Para 2' },
          { template: 'Para3 #', label: 'Para 3' },
          { template: 'Para4 #', label: 'Para 4' },
        ],
        from: 0,
        to: 9,
        names: EXITS,
      },
    },
    {
      title: 'NPC / Items / Coins',
      fields: [
        f('Perm NPC', 'Perm NPC'), f('Mon Type', 'Monster Type'),
        f('Min Index'), f('Max Index'), f('Max Regen'), f('Max Area'), f('Runic'), f('Platinum'),
        f('Gold'), f('Silver'), f('Copper'), f('InvisRunic', 'Hidden Runic'),
        f('InvisPlatinum', 'Hidden Platinum'), f('InvisGold', 'Hidden Gold'),
        f('InvisSilver', 'Hidden Silver'), f('InvisCopper', 'Hidden Copper'),
      ],
      grid: { columns: [{ template: 'Placed Item #', label: 'Placed Item' }], from: 0, to: 9 },
    },
    {
      title: 'Visible Items',
      grid: {
        columns: [
          { template: 'Room Item #', label: 'Item' },
          { template: 'Room Item # USES', label: 'Uses' },
          { template: 'Room Item # QTY', label: 'Qty' },
        ],
        from: 0,
        to: 16,
      },
    },
    {
      title: 'Hidden Items',
      grid: {
        columns: [
          { template: 'Hidden Item #', label: 'Item' },
          { template: 'Hidden Item # USES', label: 'Uses' },
          { template: 'Hidden Item # QTY', label: 'Qty' },
        ],
        from: 0,
        to: 14,
      },
    },
    {
      title: 'Other',
      fields: [
        f('Control Room'), f('Death Room'), f('Exit Room'), f('Command Text'), f('Delay'),
        f('Spell', 'Spell'),
      ],
    },
  ],
  // frmMessage
  Messages: [
    {
      title: 'General',
      fields: [f('Number'), wide('Line 1'), wide('Line 2'), wide('Line 3')],
      preview: ['Line 1', 'Line 2', 'Line 3'],
    },
  ],
  // frmTextblock
  Textblocks: [
    {
      title: 'General',
      fields: [f('Number'), f('Part #', 'Part'), f('Link To', 'Link To'), wide('Data', 'Text')],
      preview: ['Data'],
    },
  ],
  // frmAction
  Actions: [
    {
      title: 'General',
      fields: [
        f('Action'), wide('Single to User'), wide('Single to Room'), wide('User to User'),
        wide('User to Other User'), wide('User to Room'), wide('Monster to User'),
        wide('Monster to Room'), wide('Inventory to User'), wide('Inventory to Room'),
        wide('Floor Item to User'), wide('Floor Item to Room'),
      ],
    },
  ],

  // frmUser: NMR's tabs, the Misc tab's spells cast on a tab of their own
  Users: [
    {
      title: 'Stats',
      fields: [
        f('BBS Name'), f('First Name'), f('Last Name'), f('Title'),
        f('Race', 'Race'), f('Class', 'Class'), f('Level'),
        f('Exp Billions', 'Exp (billions)'), f('Exp'), f('Lives'), f('CP'), f('Total CP'),
        f('Max HP'), f('HP', 'Current HP'), f('Max Mana'), f('Mana', 'Current Mana'),
        f('HP Bonus'), f('Mana Bonus'), f('Max MR', 'Max M.R.'), f('MR', 'M.R.'),
        f('Max Energy'), f('Energy'), f('Max Weight', 'Max Enc.'), f('Weight', 'Current Enc.'),
      ],
      matrix: {
        headers: ['Base', 'Current'],
        rows: ['INT', 'WIL', 'STR', 'HEA', 'AGL', 'CHM'].map((s) => ({ label: s, cols: [`Base ${s}`, s] })),
      },
    },
    {
      title: 'Inven.',
      fields: [f('Weapon', 'Weapon (readied)')],
      grid: {
        columns: [
          { template: 'Item #', label: 'Item' },
          { template: 'Item # Uses', label: 'Uses' },
        ],
        from: 0,
        to: 99,
      },
    },
    {
      title: 'Keys',
      grid: {
        columns: [
          { template: 'Key #', label: 'Key' },
          { template: 'Key # Uses', label: 'Uses' },
        ],
        from: 0,
        to: 49,
      },
    },
    {
      title: 'Spellbk.',
      grid: { columns: [{ template: 'Spellbook #', label: 'Spell' }], from: 0, to: 99 },
    },
    {
      title: 'Abilities',
      grid: {
        columns: [
          { template: 'Ability #', label: 'Ability' },
          { template: 'Ability # Value', label: 'Value' },
        ],
        from: 0,
        to: 29,
      },
    },
    {
      title: 'Rooms',
      fields: [f('Map Number', 'Current Map'), f('Room Number', 'Current Room')],
      grid: {
        columns: [
          { template: 'Trail # Map', label: 'Map' },
          { template: 'Trail # Room', label: 'Room' },
        ],
        from: 0,
        to: 19,
      },
    },
    {
      title: 'Worn',
      grid: {
        columns: [
          { template: 'Worn #', label: 'Item' },
          { template: 'Worn # Uses', label: 'Uses' },
        ],
        from: 0,
        to: 19,
      },
    },
    {
      title: 'Misc',
      fields: [
        f('Runic'), f('Platinum'), f('Gold'), f('Silver'), f('Copper'),
        f('Perception'), f('Stealth'), f('Martial Arts'), f('Thievery'), f('Spellcasting'),
        f('Traps'), f('Disarm'), f('Picklocks'), f('Tracking'), f('Evil Points'),
        f('Broadcast Channel', 'BC Channel'), f('Gang'), f('Suicide Password'),
      ],
    },
    {
      title: 'Spells Cast',
      fields: [f('Poison', 'Poison flag')],
      grid: {
        columns: [
          { template: 'Spell Cast #', label: 'Spell' },
          { template: 'Spell Cast # Value', label: 'Value' },
          { template: 'Spell Cast # Rounds', label: 'Rounds Left' },
        ],
        from: 0,
        to: 9,
      },
    },
  ],

  // frmBank
  BankBooks: [{ title: 'General', fields: [f('BBS Name'), f('Shop #', 'Bank (shop)'), f('Amount'), f('Loan')] }],

  // frmGang
  Gangs: [
    {
      title: 'General',
      fields: [
        f('KeyName'), f('Display Name'), f('Leader'), f('Base Exp'), f('Roll Over Exp'),
        f('Times Rolled'), f('Members'), f('Date Created'),
      ],
    },
  ],
};

/** What a column of `table` picks from, as fields.json says (`#` stands for its number in a
 * grid template). */
export function pickFor(table: string, col: string): Pick | undefined {
  const c = (FIELDS.tables as Record<string, { columns: Record<string, Record<string, unknown>> }>)[table]
    ?.columns[col];
  if (!c) return undefined;
  if (typeof c.options === 'string') return { options: (FIELDS.lists as Record<string, Options>)[c.options] };
  if (typeof c.ref === 'string') return { ref: c.ref };
  if (c.abilities === true) return { abilities: true };
  return undefined;
}

// the editors' picks come from fields.json, so the server describes the same columns
for (const [table, tabs] of Object.entries(LAYOUTS)) {
  for (const tab of tabs) {
    for (const fd of tab.fields ?? []) fd.pick = pickFor(table, fd.col);
    for (const gc of tab.grid?.columns ?? []) gc.pick = pickFor(table, gc.template);
  }
}

/** The numbered-column families of a table: template ("Ability #") to numbers. */
function families(t: TableInfo): { single: string[]; grids: Grid[] } {
  const single: string[] = [];
  const byTemplate = new Map<string, number[]>();
  for (const c of t.columns) {
    const m = /^(.*?)(\d+)(\D*)$/.exec(c.name);
    if (!m || t.key.includes(c.name)) {
      single.push(c.name);
      continue;
    }
    const template = `${m[1]}#${m[3]}`;
    const list = byTemplate.get(template) ?? [];
    list.push(Number(m[2]));
    byTemplate.set(template, list);
  }
  // templates with the same numbers make one grid
  const grids = new Map<string, Grid>();
  for (const [template, numbers] of byTemplate) {
    if (numbers.length < 3) {
      for (const n of numbers) single.push(template.replace('#', String(n)));
      continue;
    }
    const from = Math.min(...numbers);
    const to = Math.max(...numbers);
    const id = `${from}-${to}`;
    const g = grids.get(id) ?? { columns: [], from, to };
    g.columns.push({
      template,
      label: template.replace('#', '').replace(/\s+/g, ' ').trim(),
      pick: /^Ability #$/.test(template) ? { abilities: true } : undefined,
    });
    grids.set(id, g);
  }
  // keep the table's own column order for the single fields
  const order = new Map(t.columns.map((c, i) => [c.name, i]));
  single.sort((a, b) => (order.get(a) ?? 0) - (order.get(b) ?? 0));
  return { single, grids: [...grids.values()] };
}

/** One tab of single fields, one per grid. */
export function defaultLayout(t: TableInfo): Layout {
  const { single, grids } = families(t);
  const tabs: Layout = [
    {
      title: 'General',
      fields: single.map((col) => {
        const info = t.columns.find((c) => c.name === col);
        return { col, wide: info?.type === 'memo' || (info?.size ?? 0) > 40 };
      }),
    },
  ];
  for (const g of grids) {
    const first = g.columns[0]?.label ?? 'List';
    tabs.push({ title: g.columns.length > 2 ? `${first} ...` : g.columns.map((c) => c.label).join(' / '), grid: g });
  }
  return tabs;
}

export function layoutFor(t: TableInfo): Layout {
  return LAYOUTS[t.name] ?? defaultLayout(t);
}

/** Every column a layout shows, so the rest can go on a last tab. */
export function columnsShown(layout: Layout): Set<string> {
  const shown = new Set<string>();
  for (const tab of layout) {
    for (const field of tab.fields ?? []) shown.add(field.col);
    for (const row of tab.matrix?.rows ?? []) for (const c of row.cols) shown.add(c);
    if (tab.grid) {
      for (let n = tab.grid.from; n <= tab.grid.to; n++) {
        for (const c of tab.grid.columns) shown.add(c.template.replace('#', String(n)));
      }
    }
  }
  return shown;
}
