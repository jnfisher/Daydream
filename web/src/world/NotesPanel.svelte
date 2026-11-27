<!--
  The notes on the map shown, in the Notes menu: a note (or label) for the room selected, a region
  around the rooms picked with Select, and the map's notes, its things to do first. Ticking one
  off is a click; choosing one goes to its room and opens it. They are the builder's alone, kept
  beside the working copy and never published.
-->
<svelte:options css="injected" />

<script lang="ts">
  import SquareDashed from '@lucide/svelte/icons/square-dashed';
  import StickyNote from '@lucide/svelte/icons/sticky-note';
  import Tag from '@lucide/svelte/icons/tag';
  import type { MapNote } from './model';
  import type { Noter } from './noter.svelte';
  import { colourOf, noteOn, toDo } from './notes';

  let {
    noter,
    selected,
    picked = [],
    onclose,
  }: {
    noter: Noter;
    /** The room selected, if one is. */
    selected: number | null;
    /** The rooms picked with Select. */
    picked?: readonly number[];
    onclose: () => void;
  } = $props();

  let only = $state<'todo' | 'all'>('all');
  const notes = $derived(noter.notes);
  /** Things to do first, then notes, labels and regions, each in the order written. */
  const ORDER = (n: MapNote) => (toDo(n) ? 0 : n.kind === 'note' ? (n.done ? 2 : 1) : n.kind === 'label' ? 3 : 4);
  const shown = $derived(
    notes
      .filter((n) => only === 'all' || toDo(n))
      .map((n, i) => [n, i] as const)
      .sort((a, b) => ORDER(a[0]) - ORDER(b[0]) || a[1] - b[1])
      .map(([n]) => n),
  );
  const host = $derived(noter.host);
  const hasNote = $derived(selected !== null && !!noteOn(notes, selected, 'note'));
  const hasLabel = $derived(selected !== null && !!noteOn(notes, selected, 'label'));

  function open(n: MapNote) {
    onclose();
    host.go(noter.roomOf(n));
    noter.edit(n.id);
  }
</script>

<div class="eyebrow head">Notes on map {noter.map}</div>
<p class="lay">Yours alone: kept beside the working copy, never published.</p>
<div class="adds">
  <button type="button" class="menu-item" disabled={selected === null} onclick={() => (onclose(), noter.write(selected!, 'note'))}>
    <StickyNote size={15} />
    <span>{hasNote ? 'The note on' : 'A note on'} {selected === null ? 'the room selected' : host.called(selected)}<span class="sub">{selected === null ? 'Select a room first' : 'Words to remember, or a thing to do'}</span></span>
    <span class="end"><span class="kbd">N</span></span>
  </button>
  <button type="button" class="menu-item" disabled={selected === null} onclick={() => (onclose(), noter.write(selected!, 'label'))}>
    <Tag size={15} />
    <span>{hasLabel ? 'Its label' : 'A label by it'}<span class="sub">Words drawn on the map</span></span>
    <span class="end"><span class="kbd">Shift N</span></span>
  </button>
  <button type="button" class="menu-item" disabled={!picked.length} onclick={() => (onclose(), noter.region(picked))}>
    <SquareDashed size={15} />
    <span>{picked.length ? `A region around the ${picked.length.toLocaleString()} rooms picked` : 'A region'}<span class="sub">{picked.length ? 'A name drawn around them' : 'Pick its rooms with Select (V) first'}</span></span>
  </button>
</div>
{#if notes.length}
  <div class="menu-sep"></div>
  <div class="seg" role="radiogroup" aria-label="Which notes">
    <button type="button" role="radio" aria-checked={only === 'todo'} class:on={only === 'todo'} onclick={() => (only = 'todo')}>To do <b class="tnum">{noter.todos}</b></button>
    <button type="button" role="radio" aria-checked={only === 'all'} class:on={only === 'all'} onclick={() => (only = 'all')}>All <b class="tnum">{notes.length}</b></button>
  </div>
  <ul class="list">
    {#each shown as n (n.id)}
      <li class:done={n.done}>
        {#if n.todo}
          <input type="checkbox" checked={!!n.done} aria-label="Done: {n.text}" onchange={(e) => noter.tick(n.id, e.currentTarget.checked)} />
        {:else}
          <span class="dot" style:--c="var(--{colourOf(n)})">
            {#if n.kind === 'label'}<Tag size={11} />{:else if n.kind === 'region'}<SquareDashed size={11} />{:else}<StickyNote size={11} />{/if}
          </span>
        {/if}
        <button type="button" class="row" onclick={() => open(n)}>
          <span class="text">{n.text}</span>
          <span class="sub">{n.kind === 'region' ? `Region of ${n.rooms.length.toLocaleString()} ${n.rooms.length === 1 ? 'room' : 'rooms'}` : `${n.kind === 'label' ? 'Label on' : n.todo ? (n.done ? 'Done on' : 'To do on') : 'On'} ${host.called(n.rooms[0]!)}`}</span>
        </button>
      </li>
    {:else}
      <li class="none faint">Nothing left to do on map {noter.map}.</li>
    {/each}
  </ul>
{:else}
  <p class="lay faint">No notes on map {noter.map} yet. Select a room and write one here<span class="keys">, or press <span class="kbd">N</span></span>.</p>
{/if}

<style>
  .head { padding: 8px 10px 2px; }
  .lay { margin: 2px 10px 6px; font-size: 12px; line-height: 1.35; color: var(--ink-2); }
  .adds :global(svg) { flex: none; color: var(--ink-2); }
  .seg { display: flex; gap: 2px; padding: 3px; margin: 2px 6px 6px; border-radius: 12px; background: var(--surface-2); border: 1px solid var(--line); }
  .seg button { flex: 1; gap: 6px; min-height: 28px; padding: 0 10px; border: 0; border-radius: 9px; background: transparent; font-size: 12.5px; color: var(--ink-2); box-shadow: none; }
  .seg button:hover:not(.on) { background: var(--surface-3); }
  .seg button.on { background: var(--lilac-2); color: var(--lilac-ink); box-shadow: 0 0 0 1px var(--line-2); }
  .seg b { font-size: 11.5px; opacity: 0.8; }
  .list { list-style: none; margin: 0; padding: 0 2px 4px; max-height: min(46vh, 380px); overflow: auto; }
  .list li { display: flex; align-items: flex-start; gap: 8px; padding: 2px 4px 2px 8px; border-radius: 10px; }
  .list li:hover { background: var(--surface-2); }
  .list input { margin-top: 9px; flex: none; }
  .dot {
    flex: none; margin-top: 7px; width: 18px; height: 18px; border-radius: 6px; display: grid; place-items: center;
    background: var(--c); color: #241d45; border: 1px solid color-mix(in srgb, var(--outline) 60%, transparent);
  }
  .row {
    flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; align-items: flex-start; gap: 1px;
    padding: 6px 4px; border: 0; background: transparent; box-shadow: none; text-align: left; white-space: normal;
  }
  .row:hover:not(:disabled) { background: transparent; }
  .row .text { font-size: 13px; font-weight: 700; line-height: 1.3; color: var(--ink); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere; }
  .row .sub { font-size: 11.5px; font-weight: 600; color: var(--ink-3); }
  li.done .text { text-decoration: line-through; color: var(--ink-3); }
  .none { padding: 8px 10px; font-size: 12.5px; font-weight: 700; }
  @media (pointer: coarse) {
    .end, .keys { display: none; }
  }
</style>
