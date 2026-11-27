// A monster as players meet it, played as silvermere plays MajorMUD 1.11p: which attack a
// blow uses, how many blows its energy buys, the lines each outcome sends to the player it
// hits and to the room, and the lines it arrives, is named, dies and talks with
// (silvermere-engine combat/attack.rs and combat/messages.rs, monsters/generate.rs,
// combat/kill_monster.rs, cycles/slow.rs, commands/ask_greet.rs).
//
// Pure: no DOM, so `node --test` runs its tests (combat.test.ts).

/** A message's three lines (a row of Messages). */
export type Lines = [string, string, string];

const ESC = '\x1b';

/** The colours the game sends each line in (silvermere-text's catalogue). */
export const COLOUR = {
  hitYou: `${ESC}[1m${ESC}[31m`,
  hitRoom: `${ESC}[1;31m`,
  armour: `${ESC}[0;31m`,
  dodge: `${ESC}[0;36m`,
  miss: `${ESC}[0;36m`,
  yellow: `${ESC}[1;33m`,
  green: `${ESC}[0;32m`,
  grey: `${ESC}[0;37m`,
};

/** The most attacks a monster has, and the most blows in a round. */
export const ATTACKS = 5;
export const MAX_BLOWS = 6;

/** Attack Type #. */
export const NORMAL = 1;
export const SPELL = 2;
export const ROB = 3;

/**
 * A line filled in as the game fills it (`fmt_msg`): each `%s`, `%d`, `%u` or `%i` (after any
 * `l` or `h`) takes the next argument, `%%` is a percent sign, a conversion past the last
 * argument prints nothing, a `%` before any other character is dropped, and one that ends the
 * line is printed.
 */
export function fmtMsg(fmt: string, args: readonly (string | number)[]): string {
  let out = '';
  let next = 0;
  for (let i = 0; i < fmt.length; i++) {
    const c = fmt[i]!;
    if (c !== '%') {
      out += c;
      continue;
    }
    let spec = '';
    while (fmt[i + 1] === 'l' || fmt[i + 1] === 'h') spec += fmt[++i];
    const p = fmt[i + 1];
    if (p === '%') {
      i++;
      out += '%';
    } else if (p === 's' || p === 'd' || p === 'u' || p === 'i') {
      i++;
      if (next < args.length) out += String(args[next++]);
    } else if (p === undefined && spec === '') out += '%';
    else out += spec;
  }
  return out;
}

/** The first letter upper-cased, as the game sends a combat line. */
export const upperFirst = (s: string) => (/^[a-z]/.test(s) ? s[0]!.toUpperCase() + s.slice(1) : s);

/** One of a monster's attack slots. */
export interface Attack {
  /** Attack Type #: 0 none, 1 normal, 2 spell, 3 rob. */
  type: number;
  /** Attack % #: cumulative, see `shares`. */
  pct: number;
  /** Attack Accu/Spell #: the accuracy, or the spell a spell attack casts. */
  acc: number;
  /** Attack Min Hit/Cast % #: the least damage, or a spell's chance to be cast. */
  min: number;
  /** Attack Max Hit/Cast LVL #: the most damage, or the level a spell is cast at. */
  max: number;
  /** Attack Energy #: what a blow costs. */
  energy: number;
  hit: number;
  dodge: number;
  miss: number;
  /** Attack Hit Spell #: cast at the player on a hit. */
  spell: number;
}

const n = (v: unknown) => Number(v) || 0;

/** The attack slots of a monster record. */
export function attacksOf(get: (col: string) => unknown): Attack[] {
  return Array.from({ length: ATTACKS }, (_, i) => ({
    type: n(get(`Attack Type ${i}`)),
    pct: n(get(`Attack % ${i}`)),
    acc: n(get(`Attack Accu/Spell ${i}`)),
    min: n(get(`Attack Min Hit/Cast % ${i}`)),
    max: n(get(`Attack Max Hit/Cast LVL ${i}`)),
    energy: n(get(`Attack Energy ${i}`)),
    hit: n(get(`Attack Hit Msg ${i}`)),
    dodge: n(get(`Attack Dodge Msg ${i}`)),
    miss: n(get(`Attack Miss Msg ${i}`)),
    spell: n(get(`Attack Hit Spell ${i}`)),
  }));
}

