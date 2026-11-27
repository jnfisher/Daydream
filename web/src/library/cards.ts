// Each table's record card: its columns grouped by what they are for, with NMR's labels and
// friendlier headings; what the list says under each record's name, and the chips that narrow
// the list; and which records a number is drawn among. A table without a card here gets
// NMR's own tabs (sections.ts `fromLayout`). Every card ends with what its groups leave out.

import type { Cell } from '../api';
import {
  ARMOUR,
  EXITS,
  ITEM_TYPE,
  MAGERY,
  MONSTER_GROUP,
  MONSTER_TYPE,
  pickFor,
  ROOM_TYPE,
  SHOP_TYPE,
  WEAPON,
  WORN,
  type Grid,
  type Options,
} from '../layouts';
import { monsterProblems, type Lines } from './combat';
import type { Problem } from './problems';
import type { FieldSpec, Section } from './sections';

export type Get = (col: string) => Cell;

/** A chip that narrows the list. */
export interface Filter {
  label: string;
  test: (get: Get, key: Cell[]) => boolean;
}

/** The records a field's strip draws it among. */
export interface Peers {
  /** Whether another record is this one's peer. */
  same: (mine: Get, other: Get) => boolean;
  /** Who they are, for a person: `every 1H Sharp weapon`. */
  says: (mine: Get) => string;
}

export interface CardDef {
  sections: Section[];
  /** The columns the list asks for besides each record's key and name. */
  listCols?: string[];
  /** What the list says under a record's name. */
  summary?: (get: Get) => string;
  filters?: (rows: { key: Cell[]; get: Get }[]) => Filter[];
  peers?: Peers;
  /** The numbers, of the list's columns, that say how alike two records are: Compare starts
   * with the closest by them. */
  measures?: string[];
  /** The chips beside the record's number in its header. */
  tags?: (get: Get) => string[];
  /** What is wrong with a record beyond what every card checks; `lines` reads a message
   * (undefined while it is read, null when the world has none). */
  problems?: (get: Get, look: { lines: (n: number) => Lines | null | undefined }) => Problem[];
}

const num = (v: Cell) => (typeof v === 'number' ? v : Number(v ?? 0) || 0);
const label = (list: Options, v: Cell) => list[num(v)] ?? String(v ?? '');
const fmt = (v: Cell) => num(v).toLocaleString();

function F(col: string, labelOrOpts?: string | Omit<FieldSpec, 'col'>, opts: Omit<FieldSpec, 'col' | 'label'> = {}): FieldSpec {
  return typeof labelOrOpts === 'string' ? { col, label: labelOrOpts, ...opts } : { col, ...labelOrOpts };
}

const range = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, i) => from + i);

const abilities = (from: number, to: number): Grid => ({
  columns: [
    { template: 'Ability #', label: 'Ability', pick: { abilities: true } },
    { template: 'Ability Value #', label: 'Value' },
  ],
  from,
  to,
});

/** A grid whose columns pick as fields.json says. */
function grid(table: string, g: Grid): Grid {
  return { ...g, columns: g.columns.map((c) => ({ ...c, pick: c.pick ?? pickFor(table, c.template) })) };
}

/** Each kind of record in a list's chips: `Weapons`. */
const PLURAL: Record<string, string> = {
  Armour: 'Armour', Weapon: 'Weapons', Projectile: 'Projectiles', Sign: 'Signs', Food: 'Food', Drink: 'Drink',
  Light: 'Lights', Key: 'Keys', Container: 'Containers', Scroll: 'Scrolls', Special: 'Special',
  Solo: 'Solo', Leader: 'Leaders', Follower: 'Followers', Stationary: 'Stationary',
};

/** One chip for each value of `col` the records have, in the list's order. */
function byList(col: string, list: Options, plural = true): CardDef['filters'] {
  return (rows) => {
    const seen = new Set(rows.map((r) => num(r.get(col))));
    return Object.entries(list)
      .filter(([v]) => seen.has(Number(v)))
      .map(([v, name]) => ({ label: plural ? (PLURAL[name] ?? name) : name, test: (get: Get) => num(get(col)) === Number(v) }));
  };
}

const isWeapon = (get: Get) => num(get('Type')) === 1;
const isArmour = (get: Get) => num(get('Type')) === 0;

