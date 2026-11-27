// Play: a pretend player in a story, run as the game runs it (silvermere-engine's
// commands/ask_greet.rs and input.rs, scripts/{actions,special}.rs, display/long_text.rs,
// movement/exits.rs). The player greets a monster, asks it about something, or types in a
// room: the words of an exit that takes them through it or works another, or a command. The
// game's text comes back as the game sends it, and what changed that the game does not say
// (coins paid, an item given, a door unlocked, a test that failed and why) as a note. A roll is
// made as the game makes it.
//
// What the world would know that a pretend player does not (its class, level, alignment, stats
// and abilities, what lies in the room) is the player's to set, and a note on a test that
// failed says what would have passed it.
//
// Pure: no DOM, so `node --test` runs its tests (play.test.ts).

import { BACK, DIRS, exitsOf, GATE, HIDDEN, REMOTE, remoteOf, roomScreen, sameWords, shown, TEXT, workExit, type Exit, type Lines } from '../library/exits.ts';
import { clauses, coinsOf, leadingInt32, MAX_PARTS, patternLines, SKILLS, TELEPORTS, wholeLines, wildcardMatch, type Clause } from '../library/script.ts';
import { nodeId, type Cell, type Values } from './story.ts';

/** A record's values, or null when the world has none. */
export type Get = (table: string, key: Cell[]) => Promise<Values | null>;

export interface Pretend {
  /** The first name the others in the room read in messages (`%s`). */
  name: string;
  /** Coins, in copper. */
  copper: number;
  /** The items carried: one number for each. */
  items: number[];
  /** The spells on the player now (what `checkspell` looks at). */
  spells: number[];
  /** The spellbook. */
  book: number[];
  /** Abilities by number. */
  abilities: Record<number, number>;
  cls: number;
  race: number;
  level: number;
  evil: number;
  /** Every stat and skill a roll is made against. */
  stats: number;
  /** Where the player is: map 0 when nobody knows. */
  map: number;
  room: number;
  /** The items lying in the room, hidden or not. */
  floor: number[];
  /** The monsters in the room, by number (-1 for one nobody named). */
  monsters: number[];
  /** Whether the game has tournament play on (what `test_tournament` asks). */
  tournament: boolean;
  /** The exits play has changed, by `map/room/direction`: the exit's Para1 now (a door's
   * lock, a hidden way's flags). */
  exits: Record<string, number>;
}

export const PLAYER: Pretend = {
  name: 'Ana',
  copper: 1_000_000,
  items: [],
  spells: [],
  book: [],
  abilities: {},
  cls: 0,
  race: 0,
  level: 10,
  evil: 0,
  stats: 50,
  map: 0,
  room: 0,
  floor: [],
  monsters: [],
  tournament: false,
  exits: {},
};

/** A change to the player that would have let a line go on. */
export interface Fix {
  label: string;
  apply: (me: Pretend) => void;
}

/** What the player reads (`you`), what the others in the room read (`room`), what everyone
 * there reads (`all`), what those in another room read (`there`, its name `where`), and a note
 * of what happened that the game does not say. */
export type Shown = { to: 'you' | 'room' | 'all'; text: string } | { to: 'there'; text: string; where: string } | { to: 'note'; text: string; bad?: boolean; fix?: Fix };

/** How a line ended: run through, stopped, or nothing in it ran. */
export type Result = 'ok' | 'stop' | 'nothing';
interface Step {
  result: Result | null;
  on: boolean;
}
const OK: Step = { result: 'ok', on: true };
const STOP: Step = { result: 'stop', on: false };
const END_OK: Step = { result: 'ok', on: false };

const GREEN = '\x1b[0;32m';
const YELLOW = '\x1b[1;33m';
const WHITE = '\x1b[1;37m';
const LINES = ['Line 1', 'Line 2', 'Line 3'] as const;
const COPPER: Record<string, number> = { runic: 1_000_000, platinum: 10_000, gold: 100, silver: 10, copper: 1 };
/** How deep blocks may run each other before the chain is cut (silvermere's MAX_DEPTH). */
const MAX_DEPTH = 32;
/** The parts of a text the game shows at most (display_long_text). */
const SHOWN_PARTS = 100;
/** The abilities of a spell that keep players of an alignment from learning it. */
const [GOOD, EVIL, NOT_GOOD, NOT_EVIL, NEUTRAL] = [97, 98, 110, 111, 112];
/** The evil points from which a player is evil (EVIL_OUTLAW), and at or under which good. */
const [OUTLAW, SAINT] = [40, -51];

const text = (v: Cell | undefined) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);
const num = (v: Cell | undefined) => Number(v) || 0;
const upper = (s: string) => s.replace(/[a-z]/g, (c) => c.toUpperCase());
const wrap16 = (v: number) => ((((v % 65536) + 98304) % 65536) - 32768);
const drop = (list: number[], n: number) => list.filter((x) => x !== n);
const several = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** A message's `%s` (and `%d`) filled in order, `%%` a percent sign, as the game's printf
 * does (`fmt_msg`). */
