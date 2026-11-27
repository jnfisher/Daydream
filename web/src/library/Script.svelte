<!--
  What a text block does, in words, as the game reads it (script.ts): a script's lines as
  steps, each record it names a chip; a room's commands by the words that run them; a random
  block's lines by their chances; a talk block's topics; a monster's names or chatter; or
  text shown as it is. It follows the text as it is typed.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { renderAnsi } from '../ansi';
  import { api, type Cell } from '../api';
  import { COLOUR, nameOdds, talkOdds } from './combat';
  import type { Draft } from './draft.svelte';
  import RefChip from './RefChip.svelte';
  import { blockOf } from './records.svelte';
  import { alternatives, linesOf, MAX_PARTS, readingFor } from './script';
  import Steps from './Steps.svelte';
  import { family, type Reference } from './where';

  let { draft, rowKey, version }: { draft: Draft; rowKey: Cell[]; version: number } = $props();

  const text = (v: unknown) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);
  const data = $derived(text(draft.get('Data')));
  const link = $derived(Number(draft.get('Link To')) || 0);
  const part = $derived(Number(rowKey[1]) || 0);

  // what names it says how the game reads it (a monster's greeting, a room's commands)
  let uses = $state<Reference[] | null>(null);
  $effect(() => {
    const key = rowKey;
    void version;
    let gone = false;
    api
      .references('Textblocks', key)
      .then((r) => {
        if (!gone) uses = r.references;
      })
      .catch(() => {
        if (!gone) uses = [];
      });
    return () => (gone = true);
  });
  const usedAs = $derived((uses ?? []).filter((r) => r.column !== null).map((r) => `${r.table} ${family(r.column!)}`));
  const reading = $derived(readingFor(data, usedAs));
  const lines = $derived(linesOf(data, reading));
  const SHOWN = 40;

  /** The map of the rooms whose commands these are, when they are all on one: a remote action's room is on it. */
  const map = $derived.by(() => {
    const maps = new Set((uses ?? []).filter((r) => r.table === 'Rooms' && r.column === 'Command Text').map((r) => Number(r.key[0])));
    return maps.size === 1 ? [...maps][0] : undefined;
  });
  /** The monster a name list or chatter is shown for. */
  const monster = $derived((uses ?? []).find((r) => r.table === 'Monsters' && /^(Desc|Talk) Txt$/.test(r.column ?? '')));

  const pct = (p: number) => (p >= 10 || Number.isInteger(p) ? `${Math.round(p)}%` : `${p.toFixed(1)}%`);
  const shownPattern = (p: string) => p.replace(/\r/g, '').replace(/\n/g, '⏎');

  function ansi(node: HTMLElement, line: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(line);
    return { update: draw };
  }
</script>

