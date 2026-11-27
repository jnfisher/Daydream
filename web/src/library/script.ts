// MajorMUD's text block scripts, read as the game reads them (silvermere-engine's
// scripts/mod.rs, scripts/actions.rs and scripts/special.rs, ported): an action line is
// clauses cut at each `:`, and a clause is the first keyword it starts with (ASCII letters
// without case, KEYWORDS' order deciding) and its words, each a number as the game reads one
// (a missing word is 0). A clause with no keyword is skipped and the line goes on.
//
// How a block is run decides how it reads (`Reading`): a whole block (a monster's answer, a
// death text, a spell's) tries its lines in turn until one runs through; a room's command
// block runs the lines whose `words:clauses` pattern matches what was typed; a random block
// rolls 0 to 99 for one line by its percentage; a talk block's `words:block` lines are what a
// monster can be asked about.
//
// Pure: no DOM, so `node --test` runs its tests (script.test.ts).

/** The keywords, in matching order: `nomonsters` before `monsters`; `teleport` takes every `teleport_` form. */
export const KEYWORDS = [
  'price', 'cast', 'giveability', 'addability', 'teleport', 'removeability', 'nomonsters', 'monsters', 'needmonster',
  'summon', 'message', 'takeitem', 'addexp', 'giveitem', 'hideitem', 'checkability', 'class', 'race', 'goodaligned',
  'evilaligned', 'minlevel', 'maxlevel', 'checkitem', 'failitem', 'text', 'roomtext', 'roomitem', 'failroomitem',
  'clearitem', 'addevil', 'testability', 'failability', 'remoteaction', 'random', 'adddelay', 'testskill', 'checkskill',
  'givecoins', 'test_tournament', 'checkspell', 'flag', 'learnspell',
] as const;
export type Keyword = (typeof KEYWORDS)[number];

/** The stock world's named teleports, in matching order (`teleport_slumsewers` before `teleport_slums`). */
export const TELEPORTS: Record<string, string> = {
  teleport_silvermere: 'Silvermere',
  teleport_sewers: 'the sewers',
  teleport_slumsewers: 'the slum sewers',
  teleport_slums: 'the slums',
  teleport_darkwood: 'the Darkwood',
  teleport_eldergrove: 'the Elder Grove',
  teleport_labyrinth: 'the labyrinth',
  teleport_blackcaverns: 'the black caverns',
  teleport_fungushazy: 'the fungus and hazy caves',
  teleport_crackedlava: 'the cracked lava fields',
  teleport_blackfortress: 'the black fortress',
  teleport_obsidian: 'the obsidian caves',
};

/** The stats `testskill` and `checkskill` name, matched in this order, with each roll's range. */
export const SKILLS: Record<string, [name: string, range: number]> = {
  agility: ['Agility', 150],
  strength: ['Strength', 150],
  intellect: ['Intellect', 150],
  wisdom: ['Willpower', 150],
  health: ['Health', 150],
  charm: ['Charm', 150],
  spellcasting: ['Spellcasting', 175],
  perception: ['Perception', 110],
  stealth: ['Stealth', 175],
  thievery: ['Thievery', 175],
  traps: ['Traps', 175],
  picklocks: ['Picklocks', 175],
  tracking: ['Tracking', 175],
  magicresistance: ['Magic Resistance', 150],
  current_hp: ['current hit points', 100],
};

const COINS: Record<string, string> = { R: 'runic', P: 'platinum', G: 'gold', S: 'silver' };
const VERBS = ['set', 'check', 'test', 'clear'];
/** The directions of exits 0 to 9. */
const DIRS = ['north', 'south', 'east', 'west', 'northeast', 'northwest', 'southeast', 'southwest', 'up', 'down'];
/** At most this many `|` alternatives of a pattern are tried. */
export const MAXALTS = 20;
/** The parts of a block a run reads at most (parts 0 to 49). */
export const MAX_PARTS = 50;

