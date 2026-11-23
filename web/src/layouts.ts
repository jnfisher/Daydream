// How each editor lays out its table's columns: tabs as NMR's forms have them, fields
// labelled as NMR labels them. A table without a layout here gets `defaultLayout`: one
// tab of its single fields, and a tab per family of numbered columns, shown as a grid.

import type { TableInfo } from './api';

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

// NMR's labels for numbers (modMMUDFunctions: GetItemType, GetWornType, ...)
export const ITEM_TYPE: Options = {
  0: 'Armour', 1: 'Weapon', 2: 'Projectile', 3: 'Sign', 4: 'Food', 5: 'Drink', 6: 'Light',
  7: 'Key', 8: 'Container', 9: 'Scroll', 10: 'Special',
};
export const WORN: Options = {
  0: 'Nowhere', 1: 'Everywhere', 2: 'Head', 3: 'Hands', 4: 'Finger', 5: 'Feet', 6: 'Arms',
  7: 'Back', 8: 'Neck', 9: 'Legs', 10: 'Waist', 11: 'Torso', 12: 'Off-Hand', 13: 'Finger',
  14: 'Wrist', 15: 'Ears', 16: 'Worn', 17: 'Wrist', 18: 'Eyes', 19: 'Face',
};
export const WEAPON: Options = { 0: '1H Blunt', 1: '2H Blunt', 2: '1H Sharp', 3: '2H Sharp' };
export const CLASS_WEAPON: Options = {
  ...WEAPON, 4: 'Any 1H', 5: 'Any 2H', 6: 'Any Sharp', 7: 'Any Blunt', 8: 'Any Weapon', 9: 'Staff',
};
export const ARMOUR: Options = {
  0: 'Natural', 1: 'Silk', 2: 'Ninja', 3: 'Leather', 4: 'Leather', 5: 'Leather', 6: 'Leather',
  7: 'Chainmail', 8: 'Scalemail', 9: 'Platemail',
};
export const COST_TYPE: Options = { 0: 'Copper', 1: 'Silver', 2: 'Gold', 3: 'Platinum', 4: 'Runic' };
export const MAGERY: Options = { 0: 'None', 1: 'Mage', 2: 'Priest', 3: 'Druid', 4: 'Bard', 5: 'Kai' };
// the lists of NMR's forms' combo boxes (their .frx), by list index
const list = (...labels: string[]): Options => Object.fromEntries(labels.map((l, i) => [i, l]));
export const CLASS_ARMOUR = list(
  'Natural', 'Robes', 'Padded', 'Soft Leather', 'Soft Studded Leather', 'Rigid Leather',
  'Studded Rigid Leather', 'Chainmail', 'Scalemail', 'Platemail',
);
export const CLASS_WEAPONS = list(
  '1H Blunt', '2H Blunt', '1H Sharp', '2H Sharp', 'Any 1H', 'Any 2H', 'Any Sharp', 'Any Blunt',
  'All Weapons', 'Limited (staff)',
);
export const MONSTER_GROUP = list(
  'Lair', 'Wanderer', 'NPC', 'Living', 'Random', 'Guard',
  ...Array.from({ length: 30 }, (_, i) => `Group ${i + 1}`),
  'Arena', 'Angel', 'Quest', 'Other',
);
export const MONSTER_TYPE = list('Solo', 'Leader', 'Follower', 'Stationary');
export const ALIGNMENT = list('Good', 'Evil', 'Chaotic Evil', 'Neutral', 'Lawful Good', 'Neutral Evil', 'Lawful Evil');
export const GENDER = list('It', 'He', 'She');
export const ATTACK_TYPE = list('None', 'Normal', 'Spell', 'Rob');
export const SPELL_TYPE = list('Auto-Combat', 'Poison', 'Change me', 'General');
export const TARGET = list(
  'Single', 'Self', 'Self or Single', 'Divided Area Not You', 'Monster', 'Divided Area Incl. You',
  'Any', 'Item', 'Monster or Single', 'Divided Attack Area', 'Divided Party Area', 'Full Area',
  'Full Attack Area', 'Full Party Area',
);
export const RESIST_ABILITY = list('Magic Resistance', 'Strength', 'Agility', 'Health', 'Intellect', 'Wisdom', 'Charm');
export const ELEMENT = list('Cold', 'Hot', 'Stone', 'Lightning', 'Normal', 'Water', 'Poison');
export const RESISTS = list('Never', 'Anti-Magic Only', 'Everyone');
export const YES_NO = list('Yes', 'No');
export const ROOM_TYPE = list('Normal', 'Shop', 'Arena', 'Lair', 'Hotel', 'Colliseum', 'Jail', 'Library');
export const EXIT_TYPE = list(
  'Normal', 'Spell', 'Key', 'Item', 'Toll', 'Action', 'Hidden', 'Door', 'map Change', 'Trap', 'Text',
  'Gate', 'Remote Action', 'Class', 'Race', 'Level', 'Timed', 'Ticket', 'User Count', 'Block Guard',
  'Alignment', 'Delay', 'Cast', 'Ability', 'Spell Trap',
);
/** Exit directions by exit number (GetFriendlyRoomExit). */
export const EXITS = ['N', 'S', 'E', 'W', 'NE', 'NW', 'SE', 'SW', 'U', 'D'];
export const SHOP_TYPE = list(
  'General', 'Weapons', 'Armour', 'Items', 'Spells', 'Hospital', 'Tavern', 'Bank', 'Training',
  'Inn', 'Specific', 'Gang Shop', 'Deed Shop',
);

