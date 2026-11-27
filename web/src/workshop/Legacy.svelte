<!--
  A Workshop tool not rebuilt yet: today's form for it (tools.ts), in the tool's page. Its code
  loads when the page first opens one.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { app, realmChanged } from '../shell/state.svelte';
  import { messageOf } from '../shell/toasts.svelte';

  let { id }: { id: string } = $props();

  let box = $state<HTMLDivElement | undefined>();
  let error = $state('');
  let count = $state(0);

  $effect(() => {
    const el = box;
    const tool = id;
    if (!el) return;
    let gone = false;
    void import('../tools')
      .then(({ legacyTool }) => {
        if (gone) return;
        const cards = legacyTool(tool, app.tables, realmChanged);
        count = cards.length;
        el.replaceChildren(...cards);
      })
      .catch((e) => (error = messageOf(e)));
    return () => {
      gone = true;
      el.replaceChildren();
    };
  });
</script>

{#if error}<p class="bad">{error}</p>{/if}
<div class="legacy" class:solo={count === 1} bind:this={box}></div>

<style>
  .bad { color: var(--bad); font-weight: 700; }
  .legacy { display: flex; flex-direction: column; gap: 16px; }
  .legacy :global(.tool) { padding: 0; border: 0; background: none; border-radius: 0; }
  .legacy:not(.solo) :global(.tool + .tool) { border-top: 1px dashed var(--line); padding-top: 16px; }
  .legacy.solo :global(.tool > h2) { display: none; }
  .legacy :global(.tool > h2) { font: 600 17px var(--font-display); margin: 0 0 4px; }
  .legacy :global(.tool .hint) { font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 75ch; }
  .legacy :global(.tool button.primary) { border-color: var(--outline); box-shadow: var(--lift); }
  .legacy :global(table.grid) { font-size: 13px; }
  .legacy :global(.tool-result) { overflow-x: auto; }
</style>