/**
 * Each keyword's words, in order: a name is a number, 32-bit, or 16-bit with a `%`; a `~`
 * word is read as a word (a skill, a coin, a flag's verb).
 */
const ARGS: Record<Keyword, string> = {
  price: 'copper msg',
  cast: 'spell',
  giveability: 'ability value',
  addability: 'ability amount',
  teleport: 'room map',
  removeability: 'ability',
  nomonsters: 'msg',
  monsters: 'msg',
  needmonster: 'monster msg',
  summon: 'monster',
  message: 'msg',
  takeitem: 'item msg',
  addexp: 'exp',
  giveitem: 'item',
  hideitem: 'item',
  checkability: 'ability% min%',
  class: 'class',
  race: 'race',
  goodaligned: 'limit msg',
  evilaligned: 'limit msg',
  minlevel: 'level msg',
  maxlevel: 'level msg',
  checkitem: 'item msg',
  failitem: 'item msg',
  text: 'block',
  roomtext: 'block',
  roomitem: 'item msg',
  failroomitem: 'item msg',
  clearitem: 'item msg',
  addevil: 'points%',
  testability: 'ability% max%',
  failability: 'ability% msg%',
  remoteaction: 'room msg action exit%',
  random: 'block',
  adddelay: 'ticks',
  testskill: '~skill modifier% block%',
  checkskill: '~skill min% msg%',
  givecoins: 'amount ~coin',
  test_tournament: '',
  checkspell: 'spell block',
  flag: 'flag% ~verb msg',
  learnspell: 'spell%',
};

/** One clause, its numbers read as the game reads them. */
export interface Clause {
  /** Its keyword, or null for a clause the game skips. */
  kw: Keyword | null;
  /** The clause as written. */
  text: string;
  /** Its numbers by name (`copper`, `msg`, `item`...): 0 for a missing word. */
  n: Record<string, number>;
  /** A named teleport's keyword, a skill's (`?word` for one the game does not know), a coin's name or a flag's verb. */
  word?: string;
  /** Whether a `:` follows it (what `nomonsters` tests). */
  more: boolean;
}

const isBlank = (c: number) => c === 32 || c === 9 || c === 10 || c === 12 || c === 13;

/** Past the word (to the first white space or the end). */
export function skipWord(s: string): string {
  let i = 0;
  while (i < s.length && !isBlank(s.charCodeAt(i))) i++;
  return s.slice(i);
}

/** Past the white space. */
export function skipBlanks(s: string): string {
  let i = 0;
  while (i < s.length && isBlank(s.charCodeAt(i))) i++;
  return s.slice(i);
}

/** ASCII letters in lower case, the rest as they are. */
const lowerAscii = (s: string) => s.replace(/[A-Z]/g, (c) => String.fromCharCode(c.charCodeAt(0) + 32));

/** `word` starts with `prefix` (lower case), ASCII letters compared without case. */
export const startsWith = (prefix: string, word: string) => word.length >= prefix.length && lowerAscii(word.slice(0, prefix.length)) === prefix;

/** A number as the game reads one: white space, a sign, then digits; anything else ends it (0 when there is none). It wraps at 32 bits. */
export function leadingInt32(s: string): number {
  const t = skipBlanks(s);
  let i = 0;
  const neg = t[0] === '-';
  if (neg || t[0] === '+') i = 1;
  let v = 0;
  for (; i < t.length; i++) {
    const d = t.charCodeAt(i) - 48;
    if (d < 0 || d > 9) break;
    v = (Math.imul(v, 10) + d) | 0;
  }
  return neg ? -v | 0 : v;
}

/** `leadingInt32` cut to 16 bits. */
export const leadingInt = (s: string) => (leadingInt32(s) << 16) >> 16;

/** The keyword a clause starts with, if any. */
export const keywordOf = (clause: string): Keyword | null => KEYWORDS.find((k) => startsWith(k, clause)) ?? null;