/**
 * How many of a blow's 100 rolls (0 to 99) pick each attack: the first whose Attack % is above
 * the roll, so the percentages add up as they go (50, 80, 100 is 50%, 30% and 20%). The `rest`
 * of the rolls, above every attack, keep the attack the last blow used: the first one, at the
 * start of a round.
 */
export function shares(attacks: readonly Attack[]): { share: number[]; rest: number } {
  let top = 0;
  const share = attacks.map((a) => {
    const p = Math.min(100, Math.max(0, a.pct));
    const s = Math.max(0, p - top);
    top = Math.max(top, p);
    return s;
  });
  return { share, rest: 100 - top };
}

/** What a blow costs, as the game's 16 bits hold it (a negative cost is a huge one). */
const cost = (energy: number) => (energy < 0 ? energy + 65536 : energy);

/**
 * The blows one attack alone makes in a round of `energy`: one while the cost is no more than
 * what is left, and none after one that leaves less than its cost; at most six.
 */
export function blowsOf(energy: number, attackEnergy: number): number {
  const c = cost(attackEnergy);
  let left = energy;
  let blows = 0;
  while (blows < MAX_BLOWS && c <= left) {
    left -= c;
    blows++;
    if (c > left) break;
  }
  return blows;
}

/** An attack type the game plays: normal, spell or rob. */
export const knownType = (type: number) => type === NORMAL || type === SPELL || type === ROB;

/** Whether a slot is an attack, or one a blow can pick. */
export const liveSlot = (a: Attack, share: number) => knownType(a.type) || share > 0;

/** Whether an empty slot still holds an old attack's numbers. */
export const holdsNumbers = (a: Attack) => [a.type, a.pct, a.acc, a.min, a.max, a.energy, a.hit, a.dodge, a.miss, a.spell].some((v) => v !== 0);

/** What is wrong with each attack slot, for a person (nothing for an empty one). */
export function attackProblems(attacks: readonly Attack[], energy: number): string[][] {
  const { share } = shares(attacks);
  const top = (i: number) => Math.max(0, ...attacks.slice(0, i).map((a) => Math.min(100, a.pct)));
  return attacks.map((a, i) => {
    const out: string[] = [];
    const s = share[i]!;
    if (a.type !== 0 && !knownType(a.type)) out.push(`Type ${a.type} is not an attack type, so its blows do nothing.`);
    if (a.type === 0 && s > 0) out.push(`It has no type, so the ${s}% of blows that pick it do nothing.`);
    if (a.type !== 0 && s === 0)
      out.push(a.pct === 0 ? 'Its Attack% is 0, so no blow picks it.' : `No blow picks it: the attacks above it already take every roll under ${top(i)}%, and the % add up.`);
    if (a.type === NORMAL && s > 0 && blowsOf(energy, a.energy) === 0)
      out.push(`A blow costs ${a.energy.toLocaleString('en-US')} energy, more than its whole round of ${energy.toLocaleString('en-US')}, so it never lands.`);
    if (a.type === NORMAL && a.min > a.max) out.push('Its least damage is more than its most.');
    return out;
  });
}

/** The columns of attack slot `i`. */
export const attackCols = (i: number) =>
  ['Type', '%', 'Accu/Spell', 'Min Hit/Cast %', 'Max Hit/Cast LVL', 'Energy', 'Hit Msg', 'Dodge Msg', 'Miss Msg', 'Hit Spell'].map((c) => `Attack ${c} ${i}`);

