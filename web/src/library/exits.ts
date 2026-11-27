// A room's exits as the game reads them (silvermere-engine's movement/exits.rs, doors.rs,
// search.rs and timed_exits.rs, display/exits.rs and room.rs, scripts/actions.rs's remote
// actions, and commands/input.rs's typed-exit stage): how "Obvious exits:" names them, what
// each asks of a player who walks that way, and the words a player types for the exits that
// answer words. Each is said in plain words, the records it names as chips (script.ts's Bit).
//
// What is said is the exit as it is stored: the state a world starts with. A door opened in
// play, a hidden way found or a trap disarmed changes only the running game's copy.
//
// Pure: no DOM, so `node --test` runs its tests (exits.test.ts).

import type { Bit, Ref, Role } from './script.ts';

type Cell = number | string | null;
/** A message's three lines. */
export type Lines = [string, string, string];
/** A message's lines: undefined while it is read, null when the world has no such message. */
export type LinesOf = (n: number) => Lines | null | undefined;

/** The ten directions, in the game's order. */
export const DIRS = ['north', 'south', 'east', 'west', 'northeast', 'northwest', 'southeast', 'southwest', 'up', 'down'];
/** The shortest word that walks each way. */
export const SHORT = ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw', 'u', 'd'];
/** Each direction's way back. */
export const BACK = [1, 0, 3, 2, 7, 6, 5, 4, 9, 8];

/** The exit types (EXIT_TYPE). */
export const NORMAL = 0, SPELL = 1, KEY = 2, ITEM = 3, TOLL = 4, ACTION = 5, HIDDEN = 6, DOOR = 7, MAP_CHANGE = 8, TRAP = 9, TEXT = 10, GATE = 11;
export const REMOTE = 12, CLASS = 13, RACE = 14, LEVEL = 15, TIMED = 16, TICKET = 17, USERS = 18, GUARD = 19, ALIGNMENT = 20, DELAY = 21, CAST = 22, ABILITY = 23, SPELL_TRAP = 24;

/** A Hidden exit's flags (`EXF_`): passable, found by searching, shown once found, shown
 * once its blocks are gone; and block 1 (each next block is the next bit, to block 10). */
export const PASSABLE = 0x1, NEEDSEARCH = 0x2, VISSEARCHED = 0x4, VISBLOCKED = 0x8, BLOCK1 = 0x10, BLOCKS = 0x3ff0;
/** The lock of a door, gate or key exit. */
export const [OPEN, SHUT, LOCKED] = [0, 1, 2];
/** A trap's state: armed, disarmed, hidden, and armed or disarmed with a failed disarm
 * throwing the player through. */
export const [ARMED, DISARMED, TRAP_HIDDEN, ARMED_THROWS, DISARMED_THROWS] = [0, 1, 2, 3, 4];
/** The maps a Map Change exit leads into without an add-on. */
export const FREE_MAPS = [1, 5, 7, 10, 11];
/** The exit types nobody may be dragged through (`exit_allows_drag`). */
const NO_DRAG = [SPELL, KEY, ITEM, TOLL, CLASS, RACE, LEVEL, ALIGNMENT];
/** The exit types closed during tournament play. */
const NO_TOURNAMENT = [CLASS, RACE, LEVEL, ALIGNMENT];
const TYPE_NAMES: Record<number, string> = { [TICKET]: 'Ticket', [USERS]: 'User Count', [DELAY]: 'Delay' };

const GREEN = '\x1b[0;32m';
const LTCYAN = '\x1b[1;36m';
const NORMAL_TEXT = '\x1b[0;37;40m';

export interface Exit {
  dir: number;
  /** The room it leads to, on the same map (a Map Change exit's on its map); 0 for none. */
  to: number;
  type: number;
  p1: number;
  p2: number;
  p3: number;
  p4: number;
}

const num = (v: Cell | undefined) => Number(v) || 0;
const wrap16 = (v: number) => ((((v % 65536) + 98304) % 65536) - 32768);
const count = (bits: number) => {
  let n = 0;
  for (let b = bits; b; b &= b - 1) n++;
  return n;
};
const plural = (n: number, one: string, many = `${one}s`) => `${n.toLocaleString('en-US')} ${n === 1 ? one : many}`;
const ref = (t: Ref['t'], n: number, role: Role, map?: number): Bit => ({ ref: map === undefined ? { t, n } : { t, n, map }, role });
/** The game's case folding: ASCII only. */
const upper = (s: string) => s.replace(/[a-z]/g, (c) => c.toUpperCase());
/** `+ 5` or `− 5` after a skill, nothing for 0. */
const plus = (n: number) => (n ? ` ${n > 0 ? '+' : '−'} ${Math.abs(n)}` : '');
/** Block `k` (1 to 10) of a hidden exit's flags. */
const block = (k: number) => BLOCK1 << (k - 1);

