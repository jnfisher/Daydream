<!--
  NMR's lists, kept current: the limited items and where their copies are, every place one item
  is, the control rooms and the rooms that name them, who stands where, and the monsters of each
  group and index. Each record is a chip that opens beside the page, and each list downloads as
  CSV. Read again after each save.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Download from '@lucide/svelte/icons/download';
  import MapIcon from '@lucide/svelte/icons/map';
  import { api, type ControlRoom, type GroupIndex, type ItemPlace, type LimitedItem, type NpcRoom, type Scan } from '../api';
  import { MONSTER_GROUP } from '../layouts';
  import { recordChoices } from '../library/choices';
  import { toCsv } from '../library/grid';
  import Picker from '../library/Picker.svelte';
  import RefChip from '../library/RefChip.svelte';
  import { sfx, write } from '../shell/prefs.svelte';
  import { app } from '../shell/state.svelte';
  import { messageOf, toast } from '../shell/toasts.svelte';
  import { csvOf, finds, limitedOrder, overBy, placeRef, type ListId } from './reports';

  let { id }: { id: ListId } = $props();

  /** Rows shown at first, and added by each "Show more". */
  const PAGE = 100;
  /** A control room's rooms shown before "and N more". */
  const ROOMS = 24;

  const n = (x: number) => x.toLocaleString();
  const count = (x: number, one: string, many = `${one}s`) => `${n(x)} ${x === 1 ? one : many}`;
  const groupName = (g: number) => String(MONSTER_GROUP[g] ?? `Group ${g}`);

  let scan = $state<Scan>({ rooms: true, shops: true, users: true });
  let item = $state('');
  let find = $state('');
  let shown = $state(PAGE);
  let open = $state<Record<string, boolean>>({});
  let error = $state('');
  let loading = $state(true);

  let limited = $state<LimitedItem[]>([]);
  let places = $state<ItemPlace[] | null>(null);
  let areas = $state<ControlRoom[]>([]);
  let npcs = $state<NpcRoom[]>([]);
  let groups = $state<GroupIndex[]>([]);

  const itemNumber = $derived(/^\s*\d+\s*$/.test(item) ? Number(item) : 0);

  // read again when the world changes, or what to read does
  let asked = 0;
  $effect(() => {
    void app.changes;
    void app.current;
    const [which, s, wanted] = [id, { ...scan }, itemNumber];
    const my = ++asked;
    error = '';
    loading = true;
    const done = <T,>(p: Promise<T>, put: (v: T) => void) =>
      p
        .then((v) => my === asked && put(v))
        .catch((e) => my === asked && (error = messageOf(e)))
        .finally(() => my === asked && (loading = false));
    if (which === 'limited') void done(api.limitedItems(s), (r) => (limited = limitedOrder(r.items)));
    else if (which === 'where') {
      if (wanted > 0) void done(api.findItem(wanted, s), (r) => (places = r.places));
      else [places, loading] = [null, false];
    } else if (which === 'areas') void done(api.controlRooms(), (r) => (areas = r.rooms));
    else if (which === 'npcs') void done(api.npcRooms(), (r) => (npcs = r.rooms));
    else void done(api.monsterGroups(), (r) => (groups = r.groups));
  });

  // finding starts the list over
  $effect(() => {
    void find;
    void id;
    shown = PAGE;
  });

  const placeWords = (p: ItemPlace) => `${p.key} ${p.name}`;
  const foundLimited = $derived(limited.filter((i) => finds(`${i.number} ${i.name} ${i.places.map(placeWords).join(' ')}`, find)));
  const foundPlaces = $derived((places ?? []).filter((p) => finds(`${p.kind} ${placeWords(p)}`, find)));
  const foundAreas = $derived(areas.filter((c) => finds(`${c.map}/${c.room} ${c.name} ${c.rooms.join(' ')}`, find)));
  const foundNpcs = $derived(npcs.filter((r) => finds(`${r.map}/${r.room} ${r.room_name} ${r.monster} ${r.monster_name}`, find)));
  const foundGroups = $derived(
    groups.filter((g) => finds(`${groupName(g.group)} ${g.group}/${g.index} ${g.monsters.map(([m, name]) => `${m} ${name}`).join(' ')}`, find)),
  );

  const total = $derived(
    { limited: foundLimited, where: foundPlaces, areas: foundAreas, npcs: foundNpcs, spawns: foundGroups }[id].length,
  );

  const summary = $derived.by(() => {
    if (id === 'limited') {
      const over = limited.filter((i) => i.over).length;
      return `${count(limited.length, 'limited item')}, ${over ? `${n(over)} over ${over === 1 ? 'its' : 'their'} limit` : 'none over its limit'}.`;
    }
    if (id === 'where') return places ? (places.length ? `In ${count(places.length, 'place')}.` : 'It is nowhere: no room, shop or player holds it.') : '';
    if (id === 'areas') return `${count(areas.length, 'control room')}, named by ${count(areas.reduce((a, c) => a + c.rooms.length, 0), 'room')}.`;
    if (id === 'npcs') return `${count(npcs.length, 'room')} with a permanent NPC.`;
    return `${count(groups.length, 'group and index', 'groups and indexes')} that monsters belong to.`;
  });

  function download() {
    const [c, file] =
      id === 'limited'
        ? [csvOf.limited(foundLimited), 'limited-items.csv']
        : id === 'where'
          ? [csvOf.places(foundPlaces), `item-${itemNumber}-places.csv`]
          : id === 'areas'
            ? [csvOf.areas(foundAreas), 'control-rooms.csv']
            : id === 'npcs'
              ? [csvOf.npcs(foundNpcs), 'npc-rooms.csv']
              : [csvOf.groups(foundGroups, groupName), 'monster-groups.csv'];
    const url = URL.createObjectURL(new Blob([toCsv(c.header, c.rows)], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = file;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    sfx('pop');
    toast(`Wrote ${count(c.rows.length, 'row')} to ${file}.`);
  }

  /** Opens the map coloured a way (the map keeps the choice). */
  function onTheMap(colour: string) {
    write('dd-map-colour', colour);
    location.hash = '#/world';
  }
</script>

<div class="lists">
  {#if id === 'where'}
    <div class="row">
      <label class="field-item"><span class="lbl">Item</span><input class="num" inputmode="numeric" placeholder="number" aria-label="Item number" bind:value={item} /></label>
      <Picker what="an item" choices={() => recordChoices('Items')} onpick={(v) => (item = String(v))} />
      {#if itemNumber}<RefChip table="Items" key={[itemNumber]} />{/if}
    </div>
  {/if}

  {#if id === 'limited' || id === 'where'}
    <div class="row scan" role="group" aria-label="Where to look">
      <span class="lbl">Look in</span>
      <label class="check"><input type="checkbox" bind:checked={scan.rooms} /> Rooms</label>
      <label class="check"><input type="checkbox" bind:checked={scan.shops} /> Shops</label>
      <label class="check"><input type="checkbox" bind:checked={scan.users} /> Players</label>
    </div>
  {/if}

  {#if id === 'areas'}
    <div class="callout info">
      <MapIcon size={16} />
      <span>On the map, the <b>Area</b> colouring tints each room by the control room it names.</span>
    </div>
    <div><button type="button" class="btn sm sticker" onclick={() => onTheMap('area')}><MapIcon size={14} />Colour the map by area</button></div>
  {:else if id === 'npcs'}
    <div class="callout info">
      <MapIcon size={16} />
      <span>On the map, zoomed in, a room with a permanent NPC wears a person badge.</span>
    </div>
  {/if}

  {#if !(id === 'where' && !itemNumber)}
    <div class="bar">
      <input type="search" class="finder" placeholder="Find in the list" aria-label="Find in the list" bind:value={find} />
      <span class="sum faint">{loading && !total ? 'Reading the world…' : find.trim() ? `${count(total, 'match', 'matches')}. ${summary}` : summary}</span>
      <button type="button" class="btn sm ghost" disabled={!total} onclick={download}><Download size={14} />CSV</button>
    </div>
  {/if}

  {#if error}
    <p class="bad">{error}</p>
  {:else if id === 'where' && !itemNumber}
    <p class="faint">Pick an item by name, or type its number.</p>
  {:else}
    <ul class="rows">
      {#if id === 'limited'}
        {#each foundLimited.slice(0, shown) as i (i.number)}
          <li class="row-item" class:over={i.over}>
            <div class="lead">
              <RefChip table="Items" key={[i.number]} />
              <span class="pill">limit {n(i.limit)}</span>
              <span class="pill info">{n(i.in_use)} in use</span>
              {#if i.over}<span class="pill bad">{n(overBy(i))} over</span>{/if}
            </div>
            {#if i.places.length}
              <div class="chips">
                {#each i.places as p (p.kind + p.key)}
                  {@const r = placeRef(p)}
                  <span class="at"><RefChip table={r.table} key={r.key} compact why={p.kind === 'hidden' ? 'hidden' : undefined} />{#if p.kind === 'hidden'}<span class="tag">hidden</span>{/if}<span class="x">×{n(p.count)}</span></span>
                {/each}
              </div>
            {/if}
          </li>
        {/each}
      {:else if id === 'where'}
        {#each foundPlaces.slice(0, shown) as p (p.kind + p.key)}
          {@const r = placeRef(p)}
          <li class="row-item">
            <div class="lead">
              <RefChip table={r.table} key={r.key} />
              {#if p.kind === 'hidden'}<span class="tag">hidden</span>{/if}
              <span class="pill info">×{n(p.count)}</span>
            </div>
          </li>
        {/each}
      {:else if id === 'areas'}
        {#each foundAreas.slice(0, shown) as c (`${c.map}/${c.room}`)}
          {@const key = `${c.map}/${c.room}`}
          <li class="row-item">
            <div class="lead">
              <RefChip table="Rooms" key={[c.map, c.room]} />
              <span class="pill lilac">named by {count(c.rooms.length, 'room')}</span>
            </div>
            <div class="chips">
              {#each open[key] ? c.rooms : c.rooms.slice(0, ROOMS) as r (r)}<RefChip table="Rooms" key={[c.map, r]} compact />{/each}
              {#if c.rooms.length > ROOMS}
                <button type="button" class="btn sm ghost" onclick={() => (open[key] = !open[key])}>{open[key] ? 'Fewer' : `and ${n(c.rooms.length - ROOMS)} more`}</button>
              {/if}
            </div>
          </li>
        {/each}
      {:else if id === 'npcs'}
        {#each foundNpcs.slice(0, shown) as r (`${r.map}/${r.room}`)}
          <li class="row-item pair">
            <RefChip table="Rooms" key={[r.map, r.room]} />
            <span class="faint" aria-hidden="true">holds</span>
            <RefChip table="Monsters" key={[r.monster]} />
          </li>
        {/each}
      {:else}
        {#each foundGroups.slice(0, shown) as g (`${g.group}/${g.index}`)}
          <li class="row-item">
            <div class="lead">
              <b class="gi">{groupName(g.group)} <span class="tnum">{g.group}/{g.index}</span></b>
              {#if g.monsters.length >= 15}
                <span class="pill" title="NMR's list keeps 15 a group and index">15 or more monsters</span>
              {:else}
                <span class="pill">{count(g.monsters.length, 'monster')}</span>
              {/if}
            </div>
            <div class="chips">
              {#each g.monsters as [m] (m)}<RefChip table="Monsters" key={[m]} compact />{/each}
            </div>
          </li>
        {/each}
      {/if}
    </ul>
    {#if total > shown}
      <button type="button" class="btn sm more" onclick={() => (shown += PAGE)}>Show {n(Math.min(PAGE, total - shown))} more of {n(total - shown)}</button>
    {/if}
  {/if}
</div>

<style>
  .lists { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 5em; }
  .field-item { display: inline-flex; align-items: center; gap: 8px; }
  .num { width: 7em; font-family: var(--font-mono); }
  .check { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; font-size: 13px; }
  .check input { flex: none; }
  .bar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .finder { width: min(18em, 100%); }
  .sum { flex: 1 1 14em; font-size: 12.5px; font-weight: 700; }
  .bad { color: var(--bad); font-weight: 700; }
  .rows { list-style: none; margin: 0; padding: 0; }
  .row-item { display: flex; flex-direction: column; gap: 6px; padding: 9px 2px; min-width: 0; }
  .row-item + .row-item { border-top: 1px dashed var(--line); }
  .row-item.over .lead :global(.ref) { border-color: var(--bad); }
  .row-item.pair { flex-direction: row; align-items: center; flex-wrap: wrap; gap: 8px; }
  .lead { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; min-width: 0; }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; padding-left: 14px; min-width: 0; }
  .at { display: inline-flex; align-items: center; gap: 4px; min-width: 0; max-width: 100%; }
  .x { font: 700 12px var(--font-mono); color: var(--ink-3); }
  .tag { font-size: 11.5px; font-weight: 800; color: var(--ink-3); text-transform: uppercase; letter-spacing: 0.04em; }
  .gi { font-weight: 800; font-size: 13.5px; }
  .gi .tnum { color: var(--ink-3); font-weight: 700; margin-left: 2px; }
  .more { align-self: flex-start; }
  @media (max-width: 560px) {
    .lbl { flex-basis: 100%; }
    .chips { padding-left: 0; }
  }
</style>