/** A line with nothing to read but its colour. */
export const blankLine = (line: string) => !line.replace(/\x1b\[[0-9;]*[A-Za-z]?/g, '').trim();

/** A monster's problem: what is wrong, the columns it is about, and its fix when there is one. */
export interface MonsterProblem {
  level: 'warn' | 'tidy';
  text: string;
  cols: string[];
  fix?: { label: string; set: Record<string, string> };
}

/**
 * What is wrong with a monster's attacks and messages: each attack's problems, empty slots
 * that still hold an old attack's numbers, and messages players read that are empty (stock
 * data names message 1, a blank one, as a placeholder). `lines` reads a message: undefined
 * while it is read, null when the world has none.
 */
export function monsterProblems(get: (col: string) => unknown, lines: (n: number) => Lines | null | undefined): MonsterProblem[] {
  const attacks = attacksOf(get);
  const energy = n(get('Energy'));
  const { share } = shares(attacks);
  const issues = attackProblems(attacks, energy);
  const out: MonsterProblem[] = [];
  const empty = (m: number, which: number[]) => {
    const l = m ? lines(m) : null;
    return !!l && which.every((k) => blankLine(l[k]!));
  };
  attacks.forEach((a, i) => {
    const name = `Attack ${i + 1}`;
    if (!liveSlot(a, share[i]!)) {
      if (holdsNumbers(a))
        out.push({
          level: 'tidy',
          text: `${name} is empty but still holds an old attack's numbers. No blow picks it.`,
          cols: attackCols(i),
          fix: { label: 'Clear it', set: Object.fromEntries(attackCols(i).map((c) => [c, '0'])) },
        });
      return;
    }
    for (const t of issues[i]!) out.push({ level: 'warn', text: `${name}: ${t}`, cols: attackCols(i) });
    if (a.type !== NORMAL || share[i] === 0) return;
    for (const [m, label, col] of [
      [a.hit, 'hit', `Attack Hit Msg ${i}`],
      [a.dodge, 'dodge', `Attack Dodge Msg ${i}`],
      [a.miss, 'miss', `Attack Miss Msg ${i}`],
    ] as const) {
      if (empty(m, [0, 1, 2]))
        out.push({
          level: 'warn',
          text: `${name}'s ${label} message is ${m}, which is empty, so players read blank lines.`,
          cols: [col],
          fix: { label: "Use the game's own lines", set: { [col]: '0' } },
        });
    }
  });
  const move = n(get('Move Msg'));
  if (empty(move, [0]))
    out.push({
      level: 'warn',
      text: `Its Move Msg is ${move}, whose first line is empty, so a room reads a blank line when it appears.`,
      cols: ['Move Msg'],
      fix: { label: "Use the game's own line", set: { 'Move Msg': '0' } },
    });
  const death = n(get('Death Msg'));
  if (empty(death, [2]))
    out.push({
      level: 'warn',
      text: `Its Death Msg is ${death}, whose third line is empty, so a room reads a blank line when it dies.`,
      cols: ['Death Msg'],
      fix: { label: 'Set it to 0', set: { 'Death Msg': '0' } },
    });
  return out;
}

export type Outcome = 'hit' | 'armour' | 'dodge' | 'miss';

/** A monster's weapon's words: its hit message's lines, its miss message's (else the hit's)
 * and its name, blank without a weapon (`monster_side`). */
export interface Verbs {
  hit: Lines;
  miss: Lines;
  weapon: string;
}

export function verbsOf(weapon: { name: string; hit: Lines | null; miss: Lines | null } | null): Verbs {
  const none: Lines = ['', '', ''];
  if (!weapon) return { hit: none, miss: none, weapon: '' };
  const hit = weapon.hit ?? none;
  return { hit, miss: weapon.miss ?? hit, weapon: weapon.name };
}

/** An attack's messages, null for 0 or a message the world does not have. */
export interface Texts {
  hit: Lines | null;
  def: Lines | null;
  miss: Lines | null;
}

/** Who a blow is aimed at, as a preview names them. */
export interface Victim {
  name: string;
  /** `his` or `her`. */
  poss: string;
  /** `he` or `she`. */
  subj: string;
}

export const SAMPLE_VICTIM: Victim = { name: 'Rowan', poss: 'her', subj: 'she' };

/**
 * A blow's two lines, in the game's colours: the one the player it is aimed at reads, and the
 * one the rest of the room reads (`monster_on_player`). A hit is the hit message (line 1 to the
 * player with the damage, line 2 to the room); armour that takes it is lines 1 and 2 of the
 * dodge message; a dodge is its line 3, with line 1 of the miss message to the room (both
 * messages are needed, else the game's own words); a miss is lines 2 and 3 of the miss message.
 */
export function blowLines(mon: string, verbs: Verbs, texts: Texts, victim: Victim, outcome: Outcome, damage: number): { you: string; room: string } {
  const star = '*';
  const { name: v, poss, subj } = victim;
  const out = (you: string, room: string, c1: string, c2 = c1) => ({ you: c1 + upperFirst(you), room: c2 + upperFirst(room) });
  switch (outcome) {
    case 'hit': {
      const h = texts.hit;
      return out(
        h ? fmtMsg(h[0], [mon, damage, star, star]) : `${mon} ${verbs.hit[1]} you for ${damage} damage!`,
        h ? fmtMsg(h[1], [mon, v, String(damage), star, star]) : `${mon} ${verbs.hit[2]} ${v} for ${damage} damage!`,
        COLOUR.hitYou,
        COLOUR.hitRoom,
      );
    }
    case 'armour': {
      const d = texts.def;
      return out(
        d ? fmtMsg(d[0], [mon, verbs.miss[1], star, star]) : `${mon}'s ${verbs.miss[1]} hits you, but your armour deflects.`,
        d ? fmtMsg(d[1], [mon, verbs.miss[1], v, poss, star, star]) : `${mon}'s ${verbs.miss[1]} hits ${v}, but glances off ${poss} armour.`,
        COLOUR.armour,
      );
    }
    case 'dodge': {
      const [d, m] = [texts.def, texts.miss];
      return d && m
        ? out(fmtMsg(d[2], [mon, verbs.miss[1], verbs.weapon, star, star]), fmtMsg(m[0], [mon, verbs.miss[2], v, verbs.weapon, subj, star, star]), COLOUR.dodge)
        : out(`${mon} ${verbs.miss[1]} you with ${verbs.weapon}, but you dodge!`, `${mon} ${verbs.miss[2]} ${v} with its ${verbs.weapon}, but ${subj} dodges.`, COLOUR.dodge);
    }
    case 'miss': {
      const m = texts.miss;
      return out(
        m ? fmtMsg(m[1], [mon, verbs.miss[1], verbs.weapon, star, star]) : `${mon} ${verbs.miss[1]} you with ${verbs.weapon}.`,
        m ? fmtMsg(m[2], [mon, verbs.miss[2], v, verbs.weapon, star, star]) : `${mon} ${verbs.miss[2]} ${v} with its ${verbs.weapon}.`,
        COLOUR.miss,
      );
    }
  }
}

/** One blow of a round. */
export type Step =
  /** A normal attack's blow; `spell` is its hit spell, cast after a hit that does damage. */
  | { kind: 'blow'; slot: number; outcome: Outcome; damage: number; spell: number }
  /** A spell attack: the spell, its chance to be cast and its level. */
  | { kind: 'cast'; slot: number; spell: number; chance: number; level: number }
  /** A spell attack without the energy left for it: nothing happens. */
  | { kind: 'tired'; slot: number }
  /** An attack with no type (or a rob, when the first attack is no normal one): nothing. */
  | { kind: 'idle'; slot: number; rob: boolean };

/**
 * The odds of each outcome in a preview. In the game they come from the player's armour,
 * dodge and the attack's accuracy, so a preview only shows how each one reads.
 */
export const PREVIEW_ODDS: [Outcome, number][] = [
  ['hit', 50],
  ['armour', 15],
  ['dodge', 15],
  ['miss', 20],
];

/** A between-rounds spell slot: Spell Number #, Spell Cast % # (cumulative), Spell Cast LVL #. */
export interface RoundSpell {
  spell: number;
  pct: number;
  level: number;
}

export function spellsOf(get: (col: string) => unknown): RoundSpell[] {
  return Array.from({ length: ATTACKS }, (_, i) => ({ spell: n(get(`Spell Number ${i}`)), pct: n(get(`Spell Cast % ${i}`)), level: n(get(`Spell Cast LVL ${i}`)) }));
}

/**
 * One round of a monster's attacks on a player, drawn with `rand` (0 to under 1), as
 * `attack_monster_user` makes it: at most six blows while its `energy` lasts, each attack picked
 * by its cumulative Attack %, and after the round a spell from its between-rounds list. Spell
 * attacks are taken as cast, and outcomes are drawn by `PREVIEW_ODDS`.
 */
export function roundOf(
  attacks: readonly Attack[],
  energy: number,
  rand: () => number,
  spells: readonly RoundSpell[] = [],
): { steps: Step[]; after: RoundSpell | null } {
  const steps: Step[] = [];
  let left = energy;
  let slot = 0;
  for (let b = 0; b < MAX_BLOWS; b++) {
    const roll = Math.floor(rand() * 100);
    const i = attacks.findIndex((a) => a.pct > roll);
    if (i >= 0) slot = i;
    let a = attacks[slot];
    if (!a) break;
    if (a.type === SPELL) {
      if (left < cost(a.energy)) steps.push({ kind: 'tired', slot });
      else {
        left -= cost(a.energy);
        steps.push({ kind: 'cast', slot, spell: a.acc, chance: a.min, level: a.max });
      }
      continue;
    }
    if (a.type === ROB) {
      // a stub in 1.11p that never steals: the first attack strikes instead
      const robbed = slot;
      slot = 0;
      a = attacks[0]!;
      if (a.type !== NORMAL) {
        steps.push({ kind: 'idle', slot: robbed, rob: true });
        continue;
      }
    } else if (a.type !== NORMAL) {
      steps.push({ kind: 'idle', slot, rob: false });
      continue;
    }
    const c = cost(a.energy);
    // not enough energy left: no blow, and the round ends
    if (c > left) break;
    left -= c;
    let outcome = draw(rand);
    const damage = outcome === 'hit' ? a.min + Math.floor(rand() * Math.max(1, a.max - a.min + 1)) : 0;
    // a hit that does no damage reads as a miss
    if (outcome === 'hit' && damage <= 0) outcome = 'miss';
    steps.push({ kind: 'blow', slot, outcome, damage: Math.max(0, damage), spell: damage > 0 ? a.spell : 0 });
    if (c > left) break;
  }
  let after: RoundSpell | null = null;
  if (spells[0] && spells[0].spell !== 0) {
    const roll = Math.floor(rand() * 100);
    after = spells.find((s) => s.pct > roll) ?? null;
    if (after && !after.spell) after = null;
  }
  return { steps, after };
}

function draw(rand: () => number): Outcome {
  let r = rand() * 100;
  for (const [o, p] of PREVIEW_ODDS) {
    if (r < p) return o;
    r -= p;
  }
  return 'miss';
}

/**
 * The line a room reads when the monster appears (`arrival_text`), in its colours: line 1 of its
 * Move Msg, with the name then where from when it has a name list (Desc Txt), else where from
 * then the name; the game's own line without a Move Msg. `from` is a direction (`north`), or
 * null for nowhere. `lines` is null for a Move Msg the world does not have.
 */
export function arrivalLine(moveMsg: number, lines: Lines | null, name: string, named: boolean, from: string | null): string {
  const where = from ? `the ${from}` : 'nowhere';
  if (moveMsg !== 0) {
    if (lines === null) return `${COLOUR.yellow}${name}${COLOUR.green} moves into the room from ${where}.`;
    if (!lines[0]) return '';
    return COLOUR.yellow + (named ? fmtMsg(lines[0], [name + COLOUR.green, where]) : fmtMsg(lines[0], [where, name]));
  }
  return `${COLOUR.yellow}${name}${COLOUR.green} just arrived from ${where}.`;
}

/** The game's longest random name: 29 characters. */
const NAME = 29;

/**
 * The names a name list (Desc Txt) gives the monster, each with its chance (`get_random_name`):
 * the lines are read in turn, each one a name (`N:` the monster's own, `F:text` the text,
 * `B:text` the text before its name, `A:text` after it, any other line as written), with a
 * chance in ten to stop after each; the last line read stands, and an empty name is its own.
 * In list order, a name once however many lines give it.
 */
export function nameOdds(text: string, own: string): { name: string; pct: number }[] {
  const lines: string[] = [];
  let rest = text;
  while (rest && lines.length < 1000) {
    const nl = rest.indexOf('\n');
    lines.push(nl < 0 ? rest : rest.slice(0, nl));
    rest = nl < 0 ? '' : rest.slice(nl + 1);
  }
  const out = new Map<string, number>();
  lines.forEach((line, i) => {
    let name: string;
    const body = line.slice(2);
    if (line.length >= 2 && line[1] === ':') {
      if (line[0] === 'N') name = own.slice(0, NAME);
      else if (line[0] === 'F') name = body.slice(0, NAME);
      else if (line[0] === 'B') name = `${body.slice(0, NAME - 1)} ${own}`.slice(0, NAME);
      else if (line[0] === 'A') name = `${own} ${body}`.slice(0, NAME);
      else name = line.slice(0, NAME);
    } else name = line.slice(0, NAME);
    if (!name) name = own;
    const p = i === lines.length - 1 ? 0.9 ** i : 0.1 * 0.9 ** i;
    out.set(name, (out.get(name) ?? 0) + p * 100);
  });
  if (!out.size) out.set(own, 100);
  return [...out].map(([name, pct]) => ({ name, pct }));
}

/** A line's leading number: blanks, a sign, digits. */
function leading(s: string): number {
  const m = /^[ \t]*([+-]?)(\d*)/.exec(s)!;
  const v = Number(m[2] || 0);
  return m[1] === '-' ? -v : v;
}

/**
 * What a monster says now and then (Talk Txt, `event_line`): each of the first 30 lines is
 * `chance text`, and a roll of 0 to 99 says the first line whose chance is above it, so the
 * chances add up as they go. Each line with its share of the rolls; the rest say nothing.
 */
export function talkOdds(text: string): { text: string; pct: number }[] {
  const lines = text.split(/[\r\n]/).filter((l) => l).slice(0, 30);
  let top = 0;
  const out: { text: string; pct: number }[] = [];
  for (const line of lines) {
    // the game keeps the number's low 16 bits
    const v = (((leading(line) % 65536) + 98304) % 65536) - 32768;
    const p = Math.min(100, Math.max(0, v));
    const share = Math.max(0, p - top);
    top = Math.max(top, p);
    const sp = line.indexOf(' ');
    if (share > 0) out.push({ text: sp < 0 ? '' : line.slice(sp + 1), pct: share });
    // a winning line without a space ends the search
    if (share > 0 && sp < 0) break;
  }
  return out;
}

/**
 * What a player may ask a monster about (Greet Txt, `ask_greet`): each `words:block` line of
 * the block, the words matched anywhere in the question, and the text block it shows.
 */
export function topicsOf(text: string): { words: string; block: number }[] {
  const out: { words: string; block: number }[] = [];
  let line = 0;
  for (;;) {
    const colon = text.indexOf(':', line);
    if (colon < 0) break;
    const nl = text.indexOf('\n', colon + 1);
    const end = nl < 0 ? text.length : nl;
    out.push({ words: text.slice(line, colon), block: leading(text.slice(colon + 1, end)) });
    if (end >= text.length) break;
    line = end + 1;
  }
  return out;
}
