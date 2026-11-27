<!--
  Where a monster spawns: how many rooms take its Group and Index (by map, and by the rooms'
  names), and its group's ladder: every monster of the group by Index, with the indexes the
  group's rooms reach marked, so a new monster can be slotted in beside the right company.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { api, type Cell, type Ladder } from '../api';
  import { MONSTER_GROUP } from '../layouts';
  import { inspect } from '../shell/state.svelte';
  import { messageOf } from '../shell/toasts.svelte';
  import type { Draft } from './draft.svelte';

  let { draft, rowKey, version }: { draft: Draft; rowKey: Cell[]; version: number } = $props();

  const num = (v: unknown) => Number(v) || 0;
  const group = $derived(num(draft.get('Group')));
  const index = $derived(num(draft.get('Index')));
  const me = $derived(num(rowKey[0]));
  const groupName = $derived(MONSTER_GROUP[group] ?? `group ${group}`);

  let ladder = $state<Ladder | null>(null);
  let error = $state('');

  $effect(() => {
    const g = group;
    void version;
    let gone = false;
    error = '';
    api
      .spawns(g)
      .then((l) => {
        if (!gone) ladder = l;
      })
      .catch((e) => {
        if (!gone) error = messageOf(e);
      });
    return () => (gone = true);
  });

  type Range = Ladder['ranges'][number];
  const takes = (r: Range, ix: number) => (r.min === 0 || ix >= r.min) && ix <= r.max;

  const ranges = $derived(ladder && ladder.group === group ? ladder.ranges : []);
  const taking = $derived(ranges.filter((r) => takes(r, index)));
  const rooms = $derived(taking.reduce((a, r) => a + r.rooms, 0));
  const maps = $derived.by(() => {
    const m = new Map<number, number>();
    for (const r of taking) for (const [map, n] of r.maps) m.set(map, (m.get(map) ?? 0) + n);
    return [...m].sort((a, b) => b[1] - a[1]);
  });
  const SHOWN = 5;
  const names = $derived.by(() => {
    const m = new Map<string, number>();
    for (const r of taking) for (const [name, n] of r.names) m.set(name, (m.get(name) ?? 0) + n);
    const all = [...m].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
    const rest = all.slice(SHOWN);
    return { top: all.slice(0, SHOWN), others: rest.length, rest: rest.reduce((a, [, n]) => a + n, 0) };
  });
  const widest = $derived(Math.max(1, ...names.top.map(([, n]) => n), names.rest));
  const reach = $derived(ranges.length ? [Math.min(...ranges.map((r) => r.min)), Math.max(...ranges.map((r) => r.max))] : null);

  /** The ladder's rungs: each Index the group has (and this monster's), with who is on it. */
  const rungs = $derived.by(() => {
    const by = new Map<number, { n: number; name: string }[]>();
    for (const [n, name, ix] of ladder && ladder.group === group ? ladder.monsters : []) {
      if (n === me) continue;
      by.set(ix, [...(by.get(ix) ?? []), { n, name }]);
    }
    const own = String(draft.get('Name') ?? '').split('\0')[0] || 'this one';
    by.set(index, [{ n: me, name: own }, ...(by.get(index) ?? [])]);
    return [...by].sort((a, b) => a[0] - b[0]).map(([ix, who]) => ({ ix, who, in: ranges.some((r) => takes(r, ix)) }));
  });

  /** The ranges that take its index, or every one of the group's when asked. */
  let every = $state(false);
  const own = $derived(ranges.filter((r) => takes(r, index)));
  const chips = $derived(every || !own.length ? ranges : own);

  let focus = $state<Range | null>(null);
  const lit = (ix: number) => !!focus && takes(focus, ix);

  let strip = $state<HTMLElement | null>(null);
  // this monster's rung in view
  $effect(() => {
    void rungs;
    const el = strip?.querySelector<HTMLElement>('.rung.me');
    if (el && strip) strip.scrollLeft = Math.max(0, el.offsetLeft - (strip.clientWidth - el.offsetWidth) / 2);
  });

  const plural = (n: number, one: string, many: string) => `${n.toLocaleString()} ${n === 1 ? one : many}`;
</script>