const ITEMS: CardDef = {
  listCols: ['Type', 'Weapon', 'Worn On', 'Min Hit', 'Max Hit', 'Speed', 'Accuracy', 'Req Str', 'Weight', 'AC', 'DR'],
  summary: (get) => {
    if (isWeapon(get)) return `${fmt(get('Min Hit'))}–${fmt(get('Max Hit'))} dmg · speed ${fmt(get('Speed'))}`;
    if (isArmour(get)) return `AC ${fmt(get('AC'))}/${fmt(get('DR'))} · ${label(WORN, get('Worn On')).toLowerCase()}`;
    return label(ITEM_TYPE, get('Type'));
  },
  filters: byList('Type', ITEM_TYPE),
  measures: ['Min Hit', 'Max Hit', 'Speed', 'Accuracy', 'Req Str', 'Weight', 'AC', 'DR'],
  peers: {
    same: (mine, other) => {
      if (num(other('Type')) !== num(mine('Type'))) return false;
      if (isWeapon(mine)) return num(other('Weapon')) === num(mine('Weapon'));
      if (isArmour(mine)) return num(other('Worn On')) === num(mine('Worn On'));
      return true;
    },
    says: (mine) => {
      if (isWeapon(mine)) return `every ${label(WEAPON, mine('Weapon'))} weapon`;
      if (isArmour(mine)) return `everything worn on the ${label(WORN, mine('Worn On')).toLowerCase()}`;
      return `every ${label(ITEM_TYPE, mine('Type')).toLowerCase()}`;
    },
  },
  tags: (get) => [
    label(ITEM_TYPE, get('Type')),
    ...(isWeapon(get) ? [label(WEAPON, get('Weapon'))] : isArmour(get) ? [label(WORN, get('Worn On')), label(ARMOUR, get('Armour'))] : []),
  ],
  sections: [
    {
      id: 'fight',
      title: 'In a fight',
      when: isWeapon,
      fields: [
        F('Weapon', 'Weapon type'),
        F('Min Hit', { strip: true }),
        F('Max Hit', { strip: true }),
        F('Speed', { strip: true, lower: true, note: 'lower is faster' }),
        F('Accuracy', { strip: true }),
        F('Req Str', 'Strength needed', { strip: true }),
        F('Weight', { strip: true }),
      ],
      scatter: { x: 'Speed', y: 'Max Hit', note: 'Lower speed is faster.' },
    },
    {
      id: 'wear',
      title: 'Wearing it',
      when: isArmour,
      fields: [
        F('Worn On', 'Worn'),
        F('Armour', 'Armour type'),
        F('AC', { strip: true }),
        F('DR', { strip: true }),
        F('Accuracy', { strip: true }),
        F('Req Str', 'Strength needed', { strip: true }),
        F('Weight', { strip: true }),
      ],
    },
    {
      id: 'item',
      title: 'The item',
      fields: [
        F('Name'),
        F('Type'),
        F('Cost', { with: 'Cost Type' }),
        F('Uses', 'Max Uses'),
        F('Game Limit', { note: '0 is no limit' }),
        F('Gettable', 'Getable'),
        F('Robable'),
        F('Not Droppable'),
        F('Destroy On Death', 'Destroys On Death'),
        F('Retain After Uses', 'Retain After Uses Expire'),
      ],
    },
    {
      id: 'see',
      title: 'What players see',
      side: true,
      text: { cols: range(1, 6).map((n) => `Desc${n}`), label: 'look' },
      fields: [F('Hit Msg', 'Hit (msg)'), F('Miss Msg', 'Miss (msg)'), F('Read Msg', 'Read (tb)'), F('Distruct Msg', 'Destroyed (msg)')],
    },
    { id: 'abilities', title: 'Abilities', grid: abilities(0, 19) },
    {
      id: 'who',
      title: 'Who may use it',
      hint: 'when any are set, only these',
      grid: grid('Items', {
        columns: [
          { template: 'Class #', label: 'Class' },
          { template: 'Race #', label: 'Race' },
          { template: 'Negate #', label: 'Negates spell' },
        ],
        from: 0,
        to: 9,
        separate: true,
      }),
    },
    {
      id: 'coins',
      title: 'Coins inside',
      folded: true,
      fields: [F('OpenRunic', 'Runic'), F('OpenPlatinum', 'Platinum'), F('OpenGold', 'Gold'), F('OpenSilver', 'Silver'), F('OpenCopper', 'Copper')],
    },
    { id: 'where', title: 'Where it is in the world', side: true, where: true },
  ],
};

