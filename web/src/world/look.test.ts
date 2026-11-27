import { test } from 'node:test';
import assert from 'node:assert/strict';
import { contrast, exitLook, luminance, MAP_STYLES, mix, palette, styleFrom, swatch, tintOf } from './look.ts';
import type { MapExit, MapRoom } from './model.ts';

/** Every map style but the theme's, which reads the page's colours. */
const STYLES = MAP_STYLES.filter((m) => m.id !== 'theme').map((m) => [m.id, palette(m.id)] as const);

const exit = (dir: number, kind = 0): MapExit => ({ dir, to: 2, kind, para1: 0, para2: 0, para3: 0, para4: 0 });
const room = (exits: MapExit[], more: Partial<MapRoom> = {}): MapRoom => ({
  room: 1, name: 'Town Square', exits, shop: 0, spell: 0, command: 0, npc: 0, control: 0, death: 0, exit_room: 0,
  room_type: 0, mon_type: 0, min_index: 0, max_index: 0, max_regen: 0, light: 0, ...more,
});

test('every map style can be read: names, chips and the selected room name stand out from what they are on', () => {
  assert.equal(STYLES.length, 5);
  for (const [id, p] of STYLES) {
    assert.ok(contrast(p.ink, p.bg) >= 4.5, `${id}: names on the map`);
    assert.ok(contrast(p.ink, p.halo) >= 4.5, `${id}: names on their halo`);
    assert.ok(contrast(p.ink, p.badge) >= 4.5, `${id}: words on chips`);
    assert.ok(contrast(p.selInk, p.sel) >= 3.5, `${id}: the selected room's name`);
    assert.ok(contrast(p.sel, p.bg) >= 3, `${id}: the selection ring`);
    // a room shows by its outline or by its colour (a big square: Starlight's plain rooms are 2.15)
    assert.ok(Math.max(contrast(p.line, p.bg), contrast(p.plain, p.bg)) >= 2, `${id}: rooms`);
  }
});

test("every map style's exit colours show on its ground and on its chips", () => {
  for (const [id, p] of STYLES) {
    for (const [sort, c] of Object.entries(p.exit)) {
      assert.ok(contrast(c, p.bg) >= 3, `${id}: ${sort} on the map`);
      assert.ok(contrast(c, p.badge) >= 3, `${id}: ${sort} on a chip`);
    }
    for (const [what, c] of [['one-way', p.oneway], ['portal', p.portal], ['missing', p.missing]] as const) assert.ok(contrast(c, p.bg) >= 3, `${id}: ${what}`);
  }
});

test('a map style is read back by its name, and anything else is the theme', () => {
  assert.equal(styleFrom('blueprint'), 'blueprint');
  assert.equal(styleFrom('classic'), 'classic');
  assert.equal(styleFrom('parchment'), 'theme');
  assert.equal(styleFrom(''), 'theme');
});

test("Classic NMR colours rooms by place as NMR's map does: by their ways up and down", () => {
  const p = palette('classic');
  const of = (exits: MapExit[]) => tintOf(room(exits), 'place', p, 'street', false);
  assert.equal(of([exit(0), exit(8)]).key, 'up');
  assert.equal(of([exit(9)]).key, 'down');
  assert.equal(of([exit(8), exit(9)]).key, 'updown');
  const none = of([exit(0), exit(1)]);
  assert.equal(none.key, 'none');
  assert.equal(none.colour, p.plain);
  // the other styles keep the room's place
  assert.equal(tintOf(room([exit(8)]), 'place', palette('terminal'), 'forest', false).colour, palette('terminal').biome.forest);
});

test("areas, light and edits take each style's own colours", () => {
  for (const [, p] of STYLES) {
    assert.ok(p.swatches.includes(swatch(17, p)));
    assert.equal(swatch(17, p), swatch(17, p));
    assert.equal(tintOf(room([]), 'edits', p, 'other', true).colour, p.warm);
    assert.equal(tintOf(room([], { light: -300 }), 'light', p, 'other', false).colour, mix(p.plain, p.deep, 0.85));
  }
});

test('actions and remote actions have a colour of their own, apart from text', () => {
  assert.equal(exitLook({ kind: 5, para4: 0 }).sort, 'action');
  assert.equal(exitLook({ kind: 12, para4: 0 }).sort, 'action');
  assert.equal(exitLook({ kind: 10, para4: 0 }).sort, 'words');
  assert.notEqual(palette('classic').exit.action, palette('classic').exit.words);
});

test('lightness reads #rrggbb and rgb() alike', () => {
  assert.equal(luminance('#000000'), 0);
  assert.equal(luminance('#ffffff'), 1);
  assert.equal(luminance('rgb(255,0,0)'), luminance('#ff0000'));
  assert.equal(contrast('#000000', '#ffffff'), 21);
});