/** The clauses of an action line, as the game cuts them: at each `:`, a last empty piece ending the line. */
export function clauses(line: string): Clause[] {
  const pieces = line.split(':');
  const n = pieces.length;
  if (pieces[n - 1] === '') pieces.pop();
  return pieces.map((p, i) => parseClause(p, i + 1 < n));
}

/** One clause with its numbers. */
export function parseClause(text: string, more = false): Clause {
  const kw = keywordOf(text);
  const c: Clause = { kw, text, n: {}, more };
  if (!kw) return c;
  if (kw === 'teleport') {
    const named = Object.keys(TELEPORTS).find((t) => startsWith(t, text));
    if (named) {
      c.word = named;
      return c;
    }
  }
  let rest = skipBlanks(skipWord(text));
  const next = () => (rest = skipBlanks(skipWord(rest)));
  for (const spec of ARGS[kw].split(' ')) {
    if (!spec) continue;
    if (spec === '~skill') {
      c.word = Object.keys(SKILLS).find((s) => startsWith(s, rest)) ?? `?${skipBlanks(rest).split(/[ \t\n\f\r]/)[0]}`;
      next();
    } else if (spec === '~verb') {
      c.word = VERBS.find((v) => startsWith(v, rest)) ?? 'other';
      next();
    } else if (spec === '~coin') {
      const ch = rest[0] ?? '';
      c.word = COINS[/[a-z]/.test(ch) ? ch.toUpperCase() : ch] ?? 'copper';
    } else if (spec.endsWith('%')) {
      c.n[spec.slice(0, -1)] = leadingInt(rest);
      next();
    } else {
      c.n[spec] = leadingInt32(rest);
      next();
    }
  }
  return c;
}

/** The lines of a block run as a whole: the first line always, then each next one while a `:` follows anywhere from its start. */
export function wholeLines(text: string): string[] {
  const out: string[] = [];
  let start = 0;
  for (;;) {
    const nl = text.indexOf('\n', start);
    const end = nl < 0 ? text.length : nl;
    out.push(text.slice(start, end));
    if (end >= text.length) break;
    start = end + 1;
    if (!text.slice(start).includes(':')) break;
  }
  return out;
}

/**
 * The lines of a pattern block (a talk block, a room's command block, a random block): the
 * pattern runs from a line's start to the next `:` anywhere further on (so a line with no `:`
 * joins the next one's pattern), the action to the line's end; the scan ends when no `:` follows.
 */
export function patternLines(text: string): { pattern: string; action: string }[] {
  const out: { pattern: string; action: string }[] = [];
  let line = 0;
  for (;;) {
    const colon = text.indexOf(':', line);
    if (colon < 0) break;
    const nl = text.indexOf('\n', colon + 1);
    const end = nl < 0 ? text.length : nl;
    out.push({ pattern: text.slice(line, colon), action: text.slice(colon + 1, end) });
    if (end >= text.length) break;
    line = end + 1;
  }
  return out;
}

/** A pattern's alternatives, as the game tries them: cut at each `|`, the first 20. */
export function alternatives(pattern: string): string[] {
  return pattern.split('|').slice(0, MAXALTS);
}

/**
 * Whether typed text matches one alternative of a room's command (`match_wildcard_pattern`):
 * letters compare without case; a `*` lets the typed words stop short from there (a typed
 * space skips the rest of the pattern's word); all the typed text must be used, and the
 * pattern too unless what is left of it is a `*` or the words may stop short.
 */
function matchOne(pattern: string, text: string): boolean {
  let [cp, tp] = [0, 0];
  const at = (s: string, i: number) => (i < s.length ? s.charCodeAt(i) : 0);
  const low = (c: number) => (c >= 65 && c <= 90 ? c + 32 : c);
  const BAR = 124;
  const SPACE = 32;
  let abbrev = false;
  while (at(pattern, cp) !== 0 && at(pattern, cp) !== BAR && at(text, tp) !== 0) {
    const c = at(pattern, cp);
    const t = at(text, tp);
    if (c === 42) abbrev = true;
    else if (abbrev && t === SPACE) {
      while (at(pattern, cp) !== 0 && at(pattern, cp) !== BAR && at(pattern, cp) !== SPACE) cp++;
      if (at(pattern, cp) === SPACE) cp++;
      tp++;
      continue;
    } else if (low(c) === low(t)) tp++;
    else return false;
    cp++;
  }
  const rest = at(pattern, cp);
  return at(text, tp) === 0 && (abbrev || rest === 0 || rest === BAR || rest === 42);
}