/** A room's ten exits, from its values. */
export function exitsOf(get: (col: string) => Cell | undefined): Exit[] {
  return DIRS.map((_, d) => ({
    dir: d,
    to: num(get(`Exit ${d}`)),
    type: num(get(`Type ${d}`)),
    p1: num(get(`Para1 ${d}`)),
    p2: num(get(`Para2 ${d}`)),
    p3: num(get(`Para3 ${d}`)),
    p4: num(get(`Para4 ${d}`)),
  }));
}

/** The map the room an exit leads to is on: a Map Change exit's Para1, else the room's own. */
export const mapOf = (e: Exit, own: number) => (e.type === MAP_CHANGE ? e.p1 : own);

/** A Remote Action exit's code (its Para2): the action (the tens) and the exit of the room it
 * leads to that it works (the units; below 0, none). */
export function remoteOf(e: Exit): { action: number; exit: number } {
  const code = wrap16(e.p2);
  return { action: Math.trunc(code / 10), exit: code % 10 };
}

/** Whether two typed lines are the same to the game: the whole line, ASCII case aside. */
export const sameWords = (a: string, b: string) => upper(a) === upper(b);

// ---- "Obvious exits:" ----------------------------------------------------------------------

/** Where a direction is, as a hidden exit's naming line says it: up and down are above and below. */
const place = (d: number) => (d === 8 ? 'above' : d === 9 ? 'below' : DIRS[d]!);

/** A found hidden exit with no naming message (sic: North's and East's capitals). */
function secretWords(d: number): string {
  const s = `secret passage ${d === 2 ? 'East' : place(d)}`;
  return d === 0 ? `S${s.slice(1)}` : s;
}

/** A door's or gate's words: up and down are trap doors. */
function doorWords(d: number, gate: boolean, open: boolean): string {
  const st = open ? 'open' : 'closed';
  return d >= 8 ? `${st} trap door ${place(d)}` : `${st} ${gate ? 'gate' : 'door'} ${DIRS[d]}`;
}

/** A message line printed with one `%s`: the first takes `arg`, `%%` is a percent sign and
 * any other conversion is left as written (`printf_s`). */
export function printfS(fmt: string, arg: string): string {
  let out = '';
  let used = false;
  for (let i = 0; i < fmt.length; i++) {
    const c = fmt[i]!;
    if (c === '%' && fmt[i + 1] === '%') {
      out += '%';
      i++;
    } else if (c === '%' && fmt[i + 1] === 's' && !used) {
      out += arg;
      used = true;
      i++;
    } else out += c;
  }
  return out;
}

/** Shown although its kind is hidden: a hidden exit found or opened, a trap not hidden, a
 * timed exit while open (`secret_visible_exit`). */
function secretlyShown(e: Exit): boolean {
  if (e.type === HIDDEN) return (e.p1 & (VISSEARCHED | VISBLOCKED)) !== 0;
  if (e.type === TRAP) return wrap16(e.p2) !== TRAP_HIDDEN;
  if (e.type === TIMED) return e.p3 === OPEN;
  return false;
}

/** Whether "Obvious exits:" names it (`visible_exit`). */
export function shown(e: Exit): boolean {
  return secretlyShown(e) || (e.to !== 0 && ![TRAP, REMOTE, HIDDEN, TIMED, TEXT].includes(e.type));
}

/** How "Obvious exits:" names it; undefined while a hidden exit's naming message is read. */
export function exitWord(e: Exit, lines: LinesOf): string | undefined {
  const d = e.dir;
  if (e.type === HIDDEN) {
    if (!e.p4) return secretWords(d);
    const m = lines(e.p4);
    if (m === undefined) return undefined;
    return m ? printfS(m[0], place(d)) : secretWords(d);
  }
  if (e.type === DOOR) return doorWords(d, false, e.p1 === OPEN);
  if (e.type === KEY) return doorWords(d, false, wrap16(e.p2) === OPEN);
  if (e.type === GATE) return doorWords(d, true, e.p1 === OPEN);
  return DIRS[d]!;
}

