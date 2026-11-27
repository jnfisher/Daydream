// NMR's Export to MMUD Explorer settings as its config file holds them (frmMME_Export's
// SaveConfig and LoadConfig), and the name it gives the export. Pure.

import type { MmeOptions, MmeRooms } from '../api';

/** NMR's config file: an INI file's [Settings], boxes as 1 or 0. */
export function toIni(o: MmeOptions): string {
  const b = (v: boolean) => (v ? '1' : '0');
  const rooms = o.excluded.map((r) => `${r.map}/${r.from}/${r.to},`).join('');
  const lines = [
    '[Settings]',
    `CustomName=${o.custom}`,
    `Legit=${b(o.legit)}`,
    'Gmud=0',
    `DisableKai=${b(o.disable_kai)}`,
    `ExcludeDefault=${b(o.exclude_default)}`,
    `DB_Ver=${o.dat_version}`,
    `UpdateURL=${o.update_url}`,
    `EnableExcludeRooms=${b(o.exclude_rooms)}`,
    `HideExcludedRooms=${b(o.hide_excluded)}`,
    `NoRooms=${b(o.no_rooms)}`,
    `ExcludedRooms=${rooms || '0'}`,
  ];
  return lines.join('\r\n') + '\r\n';
}

/** Reads NMR's config file over `base`: what the file lacks stays as it was. */
export function fromIni(text: string, base: MmeOptions): MmeOptions {
  const settings = new Map<string, string>();
  let section = '';
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    const head = /^\[(.*)\]$/.exec(line);
    if (head) {
      section = head[1]!.toLowerCase();
      continue;
    }
    const eq = line.indexOf('=');
    if (section === 'settings' && eq > 0) settings.set(line.slice(0, eq).trim().toLowerCase(), line.slice(eq + 1));
  }
  const o = { ...base, excluded: [...base.excluded] };
  const s = (k: string) => settings.get(k.toLowerCase());
  const flag = (k: string, d: boolean) => {
    const v = s(k);
    return v === undefined ? d : v.trim() === '1';
  };
  // NMR writes "0" for an empty name, version or URL
  const said = (k: string, d: string) => {
    const v = s(k);
    return v === undefined || v === '0' ? d : v;
  };
  o.custom = said('CustomName', o.custom);
  o.dat_version = said('DB_Ver', o.dat_version);
  o.update_url = said('UpdateURL', o.update_url);
  o.legit = flag('Legit', o.legit);
  o.disable_kai = flag('DisableKai', o.disable_kai);
  o.exclude_default = flag('ExcludeDefault', o.exclude_default);
  o.exclude_rooms = flag('EnableExcludeRooms', o.exclude_rooms);
  o.hide_excluded = flag('HideExcludedRooms', o.hide_excluded);
  o.no_rooms = flag('NoRooms', o.no_rooms);
  const rooms = s('ExcludedRooms');
  if (rooms !== undefined) {
    o.excluded = [];
    // "map/from/to," each; NMR reads only the entries that end in a comma
    for (const entry of rooms.split(',').slice(0, -1)) {
      const [map, from, to] = entry.split('/', 3).map((n) => Number(n.trim()));
      if (map !== undefined && from !== undefined && to !== undefined && [map, from, to].every(Number.isFinite)) o.excluded.push({ map, from, to });
    }
  }
  return o;
}

/** NMR's name for the export: `data-` and the BBS name without its spaces. */
export function fileName(o: MmeOptions, ext: string): string {
  const name = o.custom.replace(/ /g, '') || o.dat_version.replace(/ /g, '') || 'mme';
  return `data-${name}.${ext}`;
}

/** A run of rooms as typed (the end blank for one room), or why it is not one yet. */
export function roomsOf(map: string, from: string, to: string): MmeRooms | { error: string } {
  const n = (s: string) => (/^\s*\d+\s*$/.test(s) ? Number(s) : NaN);
  const r = { map: n(map), from: n(from), to: to.trim() ? n(to) : n(from) };
  if (!(r.map > 0)) return { error: 'Give the map the rooms are on.' };
  if (!(r.from > 0) || !(r.to > 0)) return { error: 'Give the first room, and the last if there are more.' };
  if (r.to < r.from) return { error: `${r.to} comes before ${r.from}: give the first room, then the last.` };
  return r;
}
