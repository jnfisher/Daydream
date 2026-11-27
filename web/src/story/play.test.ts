import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fmtMsg, Play, PLAYER, type Pretend, type Shown } from './play.ts';
import { storyOf, type Cell, type Values } from './story.ts';

const GREEN = '\x1b[0;32m';

/** A world of records by `Table/key`. */
function world(records: Record<string, Values>) {
  return async (table: string, key: Cell[]) => records[`${table}/${key.join('/')}`] ?? null;
}
const player = (over: Partial<Pretend> = {}): Pretend => ({ ...structuredClone(PLAYER), ...over });
/** What a turn showed, a note as `(note)` and a failed one as `(!note)`. */
const said = (shown: Shown[]) => shown.map((s) => (s.to === 'note' ? `(${s.bad ? '!' : ''}${s.text})` : `${s.to}: ${s.text}`));

const MERCHANT: Record<string, Values> = {
  'Monsters/381': { Name: 'dwarven merchant', 'Greet Txt': 907, 'Talk Txt': 0 },
  'Monsters/382': { Name: 'mute guard', 'Greet Txt': 0 },
  'Monsters/383': { Name: 'deaf hermit', 'Greet Txt': 920 },
  'Textblocks/907/0': { Data: 'wares:909\naxes:910\n', 'Link To': 908 },
  'Textblocks/908/0': { Data: '%s looks you over.', 'Link To': 0 },
  'Textblocks/909/0': { Data: 'With a twinkle in his eye...', 'Link To': 0 },
  'Textblocks/909/1': { Data: ' and more.', 'Link To': 0 },
  'Textblocks/910/0': { Data: 'The %s holds his hand out.', 'Link To': 911 },
  'Textblocks/911/0': { Data: 'price 200000 1458:giveitem 868:message 1459', 'Link To': 0 },
  'Textblocks/920/0': { Data: 'nothing:921', 'Link To': 0 },
  'Messages/1458': { 'Line 1': "You can't pay!", 'Line 2': '%s scrounges for cash.' },
  'Messages/1459': { 'Line 1': 'He hands over the axe.', 'Line 2': '' },
  'Items/868': { Name: 'dwarven axe' },
};
const merchant = world(MERCHANT);

test('a greeting shows the text the talk block links to, the monster named in it', async () => {
  const p = new Play(merchant, player());
  await p.ask(381, null);
  assert.deepEqual(said(p.shown), [`you: ${GREEN}dwarven merchant looks you over.`]);
  assert.deepEqual([...p.touched], ['Monsters/381', 'Textblocks/907/0', 'Textblocks/908/0']);
  assert.deepEqual([...p.went], ['Monsters/381>Textblocks/907/0@talks', 'Textblocks/907/0>Textblocks/908/0@greet']);
  // a talk block that links nowhere, and a monster with none
  const deaf = new Play(merchant, player());
  await deaf.ask(383, null);
  await deaf.ask(382, 'hello');
  assert.deepEqual(said(deaf.shown), ["you: deaf hermit doesn't understand you.", 'you: mute guard has nothing to tell you!']);
});

test('a question takes the first topic in it, shows its text and runs the script it links to', async () => {
  const p = new Play(merchant, player());
  await p.ask(381, 'got any AXES for sale?');
  assert.deepEqual(said(p.shown), [`you: ${GREEN}The dwarven merchant holds his hand out.`, '(You pay 20 platinum.)', '(You get dwarven axe.)', 'you: He hands over the axe.']);
  assert.deepEqual([p.me.copper, p.me.items], [800_000, [868]]);
  // each way by the row it leaves from, as the story's edges do (story.test.ts)
  assert.deepEqual([...p.went], [
    'Monsters/381>Textblocks/907/0@talks',
    'Textblocks/907/0>Textblocks/910/0@t1',
    'Textblocks/910/0>Textblocks/911/0@then',
    'Textblocks/911/0>Items/868@0.1',
    'Textblocks/911/0>Messages/1459@0.2',
  ]);
  // every part of a text is shown, in order
  const wares = new Play(merchant, player());
  await wares.ask(381, 'wares');
  assert.deepEqual(said(wares.shown), [`you: ${GREEN}With a twinkle in his eye... and more.`]);
  // the answer alone, as a text block's story starts: the monster's name left for the game
  const alone = new Play(merchant, player());
  await alone.show(910, true);
  assert.deepEqual(said(alone.shown).slice(0, 2), [`you: ${GREEN}The %s holds his hand out.`, '(You pay 20 platinum.)']);
  // no topic in it
  const none = new Play(merchant, player());
  await none.ask(381, 'the weather');
  assert.deepEqual(said(none.shown), ['you: dwarven merchant has nothing to tell you!']);
});