const bare = (s: string) => s.replace(/\x1b\[[0-9;]*[A-Za-z]?/g, '');

/** Words wrapped at 79 columns, as the game's host wraps a line; escapes take no room. */
export function wrap79(text: string): string {
  return text
    .split('\n')
    .map((para) => {
      const out: string[] = [];
      let line = '';
      let width = 0;
      // each word with the spaces after it
      for (const word of para.match(/[^ ]*( +|$)/g) ?? []) {
        if (!word) continue;
        const w = bare(word).length;
        if (width && width + bare(word).trimEnd().length > 79) {
          out.push(line.trimEnd());
          line = '';
          width = 0;
        }
        line += word;
        width += w;
      }
      out.push(line.trimEnd());
      return out.join('\n');
    })
    .join('\n');
}

/** The line under the room: `Obvious exits: ...` in its colour, or `NONE!!!`; undefined
 * while a hidden exit's naming message is read. */
export function obviousExits(exits: Exit[], lines: LinesOf): string | undefined {
  const words: string[] = [];
  for (const e of exits) {
    if (!shown(e)) continue;
    const w = exitWord(e, lines);
    if (w === undefined) return undefined;
    words.push(w);
  }
  return GREEN + wrap79(`Obvious exits: ${words.length ? words.join(', ') : 'NONE!!!'}`);
}

/** A room as a player arriving reads it: its name, its description (for a player who reads
 * descriptions) and its exits. Undefined while a message is read. */
export function roomScreen(get: (col: string) => Cell | undefined, lines: LinesOf): string | undefined {
  const exits = obviousExits(exitsOf(get), lines);
  if (exits === undefined) return undefined;
  const text = (v: Cell | undefined) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);
  const desc = Array.from({ length: 7 }, (_, i) => text(get(`Desc ${i}`)))
    .filter((l) => l)
    .map((l) => `${l.replace(/ +$/, '')} `)
    .join('');
  return [LTCYAN + wrap79(text(get('Name'))), ...(desc ? [NORMAL_TEXT + wrap79(`    ${desc}`)] : []), exits].join('\n');
}

// ---- ways out ------------------------------------------------------------------------------

export interface Way {
  exit: Exit;
  /** How "Obvious exits:" names it, null when it never does, undefined while read. */
  word: string | null | undefined;
  /** Whether walking that way can get a player through, as the exit is stored. */
  walks: boolean;
  /** What it is and asks, in a few words; nothing for an ordinary way. */
  says: Bit[];
  /** The rest worth knowing, a line each. */
  more: Bit[][];
}

/** `", saying {message}"` for a message, nothing for none. */
const saying = (n: number): Bit[] => (n > 0 ? [', saying ', ref('Messages', n, 'says')] : []);
const lockWord = (lock: number) => (lock === OPEN ? 'open' : lock === SHUT ? 'shut' : lock === LOCKED ? 'locked' : `in lock state ${lock}`);
const steps = (n: number) => (n < 0 ? 'as long as the game can wait' : plural(Math.max(1, n), 'step'));
const levels = (min: number, max: number) => (min && max ? `levels ${min} to ${max}` : min ? `level ${min} and up` : max ? `up to level ${max}` : 'every level');

/** How a locked door, gate or key exit is unlocked: its key, if it has one, and PICKLOCK
 * (`cmd_picklock`, `use_key`), which leave it shut for OPEN. One not locked yet is told
 * for when it locks again. */
function unlocks(key: number, dir: string, bonus: number, locked: boolean): Bit[] {
  const pick = `on a roll of 0 to 99 under their picklocks${plus(bonus)}, if they have any`;
  const once = locked ? '' : 'Once it has locked, ';
  return key
    ? [`${once}USE `, ref('Items', key, 'needs'), ` ${dir} unlocks it, as does PICKLOCK ${dir} ${pick}; then OPEN ${dir}.`]
    : [`${once}${locked ? 'No' : 'no'} key fits it, but PICKLOCK ${dir} unlocks it ${pick}; then OPEN ${dir}.`];
}

/** The exits a room has: every exit leading somewhere, and the hidden ones the game shows
 * with no room behind them. */
export const waysOf = (exits: Exit[]) => exits.filter((e) => e.to !== 0 || (e.type !== NORMAL && shown(e)));

