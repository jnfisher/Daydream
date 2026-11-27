import assert from 'node:assert/strict';
import { test } from 'node:test';
import { clauses, coinsOf, keywordOf, leadingInt, leadingInt32, linesOf, parseClause, readingFor, readingOf, stepsOf, words, type Bit } from './script.ts';

/** A clause's words as one text: a record as `{Items 868}`. */
const said = (bits: Bit[]) =>
  bits
    .map((b) => (typeof b === 'string' ? b : 'ref' in b ? `{${b.ref.t} ${b.ref.map === undefined ? '' : `${b.ref.map}/`}${b.ref.n}}` : 'ability' in b ? `{ability ${b.ability}}` : coinsOf(b.copper)))
    .join('');
const say = (clause: string, more = true) => said(words(parseClause(clause, more)).bits);

// the tests of silvermere-engine's scripts/mod.rs, and more of the same
test('numbers are read as the game reads them, a missing word as 0', () => {
  assert.equal(leadingInt32('  -42x'), -42);
  assert.equal(leadingInt32('+7'), 7);
  assert.equal(leadingInt32(''), 0);
  assert.equal(leadingInt32('lever'), 0);
  assert.equal(leadingInt32('4294967297'), 1);
  assert.equal(leadingInt('70000'), 4464);
  assert.deepEqual(parseClause('checkitem 123   45').n, { item: 123, msg: 45 });
  assert.deepEqual(parseClause('price').n, { copper: 0, msg: 0 });
});

test('a line is cut at each colon, a last empty piece ending it', () => {
  const cut = (line: string) => clauses(line).map((c) => [c.text, c.more]);
  assert.deepEqual(cut('a:b'), [['a', true], ['b', false]]);
  assert.deepEqual(cut('a::b:'), [['a', true], ['', true], ['b', true]]);
  assert.deepEqual(cut(''), []);
});

test('a keyword is the first in order the clause starts with, without case or trimming', () => {
  assert.equal(keywordOf('nomonsters 5'), 'nomonsters');
  assert.equal(keywordOf('MONSTERS'), 'monsters');
  assert.equal(keywordOf('teleport_slumsewers'), 'teleport');
  assert.equal(keywordOf('check class 3'), null);
  assert.equal(keywordOf(' message 5'), null);
  assert.equal(parseClause('teleport_slumsewers').word, 'teleport_slumsewers');
  assert.equal(parseClause('teleport_slums').word, 'teleport_slums');
  const skill = parseClause('testskill picklocks 10 77');
  assert.deepEqual([skill.word, skill.n], ['picklocks', { modifier: 10, block: 77 }]);
  assert.equal(parseClause('givecoins 5 gold').word, 'gold');
  assert.equal(parseClause('givecoins 5 Runic').word, 'runic');
  assert.equal(parseClause('givecoins 5 x').word, 'copper');
});

test("a dwarven merchant's axe, in words", () => {
  const steps = stepsOf('price 200000 1458:giveitem 868:message 1459');
  assert.deepEqual(
    steps.map((s) => [s.kind, said(s.bits)]),
    [
      ['pay', 'Take 20 platinum, or say {Messages 1458} and stop'],
      ['give', 'Give {Items 868}; one the player cannot carry drops to the floor and ends the line'],
      ['say', 'Say {Messages 1459}'],
    ],
  );
});

test('clauses that do nothing say why', () => {
  assert.equal(say(' message 5'), 'Skipped: “message 5” starts with a space, so the game finds no keyword in it');
  assert.equal(say('open sesame'), 'Skipped: “open sesame” starts with no keyword');
  assert.equal(say(''), 'Nothing: an empty clause');
  assert.equal(say('nomonsters 266', false), 'Nothing: as the last clause of its line, nomonsters never looks');
  assert.equal(say('nomonsters 266'), 'Only with no monsters here, else say {Messages 266} and stop');
  assert.equal(say('teleport 0 1'), 'Nothing: a teleport needs a room and a map');
  assert.equal(say('flag 3 set 9'), 'Nothing: flag 3 never takes effect');
  assert.equal(words(parseClause('flag 99')).kind, 'test');
  assert.equal(say('price 0 5'), 'Nothing: a price of 0 is always paid');
});