const f = (col: string, label?: string, pick?: Pick): Field => ({ col, label, pick });
const wide = (col: string, label?: string): Field => ({ col, label, wide: true });
const abilities = (from: number, to: number): Grid => ({
  columns: [
    { template: 'Ability #', label: 'Ability', pick: { abilities: true } },
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
        f('Number'), f('Name'), f('Type', 'Type', { options: ITEM_TYPE }),
        f('Worn On', 'Worn', { options: WORN }), f('Armour', 'Armour', { options: ARMOUR }),
        f('Weapon', 'Weapon', { options: WEAPON }), f('Cost'),
        f('Cost Type', 'Cost Type', { options: COST_TYPE }), f('Game Limit'), f('Uses', 'Max Uses'),
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
        f('Hit Msg', 'Hit (msg)', { ref: 'Messages' }), f('Miss Msg', 'Miss (msg)', { ref: 'Messages' }),
        f('Read Msg', 'Read (tb)'), f('Distruct Msg', 'Destrct (msg)', { ref: 'Messages' }),
        f('OpenRunic', 'Runic'), f('OpenPlatinum', 'Platinum'), f('OpenGold', 'Gold'),
        f('OpenSilver', 'Silver'), f('OpenCopper', 'Copper'),
      ],
    },
    {
      title: 'Race/Class/Negate',
      grid: {
        columns: [
          { template: 'Class #', label: 'Class Restrictions', pick: { ref: 'Classes' } },
          { template: 'Race #', label: 'Race Restrictions', pick: { ref: 'Races' } },
          { template: 'Negate #', label: 'Negate Spells', pick: { ref: 'Spells' } },
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
        f('Number'), f('Name'), f('Group', 'Group', { options: MONSTER_GROUP }), f('Index'),
        f('Type', 'Type', { options: MONSTER_TYPE }), f('Alignment', 'Alignment', { options: ALIGNMENT }),
        f('Gender', 'Gender', { options: GENDER }), f('Active'), f('Undead'), f('Experience'),
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
          { template: 'Item Number #', label: 'Item', pick: { ref: 'Items' } },
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
          { template: 'Attack Type #', label: 'Type', pick: { options: ATTACK_TYPE } },
          { template: 'Attack Accu/Spell #', label: 'Accu/Spell' },
          { template: 'Attack % #', label: 'Attack%' },
          { template: 'Attack Min Hit/Cast % #', label: 'MinH/Cast%' },
          { template: 'Attack Max Hit/Cast LVL #', label: 'MaxH/CastLvL' },
          { template: 'Attack Energy #', label: 'Energy' },
          { template: 'Attack Hit Msg #', label: 'Hit Msg', pick: { ref: 'Messages' } },
          { template: 'Attack Dodge Msg #', label: 'Dodge Msg', pick: { ref: 'Messages' } },
          { template: 'Attack Miss Msg #', label: 'Miss Msg', pick: { ref: 'Messages' } },
          { template: 'Attack Hit Spell #', label: 'Hit Spell', pick: { ref: 'Spells' } },
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
          { template: 'Spell Number #', label: 'Spell', pick: { ref: 'Spells' } },
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
        f('Weapon Number', 'Weapon', { ref: 'Items' }), f('Death Spell', 'Death Spell', { ref: 'Spells' }),
        f('Create Spell', 'Create Spell', { ref: 'Spells' }), f('Move Msg', 'Move Msg', { ref: 'Messages' }),
        f('Death Msg', 'Death Msg', { ref: 'Messages' }), f('Greet Txt'), f('Desc Txt'), f('Talk Txt'),
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
        f('Magery A', 'Magery', { options: MAGERY }), f('Magery B', 'Magery Level'),
        f('Spell Type', 'Spell Type', { options: SPELL_TYPE }), f('Target', 'Target', { options: TARGET }),
        f('Attack Type', 'Type of Attack', { options: ELEMENT }),
        f('Type of Resists', 'Type of Resists', { options: RESISTS }),
        f('Resist Ability', 'Resist Ability', { options: RESIST_ABILITY }), f('Difficulty'),
        f('Energy'), f('Mana'), f('Min', 'Base Min'), f('Max', 'Base Max'), f('Duration', 'Base Duration'),
        f('Min Increase', 'Min increase'), f('LVLS Min Increase', 'LVLs/Min increase'),
        f('Max Increase', 'Max increase'), f('LVLS Max Increase', 'LVLs/Max increase'),
        f('Dur Increase', 'Dur. increase'), f('LVLS Dur Increase', 'LVLs/Dur. increase'),
        f('MSG Style', 'Norm Msg Style', { options: YES_NO }),
        f('Cast MSG A', 'Cast Msg A', { ref: 'Messages' }), f('Cast MSG B', 'Cast Msg B', { ref: 'Messages' }),
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
        f('Armour', 'Armour', { options: CLASS_ARMOUR }), f('Weapon', 'Weapons', { options: CLASS_WEAPONS }),
        f('Combat'), f('Magic Type', 'Magery', { options: MAGERY }), f('Magic LVL', 'Magery Level'),
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
        f('Number'), f('Name'), f('Type', 'Type', { options: SHOP_TYPE }), f('Min Lvl', 'Min LVL'),
        f('Max Lvl', 'Max LVL'), f('MarkUp', 'Markup %'), f('Class Limit', 'Class', { ref: 'Classes' }),
        wide('Desc A', 'Description'), wide('Desc B', ''), wide('Desc C', ''),
      ],
    },
    {
      title: 'Items Sold',
      grid: {
        columns: [
          { template: 'Item #', label: 'Item', pick: { ref: 'Items' } },
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
        f('Type', 'Type', { options: ROOM_TYPE }), f('Shop Number', 'Shop', { ref: 'Shops' }),
        f('Gang House Number', 'Gang House'), f('Light'), f('AnsiMap', 'ANSI Map'), f('Attributes'),
      ],
    },
    {
      title: 'Exits',
      grid: {
        columns: [
          { template: 'Exit #', label: 'To Room' },
          { template: 'Type #', label: 'Type', pick: { options: EXIT_TYPE } },
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
        f('Perm NPC', 'Perm NPC', { ref: 'Monsters' }), f('Mon Type', 'Monster Type', { options: MONSTER_GROUP }),
        f('Min Index'), f('Max Index'), f('Max Regen'), f('Max Area'), f('Runic'), f('Platinum'),
        f('Gold'), f('Silver'), f('Copper'), f('InvisRunic', 'Hidden Runic'),
        f('InvisPlatinum', 'Hidden Platinum'), f('InvisGold', 'Hidden Gold'),
        f('InvisSilver', 'Hidden Silver'), f('InvisCopper', 'Hidden Copper'),
      ],
      grid: { columns: [{ template: 'Placed Item #', label: 'Placed Item', pick: { ref: 'Items' } }], from: 0, to: 9 },
    },
    {
      title: 'Visible Items',
      grid: {
        columns: [
          { template: 'Room Item #', label: 'Item', pick: { ref: 'Items' } },
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
          { template: 'Hidden Item #', label: 'Item', pick: { ref: 'Items' } },
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
        f('Spell', 'Spell', { ref: 'Spells' }),
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
};

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
