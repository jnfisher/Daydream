<!-- Which records a run covers: every one, or a run of numbers (on a map, for rooms). -->
<svelte:options css="injected" />

<script lang="ts">
  import type { Span } from './fields';
  import Seg from './Seg.svelte';

  let { span = $bindable(), table, label = 'Which records' }: { span: Span; table: string; label?: string } = $props();

  let which = $state<'all' | 'some'>(span.all ? 'all' : 'some');
  $effect(() => {
    span.all = which === 'all';
  });
</script>

<div class="row">
  <span class="lbl">{label}</span>
  <Seg {label} options={[['all', 'All of them'], ['some', 'A run of numbers']]} bind:value={which} />
  {#if which === 'some'}
    <span class="span">
      {#if table === 'Rooms'}<label>Map <input class="num" inputmode="numeric" bind:value={span.map} /></label>{/if}
      <label>From <input class="num" inputmode="numeric" bind:value={span.from} /></label>
      <label>to <input class="num" inputmode="numeric" bind:value={span.to} /></label>
    </span>
  {/if}
</div>

<style>
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  @media (max-width: 560px) { .lbl { flex-basis: 100%; } }
  .span { display: inline-flex; gap: 8px; flex-wrap: wrap; }
  .span label { display: inline-flex; align-items: center; gap: 6px; font-weight: 800; font-size: 12.5px; color: var(--ink-2); }
  .num { width: 6.5em; font-family: var(--font-mono); }
</style>