/** What walking an exit asks of a player and does, in plain words. */
export function wayOf(e: Exit, lines: LinesOf): Way {
  const d = e.dir;
  const dir = upper(DIRS[d]!);
  const way: Way = { exit: e, word: shown(e) ? exitWord(e, lines) : null, walks: true, says: [], more: [] };
  const say = (...bits: Bit[]) => way.says.push(...bits);
  const more = (...bits: Bit[]) => way.more.push(bits);
  const wall = () => {
    way.walks = false;
    say('never walked: walking it they meet a wall');
  };
  switch (e.type) {
    case SPELL:
      wall();
      more('Only a player following a party leader is taken through.');
      break;
    case KEY: {
      const lock = wrap16(e.p2);
      say(`a door, ${lockWord(lock)}`, ...(e.p1 ? [', its key ', ref('Items', e.p1, 'needs')] : []));
      if (lock === SHUT) more(`OPEN ${dir} opens it.`);
      if (lock !== OPEN) more('Until it is open, walking it says “There is a closed door in that direction!”.');
      if (lock === LOCKED) more(...unlocks(e.p1, dir, e.p3, true));
      more(`${lock === OPEN ? 'Closed and opened again' : 'Opened'}, it locks again ${steps(e.p4)} later.`);
      // it locks in time whatever it starts as, so its unlocking is worth knowing
      if (lock !== LOCKED) more(...unlocks(e.p1, dir, e.p3, false));
      break;
    }
    case ITEM:
      say('only for those carrying ', ref('Items', e.p1, 'needs'));
      more('It takes them through and uses up one of its uses', ...saying(e.p3), '.');
      more('Without it: ', ...(e.p2 > 0 ? [ref('Messages', e.p2, 'else')] : ['“You do not have the appropriate item to go that direction!”']), '.');
      break;
    case TOLL: {
      const gold = wrap16(e.p1);
      say('a toll of ', { copper: e.p1 * 100 });
      more(`Short of it: “You do not have enough to cover the toll of ${gold} ${gold === 1 ? 'gold crown' : 'gold crowns'}.”. Paying gives a sneaking player away.`);
      break;
    }
    case ACTION:
      if (e.p1) {
        say('walking it says ', ref('Messages', e.p1, 'says'));
        more('Line 1 to them, line 2 to the room they leave and line 3 to the room they reach. A sneaking player nobody sees goes the ordinary way.');
      }
      break;
    case HIDDEN: {
      const f = e.p1;
      const blocks = count(f & BLOCKS);
      const back = Math.abs(wrap16(e.p2));
      if (f & (VISSEARCHED | VISBLOCKED)) say('a secret passage, shown');
      else if (f & PASSABLE) say('a hidden way: not among the exits, but walking it works');
      else if (f & NEEDSEARCH) {
        way.walks = false;
        say(`hidden until SEARCH ${dir} finds it`);
        more('A search finds it on a roll of 0 to 99 under their perception less 15 (at least 3). Found, it is shown and open for a step, then hidden again.');
      } else if (blocks) {
        way.walks = false;
        say(`blocked, by ${plural(blocks, 'block')}`);
        more(`Remote actions and scripts clear its blocks${wrap16(e.p2) < 0 ? ', in any order' : ', from the highest down'}. With none left it opens`, ...(e.p3 > 0 ? [', saying ', ref('Messages', e.p3, 'says')] : [`: “A concealed passage opens to the ${DIRS[d]}.”`]), '.');
        more(back >= 1 && back <= 10 ? `Opened, it is blocked again a step later, by ${plural(back, 'block')}.` : 'Opened, it stays open.');
      } else wall();
      break;
    }
    case DOOR:
    case GATE: {
      const what = e.type === GATE ? 'gate' : 'door';
      const lock = wrap16(e.p1);
      const bonus = wrap16(e.p2);
      say(`a ${what}, ${lockWord(lock)}`, ...(e.p4 ? [', its key ', ref('Items', e.p4, 'needs')] : []));
      // above 0, its bonus keeps it from ever locking again (`revert_exit`)
      const relocks = bonus <= 0;
      if (lock === SHUT) more(`OPEN ${dir} opens it. Walking it shut says “The ${what} is Closed!”.`);
      if (lock === LOCKED) more(`Walking it locked says “The ${what} is closed!”.`);
      if (lock === LOCKED) more(...unlocks(e.p4, dir, bonus, true));
      if (lock !== OPEN) more(`BASH ${dir} breaks it open on a roll of 0 to 99 under their strength${plus(bonus)}.`);
      more(relocks ? `${lock === OPEN ? 'Closed and opened again' : 'Opened'}, it locks again ${steps(e.p3)} later.` : 'Once open, it never locks again.');
      if (lock !== LOCKED && relocks) more(...unlocks(e.p4, dir, bonus, false));
      break;
    }
    case MAP_CHANGE:
      say(`into map ${e.p1}`);
      if (!FREE_MAPS.includes(e.p1)) more('An add-on’s map: the game lets them through only when the room is in the world.');
      break;
    case TRAP: {
      const st = wrap16(e.p2);
      const hurt = `${Math.floor(e.p1 / 2)} to ${e.p1}`;
      if (st === TRAP_HIDDEN) {
        way.walks = false;
        say('a hidden trap: not among the exits');
        more(`Only a remote action throws players through it, hurting them ${hurt}`, ...saying(e.p3), '.');
      } else if (st === DISARMED || st === DISARMED_THROWS) say('a trap, disarmed: an ordinary way');
      else if (st === ARMED || st === ARMED_THROWS) {
        say(`a trap: walking it hurts them ${hurt}`, ...saying(e.p3), ', and through they go');
        more(`DISARM, with ${dir} as its third word, beats it for a step on a roll of 0 to 99 under their disarm skill. Missing by 10 or more sets it off`, ...saying(e.p4), st === ARMED_THROWS ? ', throwing them through.' : `, hurting them ${hurt}.`);
      } else {
        way.walks = false;
        say(`a trap in state ${st}: walking it nothing happens`);
      }
      break;
    }
    case TEXT:
      wall();
      more('Typing its words takes them through.');
      break;
    case REMOTE:
      wall();
      more('Not a way out: its words work an exit.');
      break;
    case CLASS:
    case RACE: {
      const t = e.type === CLASS ? 'Classes' : 'Races';
      if (e.p1) say('only for ', ref(t, e.p1, 'needs'));
      else if (e.p2) say('not for ', ref(t, wrap16(e.p2), 'needs'));
      more('Refused: “You may not go through this exit!”.');
      break;
    }
    case LEVEL: {
      const [min, max] = [wrap16(e.p1), wrap16(e.p2)];
      say(`only ${levels(min, max)}`);
      const refused = [...(min ? ['Too low: “You have not progressed far enough to go through this exit!”.'] : []), ...(max ? ['Too high: “You have progressed too far to go through this exit!”.'] : [])];
      if (refused.length) more(refused.join(' '));
      break;
    }
    case ALIGNMENT:
      say(`only for ${wrap16(e.p1)} to ${wrap16(e.p2)} evil points`);
      more('Too few: “You are too good to go through this exit!”. Too many: “You are too evil to go through this exit!”.');
      break;
    case TIMED:
      say(`opens and shuts by itself every ${steps(wrap16(e.p2))}; it starts ${e.p3 === OPEN ? 'open' : 'shut'}`);
      if (e.p1 > 0) more('Opening it says line 1 of ', ref('Messages', e.p1, 'says'), ', shutting line 2, and walking it shut line 3.');
      else more('Walking it shut says “You may not pass through that exit at this point in time.”.');
      break;
    case CAST:
      if (e.p1) {
        say('casts ', ref('Spells', e.p1, 'casts'), ' on them first');
        more('Felled, held or stunned by it, they stay; otherwise on they go', ...saying(e.p3), '.');
      }
      if (e.p2) more('After any try it casts ', ref('Spells', e.p2, 'casts'), ' too.');
      if (e.p4) more(`LOOK ${dir} shows `, ref('Messages', e.p4, 'says'), ' instead of the room.');
      break;
    case ABILITY:
      say('only with ', ...(e.p1 ? [{ ability: e.p1 } as Bit] : ['an ability']), ` of ${wrap16(e.p2)} to ${e.p3}`);
      more(...(e.p4 > 0 ? ['Refused, it says ', ref('Messages', e.p4, 'says'), ' and they stay.'] : ['Refused, they stay and nothing is said.']));
      break;
    case SPELL_TRAP: {
      const st = wrap16(e.p2);
      if (st === TRAP_HIDDEN) {
        way.walks = false;
        say('a hidden spell trap: not among the exits');
        more('Only a remote action throws players through it.');
      } else if (st === DISARMED || st === DISARMED_THROWS) say('a spell trap, disarmed: an ordinary way');
      else {
        say('a spell trap: walking it casts ', ref('Spells', e.p1, 'casts'), ' on them', ...saying(e.p4), ', and through they go');
        more(`DISARM, with ${dir} as its third word, beats it on a roll of 0 to 99 under their disarm skill`, ...saying(e.p3), '. A miss sets it off.');
      }
      break;
    }
    case GUARD:
      more('A Block Guard exit: players walk it as an ordinary way, and a guard never leaves its group through it.');
      break;
    default:
      if (e.type !== NORMAL) more(`${TYPE_NAMES[e.type] ? `A ${TYPE_NAMES[e.type]}` : `A type ${e.type}`} exit: the game has no rule for it, so it is an ordinary way.`);
  }
  if (NO_TOURNAMENT.includes(e.type)) more('Closed during tournament play.');
  if (NO_DRAG.includes(e.type) && way.walks) more('Nobody can be dragged through.');
  return way;
}

