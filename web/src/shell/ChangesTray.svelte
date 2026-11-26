<!--
  The changes waiting to publish, in the top bar: every save lands here until Publish writes
  it into the game. Each can be opened or taken back; the last save can be undone.
-->
<script lang="ts">
  import { Popover } from 'bits-ui';
  import History from '@lucide/svelte/icons/history';
  import Redo2 from '@lucide/svelte/icons/redo-2';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import ChangeRow from './ChangeRow.svelte';
  import Mascot from './Mascot.svelte';
  import { app, redo, undo } from './state.svelte';
  import { ago } from './time';

  const SHOWN = 40;
  let open = $state(false);
  const changes = $derived(app.changes?.changes ?? []);
  const n = $derived(changes.length);
  const since = $derived(app.changes?.since);
</script>

<Popover.Root bind:open>
  <Popover.Trigger class="btn changes-btn" title="Changes waiting to publish" aria-label="{n} change{n === 1 ? '' : 's'} waiting to publish">
    <History size={16} /><span class="hide-narrow">Changes</span><span class="count" class:some={n > 0}>{n}</span>
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content class="pop tray-pop" sideOffset={8} align="end" collisionPadding={8}>
      <div class="head">
        <b>{n ? `${n} change${n === 1 ? '' : 's'} waiting to publish` : 'Nothing waiting to publish'}</b>
        {#if since}
          <span class="faint">since the world was {app.current?.kind === 'db' ? 'opened' : 'read or last published'}, {ago(since)}</span>
        {:else if app.changes && since === null}
          <span class="faint">This world was edited before Daydream kept this list, so Publish writes it whole once.</span>
        {/if}
      </div>
      {#if n}
        <ul class="list">
          {#each changes.slice(0, SHOWN) as c (`${c.stem}:${c.id}`)}
            <ChangeRow {c} onopen={() => (open = false)} />
          {/each}
        </ul>
        {#if n > SHOWN}<p class="more faint">…and {n - SHOWN} more on the Publish page.</p>{/if}
      {:else}
        <div class="nothing">
          <Mascot mood="happy" size={54} />
          <span>Every save lands here until you publish it, and each one can be taken back.</span>
        </div>
      {/if}
      <div class="foot">
        <button type="button" class="btn sm" disabled={!app.history?.undo} title={app.history?.undo ? `Undo: ${app.history.undo.label}` : 'Nothing to undo'} onclick={() => void undo()}><Undo2 size={14} />Undo</button>
        <button type="button" class="btn sm" disabled={!app.history?.redo} title={app.history?.redo ? `Redo: ${app.history.redo.label}` : 'Nothing to redo'} onclick={() => void redo()}><Redo2 size={14} />Redo</button>
        <a class="btn sm primary review" href="#/publish" onclick={() => (open = false)}>Review and publish</a>
      </div>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.changes-btn) { height: 40px; border-radius: 14px; padding: 0 8px 0 12px; }
  .count { min-width: 22px; height: 22px; padding: 0 6px; border-radius: 999px; display: inline-grid; place-items: center; background: var(--surface-3); color: var(--ink-2); font-size: 12px; }
  .count.some { background: var(--butter); color: var(--on-pastel); }
  :global(.tray-pop) { width: min(440px, calc(100vw - 16px)); padding: 0; overflow: hidden; display: flex; flex-direction: column; max-height: min(560px, calc(100vh - 90px)); }
  .head { display: flex; flex-direction: column; gap: 2px; padding: 12px 14px 10px; border-bottom: 1px solid var(--line); }
  .head b { font: 600 16px var(--font-display); }
  .head span { font-size: 12px; font-weight: 700; }
  .list { list-style: none; margin: 0; padding: 6px; overflow: auto; flex: 1; min-height: 0; }
  .more { margin: 0; padding: 4px 14px 8px; font-size: 12px; font-weight: 700; }
  .nothing { display: flex; align-items: center; gap: 12px; padding: 14px; color: var(--ink-2); font-weight: 700; font-size: 13px; }
  .foot { display: flex; gap: 6px; align-items: center; padding: 10px; border-top: 1px solid var(--line); background: var(--surface-2); }
  .review { margin-left: auto; }
</style>
