<!--
  Where a user is: the map and room they stand in (the room's name beside its number; the
  card's On the map button goes there), and the trail of rooms they came through, newest first.
-->
<svelte:options css="injected" />

<script lang="ts">
  import CardGrid from '../library/CardGrid.svelte';
  import type { Draft } from '../library/draft.svelte';
  import FieldInput from '../library/FieldInput.svelte';
  import type { Grid } from '../layouts';

  let { draft }: { draft: Draft } = $props();

  const map = $derived(Number(draft.get('Map Number')) || 0);
  const TRAIL: Grid = { columns: [{ template: 'Trail # Map', label: 'Map' }, { template: 'Trail # Room', label: 'Room' }], from: 0, to: 19 };
  const trail = $derived(Array.from({ length: 20 }, (_, n) => n).filter((n) => Number(draft.get(`Trail ${n} Room`)) > 0).length);
  let open = $state(false);
</script>

<div class="frows">
  <div class="frow">
    <span class="flabel">Current Map</span>
    <span class="fctl"><FieldInput {draft} col="Map Number" label="Current Map" /></span>
  </div>
  <div class="frow">
    <span class="flabel">Current Room</span>
    <span class="fctl"><FieldInput {draft} col="Room Number" target="Rooms" {map} label="Current Room" /></span>
  </div>
</div>
<button type="button" class="btn sm ghost trail" aria-expanded={open} onclick={() => (open = !open)}>
  {open ? 'Hide' : 'Show'} the trail{trail ? ` (${trail} room${trail === 1 ? '' : 's'})` : ''}
</button>
{#if open}
  <p class="faint small">The rooms they came through, as the game keeps them for Track and for moving them back.</p>
  <CardGrid {draft} grid={TRAIL} table="Users" />
{/if}

<style>
  .frows { display: grid; }
  .frow { display: grid; grid-template-columns: 7.5rem minmax(0, 1fr); align-items: center; gap: 4px 12px; padding: 6px 0; border-bottom: 1px dashed var(--line); }
  .frow:last-child { border-bottom: 0; }
  .flabel { color: var(--ink-2); font-size: 13px; font-weight: 800; }
  .fctl { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; min-width: 0; }
  .trail { justify-self: start; }
  .small { font-size: 12.5px; margin: 0; }
  @container (max-width: 330px) {
    .frow { grid-template-columns: minmax(0, 1fr); gap: 4px; }
  }
</style>
