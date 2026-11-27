<!--
  The Workshop's one-step fixes, NMR's from its Items and Monsters menus: each one's few
  settings, then its preview (Run).
-->
<svelte:options css="injected" />

<script lang="ts">
  import { toWire } from '../ansi';
  import { api, type RunBody } from '../api';
  import Run from './Run.svelte';
  import Seg from './Seg.svelte';

  let { id }: { id: string } = $props();

  const plural = (n: number, one: string, many = `${one}s`) => `${n.toLocaleString()} ${n === 1 ? one : many}`;
  const pad = (n: number) => String(n).padStart(2, '0');
  const now = new Date();
  const today = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;

  // the settings, each tool's own
  let limits = $state<'game' | 'level'>('game');
  let kills = $state<'day' | 'clear'>('day');
  let day = $state(today);
  let buffer = $state<'add' | 'remove'>('add');
  let map = $state('');
  let strip = $state('');
  let dropMode = $state<'set' | 'add'>('set');
  let dropValue = $state('20');
  let dropCmp = $state<'any' | 'le' | 'ge' | 'eq' | 'ne'>('le');
  let dropAt = $state('20');
  let kinds = $state({ weapons: true, armour: true, keys: true, containers: true, scrolls: true });
  let limited = $state<'include' | 'exclude' | 'only'>('include');
  let maint = $state(true);
  let regenOn = $state(true);
  let regen = $state('1');

  const whole = (s: string) => (/^\s*-?\d+\s*$/.test(s) ? Number(s) : NaN);

  interface Fix {
    body: RunBody;
    blocked?: string;
    done: (n: number) => string;
    verb?: string;
  }

  const fix: Fix = $derived.by((): Fix => {
    switch (id) {
      case 'uses':
        return { body: { preset: 'fix_item_uses' }, done: (n) => `Fixed the uses on ${plural(n, 'item')}.` };
      case 'dropuses':
        return { body: { preset: 'fix_drop_uses' }, done: (n) => `Fixed the uses on the drops of ${plural(n, 'monster')}.` };
      case 'fold':
        return { body: { preset: 'combine_exp' }, done: (n) => `Folded the multiplier into the experience of ${plural(n, 'monster')}.` };
      case 'limits':
        return limits === 'game'
          ? { body: { preset: 'no_limited_items' }, done: (n) => `Lifted the game limits on ${plural(n, 'item')}.` }
          : { body: { preset: 'no_level_restrictions' }, done: (n) => `Lifted the level restrictions on ${plural(n, 'item')}.` };
      case 'kills': {
        if (kills === 'clear') return { body: { preset: 'reset_kill_times' }, done: (n) => `Cleared the kill timers of ${plural(n, 'monster')}.` };
        const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
        return {
          body: m ? { preset: 'set_kill_times', year: Number(m[1]), month: Number(m[2]), day: Number(m[3]) } : {},
          blocked: m ? undefined : 'Pick the day they were killed.',
          done: (n) => `Set the kill times of ${plural(n, 'monster')} to ${day}.`,
        };
      }
      case 'restock':
        return { body: { preset: 'restock_shops' }, done: (n) => `Restocked ${plural(n, 'shop')}.`, verb: 'Restock' };
      case 'floors':
        return { body: { preset: 'combine_room_items' }, done: (n) => `Tidied the floors of ${plural(n, 'room')}.`, verb: 'Tidy' };
      case 'buffer': {
        const m = map.trim() === '' ? null : whole(map);
        const bad = m !== null && !(m > 0) ? 'A map is a number from 1.' : undefined;
        return buffer === 'add'
          ? { body: { preset: 'insert_buffer_rooms', map: m }, blocked: bad, done: (n) => `Added ${plural(n, 'buffer room')}.`, verb: 'Add' }
          : { body: { preset: 'delete_buffer_rooms', map: m }, blocked: bad, done: (n) => `Deleted ${plural(n, 'buffer room')}.`, verb: 'Delete' };
      }
      case 'strip':
        return {
          body: { preset: 'strip_textblocks', text: toWire(strip) },
          blocked: strip ? undefined : 'Type the characters to take off the end, such as a space or ␛.',
          done: (n) => `Stripped the endings of ${plural(n, 'text block part')}.`,
        };
      case 'drops': {
        const v = whole(dropValue);
        const at = dropCmp === 'any' ? 0 : whole(dropAt);
        const none = !Object.values(kinds).some(Boolean);
        const r = whole(regen);
        const blocked = Number.isNaN(v)
          ? 'Give the drop % as a whole number.'
          : Number.isNaN(at)
            ? 'Give the drop % to compare with as a whole number.'
            : none
              ? 'Pick at least one kind of item.'
              : regenOn && Number.isNaN(r)
                ? 'Give the regen time as a whole number.'
                : undefined;
        return {
          body: {
            preset: 'item_drop_pct',
            set: dropMode === 'set',
            value: v,
            only_if: dropCmp === 'any' ? ['ge', 0] : [dropCmp, at],
            limited,
            exclude_del_at_maint: maint,
            min_regen: regenOn ? r : null,
            ...kinds,
          },
          blocked,
          done: (n) => `Changed the drop rates of ${plural(n, 'monster')}.`,
        };
      }
      default:
        return { body: {}, blocked: 'This tool has nothing to run.', done: () => '' };
    }
  });

  const key = $derived(JSON.stringify(fix.body));