export function fmtMsg(fmt: string, args: string[]): string {
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
    const conv = fmt[i + 1];
    if (conv === '%') {
      i++;
      out += '%';
    } else if (conv === 's' || conv === 'd' || conv === 'u' || conv === 'i') {
      i++;
      if (next < args.length) out += args[next++];
    } else if (conv === undefined && !spec) out += '%';
    else out += spec;
  }
  return out;
}

export interface PlayOptions {
  /** A roll from 0 up to `below` (not including it), as the game's `range(0, below)`. */
  dice?: (below: number) => number;
  /** An ability's name. */
  ability?: (n: number) => string;
}

/** One turn of a pretend player in a world: the text it brought, the records it reached and
 * the ways between them, and the player as it left them. */
export class Play {
  shown: Shown[] = [];
  /** The story's nodes the turn reached (`nodeId`s). */
  touched = new Set<string>();
  /** The ways it went between them, as the story's edges leave their nodes: `from>to@port`
   * (story.ts), by `nodeId`. */
  went = new Set<string>();
  private depth = 0;
  /** The records being run, the innermost last, and the row of it running: what a record
   * reaches is reached from there. */
  private at: { id: string; port: string }[] = [];
  private get: Get;
  me: Pretend;
  private dice: (below: number) => number;
  private abilityName: (n: number) => string;

  constructor(get: Get, me: Pretend, opts: PlayOptions = {}) {
    this.get = get;
    this.me = me;
    this.dice = opts.dice ?? ((below) => Math.floor(Math.random() * below));
    this.abilityName = opts.ability ?? ((n) => `ability ${n}`);
  }

  private touch(table: string, key: Cell[]) {
    const id = nodeId(table, key);
    this.touched.add(id);
    const from = this.at.at(-1);
    if (from && from.id !== id) this.went.add(`${from.id}>${id}@${from.port}`);
  }
  /** Runs `run` inside a record, from its row `port` on. */
  private async within<T>(table: string, key: Cell[], port: string, run: () => Promise<T>): Promise<T> {
    this.at.push({ id: nodeId(table, key), port });
    try {
      return await run();
    } finally {
      this.at.pop();
    }
  }
  /** The row of the record being run that what comes next leaves by. */
  private from(port: string) {
    const f = this.at.at(-1);
    if (f) f.port = port;
  }
  private you(t: string) {
    this.shown.push({ to: 'you', text: t });
  }
  private note(t: string, bad = false, fix?: Fix) {
    this.shown.push({ to: 'note', text: t, ...(bad ? { bad } : {}), ...(fix ? { fix } : {}) });
  }
  private stop(why: string, fix?: Fix): Step {
    this.note(why, true, fix);
    return STOP;
  }
  /** Says the message a failed test names, then why it failed, and stops. */
  private async fail(msg: number, why: string, fix?: Fix): Promise<Step> {
    await this.message(msg);
    return this.stop(why, fix);
  }

  /** A record's name, for a note. */
  private async nameOf(table: string, n: number, one: string): Promise<string> {
    const v = await this.get(table, [n]);
    return (v && text(v['Name'])) || `${one} ${n}`;
  }
  private ability = (n: number) => this.abilityName(n);

  /** The parts of a block in order, at most `most`, until one is missing; and the first
   * link any of them has. */
  private async parts(n: number, most: number): Promise<{ texts: string[]; link: number }> {
    const texts: string[] = [];
    let link = 0;
    for (let part = 0; part < most; part++) {
      const v = await this.get('Textblocks', [n, part]);
      if (!v) break;
      if (part === 0) this.touch('Textblocks', [n, 0]);
      if (!link) link = num(v['Link To']);
      texts.push(text(v['Data']));
    }
    return { texts, link };
  }

  /** `display_long_text`: the block's text in green (`%s` the monster's name when it
   * answers), to the player or to everyone in the room; and its link. */
  private async longText(n: number, arg: string | null, to: 'you' | 'all' = 'you'): Promise<number> {
    const { texts, link } = await this.parts(n, SHOWN_PARTS);
    if (!texts.length) this.note(n ? `There is no text block ${n}, so an empty line is shown.` : 'No text block is named, so an empty line is shown.', true);
    else this.shown.push({ to, text: GREEN + texts.map((t) => (arg === null ? t : fmtMsg(t, [arg]))).join('') });
    return link;
  }

  /** `show_action_message`: line 1 to the player, line 2 to the others in the room with the
   * player's name; a remote action tells the room first. */
  private async message(n: number, roomFirst = false) {
    if (!n) return;
    const v = await this.get('Messages', [n]);
    if (!v) {
      this.note(`There is no message ${n}, so nothing is said.`, true);
      return;
    }
    this.touch('Messages', [n]);
    const said: Shown[] = [];
    if (text(v['Line 1'])) said.push({ to: 'you', text: text(v['Line 1']) });
    if (text(v['Line 2'])) said.push({ to: 'room', text: fmtMsg(text(v['Line 2']), [this.me.name]) });
    this.shown.push(...(roomFirst ? said.reverse() : said));
  }

