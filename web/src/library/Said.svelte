<!--
  Words in which records are chips (script.ts's Bit): a step of a script, what an exit does.
  An ability is its name, money its coins. A room on the player's own map is a chip when the
  map is known.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { ABILITIES } from '../abilities';
  import { kindOf } from '../shell/kinds';
  import { refKey } from './problems';
  import RefChip from './RefChip.svelte';
  import { coinsOf, type Bit } from './script';

  let { bits, map }: { bits: Bit[]; map?: number } = $props();
</script>

<!-- one line, so no space comes between a word and a chip that the words do not have -->
{#each bits as b, i (i)}{#if typeof b === 'string'}{b}{:else if 'ref' in b}{#if b.ref.n <= 0}<span class="none">{kindOf(b.ref.t).one.toLowerCase()} {b.ref.n}</span>{:else if b.ref.t === 'Rooms' && b.ref.map === undefined && map === undefined}<span class="none" title="A room on the map the player is on">room {b.ref.n}</span>{:else}<RefChip table={b.ref.t} key={refKey(b.ref.t, b.ref.n, b.ref.map ?? map)} compact />{/if}{:else if 'ability' in b}<span class="ab" title={ABILITIES[b.ability]?.[1] ?? ''}>{ABILITIES[b.ability]?.[0] ?? `ability ${b.ability}`}</span>{:else}<b class="coin" title="{b.copper.toLocaleString()} copper in all, paid from any coins">{coinsOf(b.copper)}</b>{/if}{/each}

<style>
  .coin { color: var(--butter-ink); }
  .none { font: 600 12px var(--font-mono); color: var(--ink-3); }
  .ab {
    display: inline-flex; align-items: center; min-height: 24px; padding: 0 9px; border-radius: 999px;
    background: var(--surface); border: 1px solid var(--line); font-size: 12px; font-weight: 800; color: var(--ink-2); vertical-align: middle;
  }
</style>