const MONSTERS: CardDef = {
  listCols: ['Group', 'Index', 'Type', 'Undead', 'Hit Points', 'Experience', 'AC', 'DR', 'MR', 'Energy', 'HP Regen', 'BS Defense'],
  summary: (get) => `${fmt(get('Hit Points'))} HP · ${fmt(get('Experience'))} exp · group ${num(get('Group'))}/${num(get('Index'))}`,
  filters: (rows) => [
    ...byList('Type', MONSTER_TYPE)!(rows),
    ...(rows.some((r) => num(r.get('Undead'))) ? [{ label: 'Undead', test: (get: Get) => num(get('Undead')) !== 0 }] : []),
  ],
  measures: ['Hit Points', 'Experience', 'AC', 'DR', 'MR', 'Energy', 'HP Regen', 'BS Defense'],
  peers: {
    same: (mine, other) => num(other('Group')) === num(mine('Group')),
    says: (mine) => `every ${label(MONSTER_GROUP, mine('Group'))} monster`,
  },
  tags: (get) => [label(MONSTER_GROUP, get('Group')), label(MONSTER_TYPE, get('Type')), ...(num(get('Undead')) ? ['Undead'] : [])],
  problems: (get, look) => monsterProblems(get, look.lines),
  sections: [
    {
      id: 'tough',
      title: 'Toughness',
      fields: [
        F('Hit Points', 'Hitpoints', { strip: true }),
        F('HP Regen', { strip: true }),
        F('AC', { strip: true }),
        F('DR', { strip: true }),
        F('MR', 'M.R.', { strip: true }),
        F('BS Defense', { strip: true }),
        F('Charm LvL', 'Charm Lvl'),
        F('Charm RES', 'Charm Resist'),
      ],
    },
    {
      id: 'fight',
      title: 'In a fight',
      fields: [F('Energy', { strip: true, note: 'spent by its attacks each round' }), F('Follow', 'Follow %'), F('Experience', { strip: true }), F('Exp Multiplier', 'Multiplier')],
    },
    {
      id: 'attacks',
      title: 'Attacks',
      hint: 'up to 6 blows a round, while its Energy lasts',
      custom: 'attacks',
      grid: grid('Monsters', {
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
      }),
    },
    {
      id: 'spells',
      title: 'Spells between rounds',
      grid: grid('Monsters', {
        columns: [
          { template: 'Spell Number #', label: 'Spell' },
          { template: 'Spell Cast % #', label: '%' },
          { template: 'Spell Cast LVL #', label: 'LvL' },
        ],
        from: 0,
        to: 4,
      }),
    },
    {
      id: 'drops',
      title: 'What it drops',
      fields: [F('Runic'), F('Platinum'), F('Gold'), F('Silver'), F('Copper')],
      grid: grid('Monsters', {
        columns: [
          { template: 'Item Number #', label: 'Item' },
          { template: 'Item Uses #', label: 'Uses' },
          { template: 'Item Drop % #', label: '%' },
        ],
        from: 0,
        to: 9,
      }),
    },
    { id: 'abilities', title: 'Abilities', grid: abilities(0, 9) },
    {
      id: 'who',
      title: 'Who it is',
      side: true,
      fields: [
        F('Name'),
        F('Group'),
        F('Index'),
        F('Type'),
        F('Alignment'),
        F('Gender'),
        F('Undead'),
        F('Active'),
        F('Game Limit', { note: '0 is no limit' }),
        F('Regen Time'),
      ],
    },
    { id: 'see', title: 'What players see', side: true, text: { cols: range(1, 4).map((n) => `Desc ${n}`), label: 'look' } },
    {
      id: 'wired',
      title: 'How it is wired',
      side: true,
      hint: 'the records it names',
      custom: 'wired',
      fields: [
        F('Weapon Number', 'Weapon'),
        F('Death Spell'),
        F('Create Spell'),
        F('Move Msg'),
        F('Death Msg'),
        F('Greet Txt'),
        F('Desc Txt'),
        F('Talk Txt'),
      ],
    },
    { id: 'spawns', title: 'Where it spawns', side: true, custom: 'spawns' },
    { id: 'where', title: 'Where it is in the world', side: true, where: true },
  ],
};