<div class="does">
  {#if part >= MAX_PARTS && (reading === 'script' || reading === 'commands' || reading === 'chances')}
    <p class="callout warn">The game runs at most parts 0 to {MAX_PARTS - 1} of a block, so it never runs this part's lines.</p>
  {/if}

  {#if reading === 'script'}
    <p class="how">
      Run as a whole{lines.length > 1 ? ': each line is tried in turn, its steps in order, until one runs through to its end' : ': its steps in order, until one stops'}.
    </p>
    {#each lines.slice(0, SHOWN) as l, k (l.no)}
      {#if k}<div class="or">if it stops, the next line</div>{/if}
      <div class="ln" class:never={l.never}>
        {#if lines.length > 1}<span class="lno">Line {l.no}</span>{/if}
        {#if l.never}<p class="note">Never run: {l.never}.</p>{:else}<Steps steps={l.steps} {map} />{/if}
      </div>
    {/each}
  {:else if reading === 'commands'}
    <p class="how">What players can type in a room that uses it: each line whose words match the typed line runs, until one runs through. A <code>*</code> lets the rest of the words be cut short.</p>
    {#each lines.slice(0, SHOWN) as l (l.no)}
      <div class="ln">
        <div class="pats">
          <span class="faint">Typed</span>
          {#each alternatives(l.pattern ?? '') as a, j (j)}{#if j}<span class="faint">or</span>{/if}<code class="pat">{shownPattern(a) || ' '}</code>{/each}
        </div>
        <Steps steps={l.steps} {map} />
      </div>
    {/each}
  {:else if reading === 'chances'}
    <p class="how">One roll of 0 to 99 runs one line: the first whose number is above the roll.</p>
    <div class="rows">
      {#each lines.slice(0, SHOWN) as l (l.no)}
        <div class="chance" class:never={l.never}>
          <span class="pct" title="Its number is {l.pattern}">{pct(l.pct ?? 0)}</span>
          {#if l.never}<p class="note">Never run: {l.never}.</p>{:else}<Steps steps={l.steps} {map} />{/if}
        </div>
      {/each}
    </div>
  {:else if reading === 'topics'}
    <p class="how">What a monster says when greeted, and when asked: a question takes the first line whose words appear anywhere in it, in any case.</p>
    <div class="topics">
      <span class="lead">Greeted</span>
      <span class="said-row">{#if link}<span class="faint">shows</span><RefChip table="Textblocks" key={[link, 0]} compact />{:else}<span class="faint">“… doesn't understand you.”</span>{/if}</span>
      {#if lines.length}<span class="lead whole">Asked about</span>{/if}
      {#each lines.slice(0, SHOWN) as l (l.no)}
        {@const block = l.block ?? 0}
        {@const shown = block > 0 ? blockOf(block) : null}
        <span class="tag word" title={l.pattern ? `“${l.pattern}”` : 'An empty line matches every question'}>{l.pattern ? shownPattern(l.pattern) : 'anything'}</span>
        <span class="said-rows">
          <span class="said-row"><span class="faint">shows</span>{#if block > 0}<RefChip table="Textblocks" key={[block, 0]} compact />{:else}<span class="none">nothing (text block {block})</span>{/if}</span>
          {#if shown?.link}<span class="said-row"><span class="faint">then runs</span><RefChip table="Textblocks" key={[shown.link, 0]} compact /></span>{/if}
        </span>
      {/each}
    </div>
    {#if !lines.length}<p class="note">No line is <code>words:block</code>, so every question gets “… has nothing to tell you!”</p>{/if}
  {:else if reading === 'names'}
    {@const odds = nameOdds(data, monster?.title ? String(monster.title) : 'its name')}
    <p class="how">The names a monster is given: the list is read line by line with a 1 in 10 chance to stop at each, so its first lines are the likeliest and the last takes what is left.</p>
    <div class="chips">
      {#each odds.slice(0, 24) as o (o.name)}<span class="tag" title="{pct(o.pct)} of the time">{o.name} <small>{pct(o.pct)}</small></span>{/each}
      {#if odds.length > 24}<span class="faint">+{odds.length - 24} more</span>{/if}
    </div>
  {:else if reading === 'talk'}
    {@const chatter = talkOdds(data)}
    <p class="how">What a monster says now and then: the first line whose chance beats a roll of 0 to 99, so the chances add up as they go.</p>
    {#if chatter.length}
      <div class="mini">
        {#each chatter.slice(0, 12) as l, k (k)}<div><span class="p">{pct(l.pct)}</span><span use:ansi={COLOUR.grey + l.text}></span></div>{/each}
      </div>
    {:else}<p class="note">No line is <code>chance text</code>, so it never says anything.</p>{/if}
  {:else}
    <p class="how">Shown to players as it is: it runs nothing.</p>
    {#if link}
      <div class="topic"><span class="faint">Shown as a monster's answer, it then runs</span><RefChip table="Textblocks" key={[link, 0]} compact /></div>
    {/if}
  {/if}
  {#if lines.length > SHOWN && reading !== 'names' && reading !== 'talk'}<p class="faint">+{lines.length - SHOWN} more lines</p>{/if}
</div>

<style>
  .does { display: grid; gap: 8px; }
  .how { margin: 0; color: var(--ink-3); font-size: 12px; font-weight: 700; line-height: 1.45; }
  .ln { display: grid; gap: 6px; padding: 8px 10px; border-radius: 12px; background: var(--surface-2); }
  .ln.never { opacity: 0.7; }
  .lno { font: 800 11px/1.2 var(--font-ui); letter-spacing: 0.05em; text-transform: uppercase; color: var(--ink-3); }
  .or { justify-self: start; margin-left: 14px; padding-left: 10px; border-left: 2px dashed var(--line); color: var(--ink-3); font-size: 11.5px; font-weight: 800; }
  .none { font: 600 12px var(--font-mono); color: var(--ink-3); }
  .note { margin: 0; color: var(--ink-3); font-size: 12px; font-weight: 700; }
  .pats { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 12px; font-weight: 800; }
  .pat { font: 700 12.5px var(--font-mono); padding: 2px 8px; border-radius: 8px; background: var(--surface); border: 1px solid var(--line); white-space: pre; }
  .rows { display: grid; }
  .chance { display: grid; grid-template-columns: 3.4rem minmax(0, 1fr); gap: 8px; align-items: start; padding: 6px 0; border-bottom: 1px dashed var(--line); }
  .chance:last-child { border-bottom: 0; }
  .chance.never { opacity: 0.7; }
  .pct { justify-self: start; margin-top: 2px; padding: 3px 8px; border-radius: 999px; background: var(--lilac-2); color: var(--lilac-ink); font: 800 12px/1.2 var(--font-ui); }
  .topics { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 8px 12px; align-items: center; }
  .lead { font-weight: 800; font-size: 13px; }
  .lead.whole { grid-column: 1 / -1; margin-top: 4px; }
  .word { justify-self: start; max-width: 14rem; }
  .said-rows { display: grid; gap: 4px; min-width: 0; }
  .said-row { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 12.5px; font-weight: 700; min-width: 0; }
  .said-row .faint { min-width: 4.2rem; }
  .topic { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 12.5px; font-weight: 700; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .tag { display: inline-flex; align-items: center; gap: 4px; min-height: 24px; padding: 0 9px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); font-size: 12px; font-weight: 800; color: var(--ink-2); vertical-align: middle; white-space: pre-wrap; }
  .tag small { color: var(--ink-3); font-weight: 700; }
  .mini { font: 13px/1.3 var(--font-mono); background: var(--term-bg); color: var(--term-fg); border-radius: 10px; padding: 7px 10px; white-space: pre-wrap; overflow-wrap: anywhere; border: var(--stroke) solid var(--term-frame); display: grid; gap: 2px; }
  .mini .p { display: inline-block; min-width: 3.2rem; color: #9b97c4; font: 800 11px var(--font-ui); }
  code { font: 700 12px var(--font-mono); }
  /* a narrow card: a topic's words over what it shows */
  @container (max-width: 420px) {
    .topics { grid-template-columns: minmax(0, 1fr); gap: 6px; }
    .topics .word { margin-top: 6px; }
    .said-row .faint { min-width: 0; }
  }
</style>
