import assert from 'node:assert/strict';
import { test } from 'node:test';
import { titleOf } from './ansi.ts';

test('a title is the first line with words, without escapes', () => {
  assert.equal(titleOf('\r\n  \r\n\x1b[1;33mA rat\x1b[0m bites\r\nmore'), 'A rat bites');
  assert.equal(titleOf('␛[32mgreen'), 'green');
  assert.equal(titleOf('x'.repeat(70)), `${'x'.repeat(60)}...`);
});

test('a NUL ends the text: an older value after it is no title', () => {
  assert.equal(titleOf('\0ou flake off pieces of the wall'), '');
  assert.equal(titleOf('Giant rat\0old name'), 'Giant rat');
});
