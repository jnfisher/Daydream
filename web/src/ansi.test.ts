import assert from 'node:assert/strict';
import { test } from 'node:test';
import { PEN, plainOf, runsOf, titleOf } from './ansi.ts';

test('a title is the first line with words, without escapes', () => {
  assert.equal(titleOf('\r\n  \r\n\x1b[1;33mA rat\x1b[0m bites\r\nmore'), 'A rat bites');
  assert.equal(titleOf('␛[32mgreen'), 'green');
  assert.equal(titleOf('x'.repeat(70)), `${'x'.repeat(60)}...`);
});

test('a NUL ends the text: an older value after it is no title', () => {
  assert.equal(titleOf('\0ou flake off pieces of the wall'), '');
  assert.equal(titleOf('Giant rat\0old name'), 'Giant rat');
});

test('a step of the cursor forward is the spaces it leaves', () => {
  assert.equal(plainOf('\x1b[0mYou\x1b[Cheft\x1b[3Cyour\x1b[s pickaxe'), 'You heft   your pickaxe');
  assert.equal(titleOf('\x1b[2J\x1b[255D\x1b[0mYou\x1b[Cheft'), 'You heft');
});

test("a terminal's colours carry on from one text to the next", () => {
  const green = runsOf('\x1b[0;32mThe merchant nods.\r\n');
  assert.deepEqual(green.runs, [['The merchant nods.\n', { fg: 2, bg: 0, bold: false }]]);
  const next = runsOf('He hands \x1b[1mit\x1b[22m over.', green.pen);
  assert.deepEqual(
    next.runs.map(([t, p]) => [t, p.fg, p.bold]),
    [['He hands ', 2, false], ['it', 2, true], [' over.', 2, false]],
  );
  assert.deepEqual(runsOf('\x1b[0mplain', next.pen).pen, PEN);
});