/** Whether typed text runs a line of a room's commands: any of its first 20 alternatives
 * matches it (`wildcard_match`). */
export function wildcardMatch(pattern: string, text: string): boolean {
  const starts = [0];
  for (let i = 0; i < pattern.length && starts.length < MAXALTS; i++) if (pattern[i] === '|') starts.push(i + 1);
  return starts.some((a) => matchOne(pattern.slice(a), text));
}

/**
 * How a text block reads: a script run as a whole, a talk block's topics, a room's commands
 * (`words:clauses`), a random block's chances (`percent:clauses`), a monster's name list, what
 * it says now and then (`chance text`), or text to show.
 */
export type Reading = 'script' | 'topics' | 'commands' | 'chances' | 'names' | 'talk' | 'text';

const NUMBER = /^[ \t]*[+-]?\d+[ \t\r]*$/;

/**
 * How a block's text reads by its shape alone: shown text has colour codes; each line of a
 * script starts with a keyword; a talk block's lines are `words:number`, a random block's
 * `number:clauses` and a room's `words:clauses`.
 */
export function readingOf(text: string): Reading {
  if (!text.trim() || text.includes('\x1b')) return 'text';
  const lines = wholeLines(text).filter((l) => l.trim());
  if (lines.length && lines.every((l) => keywordOf(l))) return 'script';
  const pl = patternLines(text).filter((p) => p.pattern.trim() || p.action.trim());
  if (!pl.length) return 'text';
  if (pl.every((p) => p.pattern.trim() && !keywordOf(p.pattern) && NUMBER.test(p.action))) return 'topics';
  if (pl.every((p) => NUMBER.test(p.pattern) && keywordOf(p.action))) return 'chances';
  if (pl.every((p) => p.pattern.trim() && keywordOf(p.action))) return 'commands';
  return 'text';
}

/** What each column that names a text block makes of it. */
const USED_AS: [string, Reading][] = [
  ['Monsters Greet Txt', 'topics'],
  ['Rooms Command Text', 'commands'],
  ['Monsters Desc Txt', 'names'],
  ['Monsters Talk Txt', 'talk'],
  ['Items Read Msg', 'text'],
];

/** How a block reads: as the first column that names it makes of it (`Monsters Greet Txt`), else by its shape. */
export function readingFor(text: string, usedAs: readonly string[]): Reading {
  return USED_AS.find(([col]) => usedAs.includes(col))?.[1] ?? readingOf(text);
}

// ---- in words ------------------------------------------------------------------------------

/** A record a clause names. A room's `map` is undefined when it is the player's own. */
export interface Ref {
  t: 'Items' | 'Messages' | 'Monsters' | 'Spells' | 'Textblocks' | 'Classes' | 'Races' | 'Rooms';
  n: number;
  map?: number;
}

/**
 * What a record is to the clause that names it: said or shown, said when a test fails, given,
 * taken, needed, run, moved to, its exit worked, cast or summoned.
 */
export type Role = 'says' | 'else' | 'gives' | 'takes' | 'needs' | 'runs' | 'moves' | 'works' | 'casts' | 'summons';

/** A piece of a clause in words: text, a record, an ability, or an amount of money in copper. */
export type Bit = string | { ref: Ref; role: Role } | { ability: number } | { copper: number };

/**
 * What kind of step a clause is, for its icon: a test, money paid, something taken, given or
 * said, a move, another block run, anything else, or a clause that does nothing.
 */
