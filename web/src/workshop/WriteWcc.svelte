<!--
  Import and export › WCC files: the world's edits written into a folder of WCC*.DAT files by
  hand, for Worldgroup and NMR. Only the edits go in, onto the files as the game has them now,
  so nothing the game did meanwhile (floors, shop stock, players) is undone. Publishing does
  this for the open game; this is for any folder, and cannot be undone here.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Info from '@lucide/svelte/icons/info';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import { onMount } from 'svelte';
  import { api, type WrittenFile } from '../api';
  import { games } from '../games';
  import Tally from './Tally.svelte';
  import Transfer from './Transfer.svelte';

  /** Where the WCC files were last written, so the page remembers it. */
  const KEY = 'daydream.wcc';
  const remembered = () => {
    try {
      return localStorage.getItem(KEY) ?? '';
    } catch {
      return '';
    }
  };

  type Result = { dry_run: boolean; files: WrittenFile[]; backup?: string | null };

  let folder = $state('');
  /** The open game's folder, when it is a WCC game: then the box may stay empty. */
  let gameDir = $state<string | null>(null);
  /** The open game keeps its world in .db files: `copy` (publish it) or `live` (saved already). */
  let dbGame = $state<'' | 'copy' | 'live'>('');
  let custom = $state(true);
  let out = $state('');
  let backup = $state(true);
  let force = $state(false);

  onMount(() => {
    games
      .list()
      .then((l) => {
        custom = l.custom;
        if (l.current?.kind === 'wcc') gameDir = l.current.location ?? null;
        else folder = remembered();
        if (l.current?.kind === 'db') dbGame = l.current.live ? 'live' : 'copy';
      })
      .catch(() => (folder = remembered()));
  });

  const body = $derived({ wcc: folder.trim() || null, out: custom ? out.trim() || null : null, backup, force });
  const where = $derived(body.out ?? body.wcc ?? gameDir ?? '');
  const written = (r: Result) => r.files.filter((f) => f.action !== 'unchanged');
  const files = (n: number) => `${n.toLocaleString()} ${n === 1 ? 'file' : 'files'}`;
  /** A path's folder and its file's name, on either kind of machine. */
  const split = (path: string): [string, string] => {
    const i = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
    return [path.slice(0, Math.max(0, i)), path.slice(i + 1)];
  };
  const folderOf = (r: Result) => split(r.files[0]?.dest ?? '')[0];

  async function run(dry: boolean): Promise<Result> {
    const r = await api.writeDat(body, dry);
    const typed = folder.trim();
    if (typed) {
      try {
        localStorage.setItem(KEY, typed);
      } catch {
        // a private window: nothing to remember
      }
    }
    return r;
  }

  /** What happens to a file. */
  const WHAT: Record<WrittenFile['action'], string> = {
    updated: 'Updated in place',
    rebuilt: 'Rebuilt',
    written: 'Rows written',
    unchanged: 'Unchanged',
  };
</script>

<div class="ws-form">
  {#if dbGame}
    <div class="callout info">
      <Info size={16} />
      <span>
        {dbGame === 'copy'
          ? 'The open game keeps its world in .db files, not WCC files: publish it from Publish, which writes them with the game stopped.'
          : 'The open game’s own .db files are being edited: every save is in the game already, so there is nothing to write back.'}
        This page is for a folder of WCC files.
      </span>
    </div>
  {/if}
  <label class="entry">
    <span class="lbl">WCC folder</span>
    <input
      class="path"
      bind:value={folder}
      placeholder={gameDir ? `${gameDir} (the open game’s)` : 'C:\\WGSERV'}
      spellcheck="false"
      autocomplete="off"
    />
    <small>On the machine Daydream runs on.</small>
  </label>
  <label class="entry">
    <span class="lbl">Into another folder</span>
    <input
      class="path"
      bind:value={out}
      disabled={!custom}
      placeholder={custom ? 'Leave empty to write over the WCC folder’s files' : 'Not with an access token: start Daydream with --allow-any-source'}
      spellcheck="false"
      autocomplete="off"
    />
  </label>
  <div class="checks">
    <label class="check"><input type="checkbox" bind:checked={backup} disabled={!!body.out} /> Back up each file before changing it</label>
    <label class="check">
      <input type="checkbox" bind:checked={force} /> Write the whole world even over files the game changed since it was converted (for a world edited before Daydream recorded edits)
    </label>
  </div>
  <div class="callout warn">
    <TriangleAlert size={16} />
    <span>Stop the game first: Daydream will not write while it sees the game running on the folder.</span>
  </div>
</div>

<Transfer
  {run}
  key={JSON.stringify(body)}
  blocked={folder.trim() || gameDir ? '' : 'Give the folder that holds the WCC files.'}
  way="out"
  verb="Write the files"
  act={(r: Result) => (written(r).length ? `Write ${files(written(r).length)}` : '')}
  done={(r: Result) => `Wrote ${files(written(r).length)} into ${split(folderOf(r) || where)[1] || 'the folder'}.`}
>
  {#snippet result(r, dry)}
    {@const changed = written(r)}
    {@const skipped = r.files.flatMap((f) => f.skipped ?? [])}
    <p class="said">
      {changed.length
        ? `${changed.length.toLocaleString()} of ${files(r.files.length)} ${dry ? 'would change' : 'written'}`
        : `All ${files(r.files.length)} hold the world’s edits already: nothing to write`}{folderOf(r) ? `, in ${folderOf(r)}` : ''}.
    </p>
    {#if changed.length}
      <Tally
        heads={['File', 'What happens', 'Records']}
        rows={changed.map((f) => [split(f.dest)[1], `${WHAT[f.action]}${f.merged ? '' : ' (written whole)'}`, f.records ?? 0])}
      />
    {/if}
    {#if skipped.length}
      <div class="callout info">
        <Info size={16} />
        <div>
          <b>{skipped.length === 1 ? 'One edit is' : `${skipped.length.toLocaleString()} edits are`} left out, and kept for the next write:</b>
          <ul>{#each skipped.slice(0, 12) as s, i (i)}<li>{s}</li>{/each}</ul>
          {#if skipped.length > 12}<span>and {(skipped.length - 12).toLocaleString()} more.</span>{/if}
        </div>
      </div>
    {/if}
    {#if r.backup}
      <p class="faint said">The files as they were are in {r.backup}.</p>
    {:else if dry && changed.length && backup && !body.out}
      <p class="faint said">Each file is backed up first, into a daydream-backup folder beside it.</p>
    {/if}
  {/snippet}
</Transfer>

<style>
  .ws-form { display: flex; flex-direction: column; gap: 12px; }
  .entry { display: grid; grid-template-columns: 9.5em minmax(0, 1fr); align-items: center; gap: 4px 10px; }
  .entry small { grid-column: 2; font-size: 12px; font-weight: 700; color: var(--ink-3); }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); }
  .path { width: 100%; max-width: 560px; font-family: var(--font-mono); }
  .checks { display: flex; flex-direction: column; gap: 6px; }
  .check { display: inline-flex; align-items: flex-start; gap: 6px; font-weight: 700; font-size: 13px; max-width: 80ch; }
  .check input { flex: none; margin-top: 2px; }
  .said { margin: 0; font-weight: 700; color: var(--ink-2); }
  .faint.said { font-size: 12.5px; color: var(--ink-3); }
  .callout ul { margin: 4px 0 0; padding-left: 18px; }
  @media (max-width: 560px) {
    .entry { grid-template-columns: minmax(0, 1fr); }
    .entry small { grid-column: 1; }
  }
</style>
