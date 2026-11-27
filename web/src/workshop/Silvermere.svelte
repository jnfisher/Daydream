<!--
  Import and export › silvermere database: the world (rooms, items, monsters, spells, classes,
  races, shops, messages, text blocks and actions) written straight into a silvermere server's
  database, or read back from it. Players are not touched. Writing into it cannot be undone
  here; reading it in is one step in Changes.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Activity from '@lucide/svelte/icons/activity';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import { onMount } from 'svelte';
  import { api, type SilvermereInfo, type SilvermereTable } from '../api';
  import { games } from '../games';
  import { kindOf } from '../shell/kinds';
  import { messageOf } from '../shell/toasts.svelte';
  import Seg from './Seg.svelte';
  import Tally from './Tally.svelte';
  import Transfer from './Transfer.svelte';

  const KEY = 'daydream.silvermere';

  type Result = { dry_run: boolean; tables: SilvermereTable[]; step?: number | null };

  let db = $state('');
  let placeholder = $state('C:\\silvermere\\data\\silvermere.db, or postgres://…');
  /** A database is known without one typed: the open game's, or the server's. */
  let known = $state(false);
  let custom = $state(true);
  let way = $state<'out' | 'in'>('out');
  let resetState = $state(false);
  let keepExtra = $state(false);
  let noRaw = $state(false);
  let info = $state.raw<SilvermereInfo | null>(null);
  let infoError = $state('');
  /** The database named is not made yet, which is no error: writing into it makes it. */
  let infoNew = $state(false);
  let checking = $state(false);

  onMount(() => {
    try {
      db = localStorage.getItem(KEY) ?? '';
    } catch {
      // a private window
    }
    Promise.all([api.silvermere(), games.list()])
      .then(([c, l]) => {
        custom = c.custom;
        if (l.current?.kind === 'silvermere') {
          // the open game is this database: what was typed for another is not meant now
          db = '';
          placeholder = `${l.current.location} (the open game’s)`;
          known = true;
        } else if (c.database) {
          placeholder = `${c.database} (the server’s)`;
          known = true;
        }
        if (!c.custom) {
          db = '';
          if (!known) placeholder = 'Open a silvermere game first (typing a database needs --allow-any-source)';
        }
      })
      .catch(() => {});
  });

  /** The database named, remembered for next time; null for the known one. */
  function database(): string | null {
    const d = db.trim();
    if (d) {
      try {
        localStorage.setItem(KEY, d);
      } catch {
        // a private window
      }
    }
    return d || null;
  }

  async function check() {
    checking = true;
    infoError = '';
    infoNew = false;
    try {
      info = await api.silvermereInfo(database());
    } catch (e) {
      info = null;
      const m = messageOf(e);
      if (m.endsWith('no such database file')) infoNew = true;
      else infoError = m;
    } finally {
      checking = false;
    }
  }

  const blocked = $derived(db.trim() || known ? '' : 'Give silvermere’s database: its SQLite file, or a URL.');
  const named = (t: string) => kindOf(t.replace(/_/g, '-')).many;
  const moving = (r: Result) => r.tables.reduce((n, t) => n + t.inserted + t.updated + t.deleted, 0);
  const records = (n: number, what = 'record') => `${n.toLocaleString()} ${n === 1 ? what : `${what}s`}`;
  const where = $derived(db.trim() || placeholder.replace(/ \(.*\)$/, ''));
  const options = $derived(way === 'out' ? { reset_state: resetState, keep_extra: keepExtra, no_raw: noRaw } : { keep_extra: keepExtra });
  const key = $derived(JSON.stringify([db.trim(), way, options]));
</script>

