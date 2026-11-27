<!--
  Advanced › Delete records: a run of a table's records, or all of them, previewed record by
  record. One step of the history, so it can be taken back until you publish.
-->
<svelte:options css="injected" />

<script lang="ts">
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import { api, type Cell, type Plan } from '../api';
  import RefChip from '../library/RefChip.svelte';
  import { kindOf } from '../shell/kinds';
  import { app, rowsOf } from '../shell/state.svelte';
  import { rangeOf, RUN_TABLES, type Span } from './fields';
  import Run from './Run.svelte';
  import SpanPick from './SpanPick.svelte';

  const TABLES = [...RUN_TABLES, 'Actions'];
  const FIRST = 60;

  let table = $state('Items');
  let span = $state<Span>({ all: false, map: '', from: '', to: '' });
  let shown = $state(FIRST);
  let keys = $state.raw<Cell[][]>([]);

  const tables = $derived(TABLES.filter((n) => app.tables.some((x) => x.name === n)));
  // actions are named, not numbered: all of them or none
  const named = $derived(table === 'Actions');
  const range = $derived(named ? { range: null } : rangeOf(table, span));
  const k = $derived(kindOf(table));

  const blocked = $derived('error' in range ? range.error : '');
  const body = $derived({ table, range: 'range' in range ? range.range : null, all: named || span.all });

  async function run(dry: boolean): Promise<Plan> {
    const r = await api.deleteRecords(body, dry);
    if (dry) {
      // the records that would go, to list them
      const rows = await rowsOf(table).catch(() => [] as [Cell[], Cell][]);
      const q = body.range;
      keys = rows
        .map(([key]) => key)
        .filter((key) => !q || (q.map === undefined ? inSpan(key[0], q) : key[0] === q.map && inSpan(key[1], q)));
      shown = FIRST;
    }
    return { dry_run: r.dry_run, table, changed: r.report.deleted, records: r.report.deleted, skipped: 0, unchanged: 0, notes: r.report.errors, changes: [], report: r.report, step: r.step };
  }

  const inSpan = (v: Cell, q: { from: number; to: number }) => typeof v === 'number' && v >= q.from && v <= q.to;
</script>

<div class="ws-form">
  <div class="row">
    <span class="lbl">Delete from</span>
    <select bind:value={table} aria-label="Table">
      {#each tables as n (n)}<option value={n}>{kindOf(n).many}</option>{/each}
    </select>
  </div>
  {#if named}
    <p class="hint">Actions are named, not numbered: this deletes every one.</p>
  {:else}
    <SpanPick bind:span {table} label="Which {k.many.toLowerCase()}" />
  {/if}
  <div class="callout warn">
    <TriangleAlert size={16} />
    <span>Records that name the ones deleted keep naming them. Look at what points at a record first: its card shows where it is used.</span>
  </div>
</div>

<Run {run} key={JSON.stringify(body)} {blocked} verb="Delete" changing="going" danger done={(n) => `Deleted ${n.toLocaleString()} ${(n === 1 ? k.one : k.many).toLowerCase()}.`}>
  {#snippet list(plan)}
    {#if plan.records}
      <div class="chips">
        {#each keys.slice(0, shown) as key (key.join('/'))}<RefChip {table} {key} compact />{/each}
      </div>
      {#if keys.length > shown}
        <div class="more"><button type="button" class="btn sm" onclick={() => (shown += 200)}>Show {Math.min(200, keys.length - shown).toLocaleString()} more</button></div>
      {/if}
    {/if}
  {/snippet}
</Run>

<style>
  .ws-form { display: flex; flex-direction: column; gap: 10px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  @media (max-width: 560px) { .lbl { flex-basis: 100%; } }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; max-height: 320px; overflow: auto; }
  .more { display: flex; justify-content: flex-end; }
</style>
