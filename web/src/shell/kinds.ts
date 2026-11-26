// What each kind of record looks like everywhere it appears: its pastel, its icon and its
// name. Rooms are mint, items butter, monsters peach, spells pink, text blocks lilac,
// messages sky and shops teal, so a chip says what it points at before it is read.

import type { Component } from 'svelte';
import DoorOpen from '@lucide/svelte/icons/door-open';
import Drama from '@lucide/svelte/icons/drama';
import Footprints from '@lucide/svelte/icons/footprints';
import Ghost from '@lucide/svelte/icons/ghost';
import Landmark from '@lucide/svelte/icons/landmark';
import MessageSquareText from '@lucide/svelte/icons/message-square-text';
import ScrollText from '@lucide/svelte/icons/scroll-text';
import ShieldHalf from '@lucide/svelte/icons/shield-half';
import Store from '@lucide/svelte/icons/store';
import Sword from '@lucide/svelte/icons/sword';
import User from '@lucide/svelte/icons/user';
import UsersRound from '@lucide/svelte/icons/users-round';
import WandSparkles from '@lucide/svelte/icons/wand-sparkles';

export interface Kind {
  /** The table's name in the editors (`Items`). */
  table: string;
  /** The class that gives it its colours (`t-item`). */
  cls: string;
  icon: Component<{ size?: number | string; class?: string }>;
  one: string;
  many: string;
}

const kind = (table: string, cls: string, icon: Kind['icon'], one: string, many: string): Kind => ({ table, cls, icon, one, many });

const KINDS: Kind[] = [
  kind('Items', 't-item', Sword, 'Item', 'Items'),
  kind('Monsters', 't-monster', Ghost, 'Monster', 'Monsters'),
  kind('Spells', 't-spell', WandSparkles, 'Spell', 'Spells'),
  kind('Classes', 't-other', ShieldHalf, 'Class', 'Classes'),
  kind('Races', 't-other', Footprints, 'Race', 'Races'),
  kind('Shops', 't-shop', Store, 'Shop', 'Shops'),
  kind('Rooms', 't-room', DoorOpen, 'Room', 'Rooms'),
  kind('Messages', 't-msg', MessageSquareText, 'Message', 'Messages'),
  kind('Textblocks', 't-text', ScrollText, 'Text block', 'Text blocks'),
  kind('Actions', 't-other', Drama, 'Action', 'Actions'),
  kind('Users', 't-other', User, 'User', 'Users'),
  kind('BankBooks', 't-other', Landmark, 'Bank book', 'Bank books'),
  kind('Gangs', 't-other', UsersRound, 'Gang', 'Gangs'),
];

/** The record types' slugs in the server's answers (`text-blocks`), by editor table. */
const SLUGS: Record<string, string> = {
  rooms: 'Rooms', items: 'Items', monsters: 'Monsters', spells: 'Spells', classes: 'Classes',
  races: 'Races', shops: 'Shops', messages: 'Messages', 'text-blocks': 'Textblocks',
  actions: 'Actions', users: 'Users', 'bank-books': 'BankBooks', gangs: 'Gangs',
};

/** The editors' tables in NMR's menu order. */
export const TABLE_ORDER = KINDS.map((k) => k.table);

/** The kind of a table, by its editor name (`Items`) or record type slug (`items`). */
export function kindOf(table: string | null | undefined): Kind {
  const name = table ? (SLUGS[table] ?? table) : '';
  return KINDS.find((k) => k.table === name) ?? kind(name || 'Record', 't-other', ScrollText, name || 'Record', name || 'Records');
}