// ---- the words that work -------------------------------------------------------------------

export interface Words {
  exit: Exit;
  /** A Text exit's words move the player through it; a Remote Action's work an exit. */
  kind: 'text' | 'remote';
  /** The words, each a whole line in any case; undefined while read. */
  words: string[] | undefined;
  /** What typing them does. */
  says: Bit[];
  more: Bit[][];
  /** Why some of them never work: an exit the game tries first takes the same words. */
  taken?: string;
}

/** What a remote action did to the exit it worked: a door or gate swapped between open and
 * locked, everyone in a hidden trap's room thrown through, a hidden way's blocks cleared
 * (`left` none: it opens, shown among the exits), or nothing. */
export type Worked = { kind: 'swapped'; exit: Exit } | { kind: 'thrown' } | { kind: 'cleared'; exit: Exit; cleared: number; left: number } | { kind: 'nothing' };

/** A remote action of `action` on exit `e` (`run_room_exit_action`, `hidden_action`): the
 * exit as it leaves it. The same door from the other side is the caller's to swap. */
export function workExit(e: Exit, action: number): Worked {
  switch (e.type) {
    case DOOR:
    case GATE:
      return { kind: 'swapped', exit: { ...e, p1: wrap16(e.p1) === OPEN ? LOCKED : OPEN } };
    case TRAP:
    case SPELL_TRAP:
      return wrap16(e.p2) === TRAP_HIDDEN ? { kind: 'thrown' } : { kind: 'nothing' };
    case HIDDEN: {
      const had = count(e.p1 & BLOCKS);
      let f = e.p1;
      if (action === 0) f &= ~BLOCKS;
      else if (action === 10) f &= ~block(10);
      // block n goes; in order, only once block n + 1 has gone
      else if (action >= 1 && action <= 9 && (wrap16(e.p2) < 0 || !(f & block(action + 1)))) f &= ~block(action);
      const left = count(f & BLOCKS);
      return { kind: 'cleared', exit: { ...e, p1: left ? f : VISBLOCKED }, cleared: had - left, left };
    }
    default:
      return { kind: 'nothing' };
  }
}

