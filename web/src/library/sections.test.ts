import assert from 'node:assert/strict';
import { test } from 'node:test';
import { columnsOf, complete, freeSlot, fromLayout, type Section, slotsInUse, standing, standingWords } from './sections.ts';

const columns = ['Number', 'Name', 'Min Hit', 'Max Hit', 'Cost', 'Cost Type', 'Ability 0', 'Ability Value 0', 'Ability 1', 'Ability Value 1', 'Weight'];
const abilities = {
  columns: [{ template: 'Ability #' }, { template: 'Ability Value #' }],
  from: 0,
  to: 1,
};

test('a card shows every column: what its sections leave out is under More fields', () => {
  const sections: Section[] = [
    { id: 'fight', title: 'In a fight', fields: [{ col: 'Min Hit' }, { col: 'Max Hit' }, { col: 'Cost', with: 'Cost Type' }], when: (get) => get('Name') === 'axe' },
    { id: 'abilities', title: 'Abilities', grid: abilities },
  ];
  const axe = complete(sections, columns, ['Number'], (c) => (c === 'Name' ? 'axe' : 0));
  assert.deepEqual(
    axe.map((s) => s.id),
    ['fight', 'abilities', 'more'],
  );
  assert.deepEqual(axe.at(-1)!.fields!.map((f) => f.col), ['Name', 'Weight']);
  // a section the record is not for leaves its columns to More fields
  const robe = complete(sections, columns, ['Number'], (c) => (c === 'Name' ? 'robe' : 0));
  assert.deepEqual(robe.at(-1)!.fields!.map((f) => f.col), ['Name', 'Min Hit', 'Max Hit', 'Cost', 'Cost Type', 'Weight']);
  assert.deepEqual(columnsOf(sections[0]!), ['Min Hit', 'Max Hit', 'Cost', 'Cost Type']);
});

test("All fields is NMR's tabs, the game text as text, and the rest after", () => {
  const out = fromLayout(
    [
      { title: 'General', fields: [{ col: 'Number' }, { col: 'Name' }, { col: 'Min Hit' }] },
      { title: 'Abilities', grid: abilities },
      { title: 'Text', fields: [{ col: 'Max Hit' }], preview: ['Max Hit'] },
    ],
    columns,
    ['Number'],
  );
  assert.deepEqual(
    out.map((s) => s.title),
    ['General', 'Abilities', 'Text', 'More'],
  );
  assert.deepEqual(out[0]!.fields!.map((f) => f.col), ['Name', 'Min Hit']);
  assert.deepEqual(out[2]!.fields, []);
  assert.deepEqual(out[2]!.text, { cols: ['Max Hit'] });
  assert.deepEqual(out[3]!.fields!.map((f) => f.col), ['Cost', 'Cost Type', 'Weight']);
});

test('the slots of a grid in use, and the next free one', () => {
  const values: Record<string, number> = { 'Ability 1': 43, 'Ability Value 1': 5, 'Ability Value 0': 9 };
  const get = (c: string) => values[c] ?? 0;
  // a value with no ability is an empty slot
  assert.deepEqual(slotsInUse(abilities, get), [1]);
  assert.equal(freeSlot(abilities, get), 0);
  values['Ability 0'] = 2;
  assert.equal(freeSlot(abilities, get), null);
  // separate lists: a slot is in use when any of them is set
  const lists = { columns: [{ template: 'Class #' }, { template: 'Race #' }], from: 0, to: 2, separate: true };
  assert.deepEqual(slotsInUse(lists, (c) => (c === 'Race 1' ? 4 : 0)), [1]);
});

test('a value among its peers', () => {
  const s = standing([5, 9, 12, 12, 20], 12);
  assert.deepEqual(s, { min: 5, max: 20, below: 2, same: 2, above: 1 });
  assert.equal(standingWords(s), 'higher than 2 of the other 5');
  assert.equal(standingWords(standing([1, 2], 3)), 'the highest of 3');
  assert.equal(standingWords(standing([4, 5], 3)), 'the lowest of 3');
  assert.equal(standingWords(standing([], 3)), 'the only one');
  assert.equal(standingWords(standing([3, 3], 3)), 'the same as all 2 others');
  assert.deepEqual(standing([], 7), { min: 7, max: 7, below: 0, same: 0, above: 0 });
});
