// The server's editor API (crates/daydream-server/src/tables.rs).

import type { Ref } from './library/script';
import type { References } from './library/where';
import type { Dig, MapAnswer, MapFile, MapNote, PinCell, SavedStamp, StampsFile } from './world/model';
import type { StampRequest } from './world/stamps';

export type Cell = number | string | null;

export interface ColumnInfo {
  name: string;
  type: 'long' | 'double' | 'text' | 'memo';
  size: number | null;
  /** The whole numbers it holds, when its record element limits them. */
  range?: [number, number] | null;
}

export interface TableInfo {
  name: string;
  /** The index columns, in key order. */
  key: string[];
  /** The column a row list shows beside the key. */
  title: string | null;
  columns: ColumnInfo[];
}

export interface RowList {
  table: string;
  key: string[];
  /** Each row's key and title. */
  rows: [Cell[], Cell][];
}

export interface Row {
  table: string;
  key: Cell[];
  values: Record<string, Cell>;
  /**
   * What it was when the world was last published (or read from its game): null when it is
   * as it was then, `added` when it is new since, or the columns that changed, as they were.
   */
  published?: { added: true } | { was: Record<string, Cell> } | null;
}

export interface Report {
  table: string;
  inserted: number;
  updated: number;
  unchanged: number;
  skipped: number;
  deleted: number;
  errors: string[];
}

/** One value a Workshop run changes (`daydream_nmr::modify::Change`). */
export interface PlanChange {
  key: Cell[];
  column: string;
  before: Cell;
  after: Cell;
}

/** What a Workshop run (a bulk edit, one of NMR's fixes) changes: previewed, or written. */
export interface Plan {
  dry_run: boolean;
  table: string;
  /** Values that change. */
  changed: number;
  /** Records they are in. */
  records: number;
  /** Records the run's conditions left out. */
  skipped: number;
  /** Records already as the run would make them. */
  unchanged: number;
  /** Records worth a look, left as they are. */
  notes: string[];
  /** The changes, the first 2,000 of them. */
  changes: PlanChange[];
  report: Report | null;
  /** The step of the history a write made (none for a preview). */
  step?: number | null;
}

/** A run over several tables (a renumbering, a move of monster groups and indexes): each
 * table's plan, in the order they are written, and the one step of the history they make. */
export interface Plans {
  dry_run: boolean;
  tables: Plan[];
  step?: number | null;
}

/** A Workshop run's request: previewed (`dry_run`) first, then written. */
export type RunBody = Record<string, unknown>;

/** What NMR's Database Importer brings in (`daydream_nmr::ImportOptions`): the tables, a
 * run of numbers for some, whether records the world has are left alone, and which parts of
 * rooms. */
export interface ImportNmr {
  tables: string[];
  ranges: { table: string; map?: number; from: number; to: number }[];
  skip_existing: boolean;
  room_parts: 'all' | 'only_items' | 'not_items';
  extra_parts: boolean;
}

/** What a write back to the WCC files does to one of them (`POST /api/tools/write-dat`). */
export interface WrittenFile {
  stem: string;
  dest: string;
  /** `written`: rows of a game's own .db files, changed in place. */
  action: 'unchanged' | 'updated' | 'rebuilt' | 'written';
  records: number | null;
  /** Only the world's edits went in, onto the file as the game has it. */
  merged: boolean;
  /** Edits left out (the game has made them impossible meanwhile), and why. */
  skipped: string[];
}

/** One of silvermere's tables, written or read back. */
export interface SilvermereTable {
  table: string;
  inserted: number;
  updated: number;
  unchanged: number;
  deleted: number;
}

/** What a silvermere database holds, and whether silvermere runs on it. */
export interface SilvermereInfo {
  url: string;
  running: string | null;
  tables: [string, number][];
  raw: boolean;
}

/** A run of rooms MMUD Explorer's export leaves out. */
export interface MmeRooms {
  map: number;
  from: number;
  to: number;
}

/** NMR's Export to MMUD Explorer boxes (`daydream_nmr::mme::MmeOptions`). */
export interface MmeOptions {
  legit: boolean;
  disable_kai: boolean;
  exclude_default: boolean;
  exclude_rooms: boolean;
  excluded: MmeRooms[];
  hide_excluded: boolean;
  no_rooms: boolean;
  custom: string;
  update_url: string;
  dat_version: string;
}