test('a test that fails says its message, says why, and what would pass it', async () => {
  const p = new Play(merchant, player({ copper: 150_000 }));
  await p.ask(381, 'axes');
  assert.deepEqual(said(p.shown).slice(1), [
    "you: You can't pay!",
    'room: Ana scrounges for cash.',
    '(!You have 15 platinum, 5 platinum short of the price of 20 platinum, so the line stops.)',
  ]);
  assert.deepEqual([p.me.copper, p.me.items], [150_000, []]);
  assert.ok(p.touched.has('Messages/1458') && !p.touched.has('Items/868'));
  // the fix, made to the player as the turn found them, lets it go on
  const fix = p.shown.at(-1)!;
  assert.ok(fix.to === 'note' && fix.fix);
  assert.equal(fix.fix.label, 'Add 5 platinum');
  const before = player({ copper: 150_000 });
  fix.fix.apply(before);
  const again = new Play(merchant, before);
  await again.ask(381, 'axes');
  assert.deepEqual([again.me.copper, again.me.items], [0, [868]]);
});

const MINE: Record<string, Values> = {
  'Rooms/6/1664': { Name: 'Copper Mine', 'Command Text': 1061 },
  'Textblocks/1061/0': { Data: 'mine ore|dig*ging:checkitem 936 1608:random 1064\npull lever:remoteaction 1665 0 0 2', 'Link To': 0 },
  'Textblocks/1064/0': { Data: '25:giveitem 943:text 1065\n100:text 1066', 'Link To': 0 },
  'Textblocks/1065/0': { Data: 'You find ore!', 'Link To': 0 },
  'Textblocks/1066/0': { Data: 'Nothing.', 'Link To': 0 },
  'Items/936': { Name: 'pickaxe' },
  'Items/943': { Name: 'copper ore' },
  'Messages/1608': { 'Line 1': 'You have no pickaxe.' },
  'Rooms/6/1665': { Name: 'Mine Shaft' },
};
const mine = world(MINE);

test("typed words run a room's command; a roll picks a line as the game rolls it", async () => {
  const without = new Play(mine, player());
  assert.equal(await without.type(6, 1664, 'MINE ORE'), 'stop');
  assert.deepEqual(said(without.shown), ['you: You have no pickaxe.', '(!You are not carrying pickaxe, so the line stops.)']);

  const lucky = new Play(mine, player({ items: [936] }), { dice: () => 10 });
  assert.equal(await lucky.type(6, 1664, 'dig'), 'ok');
  assert.deepEqual(said(lucky.shown), ['(The roll is 10 (0 to 99): the first line numbered above it, 25, runs.)', '(You get copper ore.)', `you: ${GREEN}You find ore!`]);
  assert.deepEqual(lucky.me.items, [936, 943]);
  assert.deepEqual([...lucky.went], [
    'Rooms/6/1664>Textblocks/1061/0@types',
    'Textblocks/1061/0>Items/936@0.0',
    'Textblocks/1061/0>Textblocks/1064/0@0.1',
    'Textblocks/1064/0>Items/943@0.0',
    'Textblocks/1064/0>Textblocks/1065/0@0.1',
  ]);

  const not = new Play(mine, player({ items: [936] }), { dice: () => 25 });
  await not.type(6, 1664, 'mine ore');
  assert.deepEqual(said(not.shown).slice(1), [`you: ${GREEN}Nothing.`]);

  // an exit worked in a room on the player's map, and words no command takes
  const lever = new Play(mine, player());
  await lever.type(6, 1664, 'pull lever');
  await lever.type(6, 1664, 'dance');
  assert.deepEqual(said(lever.shown), [
    '(The east exit of Mine Shaft is worked (a door, a hidden way or a trap).)',
    '(None of the room’s commands takes it, so the game tries it as a spell or an action next.)',
  ]);
  assert.ok(lever.touched.has('Rooms/6/1665'));
});

test("the player's level, class, alignment and stats, and what is in the room", async () => {
  const get = world({
    'Textblocks/1/0': { Data: 'minlevel 20 5:class 3:goodaligned -10 6:nomonsters 7:message 8', 'Link To': 0 },
    'Textblocks/2/0': { Data: 'testskill strength 10 3', 'Link To': 0 },
    'Textblocks/3/0': { Data: 'text 4', 'Link To': 0 },
    'Textblocks/4/0': { Data: 'You strain.', 'Link To': 0 },
    'Textblocks/5/0': { Data: 'nomonsters 9', 'Link To': 0 },
    'Messages/5': { 'Line 1': 'Too young.' },
    'Messages/6': { 'Line 1': 'Too wicked.' },
    'Messages/7': { 'Line 1': 'Not with company.' },
    'Messages/8': { 'Line 1': 'Welcome.' },
    'Classes/3': { Name: 'Priest' },
  });
  const run = async (me: Pretend, n = 1, dice = 0) => {
    const p = new Play(get, me, { dice: () => dice });
    await p.runBlock(n);
    return said(p.shown);
  };
  assert.deepEqual(await run(player()), ['you: Too young.', '(!You are level 10, under 20, so the line stops.)']);
  assert.deepEqual(await run(player({ level: 20 })), ['(!You are not a Priest, so the line stops.)']);
  assert.deepEqual(await run(player({ level: 20, cls: 3, evil: 5 })), ['you: Too wicked.', '(!You have 5 evil points, over -10, so the line stops.)']);
  assert.deepEqual(await run(player({ level: 20, cls: 3, evil: -10, monsters: [381] })), ['you: Not with company.', '(!A monster is here, so the line stops.)']);
  assert.deepEqual(await run(player({ level: 20, cls: 3, evil: -10 })), ['you: Welcome.']);
  // a nomonsters clause that ends its line is never tested
  assert.deepEqual(await run(player({ monsters: [381] }), 5), []);
  // a roll of 0 to 149 for strength, 10 added: above the player's 50, the block runs and the line stops
  const [strained, passed] = [await run(player(), 2, 41), await run(player(), 2, 40)];
  assert.deepEqual(strained, ['(!The roll is 51 against your Strength of 50: above it, so block 3 runs and the line stops.)', `you: ${GREEN}You strain.`]);
  assert.deepEqual(passed, ['(The roll is 50 against your Strength of 50: not above it, so the line goes on.)']);
});

