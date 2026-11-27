import assert from 'node:assert/strict';
import { test } from 'node:test';
import { endOf, indexChangesOf, lineText, movesOf, runsOf, textChange, type IndexLine, type Line } from './moves.ts';

const line = (from: string, to: string, start: string): Line => ({ from, to, start });

test('a change list makes a move for each number of each line, blank lines skipped', () => {
  assert.deepEqual(movesOf([line('868', '', '3000')]), { moves: [{ from: 868, to: 3000 }] });
  assert.deepEqual(movesOf([line('12', '14', '40'), line('', '', ''), line('2', '', '7')]), {
    moves: [
      { from: 12, to: 40 },
      { from: 13, to: 41 },
      { from: 14, to: 42 },
      { from: 2, to: 7 },
    ],
  });
});

test('a change list says why it cannot run yet', () => {
  const why = (lines: Line[]) => {
    const r = movesOf(lines);
    return 'error' in r ? r.error : '';
  };
  assert.match(why([line('', '', '')]), /Give the number to move/);
  assert.match(why([line('12', '', '')]), /new number for 12/);
  assert.match(why([line('20', '12', '40')]), /12 comes before 20/);
  assert.match(why([line('x', '', '4')]), /whole numbers/);
  assert.match(why([line('0', '', '4')]), /start at 1/);
  assert.match(why([line('1', '20000', '30000')]), /at most 10,000/);
});

test('a group and index change list moves each index of a run, and refuses one twice', () => {
  const l = (group: number, from: string, to: string, toGroup: number, start: string): IndexLine => ({ group, from, to, toGroup, start });
  assert.deepEqual(indexChangesOf([l(2, '0', '1', 3, '10')]), {
    changes: [
      { from_group: 2, from_index: 0, to_group: 3, to_index: 10 },
      { from_group: 2, from_index: 1, to_group: 3, to_index: 11 },
    ],
  });
  const twice = indexChangesOf([l(2, '5', '6', 2, '9'), l(2, '6', '', 4, '1')]);
  assert.ok('error' in twice && /Index 6 of that group is in two lines/.test(twice.error));
  // the same index of another group is another change
  assert.ok('changes' in indexChangesOf([l(2, '5', '', 2, '9'), l(3, '5', '', 3, '9')]));
});

test('a line is said as its runs, and its new end follows its start', () => {
  assert.equal(lineText(line('12', '20', '40')), '12–20 → 40–48');
  assert.equal(lineText(line('868', '', '3000')), '868 → 3000');
  assert.equal(lineText(line('868', '', '')), '');
  assert.equal(endOf(line('12', '20', '40')), '48');
  assert.equal(endOf(line('12', '20', '')), '');
});

test('numbers fall into runs of consecutive ones', () => {
  assert.deepEqual(runsOf([5, 1, 2, 3, 9, 3, 10]), [
    [1, 3],
    [5, 5],
    [9, 10],
  ]);
  assert.deepEqual(runsOf([]), []);
});

test('a long text change is cut to the lines that change, or the part that differs', () => {
  assert.deepEqual(textChange('short', 'shorter'), ['short', 'shorter']);
  const lines = (n: number) => Array.from({ length: 12 }, (_, i) => `${i}:message 1434:giveitem ${i === 7 ? n : i}`).join('\n');
  assert.deepEqual(textChange(lines(868), lines(3000)), ['7:message 1434:giveitem 868', '7:message 1434:giveitem 3000']);
  const pad = 'x'.repeat(60);
  const [a, b] = textChange(`${pad}giveitem 868${pad}`, `${pad}giveitem 3000 and more\n${pad}`);
  assert.ok(a.startsWith('…') && a.includes('giveitem 868'), a);
  assert.ok(b.includes('giveitem 3000 and more'), b);
});
