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
}

export interface Tab {
  title: string;
  fields?: Field[];
  grid?: Grid;
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
    if (tab.grid) {
      for (let n = tab.grid.from; n <= tab.grid.to; n++) {
        for (const c of tab.grid.columns) shown.add(c.template.replace('#', String(n)));
      }
    }
  }
  return shown;
}
