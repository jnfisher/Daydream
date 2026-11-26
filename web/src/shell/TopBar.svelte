<!--
  The top bar: the brand, the open world, one search box for everything, the theme, the
  changes waiting to publish, and Publish (the one rainbow thing on screen).
-->
<script lang="ts">
  import CloudUpload from '@lucide/svelte/icons/cloud-upload';
  import Search from '@lucide/svelte/icons/search';
  import ChangesTray from './ChangesTray.svelte';
  import Mascot from './Mascot.svelte';
  import ThemeMenu from './ThemeMenu.svelte';
  import WorldPill from './WorldPill.svelte';
  import { app } from './state.svelte';

  const mod = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent) ? '⌘' : 'Ctrl';
</script>

<a class="brand" href="#/home" title="Home"><Mascot mood="happy" size={38} /><b class="hide-narrow">Daydream</b></a>
<WorldPill />
<button class="search-box" type="button" onclick={() => (app.paletteOpen = true)} aria-label="Search everything ({mod} K)">
  <Search size={16} /><span class="grow hide-narrow">Search records, places, commands…</span><span class="kbd hide-narrow">{mod} K</span>
</button>
<div class="top-right">
  <span class="hide-narrow"><ThemeMenu /></span>
  <ChangesTray />
  <a class="btn rainbow" href="#/publish" title="Send your changes to the game"><CloudUpload size={16} /><span class="hide-narrow">Publish</span></a>
</div>

<style>
  .brand { display: flex; align-items: center; gap: 8px; text-decoration: none; color: var(--ink); flex: none; }
  .brand b { font: 600 21px/1 var(--font-display); letter-spacing: .01em; }
  .search-box {
    flex: 1 1 360px; max-width: 560px; min-width: 0; height: 40px;
    display: flex; align-items: center; justify-content: flex-start; gap: 10px; padding: 0 8px 0 14px;
    border-radius: 14px; border: 1px solid var(--line-2); background: var(--surface-2);
    color: var(--ink-3); cursor: text; text-align: left; font-weight: 700; box-shadow: none;
  }
  .search-box:hover { border-color: var(--ink-3); }
  .grow { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .top-right { margin-left: auto; display: flex; align-items: center; gap: 8px; flex: none; }
  @media (max-width: 760px) {
    .search-box { flex: 0 0 40px; padding: 0; justify-content: center; }
  }
</style>
