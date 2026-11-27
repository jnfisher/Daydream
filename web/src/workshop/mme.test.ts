import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { MmeOptions } from '../api.ts';
import { fileName, fromIni, roomsOf, toIni } from './mme.ts';

const base: MmeOptions = {
  legit: true,
  disable_kai: false,
  exclude_default: true,
  exclude_rooms: false,
  excluded: [],
  hide_excluded: false,
  no_rooms: false,
  custom: '',
  update_url: '',
  dat_version: 'v1.11p',
};

test("the settings go into NMR's config file and come back as they were", () => {
  const o: MmeOptions = { ...base, legit: false, custom: 'Dark Realm', exclude_rooms: true, excluded: [{ map: 1, from: 5, to: 9 }, { map: 2, from: 7, to: 7 }], hide_excluded: true };
  const ini = toIni(o);
  assert.match(ini, /^\[Settings\]\r\n/);
  assert.match(ini, /\r\nExcludedRooms=1\/5\/9,2\/7\/7,\r\n$/);
  assert.match(ini, /\r\nGmud=0\r\n/);
  assert.deepEqual(fromIni(ini, base), o);
  // no rooms is NMR's 0
  assert.match(toIni(base), /ExcludedRooms=0\r\n/);
});

test("NMR's config file is read over what is set, its empty names left alone", () => {
  const ini = '[Other]\nLegit=0\n[Settings]\nLegit=0\nCustomName=0\nDB_Ver=v2\nExcludedRooms=3/1/4,9/9\n';
  const o = fromIni(ini, { ...base, custom: 'Kept' });
  assert.equal(o.legit, false);
  assert.equal(o.custom, 'Kept');
  assert.equal(o.dat_version, 'v2');
  // only the entries that end in a comma count, as in NMR
  assert.deepEqual(o.excluded, [{ map: 3, from: 1, to: 4 }]);
  assert.equal(o.disable_kai, false);
});

test("the export is named as NMR names it", () => {
  assert.equal(fileName({ ...base, custom: 'Dark Realm' }, 'mdb'), 'data-DarkRealm.mdb');
  assert.equal(fileName(base, 'plan'), 'data-v1.11p.plan');
  assert.equal(fileName({ ...base, dat_version: '' }, 'mdb'), 'data-mme.mdb');
});

test('a run of rooms to leave out is read as typed', () => {
  assert.deepEqual(roomsOf('1', '5', ''), { map: 1, from: 5, to: 5 });
  assert.deepEqual(roomsOf('1', '5', '9'), { map: 1, from: 5, to: 9 });
  assert.match((roomsOf('', '5', '') as { error: string }).error, /map/);
  assert.match((roomsOf('1', 'x', '') as { error: string }).error, /first room/);
  assert.match((roomsOf('1', '9', '5') as { error: string }).error, /5 comes before 9/);
});
