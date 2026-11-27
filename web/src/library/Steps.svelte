<!--
  A line's steps in words, each with an icon for its kind (script.ts): the same clause written
  again and again is one step with how many times.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Ban from '@lucide/svelte/icons/ban';
  import Dices from '@lucide/svelte/icons/dices';
  import Footprints from '@lucide/svelte/icons/footprints';
  import Funnel from '@lucide/svelte/icons/funnel';
  import Gift from '@lucide/svelte/icons/gift';
  import HandCoins from '@lucide/svelte/icons/hand-coins';
  import MessageSquare from '@lucide/svelte/icons/message-square';
  import PackageMinus from '@lucide/svelte/icons/package-minus';
  import Zap from '@lucide/svelte/icons/zap';
  import type { Component } from 'svelte';
  import Said from './Said.svelte';
  import type { Step, StepKind } from './script';

  let { steps, map }: { steps: Step[]; map?: number } = $props();

  const ICONS: Record<StepKind, [Component<{ size?: number | string; class?: string }>, string]> = {
    test: [Funnel, 't-other'],
    pay: [HandCoins, 't-item'],
    take: [PackageMinus, 't-monster'],
    give: [Gift, 't-room'],
    say: [MessageSquare, 't-msg'],
    move: [Footprints, 't-shop'],
    run: [Dices, 't-text'],
    other: [Zap, 't-spell'],
    skip: [Ban, 't-other'],
  };

  const runs = $derived.by(() => {
    const out: { s: Step; n: number }[] = [];
    for (const s of steps) {
      const last = out.at(-1);
      if (last && last.s.clause.text.trim() === s.clause.text.trim()) last.n++;
      else out.push({ s, n: 1 });
    }
    return out;
  });
</script>

<ol class="steps">
  {#each runs as { s, n }, i (i)}
    {@const [Icon, cls] = ICONS[s.kind]}
    <li class="step" class:skip={s.kind === 'skip'} title={s.clause.text.trim()}>
      <span class="gi {cls}"><Icon size={14} /></span>
      <span class="said"><Said bits={s.bits} {map} />{#if n > 1}<b class="times"> ×{n}</b>{/if}</span>
    </li>
  {/each}
</ol>

<style>
  .steps { list-style: none; margin: 0; padding: 0; display: grid; gap: 5px; }
  .step { display: grid; grid-template-columns: 24px minmax(0, 1fr); gap: 8px; align-items: start; }
  .gi { width: 24px; height: 24px; border-radius: 8px; display: grid; place-items: center; background: var(--t-bg); color: var(--t-ink); }
  .said { font-size: 13px; font-weight: 700; line-height: 26px; color: var(--ink); overflow-wrap: anywhere; }
  .step.skip .said { color: var(--ink-3); font-style: italic; }
  .times { color: var(--ink-3); }
</style>
