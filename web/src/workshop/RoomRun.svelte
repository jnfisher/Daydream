<!--
  Edit rooms together › a run of rooms by number: NMR's Mass Room Editor. The fields of its form,
  grouped as it groups them, set on every room of a map from one room number to another. The
  values can be taken from a room. It previews every value it changes, and applies as one step.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Download from '@lucide/svelte/icons/download';
  import { toShown, toWire } from '../ansi';
  import { api, type Cell } from '../api';
  import { EXIT_TYPE, EXITS, MONSTER_GROUP, ROOM_TYPE, type Options } from '../layouts';
  import { recordChoices } from '../library/choices';
  import Picker from '../library/Picker.svelte';
  import { app } from '../shell/state.svelte';
  import { messageOf, toast } from '../shell/toasts.svelte';
  import { whole } from './fields';
  import Run from './Run.svelte';

  const count = (n: number) => Array.from({ length: n }, (_, i) => i);
  const COINS = ['Runic', 'Platinum', 'Gold', 'Silver', 'Copper'];

  /** A field of a group: its column, its label, and its choices if it has a list. */
  type Field = [column: string, label: string, options?: Options];

  /** NMR's groups, in its form's order. */
  const GROUPS: [string, Field[]][] = [
    ['Name', [['Name', 'Name']]],
    ['Description', count(7).map((i): Field => [`Desc ${i}`, `Line ${i + 1}`])],
    ['Ansi Map', [['AnsiMap', 'Ansi Map']]],
    ['Room Type', [['Type', 'Room Type', ROOM_TYPE]]],
    ['Shop #', [['Shop Number', 'Shop #']]],
    ['Gang House #', [['Gang House Number', 'Gang House #']]],
    ['Light', [['Light', 'Light']]],
    ['Attributes', [['Attributes', 'Attributes']]],
    ['Permanent NPC', [['Perm NPC', 'Permanent NPC']]],
    ['Monster Type', [['Mon Type', 'Monster Type', MONSTER_GROUP]]],
    ['Min Index', [['Min Index', 'Min Index']]],
    ['Max Index', [['Max Index', 'Max Index']]],
    ['Max Regen', [['Max Regen', 'Max Regen']]],
    ['Max Area', [['Max Area', 'Max Area']]],
    ['Control Room', [['Control Room', 'Control Room']]],
    ['Death Room', [['Death Room', 'Death Room']]],
    ['Exit Room', [['Exit Room', 'Exit Room']]],
    ['CMD Text', [['Command Text', 'CMD Text']]],
    ['Delay', [['Delay', 'Delay']]],
    ['Room Spell', [['Spell', 'Room Spell']]],
    ['Visible Coins', COINS.map((c): Field => [c, c])],
    ['Hidden Coins', COINS.map((c): Field => [`Invis${c}`, c])],
    ...EXITS.map((d, i): [string, Field[]] => [
      `Exit ${d}`,
      [[`Exit ${i}`, 'To room'], [`Type ${i}`, 'Exit type', EXIT_TYPE], [`Para1 ${i}`, 'Para 1'], [`Para2 ${i}`, 'Para 2'], [`Para3 ${i}`, 'Para 3'], [`Para4 ${i}`, 'Para 4']],
    ]),
    ['Items In Room', count(17).flatMap((i): Field[] => [[`Room Item ${i}`, `Item ${i}`], [`Room Item ${i} USES`, 'Uses'], [`Room Item ${i} QTY`, 'Qty − 1']])],
    ['Hidden Items In Room', count(15).flatMap((i): Field[] => [[`Hidden Item ${i}`, `Item ${i}`], [`Hidden Item ${i} USES`, 'Uses'], [`Hidden Item ${i} QTY`, 'Qty − 1']])],
    ['Placed Items', count(10).map((i): Field => [`Placed Item ${i}`, `Placed ${i}`])],
    ['Current Monsters In Room', count(15).map((i): Field => [`CurrentRoomMon ${i}`, `Monster ${i}`])],
  ];

  const rooms = $derived(app.tables.find((t) => t.name === 'Rooms'));
  const textual = $derived(new Set(rooms?.columns.filter((c) => c.type === 'text' || c.type === 'memo').map((c) => c.name) ?? []));
  const has = $derived(new Set(rooms?.columns.map((c) => c.name) ?? []));

  let map = $state('1');
  let from = $state('');
  let to = $state('');
  let srcMap = $state('1');
  let srcRoom = $state('');
  let on = $state<Record<string, boolean>>({});
  /** Each column's value as typed (a list's as its number). */
  let vals = $state<Record<string, string>>({});
  let taking = $state(false);

  const blank = (column: string, options?: Options) => (options ? (Object.keys(options)[0] ?? '0') : textual.has(column) ? '' : '0');
  const valueOf = (column: string, options?: Options) => vals[column] ?? blank(column, options);

  const chosen = $derived(GROUPS.filter(([g, fields]) => on[g] && fields.some(([c]) => has.has(c))));

  async function take() {
    const [m, r] = [whole(srcMap), whole(srcRoom)];
    if (!(m > 0) || !(r > 0)) {
      toast('Give the map and room to take the values from.', { kind: 'error' });
      return;
    }
    taking = true;
    try {
      const row = await api.row('Rooms', [m, r]);
      const next: Record<string, string> = {};
      for (const [, fields] of GROUPS) {
        for (const [column] of fields) {
          const v: Cell = row.values[column] ?? null;
          if (v === null) continue;
          next[column] = textual.has(column) ? toShown(String(v).split('\0')[0]!) : String(v);
        }
      }
      vals = next;
      toast(`Took the values of room ${m}/${r}.`);
    } catch (e) {
      toast(messageOf(e), { kind: 'error' });
    } finally {
      taking = false;
    }
  }

  const span = $derived({ map: whole(map), from: whole(from), to: whole(to) });

  /** The values to set, or why one cannot be. */
  const values = $derived.by((): { values: [string, Cell][] } | { error: string } => {
    const out: [string, Cell][] = [];
    for (const [, fields] of chosen) {
      for (const [column, label, options] of fields) {
        if (!has.has(column)) continue;
        const v = valueOf(column, options);
        if (textual.has(column)) out.push([column, toWire(v)]);
        else {
          const n = whole(v.trim() || '0');
          if (Number.isNaN(n)) return { error: `${label}: ${v} is not a whole number.` };
          out.push([column, n]);
        }
      }
    }
    return { values: out };
  });

  const blocked = $derived(
    !rooms
      ? 'The world has no rooms.'
      : !(span.map > 0) || Number.isNaN(span.from) || Number.isNaN(span.to)
        ? 'Give the map, and the first and last rooms.'
        : span.to < span.from
          ? 'The last room comes before the first.'
          : !chosen.length
            ? 'Pick the fields to set.'
            : 'error' in values
              ? values.error
              : '',
  );

  const body = $derived({
    table: 'Rooms',
    action: { kind: 'set', values: 'values' in values ? values.values : [] },
    range: { table: 'Rooms', map: span.map, from: span.from, to: span.to },
    only_if: [],
  });

  const label = $derived(`Set ${chosen.map(([g]) => g).join(', ')} on map ${span.map} rooms ${span.from} to ${span.to}`);
