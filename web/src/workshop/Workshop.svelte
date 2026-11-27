<!--
  The Workshop: big jobs that touch many records at once, as cards grouped by what they are for.
  Its search finds a tool by its name or by what NMR called it. A tool opens as its own page
  (`#/workshop/bulk`): its settings, then a preview of every value it changes, then Apply,
  which lands in Changes as one step to undo until you publish.
-->
<svelte:options css="injected" />

<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import ArrowRightLeft from '@lucide/svelte/icons/arrow-right-left';
  import BookText from '@lucide/svelte/icons/book-text';
  import Calculator from '@lucide/svelte/icons/calculator';
  import Combine from '@lucide/svelte/icons/combine';
  import Crown from '@lucide/svelte/icons/crown';
  import Database from '@lucide/svelte/icons/database';
  import Download from '@lucide/svelte/icons/download';
  import FileText from '@lucide/svelte/icons/file-text';
  import HardDriveDownload from '@lucide/svelte/icons/hard-drive-download';
  import Hash from '@lucide/svelte/icons/hash';
  import HeartPulse from '@lucide/svelte/icons/heart-pulse';
  import Lasso from '@lucide/svelte/icons/lasso';
  import Layers from '@lucide/svelte/icons/layers';
  import ListTree from '@lucide/svelte/icons/list-tree';
  import Lock from '@lucide/svelte/icons/lock';
  import MapIcon from '@lucide/svelte/icons/map';
  import Merge from '@lucide/svelte/icons/merge';
  import Percent from '@lucide/svelte/icons/percent';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import ScanSearch from '@lucide/svelte/icons/scan-search';
  import Scissors from '@lucide/svelte/icons/scissors';
  import Search from '@lucide/svelte/icons/search';
  import Server from '@lucide/svelte/icons/server';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import SquareDashed from '@lucide/svelte/icons/square-dashed';
  import Store from '@lucide/svelte/icons/store';
  import Swords from '@lucide/svelte/icons/swords';
  import Timer from '@lucide/svelte/icons/timer';
  import Trash from '@lucide/svelte/icons/trash';
  import Users from '@lucide/svelte/icons/users';
  import WandSparkles from '@lucide/svelte/icons/wand-sparkles';
  import Workflow from '@lucide/svelte/icons/workflow';
  import Wrench from '@lucide/svelte/icons/wrench';
  import type { Component } from 'svelte';
  import { tick } from 'svelte';
  import Health from '../shell/Health.svelte';
  import Sky from '../shell/Sky.svelte';
  import { GROUPS, search, toolById, toolHref, type Group, type Tool } from './catalogue';
  import { isList } from './reports';

  const ICONS: Record<string, Component<{ size?: number }>> = {
    wand: WandSparkles, pulse: HeartPulse, sliders: SlidersHorizontal, percent: Percent, combine: Combine, crown: Crown,
    lasso: Lasso, scan: ScanSearch, tree: ListTree, layers: Layers, users: Users, workflow: Workflow, book: BookText,
    swap: ArrowRightLeft, download: Download, database: Database, drive: HardDriveDownload, server: Server, file: FileText,
    timer: Timer, store: Store, merge: Merge, refresh: RefreshCw, swords: Swords, calculator: Calculator, dashed: SquareDashed,
    scissors: Scissors, trash: Trash, hash: Hash, lock: Lock,
  };

  /** How each tool's page is made: rebuilt, or today's form (Legacy). */
  const FIXES = new Set(['uses', 'dropuses', 'fold', 'limits', 'kills', 'restock', 'floors', 'buffer', 'strip', 'drops']);

  // the tools' pages load together when the first one opens, so the Workshop's own code stays small
  type Pages = typeof import('./pages');
  let pages: Promise<Pages> | null = null;
  const loadPages = () => (pages ??= import('./pages'));

  let open = $state<{ group: Group; tool: Tool } | null>(null);
  let q = $state('');
  let page = $state<HTMLDivElement | undefined>();

  const found = $derived(search(q));

  /** Shows the cards (scrolled to a group, if `rest` names one) or a tool's page. */
  export function show(rest: string[]): void {
    const id = rest[0] ?? '';
    const hit = toolById(id);
    open = hit && !hit.tool.href && !hit.tool.later ? hit : null;
    if (open) return;
    const group = GROUPS.find((g) => g.id === (id === 'lists' ? 'find' : id));
    if (group) {
      q = '';
      void tick().then(() => {
        const el = page?.querySelector<HTMLElement>(`#ws-${group.id}`);
        el?.scrollIntoView({ block: 'start' });
        el?.classList.remove('glow');
        void el?.offsetWidth;
        el?.classList.add('glow');
      });
    }
  }
</script>