const ROOMS: CardDef = {
  filters: (rows) => {
    const maps = [...new Set(rows.map((r) => num(r.key[0] ?? 0)))].sort((a, b) => a - b);
    return maps.length > 1 && maps.length <= 40 ? maps.map((m) => ({ label: `Map ${m}`, test: (_get: Get, key: Cell[]) => num(key[0] ?? 0) === m })) : [];
  },
  tags: (get) => [label(ROOM_TYPE, get('Type'))],
  sections: [
    // a room is what it looks like and where it leads: those first, the rest beside them
    { id: 'see', title: 'What players see', text: { cols: range(0, 6).map((n) => `Desc ${n}`), label: 'look' } },
    {
      id: 'room',
      title: 'The room',
      side: true,
      fields: [F('Name'), F('Type'), F('Shop Number', 'Shop'), F('Gang House Number', 'Gang House'), F('Light'), F('AnsiMap', 'ANSI Map'), F('Attributes')],
    },
    {
      id: 'exits',
      title: 'Exits',
      grid: grid('Rooms', {
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
      }),
    },
    {
      id: 'monsters',
      title: 'Monsters here',
      fields: [F('Perm NPC'), F('Mon Type', 'Monster Type'), F('Min Index'), F('Max Index'), F('Max Regen'), F('Max Area'), F('Control Room')],
    },
    {
      id: 'items',
      title: 'Items lying here',
      grid: grid('Rooms', {
        columns: [
          { template: 'Room Item #', label: 'Item' },
          { template: 'Room Item # USES', label: 'Uses' },
          { template: 'Room Item # QTY', label: 'Qty' },
        ],
        from: 0,
        to: 16,
      }),
    },
    {
      id: 'hidden',
      title: 'Hidden here',
      grid: grid('Rooms', {
        columns: [
          { template: 'Hidden Item #', label: 'Item' },
          { template: 'Hidden Item # USES', label: 'Uses' },
          { template: 'Hidden Item # QTY', label: 'Qty' },
        ],
        from: 0,
        to: 14,
      }),
      fields: [F('InvisRunic', 'Hidden Runic'), F('InvisPlatinum', 'Hidden Platinum'), F('InvisGold', 'Hidden Gold'), F('InvisSilver', 'Hidden Silver'), F('InvisCopper', 'Hidden Copper')],
    },
    { id: 'placed', title: 'Placed items', side: true, grid: grid('Rooms', { columns: [{ template: 'Placed Item #', label: 'Placed Item' }], from: 0, to: 9 }) },
    { id: 'coins', title: 'Coins here', side: true, folded: true, fields: [F('Runic'), F('Platinum'), F('Gold'), F('Silver'), F('Copper')] },
    { id: 'other', title: 'Other', side: true, fields: [F('Death Room'), F('Exit Room'), F('Command Text'), F('Delay'), F('Spell')] },
    { id: 'where', title: 'Where it is in the world', side: true, where: true },
  ],
};

