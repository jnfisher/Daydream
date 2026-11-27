<!--
  What players can type in a room, besides the game's own commands, in the order the game
  tries it (exits.ts): the words that move them through a Text exit or work an exit with a
  Remote Action, then the room's commands. Then every way out: how "Obvious exits:" reads,
  and what each exit asks of a player who walks it. It follows the record as it is edited.
-->
<svelte:options css="injected" />

<script lang="ts">
  import DoorOpen from '@lucide/svelte/icons/door-open';
  import Footprints from '@lucide/svelte/icons/footprints';
  import Hand from '@lucide/svelte/icons/hand';
  import PlayIcon from '@lucide/svelte/icons/play';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import { renderAnsi } from '../ansi';
  import type { Cell } from '../api';
  import { storyHref } from '../shell/routes';
  import type { Draft } from './draft.svelte';
  import { DIRS, exitsOf, mapOf, obviousExits, SHORT, waysOf, wayOf, wordsOf, type Way } from './exits';
  import RefChip from './RefChip.svelte';
  import { blockOf, linesOf as messageLines, recordOf } from './records.svelte';
  import Said from './Said.svelte';
  import { alternatives, linesOf, wildcardMatch } from './script';
  import Steps from './Steps.svelte';

  let { draft, rowKey }: { draft: Draft; rowKey: Cell[] } = $props();

  const num = (v: unknown) => Number(v) || 0;
  const own = $derived({ map: num(rowKey[0]), room: num(rowKey[1]) });
  const exits = $derived(exitsOf((c) => draft.get(c)));
  const lines = (n: number) => messageLines(n);
  /** Another room's exits, on this map. */
  const far = (n: number) => {
    const r = recordOf('Rooms', [own.map, n]);
    return r ? exitsOf((c) => r[c]) : r;
  };

  const words = $derived(wordsOf(exits, own, lines, far));
  const cmd = $derived(num(draft.get('Command Text')));
  const block = $derived(cmd ? blockOf(cmd) : null);
  const commands = $derived(block ? linesOf(block.text, 'commands') : []);
  /** The words the exits answer, which the game tries before the room's commands. */
  const exitWords = $derived(words.flatMap((w) => (w.words ?? []).map((t) => ({ t, dir: w.exit.dir }))));
  const SHOWN = 30;

  const ways = $derived(waysOf(exits).map((e) => wayOf(e, lines)));
  /** Ways with nothing to say past where they lead: one row of chips. */
  const plain = $derived(ways.filter((w) => !w.says.length && !w.more.length));
  const told = $derived(ways.filter((w) => w.says.length || w.more.length));
  const obvious = $derived(obviousExits(exits, lines));

  const toKey = (w: Way): Cell[] => [mapOf(w.exit, own.map), w.exit.to];
  const shadowOf = (pattern: string) => exitWords.find((x) => wildcardMatch(pattern, x.t));

  function ansi(node: HTMLElement, text: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(text);
    return { update: draw };
  }
</script>

