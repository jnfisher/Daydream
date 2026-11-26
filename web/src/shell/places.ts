// The six places in the rail. Worlds (the picker) and Publish open from the top bar.

export interface PlaceInfo {
  id: 'home' | 'world' | 'library' | 'story' | 'players' | 'workshop';
  label: string;
  hint: string;
  href: string;
  /** The letter after g that jumps here. */
  key: string;
}

export const PLACES: PlaceInfo[] = [
  { id: 'home', label: 'Home', hint: 'your world at a glance', href: '#/home', key: 'h' },
  { id: 'world', label: 'World', hint: 'the map', href: '#/world', key: 'w' },
  { id: 'library', label: 'Library', hint: 'every record, table by table', href: '#/library', key: 'l' },
  { id: 'story', label: 'Story', hint: 'quests and the text they are made of', href: '#/story', key: 's' },
  { id: 'players', label: 'Players', hint: 'users, gangs and bank books', href: '#/players', key: 'p' },
  { id: 'workshop', label: 'Workshop', hint: 'bulk tools and lists', href: '#/workshop', key: 'k' },
];

/** g then a letter: the places, then Items and Monsters straight away. */
export const JUMPS: Record<string, string> = {
  ...Object.fromEntries(PLACES.map((p) => [p.key, p.href])),
  i: '#/items',
  m: '#/monsters',
};