</script>

<div class="ws-form">
  <div class="row">
    <span class="lbl">Rooms</span>
    <label class="inline">Map <input class="num" inputmode="numeric" aria-label="Map" bind:value={map} /></label>
    <label class="inline">from <input class="num" inputmode="numeric" aria-label="First room" bind:value={from} /></label>
    <label class="inline">to <input class="num" inputmode="numeric" aria-label="Last room" bind:value={to} /></label>
  </div>

  <div class="row">
    <span class="lbl">Values from</span>
    <label class="inline">Map <input class="num" inputmode="numeric" aria-label="Map of the room to take values from" bind:value={srcMap} /></label>
    <label class="inline">room <input class="num" inputmode="numeric" aria-label="Room to take values from" bind:value={srcRoom} /></label>
    <Picker what="a room" choices={() => recordChoices('Rooms', whole(srcMap))} onpick={(v) => (srcRoom = String(v))} />
    <button type="button" class="btn sm" disabled={taking} onclick={() => void take()}><Download size={14} />{taking ? 'Taking…' : 'Take its values'}</button>
  </div>

  <div class="row top">
    <span class="lbl">Fields to set</span>
    <div class="groups" role="group" aria-label="Fields to set">
      {#each GROUPS as [g, fields] (g)}
        {#if fields.some(([c]) => has.has(c))}
          <button type="button" class="chip" class:on={on[g]} aria-pressed={!!on[g]} onclick={() => (on[g] = !on[g])}>{g}</button>
        {/if}
      {/each}
      {#if chosen.length}<button type="button" class="btn sm ghost" onclick={() => (on = {})}>None</button>{/if}
    </div>
  </div>

  {#each chosen as [g, fields] (g)}
    {@const mine = fields.filter(([c]) => has.has(c))}
    <fieldset class="group">
      <legend>{g}</legend>
      <div class="fields" class:wide={mine.some(([c]) => textual.has(c))}>
        {#each mine as [column, label, options] (column)}
          <!-- a group of one is named by its legend -->
          <label class="field">
            {#if mine.length > 1}<span>{label}</span>{/if}
            {#if options}
              <select aria-label={mine.length > 1 ? undefined : label} value={valueOf(column, options)} onchange={(e) => (vals[column] = e.currentTarget.value)}>
                {#each Object.entries(options) as [v, l] (v)}<option value={v}>{l}</option>{/each}
              </select>
            {:else}
              <input
                class:num={!textual.has(column)}
                aria-label={mine.length > 1 ? undefined : label}
                inputmode={textual.has(column) ? undefined : 'numeric'}
                value={valueOf(column)}
                oninput={(e) => (vals[column] = e.currentTarget.value)}
              />
            {/if}
          </label>
        {/each}
      </div>
    </fieldset>
  {/each}
  {#if chosen.some(([g]) => g.includes('Items'))}<p class="hint">Item quantities are the count less one: 0 for one.</p>{/if}
</div>

<Run run={(dry) => api.modify(body, dry, label)} key={JSON.stringify(body)} {blocked} done={(n) => `Set ${chosen.map(([g]) => g).join(', ')} on ${n.toLocaleString()} ${n === 1 ? 'room' : 'rooms'}.`} />

<style>
  .ws-form { display: flex; flex-direction: column; gap: 10px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .row.top { align-items: flex-start; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  .row.top .lbl { padding-top: 5px; }
  .inline { display: inline-flex; align-items: center; gap: 6px; font-weight: 800; font-size: 12.5px; color: var(--ink-2); }
  .num { width: 6.5em; font-family: var(--font-mono); }
  .groups { display: flex; flex-wrap: wrap; gap: 5px; flex: 1 1 0; min-width: 0; }
  .chip { min-height: 28px; padding: 0 10px; border-radius: 999px; font-size: 12.5px; font-weight: 700; background: var(--surface-2); border: 1px solid var(--line); color: var(--ink-2); box-shadow: none; }
  .chip:hover:not(.on) { background: var(--surface-3); }
  .chip.on { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); }
  .group { border: 1px solid var(--line); border-radius: 14px; padding: 8px 12px 12px; margin: 0; min-width: 0; }
  .group legend { font: 800 12px var(--font-ui); color: var(--ink-2); padding: 0 6px; }
  .fields { display: grid; grid-template-columns: repeat(auto-fill, minmax(8.5em, 1fr)); gap: 8px 10px; }
  .fields.wide { grid-template-columns: repeat(auto-fill, minmax(min(22em, 100%), 1fr)); }
  .field { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .field span { font: 800 11px var(--font-ui); color: var(--ink-3); letter-spacing: 0.03em; text-align: left; }
  .field input, .field select { width: 100%; min-width: 0; }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 75ch; }
  @media (max-width: 560px) {
    .lbl { flex-basis: 100%; }
    .row.top .lbl { padding-top: 0; }
  }
</style>