test('a spell is learnt as the game teaches it: the class, its magic, the level, the alignment', async () => {
  const get = world({
    'Textblocks/1/0': { Data: 'learnspell 10:message 2', 'Link To': 0 },
    'Messages/2': { 'Line 1': 'Done.' },
    'Spells/10': { Name: 'holy light', Level: 5, 'Magery A': 2, 'Magery B': 1, 'Ability 0': 112 },
    'Classes/1': { Name: 'Warrior', 'Magic Type': 0, 'Magic LVL': 0 },
    'Classes/3': { Name: 'Priest', 'Magic Type': 2, 'Magic LVL': 3 },
  });
  const learn = async (me: Pretend) => {
    const p = new Play(get, me);
    await p.runBlock(1);
    return [said(p.shown), p.me.book] as const;
  };
  assert.deepEqual((await learn(player()))[0], ["you: You don't know what to do with this!", '(!You have no class the game knows, so you cannot learn it, and the line stops.)']);
  assert.deepEqual((await learn(player({ cls: 1 })))[0][1], '(!Warrior casts another kind of magic, so you cannot learn it, and the line stops.)');
  assert.deepEqual((await learn(player({ cls: 3, level: 4 })))[0], [
    "you: This spell is too powerful for you - You don't know what to do with this!",
    "(!You are level 4, under the spell's 5, so you cannot learn it, and the line stops.)",
  ]);
  // a spell not for the neutral is not for the good either; the evil may not learn it
  assert.deepEqual((await learn(player({ cls: 3, evil: -60 })))[0][1], '(!The spell is not for good players, so you cannot learn it, and the line stops.)');
  assert.deepEqual(await learn(player({ cls: 3 })), [['you: You learn the spell holy light.', 'you: Done.'], [10]]);
  // known already: the line goes on, nothing said
  assert.deepEqual(await learn(player({ cls: 3, book: [10] })), [['you: Done.'], [10]]);
});

test('a block that runs itself is cut off as deep as the game lets it go', async () => {
  const get = world({
    'Textblocks/1/0': { Data: 'message 2:random 3', 'Link To': 0 },
    'Textblocks/3/0': { Data: '100:message 2:random 3', 'Link To': 0 },
    'Messages/2': { 'Line 1': 'Again.' },
  });
  const p = new Play(get, player(), { dice: () => 0 });
  await p.runBlock(1);
  const again = p.shown.filter((s) => s.to === 'you').length;
  assert.equal(again, 32);
  assert.ok(said(p.shown).includes('(!Text block 3 is run too deep inside others, so the game cuts it off here.)'));
});

test('a message is filled in as printf fills it', () => {
  assert.equal(fmtMsg('%s waves at you.', ['Ana']), 'Ana waves at you.');
  assert.equal(fmtMsg('100%% sure, %s and %s', ['Ana']), '100% sure, Ana and ');
  assert.equal(fmtMsg('%ld coins, %hs', ['5', 'b']), '5 coins, b');
  assert.equal(fmtMsg('odd %x and %', ['Ana']), 'odd x and %');
});

test("every way a turn goes is one of the story's edges, by the row it leaves from", async () => {
  const look = (records: Record<string, Values>) => (table: string, key: Cell[]) => records[`${table}/${key.join('/')}`] ?? null;
  const edges = (s: ReturnType<typeof storyOf>) => new Set(s.edges.map((e) => `${e.from}>${e.to}@${e.port}`));
  const talk = edges(storyOf({ table: 'Monsters', key: [381] }, look(MERCHANT)));
  for (const q of [null, 'wares', 'axes']) {
    const p = new Play(merchant, player());
    await p.ask(381, q);
    for (const w of p.went) assert.ok(talk.has(w), `${w} is no edge of the story`);
  }
  const room = edges(storyOf({ table: 'Rooms', key: [6, 1664] }, look(MINE)));
  for (const [typed, roll] of [['mine ore', 10], ['digging', 60], ['pull lever', 0]] as const) {
    const p = new Play(mine, player({ items: [936] }), { dice: () => roll });
    await p.type(6, 1664, typed);
    assert.ok(p.went.size);
    for (const w of p.went) assert.ok(room.has(w), `${w} is no edge of the story`);
  }
});