{#snippet icon(t: Tool, size: number)}
  {@const I = ICONS[t.icon] ?? Wrench}
  <I {size} />
{/snippet}

{#snippet lazy(name: keyof Pages, props: Record<string, unknown> = {})}
  {#await loadPages() then p}
    {@const C = p[name] as Component<any>}
    <C {...props} />
  {:catch}
    <p class="bad">This tool did not load. Reload the page to try again.</p>
  {/await}
{/snippet}

<div class="ws" bind:this={page}>
  {#if open}
    {@const t = open.tool}
    <header class="tool-head">
      <nav class="crumbs" aria-label="Where this is">
        <a href="#/workshop"><ArrowLeft size={14} />Workshop</a>
        <span aria-hidden="true">›</span>
        <a href="#/workshop/{open.group.id}">{open.group.title}</a>
      </nav>
      <div class="title">
        <span class="ic big tone-{t.tone}">{@render icon(t, 26)}</span>
        <div class="words">
          <h1>{t.name}</h1>
          <p>{t.about}</p>
          {#if t.was.length}<p class="was">In NMR: {#each t.was as w (w)}<span>{w}</span>{/each}</p>{/if}
        </div>
      </div>
    </header>

    <section class="card body">
      {#key t.id}
        {#if FIXES.has(t.id)}
          {@render lazy('Fixes', { id: t.id })}
        {:else if t.id === 'scale'}
          {@render lazy('Scale')}
        {:else if t.id === 'bulk'}
          {@render lazy('BulkEdit')}
        {:else if t.id === 'delete'}
          {@render lazy('Delete')}
        {:else if t.id === 'abilities'}
          {@render lazy('Glossary')}
        {:else if isList(t.id)}
          {@render lazy('Lists', { id: t.id })}
          {#if t.id === 'spawns'}
            <h3 class="sub">Move a group or index</h3>
            {@render lazy('Legacy', { id: 'spawns' })}
          {/if}
        {:else if t.id === 'rooms'}
          <div class="callout info">
            <Lasso size={16} />
            <span>
              On the World map, <b>Select</b> (or <span class="kbd">V</span>) picks rooms with a box, a lasso, a brush or the wand. Its panel sets their name,
              light, monsters, control room or room type on them all at once.
            </span>
          </div>
          <div><a class="btn sticker" href="#/world"><MapIcon size={16} />Open the map</a></div>
          <h3 class="sub">Or a run of rooms by number</h3>
          {@render lazy('Legacy', { id: 'rooms' })}
        {:else}
          {@render lazy('Legacy', { id: t.id })}
        {/if}
      {/key}
    </section>
  {:else}
    <header class="hero sky">
      <Sky clouds={3} stars={14} seed={8} />
      <div class="in">
        <span class="hero-mark"><Wrench size={34} /></span>
        <div class="words">
          <h1>Workshop</h1>
          <p>Big jobs that touch many records at once. Each one shows a preview first, and every run lands in Changes, so nothing goes out until you publish.</p>
        </div>
        <label class="find">
          <Search size={16} />
          <input type="search" placeholder="Find a tool. NMR’s names work too." aria-label="Find a tool" bind:value={q} />
        </label>
      </div>
    </header>

    {#each found as { group, tools } (group.id)}
      <section class="grp" id="ws-{group.id}" aria-labelledby="ws-{group.id}-h">
        <div class="grp-h">
          <h2 id="ws-{group.id}-h">{group.title}</h2>
          <span>{group.about}</span>
        </div>
        {#if group.id === 'health'}
          <section class="card hpanel" aria-label="Health checks"><Health place="workshop" full /></section>
        {/if}
        <div class="cards">
          {#each tools as t (t.id)}
            {#if t.later}
              <div class="tcard later">
                <span class="top"><span class="ic tone-{t.tone}">{@render icon(t, 17)}</span><b>{t.name}</b><span class="pill">later</span></span>
                <p>{t.about}</p>
                {#if t.was.length}<span class="was">In NMR: {#each t.was as w (w)}<s>{w}</s>{/each}</span>{/if}
              </div>
            {:else}
              <a class="tcard" href={toolHref(t)}>
                <span class="top">
                  <span class="ic tone-{t.tone}">{@render icon(t, 17)}</span><b>{t.name}</b>
                  {#if t.href}<ArrowRight size={15} class="away" />{/if}
                </span>
                <p>{t.about}</p>
                {#if t.was.length}<span class="was">In NMR: {#each t.was as w (w)}<s>{w}</s>{/each}</span>{/if}
              </a>
            {/if}
          {/each}
        </div>
      </section>
    {:else}
      <p class="none">No tool goes by “{q.trim()}”. Try a word from what it does, such as <b>experience</b>, <b>shops</b> or <b>rooms</b>.</p>
    {/each}

    {#if !q.trim()}
      <section class="grp">
        <div class="grp-h"><h2>Retired</h2><span>Not needed in a web page.</span></div>
        <p class="retired">Cascade, Close All, Minimize All, Restore All and UnDock from NMR’s Window menu: every place is its own page now, and records open beside whatever you are doing.</p>
      </section>
    {/if}
  {/if}
</div>

<style>
  .ws { max-width: 1180px; margin: 0 auto; padding: 18px clamp(12px, 2.5vw, 26px) 80px; }
  .hero { border-radius: 26px; border: var(--stroke) solid var(--outline); box-shadow: 0 5px 0 var(--shadow); padding: 24px clamp(16px, 3vw, 34px); }
  .hero .in { position: relative; display: flex; gap: 18px; align-items: center; flex-wrap: wrap; }
  .hero-mark { width: 72px; height: 72px; border-radius: 22px; display: grid; place-items: center; flex: none; background: var(--peach); color: var(--on-pastel); border: var(--stroke) solid var(--outline); box-shadow: 0 4px 0 var(--shadow); transform: rotate(-5deg); }
  .words { min-width: 0; flex: 1 1 320px; }
  h1 { margin: 0; font-size: clamp(26px, 3.4vw, 36px); line-height: 1.05; }
  .hero p { margin: 6px 0 0; color: var(--ink-2); font-weight: 700; max-width: 640px; }
  .find { margin-left: auto; position: relative; width: min(360px, 100%); display: flex; align-items: center; color: var(--ink-3); }
  .find :global(svg) { position: absolute; left: 12px; pointer-events: none; }
  .find input { width: 100%; height: 42px; padding-left: 36px; border-radius: 14px; background: var(--surface); font-weight: 700; }
  .grp { margin-top: 26px; scroll-margin-top: 14px; border-radius: 22px; }
  .grp.glow { animation: glow 1.6s ease-out; }
  .hpanel { margin-bottom: 12px; padding: 8px 16px; }
  .bad { color: var(--bad); font-weight: 700; }
  @keyframes glow { from { box-shadow: 0 0 0 6px var(--focus); } }
  @media (prefers-reduced-motion: reduce) { .grp.glow { animation: none; } }
  .grp-h { display: flex; align-items: baseline; gap: 10px; margin-bottom: 10px; flex-wrap: wrap; }
  .grp-h h2 { margin: 0; font-size: 22px; }
  .grp-h span { color: var(--ink-2); font-weight: 700; font-size: 13px; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; }
  .tcard { display: flex; flex-direction: column; gap: 6px; padding: 14px; border-radius: 18px; border: 1px solid var(--line); background: var(--surface); color: var(--ink); text-decoration: none; transition: transform .2s var(--pop), border-color .15s, box-shadow .2s; min-width: 0; }
  a.tcard:hover { transform: translateY(-2px); border-color: var(--outline); box-shadow: 0 3px 0 var(--shadow); }
  a.tcard:focus-visible { outline: 3px solid var(--focus); outline-offset: 2px; }
  .tcard .top { display: flex; align-items: center; gap: 9px; }
  .tcard .top :global(.away) { margin-left: auto; color: var(--ink-3); flex: none; }
  .tcard b { font: 600 16px/1.15 var(--font-display); }
  .tcard p { margin: 0; font-size: 12.5px; font-weight: 600; color: var(--ink-2); line-height: 1.4; }
  .was { font: 700 11px/1.35 var(--font-ui); color: var(--ink-3); margin-top: auto; padding-top: 4px; }
  .was :is(s, span) { text-decoration: none; background: var(--surface-2); border-radius: 6px; padding: 0 5px; white-space: nowrap; display: inline-block; margin: 1px 0 1px 4px; }
  .tcard.later { border-style: dashed; background: var(--surface-2); }
  .tcard.later .pill { margin-left: auto; }
  .ic { width: 34px; height: 34px; border-radius: 11px; display: grid; place-items: center; flex: none; }
  .ic.big { width: 56px; height: 56px; border-radius: 17px; border: var(--stroke) solid var(--outline); box-shadow: 0 3px 0 var(--shadow); }
  .tone-lilac { background: var(--lilac-2); color: var(--lilac-ink); }
  .tone-mint { background: var(--mint-2); color: var(--mint-ink); }
  .tone-sky { background: var(--sky-2); color: var(--sky-ink); }
  .tone-peach { background: var(--peach-2); color: var(--peach-ink); }
  .tone-butter { background: var(--butter-2); color: var(--butter-ink); }
  .tone-pink { background: var(--pink-2); color: var(--pink-ink); }
  .tone-teal { background: var(--teal-2); color: var(--teal-ink); }
  .tone-plain { background: var(--surface-3); color: var(--ink-2); }
  .tone-bad { background: var(--bad-bg); color: var(--bad); }
  .none, .retired { color: var(--ink-2); font-weight: 700; font-size: 13px; margin: 26px 0 0; }
  .retired { margin: 0; }
  .tool-head { display: flex; flex-direction: column; gap: 10px; margin-bottom: 16px; }
  .crumbs { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 800; color: var(--ink-3); }
  .crumbs a { display: inline-flex; align-items: center; gap: 5px; color: var(--ink-2); text-decoration: none; border-radius: 8px; padding: 2px 4px; }
  .crumbs a:hover { color: var(--ink); background: var(--surface-2); }
  .title { display: flex; gap: 14px; align-items: flex-start; }
  .title p { margin: 4px 0 0; color: var(--ink-2); font-weight: 700; max-width: 70ch; }
  .title .was { margin: 6px 0 0; }
  .body { display: flex; flex-direction: column; gap: 16px; padding: clamp(14px, 2vw, 22px); }
  .sub { margin: 4px 0 0; font-size: 16px; }
  @media (max-width: 760px) { .find { margin-left: 0; } }
</style>