/** What the MME export form needs first: whether this machine writes a .mdb itself (Jet), the
 * data version, and the rooms NMR leaves out unless asked (`GET /api/mme`). */
export interface MmeInfo {
  jet: boolean;
  dat_version: string;
  defaults: (MmeRooms & { name: string })[];
}

/** One record's new number. */
export interface Move {
  from: number;
  to: number;
}

/** A Record Number Changer run (`daydream_nmr::renumber::Renumber`): records of a table move
 * (rooms within one map), or a whole map's rooms move to another map. */
export type Renumber =
  | { kind: 'records'; table: string; map?: number; moves: Move[]; users: boolean }
  | { kind: 'map'; from: number; to: number; exits: boolean; normal: boolean; textblocks: boolean; spells: boolean; users: boolean };

/** A line of the Monster Group/Index Changer: monsters of one group and index take another. */
export interface IndexChange {
  from_group: number;
  from_index: number;
  to_group: number;
  to_index: number;
}

/** Rooms that spawn one group over the same run of indexes (their Min to Max Index). */
export interface SpawnRun {
  group: number;
  from: number;
  to: number;
  /** Each room's map and number. */
  rooms: [number, number][];
}

export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}

/** `quiet`: a write that leaves the realm be (the map's side file), so nothing need be read again. */
async function call<T>(method: string, url: string, body?: unknown, quiet = false): Promise<T> {
  // a file goes as it is; anything else as JSON
  const raw = body instanceof Blob;
  const res = await fetch(url, {
    method,
    headers: body === undefined || raw ? {} : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : raw ? body : JSON.stringify(body),
  });
  if (res.status === 401) {
    // the server has an access token, and this browser's sign-in is gone
    location.href = '/login';
  }
  const text = await res.text();
  let json: unknown = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    // not JSON: an error page
  }
  if (!res.ok) {
    const msg = (json as { error?: string } | null)?.error ?? (text || res.statusText);
    throw new ApiError(res.status, msg);
  }
  // the frame keeps its changes list and history current
  if (method !== 'GET' && !quiet) window.dispatchEvent(new CustomEvent('dd:wrote'));
  return json as T;
}

/** A file the server makes from a JSON request, to save. */
async function fileOf(url: string, body: unknown): Promise<Blob> {
  const res = await fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
  if (res.status === 401) location.href = '/login';
  if (!res.ok) {
    const text = await res.text();
    let msg = text || res.statusText;
    try {
      msg = (JSON.parse(text) as { error?: string }).error ?? msg;
    } catch {
      // not JSON: an error page
    }
    throw new ApiError(res.status, msg);
  }
  return res.blob();
}

/** A record that uses a message (`/api/messages/users`). */
export interface MessageUser {
  table: string;
  key: Cell[];
  title: Cell;
}

/** The other records that use a message: how many, by table, and the first few. */
export interface SharedWith {
  others: number;
  tables: Record<string, number>;
  some: MessageUser[];
}

/** Where a monster group spawns (`/api/spawns`). */
export interface Ladder {
  group: number;
  /** The group's monsters: number, name and Index, by Index then number. */
  monsters: [number, string, number][];
  /** Its spawning rooms by their Index range, with how many are on each map and have each name. */
  ranges: { min: number; max: number; rooms: number; maps: [number, number][]; names: [string, number][] }[];
  /** Its rooms that spawn nothing (no Max Index, no Max Regen, or a type that never spawns). */
  idle: number;
}

/** One record changed since the realm was last read from its game or published (`/api/changes`). */
export interface Change {
  stem: string;
  /** Its record id in the `.db` file. */
  id: number;
  /** The record type (`items`), or null for a file Daydream does not decode. */
  table: string | null;
  how: 'added' | 'changed' | 'deleted';
  /** The record for a person: `item 12`, `room 1/5`. */
  what: string;
  name: string | null;
  /** The members a changed record changed. */
  members: string[];
  /** The same, as the editors label their columns (`Max Hit`); null where no editor shows it. */
  labels: string[] | null;
  /** What a renumbered record was. */
  was: string | null;
  /** The editors' table and key for it, or null. */
  nmr: string | null;
  key: Cell[] | null;
}

export interface ChangeList {
  /** When the realm was read or last published (seconds since 1970); null when not known. */
  since: number | null;
  changes: Change[];
}

/** One save in the realm's history (`/api/history`). */
export interface Step {
  step: number;
  /** Seconds since 1970. */
  at: number;
  label: string;
  records: number;
  state: 'done' | 'undone' | 'too_big';
}

