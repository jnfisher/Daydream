import assert from 'node:assert/strict';
import { test } from 'node:test';
import { niceAxis, spots } from './scatter.ts';

test('an axis widens to round numbers', () => {
  assert.deepEqual(niceAxis(1450, 2350), { lo: 1400, hi: 2400, ticks: [1400, 1600, 1800, 2000, 2200, 2400] });
  assert.deepEqual(niceAxis(0, 16), { lo: 0, hi: 20, ticks: [0, 5, 10, 15, 20] });
  assert.deepEqual(niceAxis(7, 7), { lo: 6, hi: 8, ticks: [6, 6.5, 7, 7.5, 8] });
  assert.deepEqual(niceAxis(0.1, 0.9).ticks, [0, 0.2, 0.4, 0.6, 0.8, 1]);
});

test('records at the same spot share a dot', () => {
  const s = spots([
    { x: 1, y: 2, n: 'a' },
    { x: 3, y: 4, n: 'b' },
    { x: 1, y: 2, n: 'c' },
  ]);
  assert.equal(s.length, 2);
  assert.deepEqual(
    s[0]!.all.map((p) => p.n),
    ['a', 'c'],
  );
});
