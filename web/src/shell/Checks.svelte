<!--
  Home's checks that read the wiring, across the whole world (the server's /api/checks):
  prices against what is said, spelling, placeholders, messages that read wrong, and spawns
  nobody reaches. Each check is a tab with its counts. Its findings name their records as
  chips, warnings first, and what was checked and holds stays folded away. Read again after
  each save, while Home is open.
-->
<svelte:options css="injected" />

<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import Coins from '@lucide/svelte/icons/coins';
  import Ghost from '@lucide/svelte/icons/ghost';
  import Info from '@lucide/svelte/icons/info';
  import MessageSquareText from '@lucide/svelte/icons/message-square-text';
  import SpellCheck from '@lucide/svelte/icons/spell-check';
  import StickyNote from '@lucide/svelte/icons/sticky-note';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import { api, type CheckId, type CheckInfo, type Finding } from '../api';
  import RefChip from '../library/RefChip.svelte';
  import Said from '../library/Said.svelte';
  import { read, write } from './prefs.svelte';
  import { app } from './state.svelte';
  import { messageOf, toast } from './toasts.svelte';

  const ICON = { prices: Coins, spelling: SpellCheck, placeholders: StickyNote, messages: MessageSquareText, spawns: Ghost };
  const LEVEL = { warn: TriangleAlert, note: Info, ok: CircleCheck };
  const FIRST = 12;
  const MORE = 30;

  let info: CheckInfo[] = $state([]);
  let pick = $state<CheckId>((['prices', 'spelling', 'placeholders', 'messages', 'spawns'] as const).find((c) => c === read('dd-home-check', '')) ?? 'prices');
  let found: Finding[] = $state([]);
  let total = $state(0);
  let wanted = $state(FIRST);
  let error = $state('');
  let loading = $state(true);
  // what was checked and holds, read when its fold opens
  let okOpen = $state(false);
  let oks: Finding[] = $state([]);
  let okWanted = $state(MORE);

  const shown = $derived(info.find((c) => c.id === pick));
  let asked = 0;

  async function load() {
    const my = ++asked;
    const check = pick;
    try {
      const r = await api.checks({ check, level: ['warn', 'note'], limit: wanted });
      const ok = okOpen ? await api.checks({ check, level: ['ok'], limit: okWanted }) : null;
      if (my !== asked) return;
      info = r.checks;
      found = r.findings;
      total = r.total;
      if (ok) oks = ok.findings;
      error = '';
    } catch (e) {
      if (my === asked) error = messageOf(e);
    } finally {
      if (my === asked) loading = false;
    }
  }

  // read again when the world changes (a save, another world opened) and Home is in view
  $effect(() => {
    void app.changes;
    void app.current;
    void pick;
    void wanted;
    void okOpen;
    void okWanted;
    if (app.route.place === 'home') void load();
  });

  function choose(c: CheckId) {
    if (c === pick) return;
    pick = c;
    write('dd-home-check', c);
    wanted = FIRST;
    okWanted = MORE;
    found = [];
    oks = [];
    loading = true;
  }

  async function accept(word: string) {
    try {
      await api.acceptWord(word);
      void load();
      toast(`Spelling leaves “${word}” alone from now on.`, {
        action: 'Undo',
        onAction: () => void api.acceptWord(word, false).then(load),
      });
    } catch (e) {
      toast(messageOf(e), { kind: 'error' });
    }
  }
</script>