  private nested<T>(n: number, run: () => Promise<T>, none: T): Promise<T> {
    if (this.depth >= MAX_DEPTH) {
      this.note(`Text block ${n} is run too deep inside others, so the game cuts it off here.`, true);
      return Promise.resolve(none);
    }
    this.depth++;
    return run().finally(() => this.depth--);
  }

  /** `run_text_block`: the block's lines in turn until one runs through. */
  async runBlock(n: number): Promise<Result> {
    return this.nested(
      n,
      async () => {
        const { texts } = await this.parts(n, MAX_PARTS);
        if (!texts.length) this.note(`There is no text block ${n}, so nothing runs.`, true);
        return this.within('Textblocks', [n, 0], '', async () => {
          let result: Result = 'nothing';
          for (const [part, t] of texts.entries()) {
            // a line's row is its place among the block's lines with words in them
            let li = 0;
            for (const line of wholeLines(t)) {
              result = await this.line(line, part ? `p${part}.${li}` : String(li));
              if (line.trim()) li++;
              if (result === 'ok') return result;
            }
          }
          return result;
        });
      },
      'nothing',
    );
  }

  /** `perform_random_text_block`: one roll of 0 to 99, and the first line whose number is
   * above it runs. */
  async chances(n: number): Promise<Result> {
    const roll = this.dice(100);
    return this.nested(
      n,
      async () => {
        const { texts } = await this.parts(n, MAX_PARTS);
        if (!texts.length) this.note(`There is no text block ${n}, so nothing runs.`, true);
        return this.within('Textblocks', [n, 0], '', async (): Promise<Result> => {
          for (const [part, t] of texts.entries())
            for (const [li, pl] of patternLines(t).entries()) {
              const above = leadingInt32(pl.pattern);
              if (roll < above) {
                this.note(`The roll is ${roll} (0 to 99): the first line numbered above it, ${above}, runs.`);
                return this.line(pl.action, part ? `p${part}.${li}` : String(li));
              }
            }
          if (texts.length) this.note(`The roll is ${roll} (0 to 99): no line is numbered above it, so none runs.`);
          return 'nothing';
        });
      },
      'nothing',
    );
  }

  /** `perform_special_command`: each line of a room's commands that the typed words match
   * runs, until one runs through. Nothing when none matched. */
  async command(n: number, typed: string): Promise<Result> {
    return this.nested(
      n,
      async () => {
        const { texts } = await this.parts(n, MAX_PARTS);
        return this.within('Textblocks', [n, 0], '', async () => {
          let result: Result = 'nothing';
          for (const [part, t] of texts.entries())
            for (const [li, pl] of patternLines(t).entries())
              if (wildcardMatch(pl.pattern, typed)) {
                result = await this.line(pl.action, part ? `p${part}.${li}` : String(li));
                if (result === 'ok') return result;
              }
          return result;
        });
      },
      'nothing',
    );
  }

  /** A player in a room types `typed`, and the room takes it as the game tries it
   * (`execute_input`, after the game's own commands): the words of its Text exits, then of its
   * Remote Action exits, each the whole line in any case; then its commands, until one
   * matches. */
  async type(map: number, room: number, typed: string): Promise<Result> {
    const r = await this.get('Rooms', [map, room]);
    if (!r) {
      this.note(`There is no room ${room} on map ${map}.`, true);
      return 'nothing';
    }
    this.touch('Rooms', [map, room]);
    [this.me.map, this.me.room] = [map, room];
    const exits = this.exitsOf(map, room, r);
    for (const kind of [TEXT, REMOTE])
      for (const e of exits) {
        if (e.type !== kind || !e.to || !e.p1) continue;
        const words = await this.get('Messages', [e.p1]);
        if (!words || !LINES.some((c) => sameWords(text(words[c]), typed))) continue;
        const port = `${kind === TEXT ? 'x' : 'r'}${e.dir}`;
        if (await this.within('Rooms', [map, room], port, () => (kind === TEXT ? this.walk(map, e) : this.remote(map, e)))) return 'ok';
      }
    const cmd = num(r['Command Text']);
    const result = cmd ? await this.within('Rooms', [map, room], 'types', () => this.command(cmd, typed)) : 'nothing';
    if (result === 'nothing') this.note(cmd ? 'None of the room’s commands takes it, so the game tries it as a spell or an action next.' : 'The room has no commands of its own.');
    return result;
  }

  /** A room's exits as play has left them. */
  private exitsOf(map: number, room: number, r: Values): Exit[] {
    return exitsOf(this.asPlayed(map, room, r));
  }
  /** A room's values, its exits as play has left them. */
  private asPlayed(map: number, room: number, r: Values): (c: string) => Cell | undefined {
    return (c) => {
      const d = /^Para1 (\d)$/.exec(c)?.[1];
      const changed = d === undefined ? undefined : this.me.exits[`${map}/${room}/${d}`];
      return changed ?? r[c];
    };
  }
  private changed(map: number, room: number, e: Exit) {
    this.me.exits[`${map}/${room}/${e.dir}`] = e.p1;
  }