</script>

<div class="ws-form">
  {#if id === 'limits'}
    <div class="row"><span class="lbl">Lift</span><Seg label="Lift" options={[['game', 'Game limits'], ['level', 'Level restrictions']]} bind:value={limits} /></div>
    <p class="hint">
      {limits === 'game'
        ? 'Every item’s Game Limit becomes 0, except the quest portals (839, 1078, 1326, 1531, 1643, 1686 and 1749) and gang items.'
        : 'The level restriction abilities (135 and 136) come off every item, values too.'}
    </p>
  {:else if id === 'kills'}
    <div class="row"><span class="lbl">Timers</span><Seg label="Timers" options={[['day', 'Killed on a day'], ['clear', 'Cleared']]} bind:value={kills} /></div>
    {#if kills === 'day'}
      <label class="row"><span class="lbl">Day</span><input type="date" bind:value={day} /></label>
      <p class="hint">Every monster counts as killed on that day, at 01:01:02, as NMR sets it.</p>
    {:else}
      <p class="hint">Every monster’s last killed date and time go to 0, so each comes back when the game next looks.</p>
    {/if}
  {:else if id === 'buffer'}
    <div class="row"><span class="lbl">Buffer rooms</span><Seg label="Buffer rooms" options={[['add', 'Add'], ['remove', 'Remove']]} bind:value={buffer} /></div>
    <label class="row"><span class="lbl">Map</span><input class="num" inputmode="numeric" placeholder={buffer === 'add' ? '1 to 18' : 'every map'} bind:value={map} /></label>
    <p class="hint">
      {buffer === 'add'
        ? 'A room named “Buffer Room” for every number missing below a map’s last room, so the game keeps loading rooms past gaps of 50. Left empty: maps 1 to 18.'
        : 'Every room whose name begins with “Buffer Room” goes. Left empty: on every map.'}
    </p>
  {:else if id === 'strip'}
    <label class="row"><span class="lbl">Take off</span><input class="text" bind:value={strip} placeholder="such as ␛[0m" /></label>
    <p class="hint">Taken off the end of every text block part that ends with it, as NMR does. Type ␛ for the escape character.</p>
    <p class="hint">A text goes on from part to part, so a part can end mid-line (“giveitem ” then “8:…” in the next): taking a space off such a part joins two words. Stray spaces at the end of a text are the Health check’s, which leaves those parts alone.</p>
  {:else if id === 'drops'}
    <div class="row">
      <span class="lbl">Drop %</span>
      <Seg label="Set or shift" options={[['set', 'Set to'], ['add', 'Shift by']]} bind:value={dropMode} />
      <input class="num" inputmode="numeric" aria-label="Drop %" bind:value={dropValue} /><span class="unit">%</span>
    </div>
    <div class="row">
      <span class="lbl">Only drops at</span>
      <select bind:value={dropCmp} aria-label="Only drops at">
        <option value="any">any %</option>
        <option value="le">at most</option>
        <option value="ge">at least</option>
        <option value="eq">exactly</option>
        <option value="ne">anything but</option>
      </select>
      {#if dropCmp !== 'any'}<input class="num" inputmode="numeric" aria-label="Drop % to compare with" bind:value={dropAt} /><span class="unit">%</span>{/if}
    </div>
    <div class="row">
      <span class="lbl">Of</span>
      <label class="check"><input type="checkbox" bind:checked={kinds.weapons} /> Weapons</label>
      <label class="check"><input type="checkbox" bind:checked={kinds.armour} /> Armour</label>
      <label class="check"><input type="checkbox" bind:checked={kinds.keys} /> Keys</label>
      <label class="check"><input type="checkbox" bind:checked={kinds.containers} /> Containers</label>
      <label class="check"><input type="checkbox" bind:checked={kinds.scrolls} /> Scrolls</label>
    </div>
    <div class="row"><span class="lbl">Limited items</span><Seg label="Limited items" options={[['include', 'Include'], ['exclude', 'Leave out'], ['only', 'Only them']]} bind:value={limited} /></div>
    <label class="row check"><input type="checkbox" bind:checked={maint} /> Leave out weapons and armour deleted at maintenance (ability 119)</label>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={regenOn} /> Only monsters whose regen time is over</label>
      <input class="num" inputmode="numeric" aria-label="Regen time" disabled={!regenOn} bind:value={regen} />
    </div>
    <p class="hint">{dropMode === 'set' ? 'A drop already at the value is left alone, as NMR leaves it.' : 'Each drop moves by the value and stays from 1 to 100.'}</p>
  {/if}
</div>

<Run run={(dry) => api.fix(fix.body, dry)} {key} blocked={fix.blocked} done={fix.done} verb={fix.verb ? fix.verb : undefined} />

<style>
  .ws-form { display: flex; flex-direction: column; gap: 10px; }
  .ws-form:empty { display: none; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  @media (max-width: 560px) { .lbl { flex-basis: 100%; } }
  .check { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; font-size: 13px; }
  .check input { flex: none; }
  .row.check { flex-wrap: nowrap; }
  .num { width: 6.5em; font-family: var(--font-mono); }
  .text { width: min(22em, 100%); font-family: var(--font-mono); }
  .unit { font-weight: 800; color: var(--ink-3); }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 70ch; }
</style>
