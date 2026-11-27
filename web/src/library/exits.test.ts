import assert from 'node:assert/strict';
import { test } from 'node:test';
import { exitsOf, exitWord, obviousExits, printfS, roomScreen, shown, wayOf, waysOf, wordsOf, wrap79, type Exit, type Lines } from './exits.ts';
import { coinsOf, type Bit } from './script.ts';

/** Words as one text: a record as `{Rooms 1/607}`. */
const said = (bits: Bit[]) =>
  bits
    .map((b) => (typeof b === 'string' ? b : 'ref' in b ? `{${b.ref.t} ${b.ref.map === undefined ? '' : `${b.ref.map}/`}${b.ref.n}}` : 'ability' in b ? `{ability ${b.ability}}` : coinsOf(b.copper)))
    .join('');
const plain = (s: string | undefined) => (s ?? '').replace(/\x1b\[[0-9;]*m/g, '');

// stock 1.11p messages
const MESSAGES: Record<number, Lines> = {
  12: ['go manhole', 'go man', 'enter manhole'],
  8335: ['You pull open the manhole cover, and slip inside the hole.', '%s enters a manhole in the ground!', '%s enters from a manhole overhead!'],
  8259: ['push button', 'push but', 'push button'],
  8332: ['You push the button.', '%s pushes a button!', ''],
  1267: ['A wooden panel in the wall slides open.', 'A wooden panel in the wall slides open.', 'A wooden panel in the wall slides open.'],
  1268: ['secret passage %s', 'secret passage %s', 'secret passage %s'],
};
const lines = (n: number) => MESSAGES[n] ?? null;

/** A room's values from exits written `dir: [to, type, p1, p2, p3, p4]`. */
function room(exits: Record<number, number[]>, more: Record<string, number | string> = {}) {
  const v: Record<string, number | string> = { ...more };
  for (const [d, [to, type, p1 = 0, p2 = 0, p3 = 0, p4 = 0]] of Object.entries(exits)) Object.assign(v, { [`Exit ${d}`]: to!, [`Type ${d}`]: type!, [`Para1 ${d}`]: p1, [`Para2 ${d}`]: p2, [`Para3 ${d}`]: p3, [`Para4 ${d}`]: p4 });
  return (c: string) => v[c] ?? (c.startsWith('Desc') ? '' : 0);
}
const exit = (dir: number, to: number, type: number, p1 = 0, p2 = 0, p3 = 0, p4 = 0): Exit => ({ dir, to, type, p1, p2, p3, p4 });

// Adventurer's Guild, Halls of Training (1/493): a button opens a panel north
const GUILD = exitsOf(room({ 0: [506, 6, 16, -1, 1267, 1268], 1: [492, 0], 2: [494, 0], 3: [493, 12, 8259, 10, 8332] }));
// Intersection of Brass St. & River St. (1/17): a manhole down
const CORNER = exitsOf(room({ 1: [386, 19], 2: [16, 19], 3: [18, 0], 9: [607, 10, 12, 0, 8335] }));

test('a room has ten exits, each read from its columns', () => {
  assert.equal(GUILD.length, 10);
  assert.deepEqual(GUILD[3], exit(3, 493, 12, 8259, 10, 8332));
  assert.deepEqual(waysOf(GUILD).map((e) => e.dir), [0, 1, 2, 3]);
});

test('Obvious exits names what the game names, as it names it', () => {
  assert.equal(plain(obviousExits(GUILD, lines)), 'Obvious exits: south, east');
  assert.equal(plain(obviousExits(CORNER, lines)), 'Obvious exits: south, east, west');
  assert.equal(plain(obviousExits(exitsOf(() => 0), lines)), 'Obvious exits: NONE!!!');
  // doors and gates by their state, up and down as trap doors, secret passages with the game's capitals
  assert.equal(exitWord(exit(0, 5, 7, 1), lines), 'closed door north');
  assert.equal(exitWord(exit(2, 5, 11, 0), lines), 'open gate east');
  assert.equal(exitWord(exit(8, 5, 2, 0, 2), lines), 'closed trap door above');
  assert.equal(exitWord(exit(0, 5, 6, 4), lines), 'Secret passage north');
  assert.equal(exitWord(exit(2, 5, 6, 4), lines), 'secret passage East');
  assert.equal(exitWord(exit(9, 5, 6, 8), lines), 'secret passage below');
  assert.equal(exitWord(exit(0, 506, 6, 8, -1, 1267, 1268), lines), 'secret passage north');
  assert.equal(exitWord(exit(0, 506, 6, 8, -1, 1267, 77), () => undefined), undefined);
  // a found hidden way and a trap are shown; a text exit, a shut timed exit and a hidden trap are not
  assert.equal(shown(exit(0, 5, 6, 4)), true);
  assert.equal(shown(exit(0, 5, 10, 12)), false);
  assert.equal(shown(exit(0, 5, 16, 0, 3, 2)), false);
  assert.equal(shown(exit(0, 5, 16, 0, 3, 0)), true);
  assert.equal(shown(exit(0, 5, 9, 10, 2)), false);
  assert.equal(shown(exit(0, 0, 9, 10, 0)), true);
  assert.equal(printfS('100%% %s %s %d', 'x'), '100% x %s %d');
});

test('a text exit takes the player through when the whole line is its words', () => {
  const [w] = wordsOf(CORNER, { map: 1, room: 17 }, lines, () => undefined);
  assert.equal(w!.kind, 'text');
  assert.deepEqual(w!.words, ['go manhole', 'go man', 'enter manhole']);
  assert.equal(said(w!.says), 'takes them down to {Rooms 1/607}, saying {Messages 8335}');
  assert.equal(w!.taken, undefined);
  const way = wayOf(CORNER[9]!, lines);
  assert.equal(way.walks, false);
  assert.equal(way.word, null);
  assert.match(said(way.says), /never walked/);
});

test('a remote action works the exit it names, here a hidden panel', () => {
  const [w] = wordsOf(GUILD, { map: 1, room: 493 }, lines, () => undefined);
  assert.equal(w!.kind, 'remote');
  // the same words twice are one
  assert.deepEqual(w!.words, ['push button', 'push but']);
  assert.equal(said(w!.says), 'works the north exit here: clears block 1 of the hidden way');
  assert.deepEqual(w!.more.map(said), [
    'With no block left it opens, saying {Messages 1267}, and shows as “secret passage north”.',
    'It says {Messages 8332}: line 2 to the room first, then line 1 to them.',
  ]);
  const north = wayOf(GUILD[0]!, lines);
  assert.equal(north.walks, false);
  assert.equal(said(north.says), 'blocked, by 1 block');
  assert.deepEqual(north.more.map(said), [
    'Remote actions and scripts clear its blocks, in any order. With none left it opens, saying {Messages 1267}.',
    'Opened, it is blocked again a step later, by 1 block.',
  ]);
});

test('a remote action on another room reads that room, and says when it is not there', () => {
  const far = (n: number) => (n === 40 ? [exit(0, 0, 0), exit(1, 41, 7, 2, 0, 3)] : n === 50 ? null : undefined);
  const here = exitsOf(room({ 0: [40, 12, 42, 11, 0, 868], 1: [50, 12, 42, 1], 2: [60, 12, 42, 2] }));
  const [a, b, c] = wordsOf(here, { map: 1, room: 9 }, (n) => (n === 42 ? ['pull lever', '', ''] : null), far);
  assert.equal(said(a!.says), 'for those carrying {Items 868}, which loses a use: works the south exit of {Rooms 1/40}: opens the door');
  assert.equal(said(a!.more[0]!), 'The same door from the other side swaps too. It locks again 3 steps later.');
  assert.equal(said(b!.says), 'nothing: there is no room 50 on map 1, so the words go on');
  assert.match(said(c!.says), /…$/);
  // the north exit answers the same words first
  assert.equal(b!.taken, 'The north exit answers the same words first, so these never work.');
});

test('every exit type is said as the game rules it', () => {
  const says = (e: Exit) => said(wayOf(e, lines).says);
  const more = (e: Exit) => wayOf(e, lines).more.map(said).join(' / ');
  assert.equal(says(exit(0, 5, 0)), '');
  assert.equal(says(exit(0, 5, 3, 868, 77, 0)), 'only for those carrying {Items 868}');
  assert.match(more(exit(0, 5, 3, 868, 77, 0)), /Without it: \{Messages 77\}/);
  assert.equal(says(exit(0, 5, 4, 50)), 'a toll of 50 gold');
  assert.match(more(exit(0, 5, 4, 1)), /toll of 1 gold crown\./);
  assert.equal(says(exit(0, 5, 7, 2, -20)), 'a door, locked');
  assert.match(more(exit(0, 5, 7, 2, -20)), /picklocks − 20/);
  assert.match(more(exit(0, 5, 7, 1, 5)), /never locks again/);
  assert.equal(says(exit(0, 5, 2, 868, 2, 10, 4)), 'a door, locked, its key {Items 868}');
  assert.match(more(exit(0, 5, 2, 868, 2, 10, 4)), /USE \{Items 868\} NORTH unlocks it, as does PICKLOCK NORTH on a roll of 0 to 99 under their picklocks \+ 10, if they have any; then OPEN NORTH\./);
  // a lock not locked yet is told for when it locks; no key, no USE
  assert.equal(more(exit(0, 5, 2, 868, 1, 0, 4)), 'OPEN NORTH opens it. / Until it is open, walking it says “There is a closed door in that direction!”. / Opened, it locks again 4 steps later. / Once it has locked, USE {Items 868} NORTH unlocks it, as does PICKLOCK NORTH on a roll of 0 to 99 under their picklocks, if they have any; then OPEN NORTH. / Nobody can be dragged through.');
  assert.match(more(exit(0, 5, 7, 2, -20)), /^Walking it locked says “The door is closed!”\. \/ No key fits it, but PICKLOCK NORTH unlocks it on a roll of 0 to 99 under their picklocks − 20, if they have any; then OPEN NORTH\./);
  assert.match(more(exit(0, 5, 11, 1, 0, 3)), /it locks again 3 steps later\. \/ Once it has locked, no key fits it/);
  assert.doesNotMatch(more(exit(0, 5, 11, 1, 5)), /PICKLOCK/);
  assert.equal(says(exit(0, 99, 8, 2)), 'into map 2');
  assert.equal(says(exit(0, 5, 9, 10, 0, 3)), 'a trap: walking it hurts them 5 to 10, saying {Messages 3}, and through they go');
  assert.equal(says(exit(0, 5, 13, 3)), 'only for {Classes 3}');
  assert.equal(says(exit(0, 5, 14, 0, 2)), 'not for {Races 2}');
  assert.equal(says(exit(0, 5, 15, 5, 20)), 'only levels 5 to 20');
  assert.equal(says(exit(0, 5, 15, 5, 0)), 'only level 5 and up');
  // only the limits it has are refused
  assert.match(more(exit(0, 5, 15, 5, 0)), /^Too low: [^/]*\. \/ Closed/);
  assert.match(more(exit(0, 5, 15, 0, 20)), /^Too high: [^/]*\. \/ Closed/);
  assert.equal(says(exit(0, 5, 20, 65486, 40)), 'only for -50 to 40 evil points');
  assert.equal(says(exit(0, 5, 22, 12, 0, 3, 4)), 'casts {Spells 12} on them first');
  assert.equal(says(exit(0, 5, 23, 61, 1, 5)), 'only with {ability 61} of 1 to 5');
  assert.equal(says(exit(0, 5, 6, 2)), 'hidden until SEARCH NORTH finds it');
  assert.equal(says(exit(0, 5, 6, 1)), 'a hidden way: not among the exits, but walking it works');
  assert.equal(says(exit(0, 5, 1)), 'never walked: walking it they meet a wall');
  assert.match(more(exit(0, 5, 17)), /Ticket exit: the game has no rule for it/);
  assert.match(more(exit(0, 5, 15, 5)), /Closed during tournament play\. \/ Nobody can be dragged through\./);
});

test('lines wrap at 79 columns, and an arriving player reads the room', () => {
  const long = `Obvious exits: ${Array.from({ length: 12 }, () => 'closed door north').join(', ')}`;
  for (const l of wrap79(long).split('\n')) assert.ok(l.length <= 79, l);
  assert.equal(wrap79('a b'), 'a b');
  const screen = roomScreen(room({ 1: [2, 0] }, { Name: 'Vault', 'Desc 0': 'A dark room.  ', 'Desc 1': 'Cold.' }), lines);
  assert.equal(plain(screen), 'Vault\n    A dark room. Cold.\nObvious exits: south');
});
