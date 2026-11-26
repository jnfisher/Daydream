<!-- The open world: its name, its engine and whether its game runs; the way to another. -->
<script lang="ts">
  import { Popover } from 'bits-ui';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import CloudUpload from '@lucide/svelte/icons/cloud-upload';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import HardDrive from '@lucide/svelte/icons/hard-drive';
  import Server from '@lucide/svelte/icons/server';
  import Terminal from '@lucide/svelte/icons/terminal';
  import { KINDS, how } from '../games';
  import { app } from './state.svelte';

  let open = $state(false);
  const c = $derived(app.current);
  const name = $derived(c ? (c.label ?? c.realm_dir.split(/[\\/]/).filter(Boolean).pop() ?? c.realm_dir) : 'No world open');
  const kind = $derived(c?.kind ? KINDS[c.kind] : 'A realm folder');
  const Engine = $derived(c?.kind === 'silvermere' ? Server : c?.kind === 'db' ? Terminal : HardDrive);
  const run = $derived(app.running === true ? 'running' : app.running === false ? 'stopped' : null);
  const waiting = $derived(app.changes?.changes.length ?? 0);
</script>

<Popover.Root bind:open>
  <Popover.Trigger class="world-pill" title="This world and its game">
    <span class="eng"><Engine size={15} /></span>
    <span class="txt">
      <b>{name}</b>
      <span class="hide-narrow">{kind}{#if run}{' · '}<span class="dot {run}"></span>{' '}{run}{/if}{#if c?.live}{' · live files'}{/if}</span>
    </span>
    <ChevronDown size={13} class="hide-narrow" />
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content class="pop world-pop" sideOffset={8} align="start" collisionPadding={8}>
      <div class="head">
        <span class="eng big"><Engine size={20} /></span>
        <span class="who"><b>{name}</b><span class="faint">{c?.location ?? c?.realm_dir ?? ''}</span></span>
      </div>
      {#if c}
        <div class="pills">
          {#if run === 'running'}<span class="pill bad"><span class="dot"></span>Running{#if app.runningBy} ({app.runningBy}){/if}</span>
          {:else if run === 'stopped'}<span class="pill ok"><span class="dot"></span>Stopped</span>{/if}
          {#if c.live}<span class="pill warn">Live files</span>{/if}
          <span class="pill info">{waiting} change{waiting === 1 ? '' : 's'} waiting</span>
        </div>
        {#if c.kind}<p class="how">{how({ kind: c.kind, live: !!c.live })}</p>{/if}
      {/if}
      <div class="menu-sep"></div>
      <a class="menu-item" href="#/publish" onclick={() => (open = false)}><CloudUpload size={16} /><span>Publish…<span class="sub">Review the changes and write them into the game</span></span></a>
      <a class="menu-item" href="#/worlds" onclick={() => (open = false)}><FolderOpen size={16} /><span>Open another world…<span class="sub">Every game Daydream found on this computer</span></span></a>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.world-pill) {
    display: flex; align-items: center; gap: 9px; min-width: 0; height: 40px; padding: 0 12px 0 7px;
    border-radius: 999px; border: 1px solid var(--line-2); background: var(--surface-2); box-shadow: none; flex: 0 1 auto;
  }
  :global(.world-pill:hover) { border-color: var(--outline); }
  .eng { width: 26px; height: 26px; border-radius: 50%; display: grid; place-items: center; background: var(--mint-2); color: var(--mint-ink); flex: none; }
  .eng.big { width: 40px; height: 40px; border-radius: 12px; }
  .txt { display: flex; flex-direction: column; line-height: 1.15; text-align: left; min-width: 0; }
  .txt b { font-weight: 800; font-size: 13.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 22ch; }
  .txt span { font-size: 11.5px; font-weight: 700; color: var(--ink-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .dot.running { color: var(--bad); }
  .dot.stopped { color: var(--ok); }
  :global(.world-pop) { width: min(380px, calc(100vw - 16px)); }
  .head { display: flex; gap: 12px; align-items: center; padding: 8px 10px 10px; }
  .who { display: flex; flex-direction: column; line-height: 1.3; min-width: 0; }
  .who b { font: 600 16px var(--font-display); }
  .who span { font-size: 12px; font-weight: 700; overflow-wrap: anywhere; }
  .pills { display: flex; flex-wrap: wrap; gap: 6px; padding: 0 10px 8px; }
  .how { margin: 0 10px 6px; font-size: 12.5px; font-weight: 600; color: var(--ink-2); }
</style>
