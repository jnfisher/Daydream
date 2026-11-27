import assert from 'node:assert/strict';
import { test } from 'node:test';
import { edgePath, layout, storyOf, type Cell, type Spacing, type StoryEdge, type Values } from './story.ts';

/** A world of records by `Table/key`. */
function world(records: Record<string, Values | null>) {
  return (table: string, key: Cell[]) => records[`${table}/${key.join('/')}`];
}

const merchant = {
  'Monsters/381': { Name: 'dwarven merchant', 'Greet Txt': 907, 'Talk Txt': 0 },
  'Textblocks/907/0': { Data: 'wares:909\naxes:910\n', 'Link To': 908 },
  'Textblocks/908/0': { Data: 'The dwarven merchant looks you over.', 'Link To': 0 },
  'Textblocks/909/0': { Data: 'With a twinkle in his eye...', 'Link To': 0 },
  'Textblocks/910/0': { Data: 'The dwarven merchant holds his hand out.', 'Link To': 911 },
  'Textblocks/911/0': { Data: 'price 200000 1458:giveitem 868:message 1459', 'Link To': 0 },
  'Messages/1458': { 'Line 1': "You can't pay!" },
  'Messages/1459': { 'Line 1': 'He hands over the axe.' },
  'Items/868': { Name: 'dwarven axe' },
};

const mine = {
  'Rooms/6/1664': { Name: 'Copper Mine', 'Command Text': 1061 },
  'Textblocks/1061/0': { Data: 'mine ore:checkitem 936 1608:random 1064\npull lever:remoteaction 1665 0 0 2', 'Link To': 0 },
  'Textblocks/1064/0': { Data: '25:giveitem 943:text 1065\n100:text 1066', 'Link To': 0 },
  'Textblocks/1065/0': { Data: 'You find ore!', 'Link To': 0 },
  'Textblocks/1066/0': { Data: 'Nothing.', 'Link To': 0 },
  'Items/936': { Name: 'pickaxe' },
  'Items/943': { Name: 'copper ore' },
  'Messages/1608': { 'Line 1': 'You have no pickaxe.' },
  'Rooms/6/1665': { Name: 'Mine Shaft' },
};

const edges = (s: ReturnType<typeof storyOf>) => s.edges.map((e) => `${e.from} ${e.kind}@${e.port} ${e.to}`);

test("a monster's story: its greeting, each topic's answer, and the script an answer runs", () => {
  const s = storyOf({ table: 'Monsters', key: [381] }, world(merchant));
  assert.deepEqual(
    s.nodes.map((n) => [n.id, n.depth, n.reading ?? '', n.state]),
    [
      ['Monsters/381', 0, '', 'ok'],
      ['Textblocks/907/0', 1, 'topics', 'ok'],
      ['Textblocks/908/0', 2, 'text', 'ok'],
      ['Textblocks/909/0', 2, 'text', 'ok'],
      ['Textblocks/910/0', 2, 'text', 'ok'],
      ['Textblocks/911/0', 3, 'script', 'ok'],
      ['Messages/1458', 4, '', 'ok'],
      ['Items/868', 4, '', 'ok'],
      ['Messages/1459', 4, '', 'ok'],
    ],
  );
  assert.deepEqual(edges(s), [
    'Monsters/381 talks@talks Textblocks/907/0',
    'Textblocks/907/0 greets@greet Textblocks/908/0',
    'Textblocks/907/0 asks@t0 Textblocks/909/0',
    'Textblocks/907/0 asks@t1 Textblocks/910/0',
    'Textblocks/910/0 then@then Textblocks/911/0',
    'Textblocks/911/0 else@0.0 Messages/1458',
    'Textblocks/911/0 gives@0.1 Items/868',
    'Textblocks/911/0 says@0.2 Messages/1459',
  ]);
  assert.equal(s.more, 0);
});

test('a text block starts a story as the game reads it there: an answer runs its link', () => {
  const s = storyOf({ table: 'Textblocks', key: [910, 0], reading: 'text', answer: true }, world(merchant));
  assert.deepEqual(edges(s), [
    'Textblocks/910/0 then@then Textblocks/911/0',
    'Textblocks/911/0 else@0.0 Messages/1458',
    'Textblocks/911/0 gives@0.1 Items/868',
    'Textblocks/911/0 says@0.2 Messages/1459',
  ]);
  // shown as it is, it runs nothing
  assert.equal(storyOf({ table: 'Textblocks', key: [910, 0], reading: 'text' }, world(merchant)).edges.length, 0);
});

test('a record still being read waits, one the world lacks is marked, and a story has a limit', () => {
  const s = storyOf({ table: 'Monsters', key: [381] }, world({ ...merchant, 'Textblocks/911/0': undefined as unknown as null, 'Items/868': null }));
  assert.equal(s.nodes.find((n) => n.id === 'Textblocks/911/0')?.state, 'pending');
  assert.ok(!s.nodes.some((n) => n.id === 'Items/868'));
  const lacking = storyOf({ table: 'Monsters', key: [381] }, world({ ...merchant, 'Items/868': null }));
  assert.equal(lacking.nodes.find((n) => n.id === 'Items/868')?.state, 'missing');
  const short = storyOf({ table: 'Monsters', key: [381] }, world(merchant), 4);
  assert.deepEqual([short.nodes.length, short.more], [4, 1]);
});

