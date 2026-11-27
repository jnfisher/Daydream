<!--
  Lines of game text as a player sees them, on the game's black screen in its colours, and
  edited line by line (escapes typed as ␛). Long text (a text block) is one box.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Pencil from '@lucide/svelte/icons/pencil';
  import { renderAnsi } from '../ansi';
  import type { Draft } from './draft.svelte';

  let { draft, cols, label }: { draft: Draft; cols: string[]; label?: string } = $props();

  const present = $derived(cols.filter((c) => draft.has(c)));
  const edited = $derived(present.some((c) => draft.edited(c)));
  let editing = $state(false);
  // another record: its text shows as players see it
  $effect(() => {
    void draft;
    editing = false;
  });
  const open = $derived(editing || edited);
  const memo = $derived(present.length === 1 && draft.column(present[0]!)?.type === 'memo');
  const blank = $derived(present.every((c) => !draft.typed(c).trim()));

  function terminal(node: HTMLElement, text: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(text);
    return { update: draw };
  }

  /** The lines as one text, without the blank lines after the last. */
  const text = $derived.by(() => {
    const lines = present.map((c) => draft.typed(c));
    while (lines.length && !lines.at(-1)!.trim()) lines.pop();
    return lines.join('\n');
  });

  /** The widest line, in characters: the screen's font shrinks so it fits unwrapped. */
  const widest = $derived(Math.max(30, ...text.split('\n').map((l) => l.replace(/[\x1b␛]\[[0-9;]*[A-Za-z]?/g, '').length)));
</script>

<div class="game-text">
  {#if label}<div class="eyebrow cmd">{label}</div>{/if}
  {#if blank && !open}
    <p class="faint none">No text yet.</p>
  {:else}
    <pre class="terminal screen" class:memo style:--cols={widest} use:terminal={text}></pre>
  {/if}
  {#if open}
    <div class="lines">
      {#each present as c (c)}
        {#if memo}
          <textarea rows="10" spellcheck="false" class:changed={draft.edited(c)} aria-label={c} value={draft.typed(c)} oninput={(e) => draft.set(c, e.currentTarget.value)}></textarea>
        {:else}
          <input
            type="text"
            spellcheck="false"
            class:changed={draft.edited(c)}
            aria-label={c}
            maxlength={draft.column(c)?.size ?? undefined}
            value={draft.typed(c)}
            oninput={(e) => draft.set(c, e.currentTarget.value)}
          />
        {/if}
      {/each}
    </div>
  {:else}
    <button type="button" class="btn sm ghost edit" onclick={() => (editing = true)}><Pencil size={13} /> Edit the text</button>
  {/if}
</div>

<style>
  .game-text { display: grid; gap: 8px; min-width: 0; container-type: inline-size; }
  .cmd { margin-bottom: -2px; }
  /* lines are as the game wraps them: never wrapped again, the font shrinking to fit them */
  .screen {
    width: 100%; min-width: 0; max-width: 100%; margin: 0; white-space: pre; overflow-x: auto;
    font-size: clamp(10px, calc((100cqw - 34px) / (var(--cols) * .61)), 14px);
  }
  .screen.memo { white-space: pre-wrap; font-size: 13px; }
  .lines { display: grid; gap: 4px; }
  .lines :is(input, textarea) { width: 100%; font: 13px/1.35 var(--font-mono); }
  .edit { justify-self: start; }
  .none { margin: 0; }
</style>