{#if error}
  <p class="faint">{error}</p>
{:else if !ladder}
  <p class="faint">Looking through the rooms…</p>
{:else}
  <div class="spawns">
    <p class="sum">
      {#if rooms}
        {plural(rooms, 'room spawns', 'rooms spawn')} it{#if maps.length === 1}, on map {maps[0]![0]}{:else}: {maps.slice(0, 4).map(([m, n]) => `${n.toLocaleString()} on map ${m}`).join(', ')}{maps.length > 4 ? `, and ${maps.length - 4} more maps` : ''}{/if}.
      {:else if reach}
        No room spawns it: the rooms of {groupName} take index {reach[0]} to {reach[1]}, and its index is {index}.
      {:else}
        No room spawns it: no room spawns from {groupName}.
      {/if}
      {#if ladder.idle}<span class="faint">{plural(ladder.idle, 'more room', 'more rooms')} of the group spawn nothing (no Max Index or Max Regen, or a room type that never spawns).</span>{/if}
    </p>

    {#if names.top.length}
      <div class="bars" role="list" aria-label="The rooms that spawn it, by name">
        {#each names.top as [name, n] (name)}
          <div class="bar-row" role="listitem" title="{name || 'No name'}: {plural(n, 'room', 'rooms')}">
            <span class="nm">{name || 'No name'}</span><span class="track"><span class="fill" style:width="{(100 * n) / widest}%"></span></span><span class="v">{n.toLocaleString()}</span>
          </div>
        {/each}
        {#if names.others}
          <div class="bar-row" role="listitem" title="{plural(names.others, 'other name', 'other names')}: {plural(names.rest, 'room', 'rooms')}">
            <span class="nm faint">{plural(names.others, 'other name', 'other names')}</span><span class="track"><span class="fill rest" style:width="{(100 * names.rest) / widest}%"></span></span><span class="v">{names.rest.toLocaleString()}</span>
          </div>
        {/if}
      </div>
    {/if}

    <div class="eyebrow">{groupName}, by index</div>
    <div class="ladder" bind:this={strip} aria-label="The monsters of {groupName} by index">
      {#each rungs as r, k (r.ix)}
        {#if k > 0 && r.ix > rungs[k - 1]!.ix + 1}<span class="gap" aria-hidden="true">…</span>{/if}
        <div class="rung" class:me={r.ix === index} class:in={r.in} class:lit={lit(r.ix)}>
          <div class="ix">{r.ix}</div>
          <div class="who">
            {#each r.who.slice(0, 3) as w (w.n)}
              {#if w.n === me}<b>{w.name}</b>{:else}<button type="button" class="link" onclick={() => inspect({ table: 'Monsters', key: [w.n], name: w.name })}>{w.name}</button>{/if}
            {/each}
            {#if r.who.length > 3}<span class="faint">+{r.who.length - 3}</span>{/if}
          </div>
        </div>
      {/each}
    </div>
    {#if ranges.length}
      <div class="ranges">
        <span class="faint">{chips === own ? `Index ${index} is within:` : "The group's rooms take:"}</span>
        {#each chips as r (`${r.min}-${r.max}`)}
          <button
            type="button"
            class="rg"
            class:mine={takes(r, index)}
            onmouseenter={() => (focus = r)}
            onmouseleave={() => (focus = null)}
            onfocus={() => (focus = r)}
            onblur={() => (focus = null)}
            title="Rooms with Min Index {r.min} and Max Index {r.max}"
          >
            {r.min === 0 ? `up to ${r.max}` : r.min === r.max ? `${r.min}` : `${r.min} to ${r.max}`}<small>{plural(r.rooms, 'room', 'rooms')}</small>
          </button>
        {/each}
        {#if own.length && own.length < ranges.length}
          <button type="button" class="btn sm ghost more" aria-expanded={every} onclick={() => (every = !every)}>
            {every ? 'Only the ones that take it' : `${plural(ranges.length - own.length, 'other range', 'other ranges')} of the group`}
          </button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .spawns { display: grid; gap: 10px; min-width: 0; }
  .sum { margin: 0; font-size: 13.5px; font-weight: 700; line-height: 1.45; }
  .sum .faint { display: block; font-size: 12px; margin-top: 2px; }
  .bars { display: grid; gap: 5px; }
  .bar-row { display: grid; grid-template-columns: minmax(6rem, 11rem) minmax(0, 1fr) 3rem; align-items: center; gap: 8px; font-size: 12.5px; font-weight: 700; }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track { height: 12px; display: block; }
  .fill { display: block; height: 100%; min-width: 3px; border-radius: 0 4px 4px 0; background: var(--mint); }
  .fill.rest { background: var(--line-2); }
  .v { text-align: right; font: 12px var(--font-mono); color: var(--ink-2); }
  .ladder { display: flex; gap: 6px; overflow-x: auto; padding: 14px 2px 8px; position: relative; }
  .rung { flex: none; width: 8.2rem; border: 1px solid var(--line); border-radius: 12px; padding: 6px 8px; background: var(--surface-2); position: relative; opacity: .7; }
  .rung.in { opacity: 1; background: var(--surface); }
  .rung.in::before { content: ''; position: absolute; left: -4px; right: -4px; top: -9px; height: 5px; border-radius: 3px; background: var(--mint); }
  .rung.lit { outline: 2px solid var(--lilac); outline-offset: 1px; }
  .rung.me { border: var(--stroke) solid var(--outline); background: var(--peach-2); opacity: 1; }
  .ix { font: 800 12px var(--font-mono); color: var(--ink-3); }
  .who { display: grid; gap: 1px; font-size: 12px; font-weight: 700; line-height: 1.25; }
  .who b, .who .link { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: left; }
  .link { border: 0; background: none; padding: 0; min-height: 0; width: 100%; box-shadow: none; font: inherit; color: var(--link); cursor: pointer; }
  .link:hover:not(:disabled) { background: none; text-decoration: underline; }
  .gap { flex: none; align-self: center; color: var(--ink-3); font-weight: 900; }
  .ranges { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 12px; font-weight: 700; }
  .rg { display: inline-flex; align-items: baseline; gap: 5px; min-height: 0; height: 24px; padding: 0 9px; border-radius: 999px; font: 800 12px var(--font-ui); }
  .rg small { color: var(--ink-3); font-weight: 700; }
  .rg.mine { background: var(--mint-2); color: var(--mint-ink); border-color: transparent; }
  .more { height: 24px; min-height: 0; font-size: 12px; color: var(--link); }
  @container (max-width: 420px) {
    .bar-row { grid-template-columns: minmax(5rem, 8rem) minmax(0, 1fr) 2.6rem; }
  }
</style>
