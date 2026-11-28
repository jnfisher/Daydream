// The Workshop's tools, grouped as its page shows them. Every Nightmare Redux tool has a home
// here, and each lists the names it had in NMR (`was`), so a search for an old name ("Universal
// Modifier") finds the tool that does its work now (Bulk edit), here and in Ctrl+K.
// Pure data: the page maps each `icon` to its picture.

/** A tool's colour: one of the pastels, plain grey, or red for the ones that delete. */
export type Tone = 'lilac' | 'mint' | 'sky' | 'peach' | 'butter' | 'pink' | 'teal' | 'plain' | 'bad';

export interface Tool {
  /** Its link: `#/workshop/<id>`. */
  id: string;
  name: string;
  /** One sentence on what it does. */
  about: string;
  /** What NMR called it. */
  was: string[];
  tone: Tone;
  icon: string;
  /** It lives in another place, and its card goes there. */
  href?: string;
  /** Planned, not built yet. */
  later?: boolean;
  /** More words it is found by. */
  words?: string;
}

export interface Group {
  id: string;
  title: string;
  about: string;
  tools: Tool[];
}

export const GROUPS: Group[] = [
  {
    id: 'health',
    title: 'Health checks',
    about: 'Problems found across the world, each fixed in one click you can take back.',
    tools: [
      { id: 'uses', name: 'Fix uses on items', about: 'Items with 0 uses break the first time they are used: they get −1, never wearing out. Items kept after their uses get a recharge.', was: ['Fix Number of Uses'], tone: 'butter', icon: 'wand', words: 'charges health' },
      { id: 'dropuses', name: 'Fix uses on drops', about: 'Monster drops that would arrive used up get their item’s own uses.', was: ['Fix Number of Uses on Item Drops'], tone: 'butter', icon: 'wand', words: 'charges health' },
      { id: 'checks', name: 'Checks that read the wiring', about: 'Prices against what is said, spelling, placeholders, messages that read wrong and spawns nobody reaches. They are on Home.', was: [], tone: 'lilac', icon: 'pulse', href: '#/home', words: 'health problems typos' },
    ],
  },
  {
    id: 'balance',
    title: 'Balance',
    about: 'Scale numbers across many records, with a before and after.',
    tools: [
      { id: 'scale', name: 'Scale a field', about: 'Multiply or divide a number on every record that matches, such as the experience monsters give.', was: ['Multiply All Exp', 'Divide All Exp', 'Multiply Boss Exp'], tone: 'butter', icon: 'sliders', words: 'experience exp boss double half' },
      { id: 'drops', name: 'Drop rates', about: 'Set or shift how often monsters drop each kind of item.', was: ['Modify Item Drop Percentages'], tone: 'peach', icon: 'percent', words: 'loot drop chance' },
      { id: 'fold', name: 'Fold Exp Multiplier into Experience', about: 'Bakes the multiplier into the experience value and sets it to 1, so damage over time pays in full.', was: ['Combine Exp and Exp Multi Fields'], tone: 'butter', icon: 'combine', words: 'experience' },
      { id: 'limits', name: 'Lift item limits', about: 'Take the game limits or the level restrictions off items.', was: ['No Limited Items', 'No Level Restrictions'], tone: 'butter', icon: 'crown', words: 'limited level' },
    ],
  },
  {
    id: 'bulk',
    title: 'Bulk edit',
    about: 'Change one field on everything that matches.',
    tools: [
      { id: 'bulk', name: 'Bulk edit', about: 'Pick a table, which records and a field or an ability. See every value that will change before it does.', was: ['Universal Modifier'], tone: 'lilac', icon: 'sliders', words: 'modify mass change ability' },
      { id: 'rooms', name: 'Edit rooms together', about: 'Pick rooms on the map and set their name, light, monsters or area at once, or set a run of rooms by number.', was: ['Mass Room Editor'], tone: 'mint', icon: 'lasso', words: 'mass rooms' },
    ],
  },
  {
    id: 'find',
    title: 'Find and list',
    about: 'The old list builders, read from the world as it is now.',
    tools: [
      { id: 'where', name: 'Where is it?', about: 'Every room, shop and player that holds an item. Item cards show it too.', was: ['Find an Item'], tone: 'sky', icon: 'scan', words: 'find search' },
      { id: 'limited', name: 'Limited items', about: 'Every limited item and where its copies are, with the ones over their limit first.', was: ['Build Limited Item List', 'Limited Items List'], tone: 'butter', icon: 'crown' },
      { id: 'spawns', name: 'Monster groups', about: 'The monsters of each group and index, and moving groups and indexes along with the rooms that spawn them.', was: ['Create Monster/Index List', 'Monster Group/Index Changer'], tone: 'peach', icon: 'tree', words: 'spawn index group' },
      { id: 'areas', name: 'Control rooms', about: 'Each control room and the rooms that name it. On the map, colour rooms by area.', was: ['Create Control Room List'], tone: 'mint', icon: 'layers', words: 'area areas' },
      { id: 'npcs', name: 'Who stands where', about: 'Every room with a permanent NPC, and who it is.', was: ['Create NPC/Room List'], tone: 'mint', icon: 'users', words: 'npc npcs' },
      { id: 'story', name: 'Story', about: 'Quests and dialogue laid out as a graph you can play.', was: ['Quest Organizer'], tone: 'lilac', icon: 'workflow', href: '#/story', words: 'quests' },
      { id: 'abilities', name: 'Ability glossary', about: 'Every ability by number: its name and what it does.', was: ['Ability List Editor'], tone: 'pink', icon: 'book', words: 'abilities' },
    ],
  },
  {
    id: 'renumber',
    title: 'Renumber',
    about: 'Move records to new numbers, and rewrite everything that points at them.',
    tools: [
      { id: 'renumber', name: 'Renumber records', about: 'Move records to new numbers, or a map’s rooms to another map. Other records, abilities and scripts that name them follow.', was: ['Record Number Changer', 'Change Map Number'], tone: 'lilac', icon: 'swap', words: 'move number' },
    ],
  },
  {
    id: 'export',
    title: 'Import and export',
    about: 'Files in and out of Daydream.',
    tools: [
      { id: 'mme', name: 'Export for MME', about: 'The data file MMUD Explorer reads: the same fields as NMR’s export, nothing more.', was: ['Export to MMUD Explorer'], tone: 'sky', icon: 'download', words: 'mmud explorer' },
      { id: 'nmr', name: 'Nightmare Redux database', about: 'Bring records in from an NMR export (.mdb or .json), all of them or a range.', was: ['Import Records', 'Database Importer'], tone: 'sky', icon: 'database', words: 'mdb import' },
      { id: 'wcc', name: 'WCC files', about: 'Write the world’s edits into a folder of WCC .DAT files by hand.', was: [], tone: 'sky', icon: 'drive', words: 'dat btrieve worldgroup write' },
      { id: 'silvermere', name: 'silvermere database', about: 'Write the world into a silvermere server’s database, or read it back.', was: [], tone: 'sky', icon: 'server', words: 'export import' },
      { id: 'update', name: 'Update files', about: 'Built for you when you publish to Worldgroup or MBBSEmu.', was: ['Compile Update File', 'Compile Full Update File', 'Compile Blank Update File'], tone: 'sky', icon: 'file', href: '#/publish', words: 'publish' },
    ],
  },
  {
    id: 'reset',
    title: 'Reset world state',
    about: 'Live-game housekeeping, applied at the next publish.',
    tools: [
      { id: 'kills', name: 'Kill timers', about: 'Set every monster as killed on a day, or clear the timers so they all come back.', was: ['Set Last Killed Times to Date', 'Reset Last Killed Times to Zero'], tone: 'peach', icon: 'timer', words: 'respawn boss' },
      { id: 'restock', name: 'Restock shops', about: 'Fill every shop back up to its normal stock.', was: ['Restock All Shops'], tone: 'teal', icon: 'store' },
      { id: 'floors', name: 'Tidy the floors', about: 'Put piles of the same item lying in a room into one slot.', was: ['Combine Like Items on Floor'], tone: 'mint', icon: 'combine', words: 'rooms items' },
    ],
  },
  {
    id: 'players',
    title: 'Players',
    about: 'Accounts, gangs and bank books.',
    tools: [
      { id: 'merge', name: 'Merge accounts', about: 'Copy the users of another user file into this world. Users it already has are left alone.', was: ['Merge Users'], tone: 'lilac', icon: 'merge', words: 'users' },
      { id: 'gangs', name: 'Change gangs', about: 'Move everyone in one gang to another.', was: ['Change Gang on All Users'], tone: 'lilac', icon: 'users', later: true },
      { id: 'retrain', name: 'Retrain everyone', about: 'Give every character their training points back.', was: ['Retrain All Users'], tone: 'lilac', icon: 'refresh', later: true },
    ],
  },
  {
    id: 'lab',
    title: 'Combat lab',
    about: 'Try a character against a monster before players do, with 1.11p’s own numbers.',
    tools: [
      { id: 'fight', name: 'Fight it', about: 'A character made from your classes, races and items against any monster, round by round, with the odds over a thousand fights.', was: ['Attack Simulator'], tone: 'pink', icon: 'swords', words: 'combat simulator damage hit chance' },
      { id: 'calc', name: 'Calculators', about: 'Experience to level, and swings a round: in Fight it, beside the fight.', was: ['Exp Calculator', 'Swing Calculator'], tone: 'pink', icon: 'calculator', href: '#/workshop/fight', words: 'experience level swings energy' },
    ],
  },
  {
    id: 'advanced',
    title: 'Advanced',
    about: 'Rarely needed. Tucked away, not gone.',
    tools: [
      { id: 'buffer', name: 'Buffer rooms', about: 'Add or remove the spare rooms that keep rooms loading past gaps in a map’s numbers.', was: ['Insert Buffer Rooms', 'Delete Buffer Rooms'], tone: 'plain', icon: 'dashed' },
      { id: 'strip', name: 'Strip trailing characters', about: 'Take stray characters off the end of every text block part that ends with them.', was: ['Strip Characters off Text Blocks'], tone: 'plain', icon: 'scissors', words: 'textblocks spaces' },
      { id: 'delete', name: 'Delete records', about: 'Delete a range of a table’s records, or all of them. It can be taken back until you publish.', was: ['Delete Records', 'Database Deleter'], tone: 'bad', icon: 'trash', words: 'remove' },
      { id: 'call', name: 'File call letters', about: 'Rename the .DAT files for a second copy of the game.', was: ['Change dat call letters'], tone: 'plain', icon: 'hash', later: true },
      { id: 'readonly', name: 'Read-only mode', about: 'Look without touching: a lock on saving.', was: ['Disable DB Writing'], tone: 'plain', icon: 'lock', later: true },
    ],
  },
];

