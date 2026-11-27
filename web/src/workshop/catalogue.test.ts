import { test } from 'node:test';
import assert from 'node:assert/strict';
import { GROUPS, named, search, TOOLS, toolById, toolHref } from './catalogue.ts';

test('every tool has its own id, and every group some tools', () => {
  const ids = TOOLS.map((t) => t.id);
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(new Set(GROUPS.map((g) => g.id)).size, GROUPS.length);
  for (const g of GROUPS) assert.ok(g.tools.length, g.id);
  for (const t of TOOLS) assert.match(t.id, /^[a-z]+$/, `${t.id}: a link's last part`);
});

test("NMR's tool names find the tool that does their work now", () => {
  const first = (q: string) => search(q)[0]?.tools[0]?.id;
  assert.equal(first('Universal Modifier'), 'bulk');
  assert.equal(first('mass room editor'), 'rooms');
  assert.equal(first('multiply all exp'), 'scale');
  assert.equal(first('Combine Exp and Exp Multi Fields'), 'fold');
  assert.equal(first('record number changer'), 'renumber');
  assert.equal(first('Restock All Shops'), 'restock');
  assert.equal(first('export to mmud explorer'), 'mme');
  assert.equal(first('compile update file'), 'update');
  assert.equal(first('database deleter'), 'delete');
});

test('a search keeps the groups in order and leaves out the ones with nothing found', () => {
  assert.equal(search('').length, GROUPS.length);
  const found = search('exp');
  assert.deepEqual(
    found.map((r) => r.group.id),
    GROUPS.map((g) => g.id).filter((id) => found.some((r) => r.group.id === id)),
  );
  assert.ok(found.every((r) => r.tools.length));
  assert.deepEqual(search('nothing like this'), []);
  // a group's own name finds its tools
  assert.ok(search('balance')[0]?.tools.some((t) => t.id === 'scale'));
});

test('Ctrl+K offers tools by name and old name, not the ones still to come', () => {
  assert.deepEqual(named('universal'), [toolById('bulk')!.tool]);
  assert.deepEqual(named(''), []);
  assert.deepEqual(named('attack simulator'), []);
});

test('a tool links to its page, or to the place it lives in', () => {
  assert.equal(toolHref(toolById('bulk')!.tool), '#/workshop/bulk');
  assert.equal(toolHref(toolById('story')!.tool), '#/story');
  assert.equal(toolById('nope'), undefined);
});
