<!--
  Import and export › Export for MME: the .mdb MMUD Explorer reads, as NMR's Export to MMUD
  Explorer writes it (the same tables and values, so it shows players what NMR's export shows
  and nothing more), over the library's export (daydream_nmr::mme), as `daydream mme-export`
  writes it. It only reads the world. NMR's "Export Only" boxes and "For GreaterMUD Engine" are
  not here: the export is always whole, and Daydream supports 1.11p only.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import Upload from '@lucide/svelte/icons/upload';
  import X from '@lucide/svelte/icons/x';
  import { onMount } from 'svelte';
  import { api, type MmeInfo, type MmeOptions } from '../api';
  import { messageOf, toast } from '../shell/toasts.svelte';
  import { fileName, fromIni, roomsOf, toIni } from './mme';
  import Seg from './Seg.svelte';
  import Tally from './Tally.svelte';
  import Transfer from './Transfer.svelte';

  type Result = { tables: { name: string; rows: number }[] } | { file: string };

  let info = $state.raw<MmeInfo | null>(null);
  let failed = $state('');
  let o = $state<MmeOptions>({
    legit: true,
    disable_kai: false,
    exclude_default: true,
    exclude_rooms: false,
    excluded: [],
    hide_excluded: false,
    no_rooms: false,
    custom: '',
    update_url: '',
    dat_version: '',
  });
  let showDefaults = $state(false);
  let room = $state({ map: '', from: '', to: '' });
  let roomError = $state('');
  let loader = $state<HTMLInputElement | undefined>();

  onMount(() => {
    api
      .mmeInfo()
      .then((i) => {
        info = i;
        if (!o.dat_version) o.dat_version = i.dat_version;
      })
      .catch((e) => (failed = messageOf(e)));
  });

  function save(blob: Blob, name: string) {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = name;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(a.href), 10_000);
  }

  /** "Legit" is the export MME's own site hands out: no custom name, update page or rooms of one's own. */
  function setLegit(on: boolean) {
    o.legit = on;
    if (on) o.exclude_rooms = false;
  }

  function addRooms() {
    const r = roomsOf(room.map, room.from, room.to);
    if ('error' in r) {
      roomError = r.error;
      return;
    }
    roomError = '';
    o.excluded.push(r);
    room = { map: room.map, from: '', to: '' };
  }

  async function load(f: File) {
    try {
      o = fromIni(await f.text(), $state.snapshot(o));
      toast(`Loaded the settings of ${f.name}.`);
    } catch (e) {
      toast(messageOf(e), { kind: 'error' });
    }
  }

  const format = $derived(info?.jet ? 'mdb' : 'plan');
  const options = (): MmeOptions => $state.snapshot(o);

  async function run(dry: boolean): Promise<Result> {
    if (dry) return api.mmeSummary(options());
    const name = fileName(o, format);
    save(await api.mmeFile(options(), format), name);
    return { file: name };
  }

  const rows = (r: Result) => ('tables' in r ? r.tables.reduce((n, t) => n + t.rows, 0) : 0);
  const span = (from: number, to: number) => (from === to ? `${from}` : `${from}–${to}`);
</script>