export type StepKind = 'test' | 'pay' | 'take' | 'give' | 'say' | 'move' | 'run' | 'other' | 'skip';

export interface Step {
  kind: StepKind;
  bits: Bit[];
  clause: Clause;
}

const ref = (t: Ref['t'], n: number, role: Role, map?: number): Bit => ({ ref: map === undefined ? { t, n } : { t, n, map }, role });
const num = (n: number) => n.toLocaleString('en-US');
const plural = (n: number, one: string, many = `${one}s`) => `${num(n)} ${n === 1 || n === -1 ? one : many}`;

/** `, or say {msg} and stop`, or `, or stop` without a message. */
const orSay = (msg: number): Bit[] => (msg > 0 ? [', or say ', ref('Messages', msg, 'else'), ' and stop'] : [', or stop']);
/** `, else say {msg} and stop`, for a test. */
const elseSay = (msg: number): Bit[] => (msg > 0 ? [', else say ', ref('Messages', msg, 'else'), ' and stop'] : [', else stop']);

/** An amount of copper in the coins it makes, largest first: `20 platinum`, `12 gold, 3 copper`. */
export function coinsOf(copper: number): string {
  if (copper <= 0) return 'no money';
  const out: string[] = [];
  let left = copper;
  for (const [name, worth] of [['runic', 1_000_000], ['platinum', 10_000], ['gold', 100], ['silver', 10], ['copper', 1]] as const) {
    const k = Math.floor(left / worth);
    if (k) out.push(`${num(k)} ${name}`);
    left -= k * worth;
  }
  return out.join(', ');
}

/** A skill clause's stat and roll range; a word the game does not know is worth 0 against 100. */
function skillOf(c: Clause): [string, number] {
  return SKILLS[c.word!] ?? [`“${c.word!.slice(1)}” (no skill the game knows, so 0)`, 100];
}

/** Why a clause with no keyword is skipped. */
function skipped(text: string): string {
  if (!text) return 'Nothing: an empty clause';
  const t = text.trim();
  if (!t) return 'Nothing: a clause of spaces';
  if (t !== text && keywordOf(skipBlanks(text))) return `Skipped: “${t}” starts with a space, so the game finds no keyword in it`;
  return `Skipped: “${t}” starts with no keyword`;
}