  /** The room as a player arriving reads it (`show_room`): its name, description and exits. */
  private async screen(map: number, room: number, r: Values): Promise<string> {
    const get = this.asPlayed(map, room, r);
    // a found hidden way's naming message
    const names = new Map<number, Lines | null>();
    for (const e of exitsOf(get))
      if (e.type === HIDDEN && e.p4 && shown(e)) {
        const v = await this.get('Messages', [e.p4]);
        names.set(e.p4, v ? [text(v['Line 1']), text(v['Line 2']), text(v['Line 3'])] : null);
      }
    return roomScreen(get, (n) => names.get(n) ?? null) ?? '';
  }

  /** A Text exit's words take the player through it (`text_exit`, the exit's `text` gate):
   * its message's line 2 to the room they leave, line 3 to the room they reach and line 1 to
   * them, then the room they reach; without a message, the ordinary way. */
  private async walk(map: number, e: Exit): Promise<boolean> {
    const dir = DIRS[e.dir]!;
    const to = await this.get('Rooms', [map, e.to]);
    if (!to) {
      this.note(`There is no room ${e.to} on map ${map} for the ${dir} exit to take you to, so you stay.`, true);
      return true;
    }
    this.touch('Rooms', [map, e.to]);
    const where = text(to['Name']) || `room ${e.to}`;
    const m = e.p3 ? await this.get('Messages', [e.p3]) : null;
    if (m) {
      this.touch('Messages', [e.p3]);
      // the name in yellow, the rest of the line green (sic); a drag's words after it
      const who = YELLOW + this.me.name + GREEN;
      const [mine, left, reached] = LINES.map((c) => text(m[c]));
      if (left) this.shown.push({ to: 'room', text: fmtMsg(left, [who, '']) });
      if (reached) this.shown.push({ to: 'there', where, text: GREEN + fmtMsg(reached, [who, '']) });
      if (mine) this.you(YELLOW + fmtMsg(mine, ['']));
    } else this.note(e.p3 ? `There is no message ${e.p3}, so you go ${dir} the ordinary way.` : `You go ${dir} the ordinary way, the others here seeing you leave.`, !!e.p3);
    Object.assign(this.me, { map, room: e.to, floor: [], monsters: [] });
    this.you(await this.screen(map, e.to, to));
    return true;
  }

  /** A Remote Action exit's words (`RemoteExitWords`): an item it names must be carried, and
   * loses a charge; then the exit it names, of the room it leads to, is worked. False when
   * the words go on to the next exit. */
  private async remote(map: number, e: Exit): Promise<boolean> {
    const dir = DIRS[e.dir]!;
    if (e.p4) {
      const item = await this.get('Items', [e.p4]);
      if (!item) {
        this.note(`There is no item ${e.p4} for the ${dir} exit's words to need, so they go on.`, true);
        return false;
      }
      const name = text(item['Name']) || `item ${e.p4}`;
      if (!this.me.items.includes(e.p4)) {
        const need = e.p4;
        this.note(`You are not carrying ${name}, so the ${dir} exit's words go on.`, true, { label: `Carry ${name}`, apply: (p) => p.items.push(need) });
        return false;
      }
      this.touch('Items', [e.p4]);
      this.note(`Your ${name} loses a charge.`);
    }
    const far = await this.get('Rooms', [map, e.to]);
    if (!far) {
      this.note(`There is no room ${e.to} on map ${map} for the ${dir} exit's words to work, so they go on.`, true);
      return false;
    }
    const { action, exit } = remoteOf(e);
    await this.worked(map, e.to, far, e.p3, action, exit);
    return true;
  }

