<!--
  A note being written, beside its room on the map (a sheet along the bottom of a small screen):
  a note's words, whether it is a thing to do and done; a label's or a region's words; its
  colour; and for a region, its rooms, which the rooms picked with Select can take the place of.
  Enter (Ctrl+Enter in a note) keeps it, Esc puts it away unchanged, and no words take it away.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import X from '@lucide/svelte/icons/x';
  import { onMount } from 'svelte';
  import { NOTE_COLOURS, type NoteColour } from './model';
  import type { Noter } from './noter.svelte';
  import { colourOf, KIND_COLOUR } from './notes';

  let {
    noter,
    picked = 0,
    onpicked,
  }: {
    noter: Noter;
    /** How many rooms Select has picked, and their region taken from them. */
    picked?: number;
    onpicked?: () => void;
  } = $props();

  const d = $derived(noter.draft!);
  const host = $derived(noter.host);
  const MAX = { note: 2000, label: 80, region: 80 } as const;
  const HEAD = { note: 'Note on', label: 'Label on', region: 'Region' } as const;
  const COLOUR_WORD: Record<NoteColour, string> = { butter: 'Butter', pink: 'Pink', peach: 'Peach', mint: 'Mint', sky: 'Sky', lilac: 'Lilac' };

  let field = $state<HTMLInputElement | HTMLTextAreaElement>();
  let high = $state(0);

  /** Where it goes: beside its room, clear of the room card, on the map. */
  const at = $derived.by(() => {
    void noter.moved;
    if (host.coarse || !d) return null;
    const s = host.scene.roomOnScreen(noter.roomOf(d));
    const cv = host.canvas;
    const W = cv.clientWidth - host.card;
    const H = cv.clientHeight;
    const w = 300;
    if (!s) return { x: Math.max(12, (W - w) / 2), y: 80 };
    const right = s.x + s.half + 18;
    const x = right + w <= W - 12 ? right : Math.max(12, s.x - s.half - 18 - w);
    // its top level with the room's, below the selected room's name
    const y = Math.min(Math.max(70, s.y - s.half - 2), Math.max(70, H - high - 70));
    return { x, y };
  });

  onMount(() => {
    field?.focus();
    field?.select();
  });

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      noter.finish(false);
      host.canvas.focus({ preventScroll: true });
    } else if (e.key === 'Enter' && (d.kind !== 'note' || e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      done();
    }
  }

  function done() {
    noter.finish(true);
    host.canvas.focus({ preventScroll: true });
  }

  const where = $derived(d.kind === 'region' ? `${d.rooms.length.toLocaleString()} ${d.rooms.length === 1 ? 'room' : 'rooms'}` : host.called(d.rooms[0]!));
</script>