/** A clause in plain words, as an order to the game: "Take 20 platinum, or say message 1458 and stop". */
export function words(c: Clause): { kind: StepKind; bits: Bit[] } {
  const n = c.n;
  const as =
    (kind: StepKind) =>
    (bits: Bit[]): { kind: StepKind; bits: Bit[] } => ({ kind, bits });
  const [test, pay, take, give, say, move, run, other, skip] = (['test', 'pay', 'take', 'give', 'say', 'move', 'run', 'other', 'skip'] as const).map(as);
  switch (c.kw) {
    case null:
      return skip([skipped(c.text)]);
    case 'price':
      return n.copper! > 0 ? pay(['Take ', { copper: n.copper! }, ...orSay(n.msg!)]) : skip([`Nothing: a price of ${num(n.copper!)} is always paid`]);
    case 'cast':
      return other(['Cast ', ref('Spells', n.spell!, 'casts'), ' as an item casts it, or stop when the cast is refused']);
    case 'giveability':
      return give(['Give ', { ability: n.ability! }, ` ${num(n.value!)} (raising a lower one), or stop when every ability slot is taken`]);
    case 'addability':
      return give(['Add ', num(n.amount!), ' to ', { ability: n.ability! }, ', or stop when every ability slot is taken']);
    case 'teleport':
      if (c.word) return move([`Teleport to a random room of ${TELEPORTS[c.word]}, or stop when it does not land`]);
      return n.room && n.map ? move(['Teleport to ', ref('Rooms', n.room, 'moves', n.map), ', or stop when it does not land']) : skip(['Nothing: a teleport needs a room and a map']);
    case 'removeability':
      return take(['Take away ', { ability: n.ability! }, ', or stop when the player has none']);
    case 'nomonsters':
      return c.more ? test(['Only with no monsters here', ...elseSay(n.msg!)]) : skip(['Nothing: as the last clause of its line, nomonsters never looks']);
    case 'monsters':
      return test(['Only with monsters here', ...elseSay(n.msg!)]);
    case 'needmonster':
      return test(['Only with ', ref('Monsters', n.monster!, 'needs'), ' here', ...elseSay(n.msg!)]);
    case 'summon':
      return other(['Summon ', ref('Monsters', n.monster!, 'summons'), ' here as an ally, or stop when it cannot come']);
    case 'message':
      return n.msg! > 0 ? say(['Say ', ref('Messages', n.msg!, 'says')]) : skip([`Nothing: there is no message ${num(n.msg!)}`]);
    case 'takeitem':
      return take(['Take ', ref('Items', n.item!, 'takes'), ...orSay(n.msg!)]);
    case 'addexp':
      return give([`Give ${plural(n.exp!, 'experience point')}`]);
    case 'giveitem':
      return give(['Give ', ref('Items', n.item!, 'gives'), '; one the player cannot carry drops to the floor and ends the line']);
    case 'hideitem':
      return other(['Hide ', ref('Items', n.item!, 'gives'), ' in the room, and end the line']);
    case 'checkability':
      return test(['Only with ', { ability: n.ability! }, ` of ${num(n.min!)} or more, else stop`]);
    case 'class':
      return test(['Only for the class ', ref('Classes', n.class!, 'needs'), ', else stop']);
    case 'race':
      return test(['Only for the race ', ref('Races', n.race!, 'needs'), ', else stop']);
    case 'goodaligned':
      return test([`Only with ${plural(n.limit!, 'evil point')} or fewer`, ...elseSay(n.msg!)]);
    case 'evilaligned':
      return test([`Only with ${plural(n.limit!, 'evil point')} or more`, ...elseSay(n.msg!)]);
    case 'minlevel':
      return test([`Only from level ${num(n.level!)}`, ...elseSay(n.msg!)]);
    case 'maxlevel':
      return test([`Only up to level ${num(n.level!)}`, ...elseSay(n.msg!)]);
    case 'checkitem':
      return test(['Only when carrying ', ref('Items', n.item!, 'needs'), ...elseSay(n.msg!)]);
    case 'failitem':
      return test(['Only when not carrying ', ref('Items', n.item!, 'needs'), ...elseSay(n.msg!)]);
    case 'text':
      return say(['Show ', ref('Textblocks', n.block!, 'says')]);
    case 'roomtext':
      return say(['Show ', ref('Textblocks', n.block!, 'says'), ' to the whole room']);
    case 'roomitem':
      return test(['Only with ', ref('Items', n.item!, 'needs'), ' in the room', ...elseSay(n.msg!)]);
    case 'failroomitem':
      return test(['Only without ', ref('Items', n.item!, 'needs'), ' in the room', ...elseSay(n.msg!)]);
    case 'clearitem':
      return n.item ? take(['Take ', ref('Items', n.item!, 'takes'), ' from the room', ...orSay(n.msg!)]) : take(['Clear every item from the room']);
    case 'addevil':
      return other([n.points! < 0 ? `Take away ${plural(-n.points!, 'evil point')}` : `Add ${plural(n.points!, 'evil point')}`]);
    case 'testability':
      return test(['Only with ', { ability: n.ability! }, ` of ${num(n.max!)} or less${n.max! < 0 ? ', or none' : ''}, else stop`]);
    case 'failability':
      return test(['Only without ', { ability: n.ability! }, ...elseSay(n.msg!)]);
    case 'remoteaction': {
      const e = n.exit!;
      if (e < 0 || e > 10) return skip([`Nothing: exit ${e} is not one of 0 to 10`]);
      const said: Bit[] = n.msg! > 0 ? ['Say ', ref('Messages', n.msg!, 'says'), ', then work'] : ['Work'];
      if (e === 10) return n.msg! > 0 ? say(['Say ', ref('Messages', n.msg!, 'says'), ' (exit 10 has no direction to work)']) : skip(['Nothing: exit 10 has no direction']);
      return other([...said, ` the ${DIRS[e]} exit of `, ref('Rooms', n.room!, 'works'), n.action ? `, clearing block ${num(n.action)} if it is a hidden exit` : '']);
    }
    case 'random':
      return run(['Run one line of ', ref('Textblocks', n.block!, 'runs'), ' by chance']);
    case 'adddelay':
      return other([`Make the player wait ${plural(n.ticks! & 255, 'tick')} more`]);
    case 'testskill': {
      const [name, range] = skillOf(c);
      const m = n.modifier!;
      return test([`Roll 0 to ${range - 1}${m ? ` ${m > 0 ? 'plus' : 'minus'} ${Math.abs(m)}` : ''}: above the player's ${name}, run `, ref('Textblocks', n.block!, 'runs'), ' and stop']);
    }
    case 'checkskill': {
      const [name] = skillOf(c);
      return test([`Only with ${name} of ${num(n.min!)} or more`, ...elseSay(n.msg!)]);
    }
    case 'givecoins':
      return give([`Give ${plural(n.amount!, `${c.word} coin`)}`]);
    case 'test_tournament':
      return test(['Only while tournament play is on, else stop']);
    case 'checkspell':
      return test(['Only while ', ref('Spells', n.spell!, 'needs'), ' is on the player, else run ', ref('Textblocks', n.block!, 'runs'), ' and stop']);
    case 'flag':
      return n.flag! >= 1 && n.flag! <= 64 ? skip([`Nothing: flag ${n.flag} never takes effect`]) : test([`Stop: flag ${n.flag} is not one of 1 to 64`]);
    case 'learnspell':
      return give(['Teach ', ref('Spells', n.spell!, 'gives'), ', or stop when the player cannot learn it']);
  }
}

