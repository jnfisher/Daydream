<!--
  One group of a record card: its fields (each with what it was when last published, struck
  through, and where a number stands among its peers'), a family of numbered slots, a small
  table, lines of game text, or where the record is used.
-->
<svelte:options css="injected" />

<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Info from '@lucide/svelte/icons/info';
  import type { Cell } from '../api';
  import { noteOf, pickFor, targetOf } from '../layouts';
  import CardGrid from './CardGrid.svelte';
  import { shown, type Draft } from './draft.svelte';
  import FieldInput from './FieldInput.svelte';
  import GameText from './GameText.svelte';
  import type { ListRow } from './list';
  import { columnsOf, type Section } from './sections';
  import Strip from './Strip.svelte';
  import Where from './Where.svelte';

  let {
    draft,
    section,
    table,
    rowKey,
    version,
    peers,
    peerRows,
    peersSay,
  }: {
    draft: Draft;
    section: Section;
    table: string;
    rowKey: Cell[];
    /** Bumped by every write, so where the record is used is read again. */
    version: number;
    /** The peers' values of a column, for its strip. */
    peers: (col: string) => number[] | null;
    /** The peers themselves, for a scatter. */
    peerRows: ListRow[];
    peersSay: string;
  } = $props();

  let open = $state(false);
  $effect(() => {
    void section.id;
    open = !section.folded;
  });
  // a problem's Show opens the section that holds its field
  $effect(() => {
    const reveal = (e: Event) => {
      if (!open && columnsOf(section).includes((e as CustomEvent<string>).detail)) open = true;
    };
    window.addEventListener('dd:reveal', reveal);
    return () => window.removeEventListener('dd:reveal', reveal);
  });

  // the parts only some records need come in the first time one is shown
  let parts = $state<typeof import('./parts') | null>(null);
  const scatter = $derived(open && section.scatter && peerRows.length ? section.scatter : null);
  $effect(() => {
    if ((section.custom || scatter) && !parts) void import('./parts').then((m) => (parts = m));
  });

  const fields = $derived((section.fields ?? []).filter((f) => draft.has(f.col)));
  const strips = $derived(fields.some((f) => f.strip));
  const own = $derived(Number(draft.get('Map Number')) || 0);
  const labelOf = (col: string, label?: string) => (label === undefined || label === '' ? col : label);
</script>

<section class="card csec" class:side={section.side} aria-label={section.title}>
  <header class="csec-h">
    {#if section.folded}
      <button type="button" class="fold" aria-expanded={open} onclick={() => (open = !open)}>
        <h3>{section.title}</h3>
        <ChevronDown size={16} class={open ? 'turned' : ''} />
      </button>
    {:else}
      <h3>{section.title}</h3>
    {/if}
    {#if strips && peersSay && open}
      <span class="hint">Dots: {peersSay}</span>
    {:else if section.hint}
      <span class="hint">{section.hint}</span>
    {/if}
  </header>
  {#if open && section.custom}
    {#if !parts}
      <p class="faint">Getting it ready…</p>
    {:else if section.custom === 'attacks'}
      <parts.Attacks {draft} />
    {:else if section.custom === 'wired'}
      <parts.Wired {draft} />
    {:else if section.custom === 'script'}
      <parts.Script {draft} {rowKey} {version} />
    {:else if section.custom === 'typed'}
      <parts.Typed {draft} {rowKey} />
    {:else if section.custom === 'reads'}
      <parts.Reads {draft} {rowKey} {version} />
    {:else}
      <parts.Spawns {draft} {rowKey} {version} />
    {/if}
  {:else if open}
    {#if section.text}
      <GameText {draft} cols={section.text.cols} label={section.text.label} />
    {/if}
    {#if fields.length}
      <div class="frows">
        {#each fields as f (f.col)}
          {@const was = draft.was(f.col)}
          {@const wasWith = f.with ? draft.was(f.with) : undefined}
          {@const note = noteOf(table, f.col)}
          {@const values = f.strip ? peers(f.col) : null}
          <div class="frow">
            <span class="flabel" title={f.col}>
              {labelOf(f.col, f.label)}
              {#if note}<span class="i" title={note}><Info size={12} /></span>{/if}
            </span>
            <span class="fctl">
              <FieldInput {draft} col={f.col} pick={f.pick} target={targetOf(table, f.col)} map={own} label={labelOf(f.col, f.label)} />
              {#if f.with && draft.has(f.with)}
                <FieldInput {draft} col={f.with} pick={pickFor(table, f.with)} label={f.with} />
              {/if}
              {#if was !== undefined || wasWith !== undefined}
                <s class="was" title="What it was when the world was last published">{[was, wasWith].filter((v) => v !== undefined).map(shown).join(' ')}</s>
              {/if}
              {#if f.note}<span class="note">{f.note}</span>{/if}
            </span>
            {#if values && values.length}
              <Strip peers={values} value={Number(draft.get(f.col)) || 0} label={labelOf(f.col, f.label)} says={peersSay} />
            {/if}
          </div>
        {/each}
      </div>
    {/if}
    {#if scatter && parts}
      <parts.Scatter {draft} {table} rows={peerRows} x={scatter.x} y={scatter.y} xLabel={scatter.xLabel} yLabel={scatter.yLabel} note={scatter.note} says={peersSay} />
    {/if}
    {#if section.matrix}
      <table class="matrix-t">
        <thead><tr><th></th>{#each section.matrix.headers as h (h)}<th>{h}</th>{/each}</tr></thead>
        <tbody>
          {#each section.matrix.rows as r (r.label)}
            <tr>
              <td class="n">{r.label}</td>
              {#each r.cols as col (col)}
                <td>
                  {#if draft.has(col)}<FieldInput {draft} {col} label="{r.label} {col}" compact />{/if}
                  {#if draft.was(col) !== undefined}<s class="was">{shown(draft.was(col))}</s>{/if}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if section.grid}
      <CardGrid {draft} grid={section.grid} {table} />
    {/if}
    {#if section.where}
      <Where {table} {rowKey} {version} />
    {/if}
  {/if}
</section>

<style>
  .csec { display: grid; gap: 10px; align-content: start; container-type: inline-size; }
  .csec-h { display: flex; align-items: baseline; justify-content: space-between; gap: 10px; flex-wrap: wrap; }
  .csec-h h3 { margin: 0; font-size: 16.5px; }
  .hint { color: var(--ink-3); font-size: 12px; font-weight: 700; }
  .fold { display: inline-flex; align-items: center; gap: 6px; border: 0; background: none; padding: 0; min-height: 0; box-shadow: none; }
  .fold:hover:not(:disabled) { background: none; }
  .fold :global(.turned) { transform: rotate(180deg); }
  .frows { display: grid; }
  .frow { display: grid; grid-template-columns: 9.5rem minmax(0, 1fr) auto; align-items: center; gap: 4px 12px; padding: 6px 0; border-bottom: 1px dashed var(--line); }
  .side .frow { grid-template-columns: 7.5rem minmax(0, 1fr) auto; }
  .frow:last-child { border-bottom: 0; }
  .flabel { color: var(--ink-2); font-size: 13px; font-weight: 800; display: inline-flex; align-items: center; gap: 4px; }
  .i { color: var(--ink-3); display: inline-flex; cursor: help; }
  .fctl { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; min-width: 0; }
  .was { color: var(--ink-3); font: 12px var(--font-mono); }
  .note { color: var(--ink-3); font-size: 12px; font-weight: 700; }
  .matrix-t { border-collapse: collapse; }
  .matrix-t th { text-align: left; font: 800 11px/1.2 var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: var(--ink-3); padding: 0 8px 6px 0; }
  .matrix-t td { padding: 3px 8px 3px 0; }
  .matrix-t td.n { color: var(--ink-2); font-weight: 800; font-size: 13px; }
  /* a narrow card: the strip under the value, then the label over it */
  @container (max-width: 470px) {
    .frow { grid-template-columns: 8.5rem minmax(0, 1fr); }
    .side .frow { grid-template-columns: 7.5rem minmax(0, 1fr); }
    .frow :global(.strip) { grid-column: 2; }
  }
  @container (max-width: 330px) {
    .frow, .side .frow { grid-template-columns: minmax(0, 1fr); gap: 4px; }
    .frow :global(.strip) { grid-column: 1; }
  }
</style>
