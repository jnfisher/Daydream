<!--
  Import and export › Nightmare Redux database: records brought in from an NMR export (an .mdb
  from NMR's Export Records, or its JSON), every table or some, each whole or a run of its
  numbers, as NMR's Database Importer does. One step in Changes.
-->
<svelte:options css="injected" />

<script lang="ts">
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import { api, type ImportNmr, type Report } from '../api';
  import { kindOf } from '../shell/kinds';
  import { rangeOf, type Span } from './fields';
  import FilePick from './FilePick.svelte';
  import Seg from './Seg.svelte';
  import Tally from './Tally.svelte';
  import Transfer from './Transfer.svelte';

  /** The tables NMR's importer lists, in its order. */
  const TABLES = ['Items', 'Monsters', 'Spells', 'Shops', 'Races', 'Classes', 'Rooms', 'Actions', 'Messages', 'Textblocks'];

  let file = $state<File | null>(null);
  let on = $state<Record<string, boolean>>(Object.fromEntries(TABLES.map((t) => [t, true])));
  let spans = $state<Record<string, Span>>(Object.fromEntries(TABLES.map((t) => [t, { all: true, map: '', from: '', to: '' }])));
  let existing = $state<'update' | 'skip'>('update');
  let rooms = $state<ImportNmr['room_parts']>('all');
  let extra = $state(true);

  const picked = $derived(TABLES.filter((t) => on[t]));
  const ranges = $derived(picked.filter((t) => t !== 'Actions').map((t) => ({ t, r: rangeOf(t, spans[t]!) })));
  const options = $derived<ImportNmr>({
    tables: picked,
    ranges: ranges.flatMap(({ r }) => ('range' in r && r.range ? [r.range] : [])),
    skip_existing: existing === 'skip',
    room_parts: rooms,
    extra_parts: extra,
  });

  const blocked = $derived.by(() => {
    if (!file) return 'Pick the NMR export to bring in.';
    if (!picked.length) return 'Pick a table to bring in.';
    const bad = ranges.find(({ r }) => 'error' in r);
    return bad && 'error' in bad.r ? `${kindOf(bad.t).many}: ${bad.r.error}` : '';
  });
  const key = $derived(JSON.stringify([file?.name, file?.size, file?.lastModified, options]));

  type Result = { dry_run: boolean; reports: Report[]; step?: number | null };
  const moved = (r: Report) => r.inserted + r.updated + r.deleted;
  const total = (res: Result) => res.reports.reduce((n, r) => n + moved(r), 0);
  const records = (n: number) => `${n.toLocaleString()} ${n === 1 ? 'record' : 'records'}`;

  function run(dry: boolean): Promise<Result> {
    return api.importNmr(file!, options, dry);
  }

  const setAll = (v: boolean) => (on = Object.fromEntries(TABLES.map((t) => [t, v])));
  /** The reports in the order the tables are picked in. */
  const inOrder = (reports: Report[]) => [...reports].sort((a, b) => TABLES.indexOf(a.table) - TABLES.indexOf(b.table));
</script>

<div class="ws-form">
  <FilePick bind:file accept=".mdb,.json" label="NMR export" hint="An .mdb from NMR's Export Records, or its JSON" />

  <div class="tables">
    <div class="row">
      <span class="lbl">Tables</span>
      <button type="button" class="btn sm" onclick={() => setAll(true)}>All</button>
      <button type="button" class="btn sm" onclick={() => setAll(false)}>None</button>
    </div>
    <div class="grid">
      {#each TABLES as t (t)}
        {@const s = spans[t]!}
        <div class="tline">
          <label class="check name"><input type="checkbox" bind:checked={on[t]} /> {kindOf(t).many}</label>
          {#if t === 'Actions'}
            <span class="faint which" class:off={!on[t]}>All of them: actions are named, not numbered</span>
          {:else}
            <span class="which" class:off={!on[t]}>
              <Seg label="Which {kindOf(t).many.toLowerCase()}" options={[['all', 'All'], ['some', 'A run']]} bind:value={() => (s.all ? 'all' : 'some'), (v) => (s.all = v === 'all')} />
              {#if !s.all}
                {#if t === 'Rooms'}<label class="inline">Map <input class="num" inputmode="numeric" bind:value={s.map} disabled={!on[t]} /></label>{/if}
                <label class="inline">From <input class="num" inputmode="numeric" bind:value={s.from} disabled={!on[t]} /></label>
                <label class="inline">to <input class="num" inputmode="numeric" bind:value={s.to} disabled={!on[t]} /></label>
              {/if}
            </span>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  <div class="row">
    <span class="lbl">Records the world has</span>
    <Seg label="Records the world has" options={[['update', 'Take the file’s'], ['skip', 'Keep the world’s']]} bind:value={existing} />
  </div>
  {#if on.Rooms}
    <div class="row">
      <span class="lbl">Of each room</span>
      <Seg label="Of each room" options={[['all', 'Everything'], ['only_items', 'Only its items'], ['not_items', 'All but its items']]} bind:value={rooms} />
    </div>
  {/if}
  {#if on.Textblocks}
    <label class="check"><input type="checkbox" bind:checked={extra} /> Text block parts past the file's last part are deleted, as NMR always does</label>
  {/if}
</div>

<Transfer
  {run}
  {key}
  {blocked}
  way="in"
  verb="Import"
  act={(r) => (total(r) ? `Import ${records(total(r))}` : '')}
  done={(r) => `Imported ${records(total(r))} from ${file?.name ?? 'the file'}.`}
>
  {#snippet result(r, dry)}
    {@const errors = r.reports.flatMap((x) => x.errors)}
    {#if total(r)}
      <p class="said">
        {records(total(r))}
        {dry ? 'would come in' : 'came in'}: {r.reports.reduce((n, x) => n + x.inserted, 0).toLocaleString()} new, {r.reports
          .reduce((n, x) => n + x.updated, 0)
          .toLocaleString()} changed{r.reports.some((x) => x.deleted) ? `, ${r.reports.reduce((n, x) => n + x.deleted, 0).toLocaleString()} deleted` : ''}.
      </p>
    {:else}
      <p class="said">The world has every record of the file as it is: nothing to bring in.</p>
    {/if}
    {@const kept = r.reports.some((x) => x.skipped) || options.skip_existing}
    {@const gone = r.reports.some((x) => x.deleted)}
    <Tally
      heads={['Table', 'New', 'Changed', 'Same', ...(kept ? ['Kept'] : []), ...(gone ? ['Deleted'] : [])]}
      rows={inOrder(r.reports).map((x) => [
        kindOf(x.table).many,
        x.inserted,
        x.updated,
        x.unchanged,
        ...(kept ? [x.skipped ?? 0] : []),
        ...(gone ? [x.deleted] : []),
      ])}
    />
    {#if errors.length}
      <div class="callout bad">
        <TriangleAlert size={16} />
        <div>
          <b>{errors.length === 1 ? 'One record' : `${errors.length.toLocaleString()} records`} could not be read:</b>
          <ul>{#each errors.slice(0, 12) as e, i (i)}<li>{e}</li>{/each}</ul>
          {#if errors.length > 12}<span>and {(errors.length - 12).toLocaleString()} more.</span>{/if}
        </div>
      </div>
    {/if}
  {/snippet}
</Transfer>

<style>
  .ws-form { display: flex; flex-direction: column; gap: 12px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 9.5em; }
  .tables { display: flex; flex-direction: column; gap: 8px; }
  /* two tables a row where there is room */
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 430px), 1fr)); gap: 6px 24px; padding: 10px 12px; border: 1px solid var(--line); border-radius: 14px; }
  .tline { display: flex; align-items: center; gap: 6px 10px; flex-wrap: wrap; min-height: 36px; }
  .name { width: 7.5em; }
  .which { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .off { opacity: .45; }
  .faint.which { font-size: 12.5px; font-weight: 700; }
  .inline { display: inline-flex; align-items: center; gap: 6px; font-weight: 800; font-size: 12.5px; color: var(--ink-2); }
  .num { width: 6.5em; font-family: var(--font-mono); }
  .check { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; font-size: 13px; }
  .check.name { font-weight: 800; }
  .said { margin: 0; font-weight: 700; color: var(--ink-2); }
  .callout ul { margin: 4px 0 0; padding-left: 18px; }
  @media (max-width: 560px) {
    .lbl { flex-basis: 100%; }
  }
</style>