  /** `run_room_exit_action` on a room of the player's map: the message, line 2 to the
   * player's room first and then line 1 to them; then its exit `exit` is worked by `action`. */
  private async worked(map: number, room: number, r: Values, msg: number, action: number, exit: number): Promise<void> {
    this.touch('Rooms', [map, room]);
    await this.message(msg, true);
    if (exit < 0 || exit > 9) {
      this.note(`${exit === 10 ? 'Exit 10 is past the last direction' : 'It names no exit'}, so only the message is said.`);
      return;
    }
    const dir = DIRS[exit]!;
    const here = map === this.me.map && room === this.me.room;
    const name = text(r['Name']) || `room ${room}`;
    const of = here ? 'here' : `of ${name}`;
    const e = this.exitsOf(map, room, r)[exit]!;
    const w = workExit(e, action);
    switch (w.kind) {
      case 'swapped': {
        this.changed(map, room, w.exit);
        // the same door from the other side, when it leads straight back
        const back = e.to ? await this.get('Rooms', [map, e.to]) : null;
        const b = back ? this.exitsOf(map, e.to, back)[BACK[exit]!]! : null;
        const bw = b && b.to === room ? workExit(b, action) : null;
        if (bw?.kind === 'swapped') this.changed(map, e.to, bw.exit);
        const what = e.type === GATE ? 'gate' : 'door';
        this.note(`The ${dir} ${what} ${of} is ${w.exit.p1 ? 'shut and locked' : 'unlocked and open'} now${bw?.kind === 'swapped' ? ', from the other side too' : ''}.`);
        return;
      }
      case 'thrown':
        this.note(`Everyone ${here ? 'here' : `in ${name}`}, players and then monsters, is thrown ${dir} through the hidden trap.`);
        return;
      case 'cleared': {
        this.changed(map, room, w.exit);
        if (w.left) {
          this.note(
            w.cleared
              ? `${w.cleared === 1 ? 'A block' : `${w.cleared} blocks`} of the ${dir} hidden way ${of} ${w.cleared === 1 ? 'goes' : 'go'}, and ${several(w.left, 'block')} ${w.left === 1 ? 'is' : 'are'} left.`
              : `No block of the ${dir} hidden way ${of} goes, so ${several(w.left, 'block')} still ${w.left === 1 ? 'blocks' : 'block'} it.`,
          );
          return;
        }
        // everyone in its room is told, in its own words or the game's
        const m = e.p3 ? await this.get('Messages', [e.p3]) : null;
        const said = m ? (text(m['Line 1']) ? fmtMsg(text(m['Line 1']), [dir]) : '') : `A concealed passage opens to the ${dir}.`;
        if (said) this.shown.push(here ? { to: 'all', text: WHITE + said } : { to: 'there', where: name, text: WHITE + said });
        this.note(`The ${dir} hidden way ${of} opens, and shows among the exits.`);
        return;
      }
      case 'nothing':
        this.note(`The ${dir} exit ${of} is no door, gate, hidden way or hidden trap, so nothing more happens.`);
    }
  }

  /** `ask_monster_a_question`: a greeting (no question) shows the text the monster's talk
   * block links to; a question, the text of the first topic whose words are in it, and then
   * runs the block that text links to. */
  async ask(monster: number, question: string | null): Promise<void> {
    const m = await this.get('Monsters', [monster]);
    if (!m) {
      this.note(`There is no monster ${monster}.`, true);
      return;
    }
    this.touch('Monsters', [monster]);
    await this.within('Monsters', [monster], 'talks', () => this.askBlock(num(m['Greet Txt']), text(m['Name']), question));
  }

  /** The asking from the talk block on: `name` is the monster's. */
  async askBlock(talk: number, name: string, question: string | null): Promise<void> {
    if (!talk) {
      this.you(`${name} has nothing to tell you!`);
      return;
    }
    // the first part only; without it the game says nothing at all
    const blk = await this.get('Textblocks', [talk, 0]);
    if (!blk) {
      this.note(`There is no text block ${talk}, so the game says nothing at all.`, true);
      return;
    }
    this.touch('Textblocks', [talk, 0]);
    await this.within('Textblocks', [talk, 0], 'greet', async () => {
      const link = num(blk['Link To']);
      if (question === null) {
        if (!link) this.you(`${name} doesn't understand you.`);
        else await this.longText(link, name);
        return;
      }
      const q = upper(question);
      // a topic's row is its place among the lines with a pattern or a block
      let ti = 0;
      for (const pl of patternLines(text(blk['Data']))) {
        const row = ti;
        if (pl.pattern || pl.action.trim()) ti++;
        if (!q.includes(upper(pl.pattern))) continue;
        this.from(`t${row}`);
        const shown = leadingInt32(pl.action);
        const then = await this.longText(shown, name);
        if (then) await this.within('Textblocks', [shown, 0], 'then', () => this.runBlock(then));
        return;
      }
      this.you(`${name} has nothing to tell you!`);
    });
  }

  /** A text block shown as the game shows it (`%s` left for the monster's name); as an answer,
   * the block it links to runs after it. */
  async show(n: number, answer: boolean): Promise<void> {
    const then = await this.longText(n, null);
    if (answer && then) await this.within('Textblocks', [n, 0], 'then', () => this.runBlock(then));
  }

  /** `perform_matched_action`: the clauses of a line in turn until one stops it; `li` is the
   * line's row, each step's `li.si`. */
  private async line(line: string, li: string): Promise<Result> {
    let result: Result = 'nothing';
    for (const [si, c] of clauses(line).entries()) {
      this.from(`${li}.${si}`);
      const step = await this.clause(c);
      if (step.result) result = step.result;
      if (!step.on) break;
    }
    return result;
  }