const SPELLS: CardDef = {
  listCols: ['Short Name', 'Level', 'Magery A', 'Mana'],
  summary: (get) => `${String(get('Short Name') ?? '').trim() || '—'} · level ${num(get('Level'))} · ${label(MAGERY, get('Magery A')).toLowerCase()}`,
  filters: byList('Magery A', MAGERY, false),
  tags: (get) => [label(MAGERY, get('Magery A')), `Level ${num(get('Level'))}`],
  sections: [
    {
      id: 'spell',
      title: 'The spell',
      fields: [
        F('Name'),
        F('Short Name'),
        F('Level', 'Req. Level'),
        F('Level Cap'),
        F('Magery A', 'Magery'),
        F('Magery B', 'Magery Level'),
        F('Spell Type'),
        F('Target'),
        F('Attack Type', 'Type of Attack'),
        F('Type of Resists'),
        F('Resist Ability'),
        F('Difficulty'),
        F('Energy'),
        F('Mana'),
      ],
    },
    {
      id: 'power',
      title: 'Its power',
      hint: 'at first, and as the caster grows',
      fields: [
        F('Min', 'Base Min'),
        F('Max', 'Base Max'),
        F('Duration', 'Base Duration'),
        F('Min Increase', 'Min increase'),
        F('LVLS Min Increase', 'LVLs/Min increase'),
        F('Max Increase', 'Max increase'),
        F('LVLS Max Increase', 'LVLs/Max increase'),
        F('Dur Increase', 'Dur. increase'),
        F('LVLS Dur Increase', 'LVLs/Dur. increase'),
      ],
    },
    {
      id: 'see',
      title: 'What players see',
      side: true,
      text: { cols: ['Desc 1', 'Desc 2'] },
      fields: [F('MSG Style', 'Norm Msg Style'), F('Cast MSG A', 'Cast Msg A'), F('Cast MSG B', 'Cast Msg B')],
    },
    { id: 'abilities', title: 'Abilities', grid: abilities(0, 9) },
    { id: 'where', title: 'Where it is in the world', side: true, where: true },
  ],
};

const SHOPS: CardDef = {
  listCols: ['Type'],
  summary: (get) => label(SHOP_TYPE, get('Type')),
  filters: byList('Type', SHOP_TYPE, false),
  tags: (get) => [label(SHOP_TYPE, get('Type'))],
  sections: [
    { id: 'shop', title: 'The shop', fields: [F('Name'), F('Type'), F('Min Lvl', 'Min LVL'), F('Max Lvl', 'Max LVL'), F('MarkUp', 'Markup %'), F('Class Limit', 'Class')] },
    { id: 'see', title: 'What players see', side: true, text: { cols: ['Desc A', 'Desc B', 'Desc C'] } },
    {
      id: 'stock',
      title: 'What it sells',
      grid: grid('Shops', {
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
      }),
    },
    { id: 'where', title: 'Where it is in the world', side: true, where: true },
  ],
};

const CLASSES: CardDef = {
  sections: [
    {
      id: 'class',
      title: 'The class',
      fields: [
        F('Name'),
        F('EXP %', 'Exp %'),
        F('Min HP', 'HP (min)'),
        F('Max HP', 'HP (max)'),
        F('Armour'),
        F('Weapon', 'Weapons'),
        F('Combat'),
        F('Magic Type', 'Magery'),
        F('Magic LVL', 'Magery Level'),
        F('Title Text'),
      ],
    },
    { id: 'abilities', title: 'Abilities', grid: abilities(0, 9) },
    { id: 'where', title: 'Where it is used', side: true, where: true },
  ],
};

const RACES: CardDef = {
  sections: [
    {
      id: 'race',
      title: 'The race',
      fields: [F('Name'), F('EXP %', 'Exp %'), F('CP', 'Starting Cp'), F('HP Bonus', 'Hp Bonus')],
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
    { id: 'abilities', title: 'Abilities', grid: abilities(0, 9) },
    { id: 'where', title: 'Where it is used', side: true, where: true },
  ],
};

const MESSAGES: CardDef = {
  sections: [
    { id: 'see', title: 'What players see', text: { cols: ['Line 1', 'Line 2', 'Line 3'] } },
    { id: 'where', title: 'Where it is used', side: true, where: true },
  ],
};

const TEXTBLOCKS: CardDef = {
  sections: [
    { id: 'see', title: 'The text', text: { cols: ['Data'] }, fields: [F('Link To')] },
    { id: 'where', title: 'Where it is used', side: true, where: true },
  ],
};

export const CARDS: Record<string, CardDef> = {
  Items: ITEMS,
  Monsters: MONSTERS,
  Rooms: ROOMS,
  Spells: SPELLS,
  Shops: SHOPS,
  Classes: CLASSES,
  Races: RACES,
  Messages: MESSAGES,
  Textblocks: TEXTBLOCKS,
};

// fields pick as fields.json says, unless a card says otherwise
for (const [table, def] of Object.entries(CARDS)) {
  for (const s of def.sections) for (const f of s.fields ?? []) f.pick ??= pickFor(table, f.col);
}