{#if d}
  <div
    bind:offsetHeight={high}
    class="note-edit"
    class:sheet={!at}
    style:left={at ? `${at.x}px` : null}
    style:top={at ? `${at.y}px` : null}
    role="dialog"
    aria-label="{d.fresh ? 'New' : 'Change the'} {d.kind} {d.kind === 'region' ? `of ${where}` : `on ${where}`}"
    tabindex="-1"
    onkeydown={keys}
  >
    <header>
      <span class="eyebrow">{d.fresh ? 'New ' : ''}{d.fresh ? HEAD[d.kind].toLowerCase() : HEAD[d.kind]} <b>{where}</b></span>
      <button type="button" class="btn icon ghost sm" aria-label="Put it away unchanged (Esc)" title="Put it away unchanged (Esc)" onclick={() => noter.finish(false)}><X size={14} /></button>
    </header>
    {#if d.kind === 'note'}
      <textarea
        bind:this={field}
        bind:value={d.text}
        rows="3"
        maxlength={MAX.note}
        placeholder="What to remember here: a thing to do, an idea, a question"
        aria-label="The note"
      ></textarea>
      <div class="checks">
        <label><input type="checkbox" bind:checked={d.todo} onchange={() => !d.todo && (d.done = false)} />A thing to do</label>
        {#if d.todo}<label><input type="checkbox" bind:checked={d.done} />Done</label>{/if}
      </div>
    {:else}
      <input
        bind:this={field}
        bind:value={d.text}
        maxlength={MAX[d.kind]}
        spellcheck="true"
        autocomplete="off"
        placeholder={d.kind === 'label' ? 'Words drawn by the room, like “North gate”' : 'Its name, like “Thieves’ quarter”'}
        aria-label={d.kind === 'label' ? 'The label' : "The region's name"}
      />
      {#if d.kind === 'region' && picked && picked !== d.rooms.length && !d.fresh}
        <button type="button" class="btn sm" onclick={onpicked}>Go around the {picked.toLocaleString()} rooms picked instead</button>
      {/if}
    {/if}
    <div class="swatches" role="radiogroup" aria-label="Colour">
      {#each NOTE_COLOURS as c (c)}
        <button
          type="button"
          role="radio"
          aria-checked={colourOf(d) === c}
          aria-label={COLOUR_WORD[c]}
          title={COLOUR_WORD[c]}
          class="swatch"
          style:--c="var(--{c})"
          onclick={() => (d.colour = c === KIND_COLOUR[d.kind] ? undefined : c)}
        ></button>
      {/each}
    </div>
    <footer>
      {#if !d.fresh}
        <button type="button" class="btn sm ghost danger" onclick={() => noter.remove(d.id)}><Trash2 size={13} />Delete</button>
      {/if}
      <span class="kbd-hint faint">{d.kind === 'note' ? 'Ctrl+Enter' : 'Enter'} keeps it</span>
      <button type="button" class="btn sm primary" onclick={done}>{d.fresh ? 'Add' : 'Done'}</button>
    </footer>
  </div>
{/if}

<style>
  .note-edit {
    position: absolute; z-index: 7; width: 300px; padding: 10px 12px 12px; border-radius: 16px;
    display: flex; flex-direction: column; gap: 8px;
    background: var(--surface); border: 1.5px solid var(--outline); box-shadow: 0 3px 0 var(--shadow), var(--float);
  }
  .note-edit.sheet {
    position: fixed; left: 8px; right: 8px; bottom: calc(68px + env(safe-area-inset-bottom)); width: auto; z-index: 30;
  }
  header { display: flex; align-items: center; gap: 8px; }
  header .eyebrow { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  header .eyebrow b { color: var(--ink); letter-spacing: 0; text-transform: none; font-size: 12px; }
  textarea, input:not([type]) {
    width: 100%; box-sizing: border-box; font: inherit; font-size: 13.5px; line-height: 1.4; color: var(--ink);
    background: var(--surface-2); border: 1.5px solid var(--line-2); border-radius: 10px; padding: 7px 9px;
  }
  textarea { resize: vertical; min-height: 68px; max-height: 260px; }
  textarea:focus, input:not([type]):focus { outline: none; border-color: var(--focus); box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus) 25%, transparent); }
  .checks { display: flex; gap: 14px; flex-wrap: wrap; }
  .checks label { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; color: var(--ink-2); cursor: pointer; }
  .swatches { display: flex; gap: 6px; }
  .swatch {
    width: 24px; height: 24px; min-height: 0; padding: 0; border-radius: 50%;
    background: var(--c); border: 1.5px solid var(--outline); box-shadow: none;
  }
  .swatch[aria-checked='true'] { box-shadow: 0 0 0 2px var(--surface), 0 0 0 4px var(--focus); }
  footer { display: flex; align-items: center; gap: 8px; }
  footer .kbd-hint { flex: 1; text-align: right; font-size: 11.5px; font-weight: 700; }
  @media (pointer: coarse) {
    .kbd-hint { visibility: hidden; }
    .swatch { width: 30px; height: 30px; }
  }
</style>