export interface History {
  steps: Step[];
  /** The steps the next undo and redo take. */
  undo: Step | null;
  redo: Step | null;
}

/** One step of publishing, as the plan lists it and a job runs it. */
export interface PublishStep {
  id: string;
  title: string;
  detail: string;
  /** `yours` is for the sysop to do (start the game again). */
  state: 'waiting' | 'running' | 'done' | 'failed' | 'skipped' | 'yours';
  log: string[];
  /** When the countdown under way ends, in seconds since 1970. */
  until?: number;
  /** The step may be called off now (a countdown, or waiting for the sysop). */
  cancel?: boolean;
}

/** `live`: into a running silvermere, its saves held for a moment (no downtime). */
export type PublishWayId = 'update' | 'now' | 'live';

/** A way of publishing into the open world's game (`/api/publish`). */
export interface PublishWay {
  id: PublishWayId;
  title: string;
  summary: string;
  button: string;
  ready: boolean;
  /** Why it cannot be taken, when it cannot. */
  why: string | null;
  /** The changes the update file cannot carry (`item 12: ...`). */
  not_carried: string[];
  steps: PublishStep[];
}

export interface PublishCheck {
  level: 'ok' | 'note' | 'warn' | 'block';
  text: string;
}

/** A publish under way, or the last one. */
export interface PublishJob {
  id: number;
  way: PublishWayId;
  engine: string;
  game: string;
  state: 'running' | 'done' | 'failed' | 'cancelled';
  started: number;
  finished: number | null;
  steps: PublishStep[];
  /** Edits that could not be written: they stay among the changes. */
  skipped: string[];
  error: string | null;
  result: string | null;
}

export interface PublishPlan {
  /** `mbbsemu` and `db`: a game's own .db files (MBBSEmu's, or wbtrv32's), worked on in a copy. */
  engine: 'worldgroup' | 'majorbbs' | 'mbbsemu' | 'db' | 'silvermere' | 'files';
  /** The game, as a sysop calls it ("Worldgroup"). */
  name: string;
  game: string | null;
  location: string | null;
  running: { running: boolean | null; by?: string | null; error?: string; elsewhere: string[] };
  changes: number;
  since: number | null;
  ways: PublishWay[];
  /** The way to offer first. */
  way: PublishWayId | null;
  checks: PublishCheck[];
  /** silvermere's remote control, when the world is silvermere's. */
  control: GameControl | null;
  job: PublishJob | null;
}

/** A player in silvermere's game, as its admin API lists them. */
export interface Player {
  name: string;
  level: number;
  class: string;
  race: string;
  idle_secs: number;
}

/** Running the game from Daydream: silvermere's admin API and what starts it (`/api/control`). */
export interface GameControl {
  engine: string | null;
  running?: boolean;
  error?: string | null;
  /** Its admin API, and where Daydream learned it: `set` by hand, `silvermere.toml`, `default`, `unknown`. */
  admin?: string | null;
  admin_from?: 'set' | 'silvermere.toml' | 'default' | 'unknown';
  /** Its `/healthz` answered. */
  answers?: boolean;
  token?: boolean;
  token_state?: 'none' | 'unchecked' | 'ok' | 'unknown' | 'no-read';
  token_text?: string;
  players?: Player[] | null;
  planned?: { at: string; in_secs: number } | null;
  /** It takes a publish while it runs (`/v1/world-hold`, silvermere 1.0.5 and later). */
  holds?: boolean;
  /** What starts it again, when Daydream found it. */
  start?: { kind: 'systemd-user' | 'systemd' | 'launchd' | 'process'; command: string; can: boolean } | null;
  /** A publish starts it again. */
  autostart?: boolean;
  /** The admin API's address may be changed here. */
  custom?: boolean;
}

/** How silvermere warns the players before it stops for a publish. */
export interface PublishWarn {
  in_secs: number;
  message: string;
}

const t = (table: string) => `/api/tables/${encodeURIComponent(table)}`;
const k = (key: Cell[]) => `?key=${encodeURIComponent(JSON.stringify(key))}`;

/** One of the checks that read the wiring across the world (`/api/checks`). */
export type CheckId = 'prices' | 'spelling' | 'placeholders' | 'messages' | 'spawns';

/** A check, and how many of its findings are at each level. */
export interface CheckInfo {
  id: CheckId;
  title: string;
  /** What it reads, in a sentence. */
  about: string;
  warn: number;
  note: number;
  ok: number;
}