{#if failed}
  <p class="bad">{failed}</p>
{:else if !info}
  <p class="faint">Loading…</p>
{:else}
  <div class="ws-form">
    <div class="row">
      <span class="lbl">NMR’s settings file</span>
      <button type="button" class="btn sm" onclick={() => loader?.click()}><Upload size={14} />Load…</button>
      <button type="button" class="btn sm" onclick={() => save(new Blob([toIni(options())], { type: 'text/plain' }), 'mme-export.ini')}>Save</button>
      <input
        bind:this={loader}
        type="file"
        accept=".ini,.txt"
        hidden
        onchange={(e) => {
          const f = e.currentTarget.files?.[0];
          e.currentTarget.value = '';
          if (f) void load(f);
        }}
      />
    </div>

    <fieldset>
      <legend>What MME says it is</legend>
      <label class="entry"><span class="lbl">BBS or database name</span><input bind:value={o.custom} disabled={o.legit} /></label>
      <label class="entry"><span class="lbl">Dat version or realm</span><input bind:value={o.dat_version} /></label>
      <label class="entry">
        <span class="lbl">Update page</span><input bind:value={o.update_url} disabled={o.legit} placeholder="https://…" spellcheck="false" />
        <small>What Help › Check for Updates opens in MME.</small>
      </label>
    </fieldset>

    <fieldset>
      <legend>What goes in</legend>
      <label class="check">
        <input type="checkbox" checked={o.legit} onchange={(e) => setLegit(e.currentTarget.checked)} /> “Legit” export, as MME’s own site hands out: no name or update page
        of your own, and no rooms of your own left out
      </label>
      <label class="check"><input type="checkbox" bind:checked={o.disable_kai} /> Turn off Kai auto-learning</label>
      <div class="row">
        <label class="check"><input type="checkbox" bind:checked={o.exclude_default} /> Leave out the default “not in the game” rooms</label>
        <button type="button" class="btn sm" aria-expanded={showDefaults} onclick={() => (showDefaults = !showDefaults)}>{showDefaults ? 'Hide them' : 'Show them'}</button>
      </div>
      {#if showDefaults}
        <Tally heads={['Map', 'Rooms', 'What']} rows={info.defaults.map((d) => [String(d.map), span(d.from, d.to), d.name])} />
      {/if}
    </fieldset>

    <fieldset disabled={o.legit}>
      <legend>Rooms of your own</legend>
      <label class="check"><input type="checkbox" bind:checked={o.exclude_rooms} /> Keep rooms of your own from players{o.legit ? ' (not with a legit export)' : ''}</label>
      {#if o.exclude_rooms && !o.legit}
        <div class="row">
          <span class="lbl">Which</span>
          <Seg label="Which rooms" options={[['some', 'These rooms'], ['all', 'Every room']]} bind:value={() => (o.no_rooms ? 'all' : 'some'), (v) => (o.no_rooms = v === 'all')} />
        </div>
        {#if !o.no_rooms}
          <div class="row">
            <span class="lbl">How</span>
            <Seg label="How" options={[['out', 'Left out'], ['hide', 'Kept, details blanked']]} bind:value={() => (o.hide_excluded ? 'hide' : 'out'), (v) => (o.hide_excluded = v === 'hide')} />
          </div>
          <div class="rooms">
            {#each o.excluded as r, i (i)}
              <span class="chip">
                Map {r.map}: {span(r.from, r.to)}
                <button type="button" class="x" aria-label="Take map {r.map} {span(r.from, r.to)} off" onclick={() => o.excluded.splice(i, 1)}><X size={12} /></button>
              </span>
            {:else}
              <span class="faint">No rooms yet.</span>
            {/each}
          </div>
          <form
            class="row add"
            onsubmit={(e) => {
              e.preventDefault();
              addRooms();
            }}
          >
            <label class="inline">Map <input class="num" inputmode="numeric" bind:value={room.map} /></label>
            <label class="inline">Rooms <input class="num" inputmode="numeric" bind:value={room.from} /></label>
            <label class="inline">to <input class="num" inputmode="numeric" bind:value={room.to} placeholder={room.from} /></label>
            <button type="submit" class="btn sm"><Plus size={14} />Add</button>
            {#if o.excluded.length}<button type="button" class="btn sm" onclick={() => (o.excluded = [])}>Clear</button>{/if}
            {#if roomError}<span class="bad">{roomError}</span>{/if}
          </form>
        {/if}
      {/if}
    </fieldset>

    {#if !info.jet}
      <p class="hint">
        This machine has no Windows Jet engine, which writing a .mdb takes, so the export comes as a plan. On any Windows machine, put it beside
        <a href="/api/mme/write-mdb.ps1" download="write-mdb.ps1">write-mdb.ps1</a> and run
        <code>powershell -ExecutionPolicy Bypass -File write-mdb.ps1 {fileName(o, 'plan')} {fileName(o, 'mdb')}</code> to write the .mdb MME reads.
      </p>
    {/if}
  </div>

  <Transfer
    {run}
    key={JSON.stringify(o)}
    way="read"
    verb="Download {fileName(o, format)}"
    act={() => `Download ${fileName(o, format)}`}
    done={(r: Result) => ('file' in r ? `Saved ${r.file}.` : 'Done.')}
  >
    {#snippet result(r, dry)}
      {#if 'tables' in r}
        <p class="said">{dry ? 'The export would hold' : 'The export holds'} {r.tables.length} tables, {rows(r).toLocaleString()} rows in all.</p>
        <Tally heads={['Table', 'Rows']} rows={r.tables.map((t) => [t.name, t.rows])} />
      {:else}
        <p class="said">{format === 'mdb' ? 'Open it in MME, or hand it out.' : 'Turn it into the .mdb with write-mdb.ps1, as above.'}</p>
      {/if}
    {/snippet}
  </Transfer>
{/if}

<style>
  .ws-form { display: flex; flex-direction: column; gap: 12px; }
  fieldset { display: flex; flex-direction: column; gap: 8px; margin: 0; padding: 10px 14px 12px; border: 1px solid var(--line); border-radius: 14px; min-width: 0; }
  fieldset:disabled { opacity: .6; }
  legend { padding: 0 6px; font: 600 14px var(--font-display); }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 10.5em; }
  .entry { display: grid; grid-template-columns: 10.5em minmax(0, 1fr); align-items: center; gap: 4px 8px; }
  .entry input { width: 100%; max-width: 420px; }
  .entry small { grid-column: 2; font-size: 12px; font-weight: 700; color: var(--ink-3); }
  .check { display: inline-flex; align-items: flex-start; gap: 6px; font-weight: 700; font-size: 13px; max-width: 85ch; }
  .check input { flex: none; margin-top: 2px; }
  .inline { display: inline-flex; align-items: center; gap: 6px; font-weight: 800; font-size: 12.5px; color: var(--ink-2); }
  .num { width: 6.5em; font-family: var(--font-mono); }
  .rooms { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; font-size: 12.5px; font-weight: 700; }
  .chip { display: inline-flex; align-items: center; gap: 4px; min-height: 26px; padding: 0 4px 0 10px; border-radius: 999px; font: 700 12px var(--font-mono); background: var(--surface-2); border: 1px solid var(--line); }
  .x { min-height: 20px; width: 20px; padding: 0; border: 0; border-radius: 999px; background: transparent; box-shadow: none; display: grid; place-items: center; }
  .x:hover { background: var(--bad-bg); color: var(--bad); }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 80ch; }
  .hint code { font-size: 12px; overflow-wrap: anywhere; }
  .said { margin: 0; font-weight: 700; color: var(--ink-2); }
  .bad { color: var(--bad); font-weight: 700; font-size: 12.5px; }
  @media (max-width: 560px) {
    .lbl { flex-basis: 100%; }
    .entry { grid-template-columns: minmax(0, 1fr); }
    .entry small { grid-column: 1; }
  }
</style>