test("a room's commands, a random block's lines by chance, and a remote action on the room's map", () => {
  const s = storyOf({ table: 'Rooms', key: [6, 1664] }, world(mine));
  assert.deepEqual(edges(s), [
    'Rooms/6/1664 types@types Textblocks/1061/0',
    'Textblocks/1061/0 needs@0.0 Items/936',
    'Textblocks/1061/0 else@0.0 Messages/1608',
    'Textblocks/1061/0 runs@0.1 Textblocks/1064/0',
    'Textblocks/1061/0 works@1.0 Rooms/6/1665',
    'Textblocks/1064/0 gives@0.0 Items/943',
    'Textblocks/1064/0 says@0.1 Textblocks/1065/0',
    'Textblocks/1064/0 says@1.0 Textblocks/1066/0',
  ]);
  assert.equal(s.nodes.find((n) => n.id === 'Textblocks/1064/0')?.reading, 'chances');
  // with no room to start from, a remote action's room is on a map nobody knows
  const block = storyOf({ table: 'Textblocks', key: [1061, 0], reading: 'commands' }, world(mine));
  assert.ok(!block.nodes.some((n) => n.table === 'Rooms'));
});

test("a room's exits that answer words come first: where they take players, what they work", () => {
  // the words of a manhole down, a lever for those with a key on another room's door, a
  // button on the room's own hidden way; an ordinary exit and one leading nowhere answer none
  const room = {
    'Rooms/1/9': {
      Name: 'Lever Room',
      'Command Text': 30,
      'Exit 0': 40, 'Type 0': 12, 'Para1 0': 42, 'Para2 0': 11, 'Para3 0': 43, 'Para4 0': 868,
      'Exit 1': 9, 'Type 1': 12, 'Para1 1': 44, 'Para2 1': 10,
      'Exit 2': 8, 'Type 2': 0,
      'Exit 3': 0, 'Type 3': 10, 'Para1 3': 45,
      'Exit 9': 607, 'Type 9': 10, 'Para1 9': 12, 'Para3 9': 8335,
    },
    'Textblocks/30/0': { Data: 'pull lever:message 31', 'Link To': 0 },
  };
  const s = storyOf({ table: 'Rooms', key: [1, 9] }, world(room));
  assert.deepEqual(edges(s), [
    'Rooms/1/9 moves@x9 Rooms/1/607',
    'Rooms/1/9 says@x9 Messages/8335',
    'Rooms/1/9 needs@r0 Items/868',
    'Rooms/1/9 works@r0 Rooms/1/40',
    'Rooms/1/9 says@r0 Messages/43',
    'Rooms/1/9 types@types Textblocks/30/0',
    'Textblocks/30/0 says@0.0 Messages/31',
  ]);
});

const SMALL: Spacing = { w: 200, gapX: 50, gapY: 10, pad: 0, entry: 10 };
const ports = (at: Record<string, number>) => (e: StoryEdge) => at[e.port];

test('nodes sit in a column per step, each level with the row that leads to it', () => {
  const s = storyOf({ table: 'Monsters', key: [381] }, world(merchant));
  const l = layout(s, () => 40, ports({ greet: 15, t0: 25, t1: 35, '0.0': 12, '0.1': 22, '0.2': 32 }), SMALL);
  const box = (id: string) => l.at.get(id)!;
  assert.deepEqual(['Monsters/381', 'Textblocks/907/0', 'Textblocks/911/0', 'Items/868'].map((id) => box(id).x), [0, 250, 750, 1000]);
  // the greeting level with its row, the answers below it as far as they fit
  assert.deepEqual(['908', '909', '910', '911'].map((n) => box(`Textblocks/${n}/0`).y), [5, 55, 105, 105]);
  assert.deepEqual(['Messages/1458', 'Items/868', 'Messages/1459'].map((id) => box(id).y), [107, 157, 207]);
  assert.deepEqual([l.width, l.height], [5 * 200 + 4 * 50, 247]);
});

test('the nodes one row leads to are centred on it together', () => {
  const s = storyOf({ table: 'Rooms', key: [6, 1664] }, world(mine));
  const l = layout(s, () => 40, ports({ '0.0': 50, '0.1': 70, '1.0': 90 }), SMALL);
  const y = (id: string) => l.at.get(id)!.y;
  // the pickaxe and the message for its lack, their arrows' ends either side of the row
  assert.deepEqual([y('Items/936'), y('Messages/1608'), y('Textblocks/1064/0'), y('Rooms/6/1665')], [15, 65, 115, 165]);
  assert.deepEqual([y('Items/943'), y('Textblocks/1065/0'), y('Textblocks/1066/0')], [155, 205, 255]);
});

test('an arrow leaves by its row and comes in at the title, or goes round to a box behind', () => {
  const a = { x: 0, y: 0, w: 200, h: 40 };
  const b = { x: 250, y: 30, w: 200, h: 40 };
  assert.equal(edgePath(a, 25, b, 10), 'M200 25 C228 25, 222 40, 250 40');
  assert.equal(edgePath(b, 10, a, 10), 'M450 40 C490 40, 490 10, 200 10');
});