  /** Whether a spell is the player's to learn (`can_use_spell`): why not, or null. */
  private async cannotLearn(spell: Values): Promise<{ why: string; fix?: Fix; strong?: boolean } | null> {
    const me = this.me;
    const cls = me.cls ? await this.get('Classes', [me.cls]) : null;
    if (!cls) return { why: 'You have no class the game knows, so you cannot learn it.' };
    const magery = num(spell['Magery A']);
    if (magery && magery !== num(cls['Magic Type'])) return { why: `${text(cls['Name']) || 'Your class'} casts another kind of magic, so you cannot learn it.` };
    if (magery && num(spell['Magery B']) > num(cls['Magic LVL'])) return { why: `${text(cls['Name']) || 'Your class'} has too little of its magic for it, so you cannot learn it.` };
    const level = num(spell['Level']);
    if (level > me.level) return { why: `You are level ${me.level}, under the spell's ${level}, so you cannot learn it.`, fix: { label: `Be level ${level}`, apply: (p) => (p.level = Math.max(p.level, level)) }, strong: true };
    const has = new Set(Array.from({ length: 10 }, (_, i) => num(spell[`Ability ${i}`])));
    const band = me.evil <= SAINT ? 'good' : me.evil < OUTLAW ? 'neutral' : 'evil';
    const barred = { good: [EVIL, NOT_GOOD, NEUTRAL], neutral: [EVIL, GOOD], evil: [GOOD, NOT_EVIL, NEUTRAL] }[band];
    if (barred.some((a) => has.has(a))) return { why: `The spell is not for ${band} players, so you cannot learn it.` };
    return null;
  }