/**
 * What a check found about one record: something players meet (`warn`), something worth a
 * look (`note`), or what was checked and holds (`ok`), in words with the records it rests on.
 */
export interface Finding {
  check: CheckId;
  level: 'warn' | 'note' | 'ok';
  about: { table: string; key: number[] };
  column: string | null;
  bits: (string | { ref: Ref })[];
  refs: { table: string; key: number[] }[];
  /** The values that put it right, when there is a sure one. */
  fix: Record<string, string>;
  /** Spelling: the word, and what it may be (likeliest first). */
  word: string | null;
  suggest: string[];
  /** It all as plain text. */
  text: string;
}

/** A health check across the world (`/api/checks/health`): what is wrong that has a sure fix,
 * and counts worth knowing. */
export type HealthId = 'uses' | 'blank_blows' | 'exits' | 'drop_uses' | 'leftovers' | 'spaces' | 'limited';

export interface Health {
  id: HealthId;
  /** Players meet it, worth a tidy, a count to know, or nothing found. */
  level: 'warn' | 'note' | 'info' | 'ok';
  /** The records it is about: for a check with a fix, the records the fix changes. */
  records: number;
  /** The values it is about (uses, messages, slots, exits). */
  values: number;
  /** The first of those records. */
  list: { table: string; key: number[] }[];
  /** What its fix could not put right, as NMR logs it. */
  notes: string[];
  /** The history's name for its fix; null when it has no sure fix. */
  fix: string | null;
}

/** What putting a health check right did (or, as a preview, would do). */
export interface HealthFix {
  dry_run: boolean;
  check: HealthId;
  records: number;
  values: number;
  tables: { table: string; records: number; values: number }[];
  notes: string[];
  step: number | null;
}

/** Where to look for an item's copies (NMR's Scan Rooms, Shops and Users). */
export interface Scan {
  rooms: boolean;
  shops: boolean;
  users: boolean;
}

/** One place an item is (`/api/reports/*`): a room's floor, hidden in a room, a shop's stock
 * or a user's inventory, and how many are there. */
export interface ItemPlace {
  kind: 'room' | 'hidden' | 'shop' | 'user';
  /** Map and room (`1/224`), a shop's number, or a user's ID. */
  key: string;
  name: string;
  count: number;
}

/** A limited item, and where its copies are. */
export interface LimitedItem {
  number: number;
  name: string;
  limit: number;
  places: ItemPlace[];
  in_use: number;
  over: boolean;
}

/** A control room, and the rooms on its map that name it. */
export interface ControlRoom {
  map: number;
  room: number;
  name: string;
  rooms: number[];
}

/** A monster group and index, and its monsters (number, name), at most 15 as NMR keeps. */
export interface GroupIndex {
  group: number;
  index: number;
  monsters: [number, string][];
}

/** A room's permanent NPC. */
export interface NpcRoom {
  map: number;
  room: number;
  room_name: string;
  monster: number;
  monster_name: string;
}

const scanned = (s: Scan) => `rooms=${s.rooms}&shops=${s.shops}&users=${s.users}`;

export interface Checks {
  checks: CheckInfo[];
  /** The findings the ask matched, of which `findings` is a page. */
  total: number;
  findings: Finding[];
  /** The words the world uses on purpose, which spelling leaves alone. */
  words: string[];
}