<section class="card checks" aria-labelledby="checks-h">
  <div class="head">
    <h3 id="checks-h">Checks that read the wiring</h3>
    <span class="faint">Across the whole world, read again after each save.</span>
  </div>

  <div class="tabs" role="tablist" aria-label="Checks">
    {#each info.length ? info : [] as c (c.id)}
      {@const Icon = ICON[c.id]}
      <button type="button" role="tab" aria-selected={c.id === pick} class:on={c.id === pick} onclick={() => choose(c.id)}>
        <Icon size={15} />
        <span>{c.title}</span>
        {#if c.warn}<span class="n warn" title="{c.warn} that players meet">{c.warn.toLocaleString()}</span>{/if}
        {#if c.note}<span class="n note" title="{c.note} worth a look">{c.note.toLocaleString()}</span>{/if}
        {#if !c.warn && !c.note}<span class="n ok" title="Nothing to look at"><CircleCheck size={12} /></span>{/if}
      </button>
    {/each}
  </div>

  {#if error}
    <p class="quiet bad">{error}</p>
  {:else if loading && !found.length}
    <p class="quiet">Reading the world…</p>
  {:else}
    {#if shown}<p class="about">{shown.about}</p>{/if}
    {#if found.length}
      <ul class="found">
        {#each found as f, i (i)}
          {@const L = LEVEL[f.level]}
          <li class={f.level}>
            <span class="ic" title={f.level === 'warn' ? 'Players meet it' : 'Worth a look'}><L size={15} /></span>
            <span class="who"><RefChip table={f.about.table} key={f.about.key} compact /></span>
            <span class="t"><Said bits={f.bits} /></span>
            {#if f.word}
              {@const w = f.word}
              <button type="button" class="btn sm ghost" onclick={() => void accept(w)} title="The world means it: spelling leaves it alone">Not a typo</button>
            {/if}
          </li>
        {/each}
      </ul>
      {#if found.length < total}
        <div class="row-end">
          <span class="faint">{found.length.toLocaleString()} of {total.toLocaleString()}</span>
          <button type="button" class="btn sm" onclick={() => (wanted += MORE)}>Show more</button>
        </div>
      {/if}
    {:else if shown}
      <p class="quiet good"><CircleCheck size={15} /> Nothing to look at{shown.ok ? `: ${shown.ok.toLocaleString()} checked and fine` : ''}.</p>
    {/if}

    {#if shown?.ok}
      <button type="button" class="fold" aria-expanded={okOpen} onclick={() => (okOpen = !okOpen)}>
        <ChevronDown size={14} class={okOpen ? 'turned' : ''} />
        {shown.ok.toLocaleString()} checked and fine
      </button>
      {#if okOpen}
        <ul class="found oks">
          {#each oks as f, i (i)}
            <li class="ok">
              <span class="ic"><CircleCheck size={15} /></span>
              <span class="who"><RefChip table={f.about.table} key={f.about.key} compact /></span>
              <span class="t"><Said bits={f.bits} /></span>
            </li>
          {/each}
        </ul>
        {#if oks.length < shown.ok}
          <div class="row-end"><button type="button" class="btn sm" onclick={() => (okWanted += MORE)}>Show more</button></div>
        {/if}
      {/if}
    {/if}
  {/if}
</section>

<style>
  .checks { grid-column: 1 / -1; }
  .head { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .head h3 { margin: 0; font-size: 17px; }
  .head .faint { font-size: 12.5px; font-weight: 700; }
  .tabs { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
  .tabs button {
    display: inline-flex; align-items: center; gap: 6px; min-height: 34px; padding: 0 12px; border-radius: 12px;
    border: 1px solid var(--line); background: var(--surface); color: var(--ink-2); font-weight: 800; font-size: 13px; box-shadow: none;
  }
  .tabs button:hover:not(.on) { background: var(--surface-2); }
  .tabs button.on { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); }
  .n { display: inline-flex; align-items: center; min-width: 20px; height: 20px; padding: 0 6px; border-radius: 999px; font-size: 11.5px; font-weight: 800; justify-content: center; }
  .n.warn { background: var(--warn-bg); color: var(--warn); }
  .n.note { background: var(--surface-2); color: var(--ink-2); border: 1px solid var(--line); }
  .n.ok { background: var(--ok-bg); color: var(--ok); }
  .about { margin: 0 0 6px; font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  .quiet { margin: 6px 0; color: var(--ink-2); font-weight: 600; display: flex; align-items: center; gap: 6px; }
  .quiet.good { color: var(--ok); }
  .quiet.bad { color: var(--bad); }
  .found { list-style: none; margin: 0 -6px; padding: 0; }
  .found li { display: grid; grid-template-columns: 20px auto minmax(0, 1fr) auto; align-items: center; gap: 4px 10px; padding: 6px; border-radius: 12px; font-size: 13px; font-weight: 700; line-height: 1.45; }
  .found li:hover { background: var(--surface-2); }
  .found .ic { display: inline-flex; }
  .found .warn .ic { color: var(--warn); }
  .found .note .ic { color: var(--lilac-ink); }
  .found .ok .ic { color: var(--ok); }
  .found .t { min-width: 0; color: var(--ink-2); }
  @media (max-width: 720px) {
    .found li { grid-template-columns: 20px minmax(0, 1fr); }
    .found .t, .found .btn { grid-column: 2; justify-self: start; }
  }
  .row-end { display: flex; justify-content: flex-end; align-items: center; gap: 10px; margin-top: 6px; font-size: 12.5px; font-weight: 700; }
  .fold { display: inline-flex; align-items: center; gap: 6px; margin-top: 8px; padding: 4px 8px; border: 0; background: none; box-shadow: none; color: var(--ink-2); font-weight: 800; font-size: 12.5px; border-radius: 10px; }
  .fold:hover { background: var(--surface-2); }
  .fold :global(.turned) { transform: rotate(180deg); }
  .oks { margin-top: 4px; }
</style>