test('each keyword in words', () => {
  assert.equal(say('teleport 2126 1'), 'Teleport to {Rooms 1/2126}, or stop when it does not land');
  assert.equal(say('teleport_slums'), 'Teleport to a random room of the slums, or stop when it does not land');
  assert.equal(say('testskill picklocks 10 77'), "Roll 0 to 174 plus 10: above the player's Picklocks, run {Textblocks 77} and stop");
  assert.equal(say('testskill luck -5 7'), "Roll 0 to 99 minus 5: above the player's “luck” (no skill the game knows, so 0), run {Textblocks 7} and stop");
  assert.equal(say('checkspell 12 34'), 'Only while {Spells 12} is on the player, else run {Textblocks 34} and stop');
  assert.equal(say('remoteaction 2051 0 0 8'), 'Work the up exit of {Rooms 2051}');
  assert.equal(say('remoteaction 2051 77 3 1'), 'Say {Messages 77}, then work the south exit of {Rooms 2051}, clearing block 3 if it is a hidden exit');
  assert.equal(say('remoteaction 2051 77 0 11'), 'Nothing: exit 11 is not one of 0 to 10');
  assert.equal(say('givecoins 1 gold'), 'Give 1 gold coin');
  assert.equal(say('addevil -5'), 'Take away 5 evil points');
  assert.equal(say('testability 126 -1'), 'Only with {ability 126} of -1 or less, or none, else stop');
  assert.equal(say('goodaligned -51 801'), 'Only with -51 evil points or fewer, else say {Messages 801} and stop');
  assert.equal(say('clearitem 0'), 'Clear every item from the room');
  assert.equal(say('adddelay 260'), 'Make the player wait 4 ticks more');
});

test('a coin amount in the coins it makes', () => {
  assert.equal(coinsOf(200000), '20 platinum');
  assert.equal(coinsOf(1234), '12 gold, 3 silver, 4 copper');
  assert.equal(coinsOf(3_000_000), '3 runic');
  assert.equal(coinsOf(0), 'no money');
});

test('a block reads as its shape says, else as what names it says', () => {
  assert.equal(readingOf('wares:909\naxes:910\nmail:912\nhammers:914\n'), 'topics');
  assert.equal(readingOf('price 200000 1458:giveitem 868:message 1459'), 'script');
  assert.equal(readingOf('maxlevel 29:cast 818\nmaxlevel 39:testskill wisdom 40 2765\n'), 'script');
  assert.equal(readingOf('go hole:nomonsters 266:message 267:teleport 2126 1\n'), 'commands');
  assert.equal(readingOf('50:giveitem 1593\n100:giveitem 1594\n\n'), 'chances');
  assert.equal(readingOf('The old man looks you over, and with a grin says "Ah"'), 'text');
  assert.equal(readingOf('\x1b[0mNote: run!'), 'text');
  assert.equal(readingOf('\n\n'), 'text');
  assert.equal(readingFor('30 Hello there', ['Monsters Talk Txt']), 'talk');
  assert.equal(readingFor('go hole:message 5', ['Textblocks Link To']), 'commands');
});

test("a block's lines, as the game reads them", () => {
  // a whole block stops reading at a line with no colon in it or after it
  const whole = linesOf('message 5\ntext 6', 'script');
  assert.deepEqual(whole.map((l) => [l.no, l.steps.length, !!l.never]), [[1, 1, false], [2, 0, true]]);
  assert.deepEqual(linesOf('message 5\ntext 6:text 7\n\n', 'script').map((l) => l.steps.length), [1, 2]);
  // a random line takes what the lines before it leave
  assert.deepEqual(linesOf('30:text 1\n20:text 2\n60:text 3\n150:text 4', 'chances').map((l) => [l.pct, !!l.never]), [[30, false], [0, true], [30, false], [40, false]]);
  assert.deepEqual(linesOf('wares:909\naxes:910\n', 'topics').map((l) => [l.pattern, l.block]), [['wares', 909], ['axes', 910]]);
  const cmd = linesOf('go hole|enter hole:nomonsters 266:teleport 2126 1', 'commands');
  assert.deepEqual(cmd.map((l) => [l.pattern, l.steps.map((s) => s.kind)]), [['go hole|enter hole', ['test', 'move']]]);
});
