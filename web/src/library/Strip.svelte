<!--
  A number among its peers': a tick for each of theirs, a dot for this one, so a weapon's max
  hit reads against every weapon like it.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { standing, standingWords } from './sections';

  let { peers, value, label, says }: { peers: number[]; value: number; label: string; says: string } = $props();

  const W = 200;
  const s = $derived(standing(peers, value));
  const x = (v: number) => (s.max === s.min ? W / 2 : 4 + ((v - s.min) / (s.max - s.min)) * (W - 8));
  const ticks = $derived([...new Set(peers)].map(x));
</script>

{#if peers.length}
  <svg class="strip" viewBox="0 0 {W} 18" width={W} height="18" role="img" aria-label="{label} {value} is {standingWords(s)} ({says})">
    <title>{label} {value.toLocaleString()} is {standingWords(s)}: {says} ({s.min.toLocaleString()} to {s.max.toLocaleString()})</title>
    <line x1="2" x2={W - 2} y1="9" y2="9" class="base" />
    {#each ticks as t, i (i)}<line x1={t} x2={t} y1="3" y2="15" class="tick" />{/each}
    <circle cx={x(value)} cy="9" r="5" class="me" />
  </svg>
{/if}

<style>
  .strip { display: block; flex: none; overflow: visible; }
  .base { stroke: var(--line-2); stroke-width: 1; }
  .tick { stroke: var(--ink-3); stroke-width: 1.5; opacity: .55; }
  .me { fill: var(--pink); stroke: var(--surface); stroke-width: 2; }
</style>