/** A line's clauses as steps. */
export const stepsOf = (line: string): Step[] => clauses(line).map((c) => ({ ...words(c), clause: c }));

/** One line of a block as the game reads it. */
export interface Line {
  /** Its number among the block's lines, from 1. */
  no: number;
  /** What the game matches it by: a command's words, a topic's, a random line's percentage. */
  pattern?: string;
  /** A random line's chance, in percent. */
  pct?: number;
  /** A topic's text block. */
  block?: number;
  steps: Step[];
  /** Why the game never runs it. */
  never?: string;
}

/** A block's lines, as the game reads them for `reading`. */
export function linesOf(text: string, reading: Reading): Line[] {
  if (reading === 'script') {
    const all = text.split('\n');
    const read = wholeLines(text).length;
    return all
      .map((l, i): Line => (i < read ? { no: i + 1, steps: stepsOf(l) } : { no: i + 1, steps: [], never: 'the game stops reading a block at the first line with no “:” in it or after it' }))
      .filter((_, i) => all[i]!.trim());
  }
  if (reading === 'topics')
    return patternLines(text)
      .filter((p) => p.pattern || p.action.trim())
      .map((p, i) => ({ no: i + 1, pattern: p.pattern, block: leadingInt32(p.action), steps: [] }));
  if (reading === 'chances') {
    let top = 0;
    return patternLines(text).map((p, i) => {
      const at = Math.min(100, Math.max(0, leadingInt32(p.pattern)));
      const pct = Math.max(0, at - top);
      top = Math.max(top, at);
      return { no: i + 1, pattern: p.pattern.trim(), pct, steps: stepsOf(p.action), ...(pct ? {} : { never: 'the lines before it take every roll it would' }) };
    });
  }
  if (reading === 'commands') return patternLines(text).map((p, i) => ({ no: i + 1, pattern: p.pattern, steps: stepsOf(p.action) }));
  return [];
}