/** What a remote action does to the exit it works, as that exit is stored
 * (`run_room_exit_action`); `far` undefined while read, null when there is none. */
function works(far: Exit | null | undefined, action: number, lines: LinesOf): { says: Bit[]; more: Bit[][] } {
  if (far === undefined) return { says: ['…'], more: [] };
  if (far === null) return { says: ['nothing more: there is no exit that way'], more: [] };
  switch (far.type) {
    case DOOR:
    case GATE: {
      const what = far.type === GATE ? 'gate' : 'door';
      if (far.p1 === OPEN) return { says: [`locks the ${what}`], more: [[`The same ${what} from the other side swaps too.`]] };
      return {
        says: [`opens the ${what}`],
        more: [[`The same ${what} from the other side swaps too.`, far.p2 > 0 ? ' It stays open.' : ` It locks again ${steps(far.p3)} later.`]],
      };
    }
    case TRAP:
    case SPELL_TRAP:
      return wrap16(far.p2) === TRAP_HIDDEN
        ? { says: ['throws everyone in that room through the hidden trap, players and then monsters'], more: [] }
        : { says: ['nothing more: only a hidden trap is sprung so'], more: [] };
    case HIDDEN: {
      const blocks = far.p1 & BLOCKS;
      const name = exitWord({ ...far, p1: VISBLOCKED }, lines);
      const opens: Bit[] = [
        blocks ? 'With no block left it opens' : 'It opens',
        ...(far.p3 > 0 ? [', saying ', ref('Messages', far.p3, 'says')] : [`: “A concealed passage opens to the ${DIRS[far.dir]}.”`]),
        name ? `, and shows as “${name}”.` : '.',
      ];
      if (!blocks) return { says: ['opens the hidden way'], more: [opens] };
      const has = (k: number) => (blocks & block(k)) !== 0;
      const inOrder = wrap16(far.p2) >= 0;
      let clears: string;
      if (action === 0) clears = 'clears every block of the hidden way';
      else if (action >= 1 && action <= 10) {
        if (!has(action)) return { says: [`nothing: it clears block ${action}, which the hidden way does not have`], more: [] };
        clears = action <= 9 && inOrder && has(action + 1) ? `clears block ${action} of the hidden way, once block ${action + 1} has gone` : `clears block ${action} of the hidden way`;
      } else return { says: [`nothing: action ${action} clears no block`], more: [] };
      return { says: [clears], more: [opens] };
    }
    default:
      return { says: ['nothing more: that exit is no door, hidden way or trap'], more: [] };
  }
}