<div class="typed">
  {#if words.length || cmd}
    <p class="how">After the game's own commands, it tries the words of exits, then the room's commands. Words must be the whole line, in any case{cmd ? ", and a command's * lets the rest of its words be cut short" : ''}.</p>
  {:else}
    <p class="how">Nothing of its own: no exit answers words, and the room has no commands. Players here have the game's commands and the ways out below.</p>
  {/if}

  {#each words as w (w.exit.dir)}
    {@const Icon = w.kind === 'text' ? Footprints : Hand}
    <div class="ln">
      <div class="pats">
        <span class="faint">Typed</span>
        {#if w.words === undefined}<span class="faint">…</span>
        {:else}
          {#each w.words as t, j (j)}{#if j}<span class="faint">or</span>{/if}<code class="pat">{t}</code>{/each}
          {#if !w.words.length}<span class="faint">nothing works it</span>{/if}
        {/if}
        <span class="from">{w.kind === 'text' ? 'its' : 'the'} {DIRS[w.exit.dir]} exit{w.kind === 'remote' ? "'s words" : ''}</span>
      </div>
      <div class="row"><span class="gi {w.kind === 'text' ? 't-room' : 't-spell'}"><Icon size={14} /></span><span class="said"><Said bits={w.says} map={own.map} /></span></div>
      {#each w.more as m, i (i)}<p class="sub"><Said bits={m} map={own.map} /></p>{/each}
      {#if w.taken}<p class="callout warn"><TriangleAlert size={15} /><span>{w.taken}</span></p>{/if}
    </div>
  {/each}

  {#if cmd}
    <div class="cmds">
      <span class="faint">The room's commands</span><RefChip table="Textblocks" key={[cmd, 0]} compact />
      {#if block === null}<span class="bad">There is no text block {cmd}, so the room has no commands.</span>{/if}
    </div>
    {#each commands.slice(0, SHOWN) as l (l.no)}
      {@const shadow = shadowOf(l.pattern ?? '')}
      <div class="ln">
        <div class="pats">
          <span class="faint">Typed</span>
          {#each alternatives(l.pattern ?? '') as a, j (j)}{#if j}<span class="faint">or</span>{/if}<code class="pat">{a.replace(/\r/g, '') || ' '}</code>{/each}
        </div>
        <Steps steps={l.steps} map={own.map} />
        {#if shadow}<p class="callout warn"><TriangleAlert size={15} /><span>“{shadow.t}” is the {DIRS[shadow.dir]} exit's words, which the game tries first, so typed that way this never runs.</span></p>{/if}
      </div>
    {/each}
    {#if commands.length > SHOWN}<p class="faint">+{commands.length - SHOWN} more commands</p>{/if}
    {#if block && !commands.length}<p class="sub">No line of it is <code>words:steps</code>, so nothing typed runs it.</p>{/if}
  {/if}

  {#if words.length || cmd}
    <a class="btn sm try" href={storyHref('Rooms', rowKey)}><PlayIcon size={13} /> Try it as a player</a>
  {/if}

  <div class="eyebrow ways-h">Ways out</div>
  {#if obvious}<pre class="terminal exits" use:ansi={obvious}></pre>{/if}
  {#if plain.length}
    <div class="plain">
      {#each plain as w (w.exit.dir)}
        <span class="pw"><b class="dir" title="Typed as {DIRS[w.exit.dir]} or {SHORT[w.exit.dir]}">{DIRS[w.exit.dir]}</b><RefChip table="Rooms" key={toKey(w)} compact /></span>
      {/each}
    </div>
  {/if}
  {#each told as w (w.exit.dir)}
    <div class="way" class:never={!w.walks}>
      <span class="gi {w.walks ? 't-room' : 't-other'}"><DoorOpen size={14} /></span>
      <div class="body">
        <div class="lead">
          <b class="dir" title="Typed as {DIRS[w.exit.dir]} or {SHORT[w.exit.dir]}">{DIRS[w.exit.dir]}</b>
          {#if w.exit.to}<RefChip table="Rooms" key={toKey(w)} compact />{/if}
          {#if w.says.length}<span class="said"><Said bits={w.says} map={own.map} /></span>{/if}
        </div>
        {#if w.word && w.word !== DIRS[w.exit.dir]}<p class="sub">Among the exits as “{w.word}”.</p>{/if}
        {#each w.more as m, i (i)}<p class="sub"><Said bits={m} map={own.map} /></p>{/each}
      </div>
    </div>
  {/each}
  {#if !ways.length}<p class="sub">No exits: players need a spell, a script or the sysop to leave.</p>{/if}
</div>

<style>
  .typed { display: grid; gap: 8px; min-width: 0; }
  .how { margin: 0; color: var(--ink-3); font-size: 12px; font-weight: 700; line-height: 1.45; }
  .ln { display: grid; gap: 6px; padding: 8px 10px; border-radius: 12px; background: var(--surface-2); }
  .pats { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 12px; font-weight: 800; }
  .pat { font: 700 12.5px var(--font-mono); padding: 2px 8px; border-radius: 8px; background: var(--surface); border: 1px solid var(--line); white-space: pre; }
  .from { margin-left: auto; color: var(--ink-3); font-size: 11.5px; }
  .row { display: grid; grid-template-columns: 24px minmax(0, 1fr); gap: 8px; align-items: start; }
  .gi { width: 24px; height: 24px; border-radius: 8px; display: grid; place-items: center; background: var(--t-bg); color: var(--t-ink); flex: none; }
  .said { font-size: 13px; font-weight: 700; line-height: 26px; color: var(--ink); overflow-wrap: anywhere; }
  .sub { margin: 0; color: var(--ink-3); font-size: 12px; font-weight: 700; line-height: 1.5; overflow-wrap: anywhere; }
  .ln .sub { margin-left: 32px; }
  .callout { margin: 0; }
  .cmds { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-top: 4px; font-size: 12.5px; font-weight: 800; }
  .bad { color: var(--bad); font-weight: 700; }
  .try { justify-self: start; }
  .ways-h { margin-top: 6px; }
  .exits { width: 100%; min-width: 0; font-size: 12.5px; }
  .plain { display: flex; flex-wrap: wrap; gap: 6px 14px; align-items: center; }
  .pw { display: inline-flex; gap: 6px; align-items: center; min-width: 0; }
  .dir { font: 800 12.5px var(--font-ui); color: var(--ink-2); }
  .way { display: grid; grid-template-columns: 24px minmax(0, 1fr); gap: 8px; align-items: start; padding: 6px 0; border-top: 1px dashed var(--line); }
  .way.never .dir { color: var(--ink-3); }
  .body { display: grid; gap: 3px; min-width: 0; }
  .lead { display: flex; flex-wrap: wrap; gap: 4px 8px; align-items: center; min-height: 26px; }
  .lead .said { line-height: 1.5; }
  code { font: 700 12px var(--font-mono); }
</style>
