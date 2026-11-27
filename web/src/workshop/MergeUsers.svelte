<!--
  Players › Merge accounts: the users of another user file (WCCUSER2.DAT, or a Daydream .db)
  copied into the world byte for byte, as NMR's Merge Users does. Users the world has already
  are left as they are. One step in Changes.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { api } from '../api';
  import FilePick from './FilePick.svelte';
  import Transfer from './Transfer.svelte';

  type Result = { dry_run: boolean; copied: number; skipped: string[]; step?: number | null };

  const SHOWN = 80;

  let file = $state<File | null>(null);

  const users = (n: number) => `${n.toLocaleString()} ${n === 1 ? 'user' : 'users'}`;
  const key = $derived(JSON.stringify([file?.name, file?.size, file?.lastModified]));
</script>

<div class="ws-form">
  <FilePick bind:file accept=".dat,.db" label="User file" hint="WCCUSER2.DAT from another game, or a Daydream .db" />
  <p class="hint">Players are matched by their user ID: one the world has already keeps its own account, and the file's is left out.</p>
</div>

<Transfer
  run={(dry) => api.mergeUsers(file!, dry)}
  {key}
  blocked={file ? '' : 'Pick the user file to merge in.'}
  way="in"
  verb="Copy users in"
  act={(r: Result) => (r.copied ? `Copy ${users(r.copied)} in` : '')}
  done={(r: Result) => `Copied ${users(r.copied)} in from ${file?.name ?? 'the file'}.`}
>
  {#snippet result(r, dry)}
    <div class="tiles">
      <div><small>{dry ? 'Copying in' : 'Copied in'}</small><b>{r.copied.toLocaleString()}</b></div>
      <div><small>The world has them</small><b>{r.skipped.length.toLocaleString()}</b></div>
    </div>
    {#if !r.copied}
      <p class="said">The world has every user of the file already: nothing to copy.</p>
    {/if}
    {#if r.skipped.length}
      <div class="left">
        <span class="faint">Left out, as the world has them:</span>
        {#each r.skipped.slice(0, SHOWN) as u, i (i)}<span class="who">{u}</span>{/each}
        {#if r.skipped.length > SHOWN}<span class="faint">and {(r.skipped.length - SHOWN).toLocaleString()} more</span>{/if}
      </div>
    {/if}
  {/snippet}
</Transfer>

<style>
  .ws-form { display: flex; flex-direction: column; gap: 10px; }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 75ch; }
  .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 8px; max-width: 420px; }
  .tiles div { background: var(--surface-2); border-radius: 12px; padding: 8px 10px; }
  .tiles small { display: block; font: 800 10.5px/1.2 var(--font-ui); letter-spacing: .06em; text-transform: uppercase; color: var(--ink-3); }
  .tiles b { font: 700 18px/1.4 var(--font-mono); }
  .said { margin: 0; font-weight: 700; color: var(--ink-2); }
  .left { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; font-size: 12.5px; font-weight: 700; max-height: 200px; overflow: auto; }
  .who { padding: 1px 8px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); font-family: var(--font-mono); font-size: 12px; }
</style>