<div class="ws-form">
  <label class="entry">
    <span class="lbl">silvermere’s database</span>
    <input class="path" bind:value={db} {placeholder} disabled={!custom} spellcheck="false" autocomplete="off" />
    <small>Its SQLite file (on the machine Daydream runs on), or a URL.</small>
  </label>

  <div class="status">
    <button type="button" class="btn sm" disabled={checking || !!blocked} onclick={() => void check()}>
      {#if checking}<LoaderCircle size={14} />{:else}<Activity size={14} />{/if}Is it running?
    </button>
    {#if infoError}
      <span class="bad">{infoError}</span>
    {:else if infoNew}
      <span class="faint">There is no database there yet: writing the world into silvermere makes one.</span>
    {:else if info}
      <span class={info.running ? 'warn' : 'ok'}>
        {#if info.running}<TriangleAlert size={14} />{:else}<CircleCheck size={14} />{/if}
        {info.running ? `silvermere is running (${info.running}): stop it before writing or reading.` : 'silvermere is not running.'}
      </span>
      <span class="faint holds">
        {info.tables.map(([t, n]) => `${named(t)} ${n.toLocaleString()}`).join(' · ')}{info.raw ? '' : ' · no record bytes from Daydream'}
      </span>
    {/if}
  </div>

  <div class="row">
    <span class="lbl">Which way</span>
    <Seg label="Which way" options={[['out', 'Write the world into silvermere'], ['in', 'Read the world from silvermere']]} bind:value={way} />
  </div>
  <div class="checks">
    {#if way === 'out'}
      <label class="check"><input type="checkbox" bind:checked={resetState} /> Rewrite the live state silvermere keeps (room floors, shop stock, kill timers) from the world</label>
      <label class="check"><input type="checkbox" bind:checked={keepExtra} /> Keep what silvermere has that the world does not, instead of deleting it</label>
      <label class="check"><input type="checkbox" bind:checked={noRaw} /> Leave out the records’ bytes (what silvermere does not store is then lost on the way back)</label>
    {:else}
      <label class="check"><input type="checkbox" bind:checked={keepExtra} /> Keep the records the world has that silvermere does not, instead of deleting them</label>
    {/if}
  </div>
  <p class="hint">Players are not touched. Stop silvermere first: Daydream takes its writer lock, and will not go on while silvermere runs.</p>
</div>

{#key way}
  <Transfer
    run={(dry): Promise<Result> =>
      way === 'out'
        ? api.silvermereExport({ database: database(), reset_state: resetState, keep_extra: keepExtra, no_raw: noRaw }, dry)
        : api.silvermereImport({ database: database(), keep_extra: keepExtra }, dry)}
    {key}
    {blocked}
    way={way === 'out' ? 'out' : 'in'}
    verb={way === 'out' ? 'Write into silvermere' : 'Read it in'}
    act={(r: Result) => (moving(r) ? (way === 'out' ? `Write ${records(moving(r), 'row')} into silvermere` : `Read ${records(moving(r))} in`) : '')}
    done={(r: Result) => (way === 'out' ? `Wrote ${records(moving(r), 'row')} into ${where}.` : `Read ${records(moving(r))} in from ${where}.`)}
  >
    {#snippet result(r, dry)}
      {#if !moving(r)}
        <p class="said">{way === 'out' ? 'silvermere has the world as it is: nothing to write.' : 'The world matches silvermere: nothing to read in.'}</p>
      {:else}
        <p class="said">{records(moving(r), way === 'out' ? 'row' : 'record')} {dry ? 'would change' : 'changed'}.</p>
      {/if}
      {#if way === 'out'}
        <Tally heads={['Table', 'New', 'Rewritten', 'Deleted']} rows={r.tables.map((t) => [named(t.table), t.inserted, t.updated, t.deleted])} />
      {:else}
        <Tally heads={['Table', 'New', 'Changed', 'Same', 'Deleted']} rows={r.tables.map((t) => [named(t.table), t.inserted, t.updated, t.unchanged, t.deleted])} />
      {/if}
    {/snippet}
  </Transfer>
{/key}

<style>
  .ws-form { display: flex; flex-direction: column; gap: 12px; }
  .entry { display: grid; grid-template-columns: 11em minmax(0, 1fr); align-items: center; gap: 4px 10px; }
  .entry small { grid-column: 2; font-size: 12px; font-weight: 700; color: var(--ink-3); }
  .row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 11em; }
  .path { width: 100%; max-width: 560px; font-family: var(--font-mono); }
  .status { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; font-size: 12.5px; font-weight: 700; }
  .status > span { display: inline-flex; align-items: center; gap: 5px; }
  .status .ok { color: var(--ok); }
  .status .warn { color: var(--warn); }
  .status .bad { color: var(--bad); }
  .holds { flex-basis: 100%; }
  .checks { display: flex; flex-direction: column; gap: 6px; }
  .check { display: inline-flex; align-items: flex-start; gap: 6px; font-weight: 700; font-size: 13px; max-width: 80ch; }
  .check input { flex: none; margin-top: 2px; }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 75ch; }
  .said { margin: 0; font-weight: 700; color: var(--ink-2); }
  @media (max-width: 560px) {
    .entry { grid-template-columns: minmax(0, 1fr); }
    .entry small { grid-column: 1; }
    .lbl { flex-basis: 100%; }
  }
</style>