  private async clause(c: Clause): Promise<Step> {
    const me = this.me;
    const n = c.n;
    switch (c.kw) {
      case null:
        return { result: null, on: true };
      case 'price': {
        const price = Math.max(0, n.copper!);
        if (me.copper >= price) {
          me.copper -= price;
          if (price) this.note(`You pay ${coinsOf(price)}.`);
          return OK;
        }
        const short = price - me.copper;
        const why = me.copper > 0 ? `You have ${coinsOf(me.copper)}, ${coinsOf(short)} short of the price of ${coinsOf(price)}` : `You have no money for the price of ${coinsOf(price)}`;
        return this.fail(n.msg!, `${why}, so the line stops.`, { label: `Add ${coinsOf(short)}`, apply: (p) => (p.copper += short) });
      }
      case 'cast': {
        const v = await this.get('Spells', [n.spell!]);
        if (v) {
          this.touch('Spells', [n.spell!]);
          if (!me.spells.includes(n.spell!)) me.spells.push(n.spell!);
          this.note(`${text(v['Name']) || `Spell ${n.spell}`} is cast on you.`);
        }
        return OK;
      }
      case 'giveability': {
        const a = n.ability!;
        if (me.abilities[a] === undefined || me.abilities[a] < n.value!) me.abilities[a] = n.value!;
        this.note(`Your ${this.ability(a)} is now ${me.abilities[a]}.`);
        return OK;
      }
      case 'addability': {
        const a = n.ability!;
        me.abilities[a] = wrap16((me.abilities[a] ?? 0) + n.amount!);
        this.note(`Your ${this.ability(a)} is now ${me.abilities[a]}.`);
        return OK;
      }
      case 'removeability': {
        const a = n.ability!;
        if (!(a in me.abilities)) return this.stop(`You have no ${this.ability(a)} to lose, so the line stops.`, { label: `Have ${this.ability(a)}`, apply: (p) => (p.abilities[a] = p.abilities[a] ?? 1) });
        delete me.abilities[a];
        this.note(`You lose your ${this.ability(a)}.`);
        return OK;
      }
      case 'teleport': {
        if (c.word) {
          this.note(`You are teleported to a room of ${TELEPORTS[c.word] ?? 'somewhere'} by chance.`);
          return OK;
        }
        const [room, map] = [n.room!, n.map!];
        if (!room || !map) return OK;
        const to = await this.get('Rooms', [map, room]);
        if (!to) return this.stop(`There is no room ${room} on map ${map}, so you go nowhere and the line stops.`);
        this.touch('Rooms', [map, room]);
        Object.assign(me, { map, room, floor: [], monsters: [] });
        this.note(`You are teleported to ${text(to['Name']) || `room ${room}`}.`);
        return OK;
      }
      case 'nomonsters':
        // only a clause another follows tests it
        if (c.more && me.monsters.length) return this.fail(n.msg!, 'A monster is here, so the line stops.', { label: 'Clear the room', apply: (p) => (p.monsters = []) });
        return OK;
      case 'monsters':
        if (!me.monsters.length) return this.fail(n.msg!, 'No monster is here, so the line stops.', { label: 'Bring a monster', apply: (p) => p.monsters.push(-1) });
        return OK;
      case 'needmonster': {
        this.touch('Monsters', [n.monster!]);
        if (me.monsters.includes(n.monster!)) return OK;
        const name = await this.nameOf('Monsters', n.monster!, 'monster');
        return this.fail(n.msg!, `${name} is not here, so the line stops.`, { label: `Bring ${name}`, apply: (p) => p.monsters.push(n.monster!) });
      }
      case 'summon': {
        const v = await this.get('Monsters', [n.monster!]);
        if (!v) return this.stop(`There is no monster ${n.monster} to summon, so the line stops.`);
        this.touch('Monsters', [n.monster!]);
        me.monsters.push(n.monster!);
        this.note(`${text(v['Name']) || `Monster ${n.monster}`} appears.`);
        return OK;
      }
      case 'message':
        await this.message(n.msg!);
        return OK;
      case 'takeitem': {
        const name = await this.nameOf('Items', n.item!, 'item');
        const at = me.items.indexOf(n.item!);
        if (at < 0) return this.fail(n.msg!, `You are not carrying ${name}, so the line stops.`, { label: `Carry ${name}`, apply: (p) => p.items.push(n.item!) });
        me.items.splice(at, 1);
        this.touch('Items', [n.item!]);
        this.note(`${name} is taken from you.`);
        return OK;
      }
      case 'addexp':
        this.note(`You get ${Math.max(0, n.exp!).toLocaleString('en-US')} experience.`);
        return OK;
      case 'giveitem': {
        const v = await this.get('Items', [n.item!]);
        if (!v) {
          this.note(`There is no item ${n.item}, so you get nothing and the line ends.`, true);
          return END_OK;
        }
        this.touch('Items', [n.item!]);
        me.items.push(n.item!);
        this.note(`You get ${text(v['Name']) || `item ${n.item}`}.`);
        return OK;
      }
      case 'hideitem': {
        const v = await this.get('Items', [n.item!]);
        if (v) {
          this.touch('Items', [n.item!]);
          me.floor.push(n.item!);
          this.note(`${text(v['Name']) || `Item ${n.item}`} is hidden in the room, and the line ends.`);
        } else this.note(`There is no item ${n.item} to hide, and the line ends.`, true);
        return END_OK;
      }
      case 'checkability': {
        const [a, min] = [n.ability!, n.min!];
        const v = me.abilities[a];
        if (v !== undefined && v >= min) return OK;
        return this.stop(`Your ${this.ability(a)} is ${v ?? 'nothing'}, under ${min}, so the line stops.`, { label: `Make ${this.ability(a)} ${min}`, apply: (p) => (p.abilities[a] = min) });
      }
      case 'class':
      case 'race': {
        const [mine, want, table] = c.kw === 'class' ? [me.cls, n.class!, 'Classes'] : [me.race, n.race!, 'Races'];
        this.touch(table, [want]);
        if (mine === want) return OK;
        const name = await this.nameOf(table, want, c.kw);
        return this.stop(`You are not a ${name}, so the line stops.`, { label: `Be a ${name}`, apply: (p) => (c.kw === 'class' ? (p.cls = want) : (p.race = want)) });
      }
      case 'goodaligned':
      case 'evilaligned': {
        const limit = n.limit!;
        if (c.kw === 'goodaligned' ? me.evil <= limit : me.evil >= limit) return OK;
        return this.fail(n.msg!, `You have ${me.evil} evil points, ${c.kw === 'goodaligned' ? 'over' : 'under'} ${limit}, so the line stops.`, { label: `Have ${limit} evil points`, apply: (p) => (p.evil = limit) });
      }
      case 'minlevel':
      case 'maxlevel': {
        const level = n.level!;
        if (c.kw === 'minlevel' ? me.level >= level : me.level <= level) return OK;
        return this.fail(n.msg!, `You are level ${me.level}, ${c.kw === 'minlevel' ? 'under' : 'over'} ${level}, so the line stops.`, { label: `Be level ${level}`, apply: (p) => (p.level = level) });
      }
      case 'checkitem':
      case 'failitem': {
        const item = n.item!;
        this.touch('Items', [item]);
        const has = me.items.includes(item);
        if (has !== (c.kw === 'failitem')) return OK;
        const name = await this.nameOf('Items', item, 'item');
        return this.fail(
          n.msg!,
          `You are ${has ? '' : 'not '}carrying ${name}, so the line stops.`,
          has ? { label: `Drop ${name}`, apply: (p) => (p.items = drop(p.items, item)) } : { label: `Carry ${name}`, apply: (p) => p.items.push(item) },
        );
      }
      case 'text':
        await this.longText(n.block!, null);
        return OK;
      case 'roomtext':
        await this.longText(n.block!, null, 'all');
        return OK;
      case 'roomitem':
      case 'failroomitem': {
        const item = n.item!;
        this.touch('Items', [item]);
        const here = me.floor.includes(item);
        if (here !== (c.kw === 'failroomitem')) return OK;
        const name = await this.nameOf('Items', item, 'item');
        return this.fail(
          n.msg!,
          `${name} is ${here ? '' : 'not '}in the room, so the line stops.`,
          here ? { label: `Take ${name} away`, apply: (p) => (p.floor = drop(p.floor, item)) } : { label: `Put ${name} here`, apply: (p) => p.floor.push(item) },
        );
      }
      case 'clearitem': {
        const item = n.item!;
        if (!item) {
          me.floor = [];
          this.note('Everything in the room is cleared away.');
          return OK;
        }
        const name = await this.nameOf('Items', item, 'item');
        if (!me.floor.includes(item)) return this.fail(n.msg!, `${name} is not in the room to clear, so the line stops.`, { label: `Put ${name} here`, apply: (p) => p.floor.push(item) });
        me.floor = drop(me.floor, item);
        this.touch('Items', [item]);
        this.note(`${name} is cleared from the room.`);
        return OK;
      }
      case 'addevil':
        me.evil = wrap16(me.evil + n.points!);
        this.note(`You have ${me.evil} evil points now.`);
        return OK;
      case 'testability': {
        const [a, max] = [n.ability!, n.max!];
        const v = me.abilities[a];
        if (v === undefined ? max < 0 : v <= max) return OK;
        const why = v === undefined ? `You have no ${this.ability(a)}, so the line stops.` : `Your ${this.ability(a)} is ${v}, over ${max}, so the line stops.`;
        return this.stop(why, { label: `Make ${this.ability(a)} ${max}`, apply: (p) => (p.abilities[a] = max) });
      }
      case 'failability': {
        const a = n.ability!;
        if (!(a in me.abilities)) return OK;
        return this.fail(n.msg!, `You have ${this.ability(a)}, so the line stops.`, { label: `Lose ${this.ability(a)}`, apply: (p) => delete p.abilities[a] });
      }
      case 'remoteaction': {
        const exit = n.exit!;
        if (exit < 0 || exit > 10) return OK;
        if (!me.map) {
          // a story from a monster or a text block: nobody knows the player's map
          await this.message(n.msg!, true);
          this.note(exit === 10 ? `Only the message is said, if room ${n.room} is on your map.` : `The ${DIRS[exit]} exit of room ${n.room} on your map is worked (a door, a hidden way or a hidden trap), if that room is there.`);
          return OK;
        }
        const room = await this.get('Rooms', [me.map, n.room!]);
        if (!room) {
          this.note(`There is no room ${n.room} on map ${me.map}, so nothing happens.`, true);
          return OK;
        }
        await this.worked(me.map, n.room!, room, n.msg!, n.action!, exit);
        return OK;
      }
      case 'random': {
        const r = await this.chances(n.block!);
        return { result: r, on: r !== 'stop' };
      }
      case 'adddelay':
        this.note(`You must wait ${n.ticks! & 255} ticks more.`);
        return OK;
      case 'testskill': {
        const known = !!c.word && c.word in SKILLS;
        const [name, range] = known ? SKILLS[c.word!]! : [`“${(c.word ?? '').replace(/^\?/, '')}”, a skill the game does not know,`, 100];
        const value = known ? me.stats : 0;
        const roll = this.dice(range) + n.modifier!;
        if (roll <= value) {
          this.note(`The roll is ${roll} against your ${name} of ${value}: not above it, so the line goes on.`);
          return OK;
        }
        const sure = range - 1 + n.modifier!;
        this.note(
          `The roll is ${roll} against your ${name} of ${value}: above it, so block ${n.block} runs and the line stops.`,
          true,
          known && sure > value ? { label: `Make stats ${sure}`, apply: (p) => (p.stats = Math.max(p.stats, sure)) } : undefined,
        );
        await this.runBlock(n.block!);
        return STOP;
      }
      case 'checkskill': {
        const known = !!c.word && c.word in SKILLS;
        const value = known ? me.stats : 0;
        if (value >= n.min!) return OK;
        const name = known ? SKILLS[c.word!]![0] : 'skill, one the game does not know,';
        return this.fail(n.msg!, `Your ${name} is ${value}, under ${n.min}, so the line stops.`, known ? { label: `Make stats ${n.min}`, apply: (p) => (p.stats = Math.max(p.stats, n.min!)) } : undefined);
      }
      case 'givecoins': {
        const copper = Math.max(0, n.amount!) * (COPPER[c.word ?? 'copper'] ?? 1);
        me.copper += copper;
        this.note(`You get ${coinsOf(copper)}.`);
        return OK;
      }
      case 'test_tournament':
        if (me.tournament) return OK;
        return this.stop('Tournament play is off, so the line stops.', { label: 'Turn tournament play on', apply: (p) => (p.tournament = true) });
      case 'checkspell': {
        const spell = n.spell!;
        this.touch('Spells', [spell]);
        if (me.spells.includes(spell)) return OK;
        const name = await this.nameOf('Spells', spell, 'spell');
        this.note(`${name} is not on you, so block ${n.block} runs and the line stops.`, true, { label: `Cast ${name} on me`, apply: (p) => p.spells.push(spell) });
        await this.runBlock(n.block!);
        return STOP;
      }
      case 'flag':
        if (n.flag! >= 1 && n.flag! <= 64) return OK;
        return this.stop(`Flag ${n.flag} is not one of 1 to 64, so the line stops.`);
      case 'learnspell': {
        const spell = n.spell!;
        const v = spell ? await this.get('Spells', [spell]) : null;
        if (!v) return this.stop(`There is no spell ${spell} to learn, so the line stops.`);
        this.touch('Spells', [spell]);
        if (me.book.includes(spell)) return OK;
        const no = await this.cannotLearn(v);
        if (no) {
          this.you(no.strong ? "This spell is too powerful for you - You don't know what to do with this!" : "You don't know what to do with this!");
          return this.stop(no.why.replace(/\.$/, ', and the line stops.'), no.fix);
        }
        me.book.push(spell);
        this.you(`You learn the spell ${text(v['Name'])}.`);
        return OK;
      }
    }
  }
}