export const api = {
  realm: () => call<{ dir: string }>('GET', '/api/realm'),
  tables: () => call<{ tables: TableInfo[] }>('GET', '/api/tables').then((r) => r.tables),
  rows: (table: string) => call<RowList>('GET', `${t(table)}/rows`),
  /** Each row's key, title and the values of `columns`; with `nonzero`, only the rows where
   * one of those columns holds a number other than 0 or some text, and with `match`, the rows
   * where one of its columns holds one of its numbers (with both, the rows either picks). */
  rowsWith: (table: string, columns: string[], nonzero: string[] = [], match?: { columns: string[]; values: number[] }) =>
    call<{ rows: [Cell[], Cell, Cell[]][] }>(
      'GET',
      `${t(table)}/rows?columns=${encodeURIComponent(columns.join(','))}${nonzero.length ? `&nonzero=${encodeURIComponent(nonzero.join(','))}` : ''}${
        match ? `&match=${encodeURIComponent(match.columns.join(','))}&values=${match.values.join(',')}` : ''
      }`,
    ),
  row: (table: string, key: Cell[]) => call<Row>('GET', `${t(table)}/row${k(key)}`),
  /** A row, or no values when the world lacks it (rather than a 404). */
  rowIfAny: (table: string, key: Cell[]) =>
    call<Omit<Row, 'values'> & { values: Row['values'] | null }>('GET', `${t(table)}/row${k(key)}&missing=null`),
  /** Where a monster group spawns: its monsters by Index, and its rooms by Index range. */
  spawns: (group: number) => call<Ladder>('GET', `/api/spawns?group=${group}`),
  /** The rows of every table that name a record; for a message, what each use reads. */
  references: (table: string, key: Cell[]) =>
    call<References>('GET', `/api/references?table=${encodeURIComponent(table)}&key=${encodeURIComponent(JSON.stringify(key))}`),
  /** The records besides `table`'s record `key` that use each message, a record once (at
   * most 64 numbers at a time). */
  messageUsers: (numbers: number[], table: string, key: Cell[]) =>
    call<{ users: Record<string, SharedWith> }>(
      'GET',
      `/api/messages/users?numbers=${numbers.join(',')}&table=${encodeURIComponent(table)}&key=${encodeURIComponent(JSON.stringify(key))}`,
    ),
  /** Gives a record a copy of a message it shares, its other users' names swapped for its own,
   * and points its columns that named the message at the copy: one save. */
  ownMessage: (table: string, key: Cell[], number: number) =>
    call<{ number: number; lines: string[]; columns: string[]; swapped: number }>('POST', '/api/messages/own', { table, key, number }),
  /** The checks that read the wiring: each with its counts, and a page of findings, warnings first. */
  checks: (q: { check?: CheckId; level?: Finding['level'][]; offset?: number; limit?: number } = {}) =>
    call<Checks>(
      'GET',
      `/api/checks?${new URLSearchParams({
        ...(q.check ? { check: q.check } : {}),
        ...(q.level ? { level: q.level.join(',') } : {}),
        offset: String(q.offset ?? 0),
        limit: String(q.limit ?? 50),
      })}`,
    ),
  /** The findings about one record, or resting on it. */
  checksOf: (table: string, key: Cell[]) =>
    call<{ findings: Finding[] }>('GET', `/api/checks/record?table=${encodeURIComponent(table)}&key=${encodeURIComponent(JSON.stringify(key))}`),
  /** The health checks, in the order Home shows them. */
  health: () => call<{ checks: Health[] }>('GET', '/api/checks/health'),
  /** Puts a health check right as one step of the history. */
  healthFix: (check: HealthId, dryRun = false) => call<HealthFix>('POST', '/api/checks/health/fix', { check, dry_run: dryRun }, dryRun),
  /** NMR's lists: every limited item and where its copies are. */
  limitedItems: (scan: Scan) => call<{ items: LimitedItem[] }>('GET', `/api/reports/limited-items?${scanned(scan)}`),
  /** Every place one item is. */
  findItem: (item: number, scan: Scan) => call<{ item: number; places: ItemPlace[] }>('GET', `/api/reports/find-item?item=${item}&${scanned(scan)}`),
  controlRooms: () => call<{ rooms: ControlRoom[] }>('GET', '/api/reports/control-rooms'),
  monsterGroups: () => call<{ groups: GroupIndex[] }>('GET', '/api/reports/monster-groups'),
  npcRooms: () => call<{ rooms: NpcRoom[] }>('GET', '/api/reports/npc-rooms'),
  /** The runs of indexes rooms spawn, each with its rooms. */
  spawnRuns: () => call<{ runs: SpawnRun[] }>('GET', '/api/reports/room-indexes'),
  /** Spelling leaves a word alone from now on (or, with `accept` false, checks it again). */
  acceptWord: (word: string, accept = true) => call<{ words: string[] }>('POST', '/api/checks/words', { word, accept }, true),
  save: (table: string, key: Cell[], values: Record<string, Cell>) =>
    call<Report>('PUT', `${t(table)}/row${k(key)}`, { values }),
  insert: (table: string, values: Record<string, Cell>) =>
    call<Report>('POST', `${t(table)}/row`, { values }),
  remove: (table: string, key: Cell[]) => call<Report>('DELETE', `${t(table)}/row${k(key)}`),
  changes: () => call<ChangeList>('GET', '/api/changes'),
  /** One map's rooms, and the realm's maps. */
  map: (map: number) => call<MapAnswer>('GET', `/api/map?map=${map}`),
  /** NMR's map editor's dig: a new room (or a line of them), or a room joined, an exit each way; answers the step of the history it made. */
  dig: (dig: Dig) => call<{ notes: string[]; step: number | null }>('POST', '/api/map/dig', dig),
  /** A stamp from the World map: its rooms and exits as one step of the history, which it answers. */
  stamp: (s: StampRequest) =>
    call<{ notes: string[]; stamp: { rooms: number; exits: number; joined: number; left: number }; step: number | null }>('POST', '/api/map/stamp', s),
  /** Sets fields on rooms picked on the World map, as one step of the history: NMR's Mass Room Editor, on those rooms alone. */
  mapSet: (b: { map: number; rooms: number[]; values: [string, string | number][]; what: string }) =>
    call<{ changed: number; records: number; unchanged: number; notes: string[] }>('POST', '/api/map/set', b),
  /**
   * Where rooms of one map moved by hand are drawn (`null`: where the layout puts them; `replace`
   * puts the map's others back too). Kept beside the working copy, not in the realm. Answers the
   * realm history's newest step.
   */
  /** Places of a map's rooms moved by hand: flat, or with `levels` in levels (and the map is drawn so from then on). */
  pins: (map: number, pins: Record<string, PinCell | null>, replace = false, levels = false) =>
    call<{ pins: number; step: number }>('POST', '/api/map/pins', { map, pins, replace, levels }, true),
  /** Every map's rooms moved by hand: the file to export. */
  mapFile: () => call<MapFile>('GET', '/api/map/pins'),
  /** An exported file: each map it has takes its places. */
  importMapFile: (file: MapFile) => call<{ maps: number; pins: number; notes: number; step: number }>('POST', '/api/map/pins/import', file, true),
  /** Notes on one map written (or put in place of those with their ids), and some taken away: answers how many it has now. */
  notes: (map: number, set: MapNote[], drop: string[] = []) => call<{ notes: number; step: number }>('POST', '/api/map/notes', { map, set, drop }, true),
  /** The stamps saved from the map, kept beside the working copy as its places are. */
  savedStamps: () => call<StampsFile>('GET', '/api/map/stamps'),
  /** Saves a stamp: a new one (no id) takes the next id, and one with an id replaces it or comes back. Answers the realm history's newest step. */
  saveStamp: (s: Omit<SavedStamp, 'id'> & { id?: string }) => call<{ stamp: SavedStamp; stamps: number; step: number }>('POST', '/api/map/stamps', s, true),
  dropStamp: (id: string) => call<{ stamps: number; step: number }>('DELETE', `/api/map/stamps/${encodeURIComponent(id)}`, undefined, true),
  publishPlan: () => call<PublishPlan>('GET', '/api/publish'),
  publish: (way: PublishWayId, warn?: PublishWarn) => call<PublishJob>('POST', '/api/publish', { way, warn }),
  publishJob: () => call<PublishJob | null>('GET', '/api/publish/job'),
  cancelPublish: () => call<{ cancelled: boolean }>('POST', '/api/publish/cancel', {}),
  control: () => call<GameControl>('GET', '/api/control'),
  /** Changes the open game's admin API, token or whether a publish starts it again (`""` clears). */
  setControl: (body: { admin?: string; token?: string; autostart?: boolean }) => call<GameControl>('PUT', '/api/control', body),
  /** Tells everyone in the game. */
  say: (text: string) => call<{ delivered: number }>('POST', '/api/control/say', { text }),
  /** Puts records back as they were when the realm was last read or published. */
  revert: (body: { changes: { stem: string; id: number }[] } | { all: true }) =>
    call<{ reverted: number; failed: { stem: string; id: number; why: string }[] }>('POST', '/api/changes/revert', body),
  history: (limit = 20) => call<History>('GET', `/api/history?limit=${limit}`),
  undo: (step?: number) => call<Step>('POST', '/api/undo', step === undefined ? {} : { step }),
  redo: (step?: number) => call<Step>('POST', '/api/redo', step === undefined ? {} : { step }),
  /** A Universal Modifier run (`daydream_nmr::modify::Modify`): previewed, or written as one
   * step of the history, called `label` there (or what the server calls it). */
  modify: (body: RunBody, dryRun: boolean, label?: string) =>
    call<Plan>('POST', '/api/tools/modify', { ...body, dry_run: dryRun, label }, dryRun),
  /** One of NMR's fixes (`daydream_nmr::presets::Preset`): previewed, or written as one step. */
  fix: (body: RunBody, dryRun: boolean, label?: string) =>
    call<Plan>('POST', '/api/tools/preset', { ...body, dry_run: dryRun, label }, dryRun),
  /** The Record Number Changer: previewed, or written as one step, called `label` there. */
  renumber: (body: Renumber, dryRun: boolean, label?: string) =>
    call<Plans>('POST', '/api/tools/renumber', { ...body, dry_run: dryRun, label }, dryRun),
  /** The Monster Group/Index Changer: the monsters, then the rooms that spawn them, as one step. */
  moveIndexes: (changes: IndexChange[], dryRun: boolean, label?: string) =>
    call<Plans>('POST', '/api/tools/monster-indexes', { changes, dry_run: dryRun, label }, dryRun),
  /** NMR's Database Importer: an export (`.mdb` or NMR's JSON) brought in, previewed or as one step. */
  importNmr: (file: Blob, options: ImportNmr, dryRun: boolean) =>
    call<{ dry_run: boolean; reports: Report[]; step?: number | null }>(
      'POST',
      `/api/tools/import?options=${encodeURIComponent(JSON.stringify({ ...options, dry_run: dryRun }))}`,
      file,
      dryRun,
    ),
  /** Merge Users: another user file's users copied in, previewed or as one step. */
  mergeUsers: (file: Blob, dryRun: boolean) =>
    call<{ dry_run: boolean; copied: number; skipped: string[]; step?: number | null }>('POST', `/api/tools/merge-users?dry_run=${dryRun}`, file, dryRun),
  /** Writes the world's edits into a folder of WCC files (none given: the open game's); then
   * the world's changes are counted from what the files hold. */
  writeDat: (body: { wcc: string | null; out: string | null; backup: boolean; force: boolean }, dryRun: boolean) =>
    call<{ dry_run: boolean; files: WrittenFile[]; backup?: string | null }>('POST', '/api/tools/write-dat', { ...body, dry_run: dryRun }, dryRun),
  /** The silvermere database the server knows (password masked), and whether another may be typed. */
  silvermere: () => call<{ database: string | null; custom: boolean }>('GET', '/api/silvermere'),
  silvermereInfo: (database: string | null) => call<SilvermereInfo>('POST', '/api/silvermere/info', { database }, true),
  /** Writes the world into a silvermere database; it is not a step of the world's history. */
  silvermereExport: (body: { database: string | null; reset_state: boolean; keep_extra: boolean; no_raw: boolean }, dryRun: boolean) =>
    call<{ dry_run: boolean; tables: SilvermereTable[] }>('POST', '/api/silvermere/export', { ...body, dry_run: dryRun }, dryRun),
  /** Rewrites the world from a silvermere database, previewed or as one step. */
  silvermereImport: (body: { database: string | null; keep_extra: boolean }, dryRun: boolean) =>
    call<{ dry_run: boolean; tables: SilvermereTable[]; step?: number | null }>('POST', '/api/silvermere/import', { ...body, dry_run: dryRun }, dryRun),
  mmeInfo: () => call<MmeInfo>('GET', '/api/mme'),
  /** The MME export's tables and how many rows each would hold; the world is only read. */
  mmeSummary: (options: MmeOptions) => call<{ tables: { name: string; rows: number }[] }>('POST', '/api/mme/export', { options, format: 'summary' }, true),
  /** The MME export as a file: the .mdb (with Jet), or the plan write-mdb.ps1 turns into one. */
  mmeFile: (options: MmeOptions, format: 'plan' | 'mdb') => fileOf('/api/mme/export', { options, format }),
  /** Deletes a table's records: a range of them, or `all`. Previewed, or written as one step. */
  deleteRecords: (body: { table: string; range: { map?: number; from: number; to: number } | null; all: boolean }, dryRun: boolean) =>
    call<{ dry_run: boolean; report: Report; step?: number | null }>('POST', '/api/tools/delete', { ...body, dry_run: dryRun }, dryRun),
  /** One of NMR's fixes (`daydream_nmr::presets::Preset`), written straight away. */
  preset: (body: Record<string, unknown>) => call<{ changes?: unknown[] }>('POST', '/api/tools/preset', { ...body, dry_run: false }),
};