/**
 * The words a player types for each Text and Remote Action exit, in the order the game tries
 * them, and what they do. `own` is the room's map and number; `far(n)` the exits of another
 * room `n` on that map (undefined while read, null when the world has no such room).
 */
export function wordsOf(exits: Exit[], own: { map: number; room: number }, lines: LinesOf, far: (room: number) => Exit[] | null | undefined): Words[] {
  const out: Words[] = [];
  const said = new Map<string, number>();
  // every Text exit first, then the Remote Actions, each in direction order
  const order = [...exits.filter((e) => e.type === TEXT), ...exits.filter((e) => e.type === REMOTE)].filter((e) => e.to !== 0);
  for (const e of order) {
    const m = e.p1 ? lines(e.p1) : null;
    // the same words twice are one
    const words = m === undefined ? undefined : m ? [...new Map(m.filter((t) => t.trim()).map((t) => [upper(t), t])).values()] : [];
    const w: Words = { exit: e, kind: e.type === TEXT ? 'text' : 'remote', words, says: [], more: [] };
    if (m === null) w.more.push([e.p1 ? `There is no message ${e.p1}, so no words work it.` : 'It names no message of words, so no words work it.']);
    if (words) {
      const before = words.filter((t) => said.has(upper(t)));
      if (before.length) {
        const first = DIRS[said.get(upper(before[0]!))!];
        w.taken = before.length === words.length ? `The ${first} exit answers the same words first, so these never work.` : `The ${first} exit answers “${before[0]}” first, so that never works here.`;
      }
      for (const t of words) if (!said.has(upper(t))) said.set(upper(t), e.dir);
    }
    if (e.type === TEXT) {
      w.says.push(`takes them ${DIRS[e.dir]} to `, ref('Rooms', e.to, 'moves', own.map), ...saying(e.p3));
      w.more.push([e.p3 > 0 ? 'Line 1 to them, line 2 to the room they leave and line 3 to the room they reach.' : 'They go the ordinary way, the room seeing them leave.']);
    } else {
      const { action, exit } = remoteOf(e);
      const room = e.to === own.room ? exits : far(e.to);
      if (e.p4) w.says.push('for those carrying ', ref('Items', e.p4, 'takes'), ', which loses a use: ');
      if (room === null) w.says.push(`nothing: there is no room ${e.to} on map ${own.map}, so the words go on`);
      else if (exit < 0) w.says.push(...(e.p3 > 0 ? ['says ', ref('Messages', e.p3, 'says'), ' and works no exit'] : ['nothing: it names no exit']));
      else {
        const what = works(room === undefined ? undefined : (room[exit] ?? null), action, lines);
        w.says.push(`works the ${DIRS[exit]} exit ${e.to === own.room ? 'here' : 'of '}`, ...(e.to === own.room ? [] : [ref('Rooms', e.to, 'works', own.map)]), ': ', ...what.says);
        w.more.push(...what.more);
      }
      if (room !== null && e.p3 > 0 && exit >= 0) w.more.push(['It says ', ref('Messages', e.p3, 'says'), ': line 2 to the room first, then line 1 to them.']);
      if (e.p4) w.more.push(['Not carrying it, the words go on to the next exit, the room’s commands and the rest.']);
    }
    out.push(w);
  }
  return out;
}