/** Every tool, in the page's order. */
export const TOOLS: Tool[] = GROUPS.flatMap((g) => g.tools);

/** A tool and its group, by its id. */
export function toolById(id: string): { group: Group; tool: Tool } | undefined {
  for (const group of GROUPS) {
    const tool = group.tools.find((t) => t.id === id);
    if (tool) return { group, tool };
  }
  return undefined;
}

/** The words a search may name a tool by: its own, its group's and NMR's names. */
export const searchText = (t: Tool, g?: Group) =>
  [t.name, t.about, ...t.was, t.words ?? '', g?.title ?? ''].join(' ').toLowerCase();

/** The tools a search finds, group by group (each word must be somewhere); all of them for none. */
export function search(q: string): { group: Group; tools: Tool[] }[] {
  const words = q.toLowerCase().split(/\s+/).filter(Boolean);
  return GROUPS.map((group) => ({ group, tools: group.tools.filter((t) => words.every((w) => searchText(t, group).includes(w))) })).filter(
    (r) => r.tools.length,
  );
}

/** The tools whose name or NMR name holds every word: the ones Ctrl+K offers. */
export function named(q: string): Tool[] {
  const words = q.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  return TOOLS.filter((t) => !t.later && words.every((w) => [t.name, ...t.was, t.words ?? ''].join(' ').toLowerCase().includes(w)));
}

/** A tool's link. */
export const toolHref = (t: Tool) => t.href ?? `#/workshop/${t.id}`;
