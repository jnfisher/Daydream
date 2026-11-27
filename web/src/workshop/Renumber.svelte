<!--
  Renumber: records move to new numbers (rooms within one map), or a whole map's rooms move to
  another map. NMR's Record Number Changer and Change Map Number: every record, ability and
  script that names what moves follows, and the preview lists each one before it runs. A card's
  More menu opens it with that record filled in (`#/workshop/renumber/items/868`).
-->
<svelte:options css="injected" />

<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import { untrack } from 'svelte';
  import { api, type Renumber } from '../api';
  import { recordChoices } from '../library/choices';
  import Picker from '../library/Picker.svelte';
  import { kindOf } from '../shell/kinds';
  import { app, rowsOf } from '../shell/state.svelte';
  import { messageOf } from '../shell/toasts.svelte';
  import { whole } from './fields';
  import { endOf, lineText, movesOf, runsOf, type Line } from './moves';
  import Run from './Run.svelte';
  import Seg from './Seg.svelte';

  /** What the link names: a table and a record's key (`['rooms', '1', '224']`). */
  let { start = [] }: { start?: string[] } = $props();

  /** NMR's list: the names the server takes, and their tables. */
  const KINDS: [string, string][] = [
    ['items', 'Items'],
    ['monsters', 'Monsters'],
    ['spells', 'Spells'],
    ['shops', 'Shops'],
    ['classes', 'Classes'],
    ['races', 'Races'],
    ['rooms', 'Rooms'],
    ['messages', 'Messages'],
    ['textblocks', 'Textblocks'],
  ];
  /** Runs of numbers in use shown before "and N more". */
  const RUNS = 40;

  interface Row extends Line {
    id: number;
  }
  let ids = 0;
  const row = (from = '', to = '', start = ''): Row => ({ id: ++ids, from, to, start });

  const [first, ...rest] = untrack(() => start);
  const known = KINDS.some(([k]) => k === first);
  let what = $state<'records' | 'map'>('records');
  let kind = $state(known ? first! : 'items');
  let map = $state(known && first === 'rooms' ? (rest[0] ?? '1') : '1');
  let lines = $state<Row[]>([row(known ? ((first === 'rooms' ? rest[1] : rest[0]) ?? '') : '')]);
  let users = $state(false);
  let fromMap = $state('');
  let toMap = $state('');
  let exits = $state(true);
  let normal = $state(true);
  let scripts = $state(true);
  let spells = $state(true);
  let allRuns = $state(false);

  const table = $derived(KINDS.find(([k]) => k === kind)?.[1] ?? 'Items');
  const k = $derived(kindOf(table));
  const one = $derived(k.one.toLowerCase());
  const many = $derived(k.many.toLowerCase());
  const rooms = $derived(kind === 'rooms');
  const mapN = $derived(whole(map));

  // the numbers the table uses (a map's rooms), read again as the world changes
  let used = $state<number[] | null>(null);
  let usedError = $state('');
  let asked = 0;
  $effect(() => {
    void app.changes;
    void app.current;
    const [t, m, onMap] = [table, mapN, rooms];
    if (what !== 'records' || (onMap && !(m > 0))) {
      used = null;
      return;
    }
    const my = ++asked;
    usedError = '';
    rowsOf(t)
      .then((rows) => {
        if (my !== asked) return;
        used = rows.flatMap(([key]) => (onMap ? (Number(key[0]) === m ? [Number(key[1])] : []) : t === 'Textblocks' && Number(key[1]) !== 0 ? [] : [Number(key[0])]));
      })
      .catch((e) => my === asked && (usedError = messageOf(e)));
  });
  const runs = $derived(used ? runsOf(used) : []);
  const highest = $derived(runs.at(-1)?.[1] ?? 0);

  /** A run (or one number) to move: into the last line if it is empty, else a line of its own
   *  that starts anew where the line before ends. */
  function take(a: number, b = a) {
    const last = lines.at(-1)!;
    const to = b === a ? '' : String(b);
    if (!last.from.trim()) {
      last.from = String(a);
      last.to = to;
      return;
    }
    const end = whole(endOf(last));
    lines.push(row(String(a), to, Number.isNaN(end) ? '' : String(end + 1)));
  }

  const moves = $derived(movesOf(lines));

  const body = $derived<Renumber>(
    what === 'map'
      ? { kind: 'map', from: whole(fromMap), to: whole(toMap), exits, normal, textblocks: scripts, spells, users }
      : { kind: 'records', table: kind, ...(rooms ? { map: mapN } : {}), moves: 'moves' in moves ? moves.moves : [], users },
  );

  const blocked = $derived.by(() => {
    if (what === 'map') {
      const [a, b] = [whole(fromMap), whole(toMap)];
      if (!(a > 0) || !(b > 0)) return 'Give the map whose rooms move, and the map they move to.';
      return a === b ? 'Give two different maps.' : '';
    }
    if (rooms && !(mapN > 0)) return 'Give the map the rooms are on.';
    return 'error' in moves ? moves.error : '';
  });

  const label = $derived.by(() => {
    if (what === 'map') return `Move map ${fromMap.trim()} to map ${toMap.trim()}`;
    if (!('moves' in moves)) return '';
    const on = rooms ? ` on map ${mapN}` : '';
    const said = lines.map(lineText).filter(Boolean);
    const m = moves.moves;
    if (m.length === 1) return `Renumber ${one} ${m[0]!.from} to ${m[0]!.to}${on}`;
    return said.length === 1 ? `Renumber ${many} ${said[0]}${on}` : `Renumber ${m.length.toLocaleString()} ${many}${on}`;
  });

  function done(n: number): string {
    const all = `${n.toLocaleString()} ${n === 1 ? 'record' : 'records'} changed in all`;
    if (what === 'map') return `Moved map ${fromMap.trim()} to map ${toMap.trim()}: ${all}.`;
    const m = 'moves' in moves ? moves.moves.length : 0;
    return `Renumbered ${m.toLocaleString()} ${m === 1 ? one : many}: ${all}.`;
  }

  /** Once it has run, the numbers are spent: the form starts over, on the same table. */
  function again() {
    if (what === 'records') lines = [row()];
  }

  const article = (word: string) => (/^[aeiou]/i.test(word) ? `an ${word}` : `a ${word}`);
</script>

<div class="ws-form">
  <div class="row">
    <span class="lbl">Move</span>
    <Seg label="What moves" options={[['records', 'Records to new numbers'], ['map', 'A whole map']]} bind:value={what} />
  </div>

  {#if what === 'records'}
    <div class="row">
      <span class="lbl">Records</span>
      <select bind:value={kind} aria-label="Records">
        {#each KINDS as [v, t] (v)}<option value={v}>{kindOf(t).many}</option>{/each}
      </select>
      {#if rooms}<label class="inline">on map <input class="num" inputmode="numeric" aria-label="Map" bind:value={map} /></label>{/if}
    </div>

    <div class="lines" role="group" aria-label="Numbers to change">
      <div class="line head" aria-hidden="true"><span>Old numbers</span><span></span><span>New numbers</span><span></span></div>
      {#each lines as l (l.id)}
        {@const end = endOf(l)}
        <div class="line">
          <span class="span">
            <input class="num" inputmode="numeric" aria-label="First number to move" bind:value={l.from} />
            <Picker what={article(one)} choices={() => recordChoices(table, mapN)} onpick={(v) => (l.from = String(v))} />
            <span class="to">to</span>
            <input class="num" inputmode="numeric" aria-label="Last number to move (blank for one)" placeholder={l.from} bind:value={l.to} />
          </span>
          <span class="arrow" aria-hidden="true">→</span>
          <span class="span">
            <span class="to becomes" aria-hidden="true">becomes</span>
            <input class="num" inputmode="numeric" aria-label="New first number" bind:value={l.start} />
            {#if end && l.to.trim() && l.to.trim() !== l.from.trim()}<span class="to">to <b>{end}</b></span>{/if}
          </span>
          <span>
            {#if lines.length > 1}
              <button type="button" class="btn sm icon ghost" aria-label="Take this line out" title="Take this line out" onclick={() => (lines = lines.filter((x) => x.id !== l.id))}><X size={14} /></button>
            {/if}
          </span>
        </div>
      {/each}
      <div class="add"><button type="button" class="btn sm" onclick={() => lines.push(row())}><Plus size={14} />Another run of numbers</button></div>
    </div>

    <div class="used">
      {#if usedError}
        <p class="hint bad">{usedError}</p>
      {:else if used}
        <p class="hint">
          {#if runs.length}
            {k.many}{rooms ? ` on map ${mapN}` : ''} use these numbers; numbers above {highest.toLocaleString()} are free. Pick a run to move it.
          {:else}
            No {many}{rooms ? ` on map ${mapN}` : ''} yet.
          {/if}
        </p>
        {#if runs.length}
          <div class="runs">
            {#each allRuns ? runs : runs.slice(0, RUNS) as [a, b] (a)}
              <button type="button" class="chip" onclick={() => take(a, b)}>{a === b ? a : `${a}–${b}`}</button>
            {/each}
            {#if runs.length > RUNS}
              <button type="button" class="btn sm ghost" onclick={() => (allRuns = !allRuns)}>{allRuns ? 'Fewer' : `and ${(runs.length - RUNS).toLocaleString()} more`}</button>
            {/if}
          </div>
        {/if}
      {/if}
    </div>

    <label class="check">
      <input type="checkbox" bind:checked={users} /> Players follow too
    </label>
    <p class="hint">
      What players carry, wear and know, their class and race, and where they stand. NMR's changer leaves players alone. New numbers must be free; the preview
      says which are not.
    </p>
  {:else}
    <div class="row">
      <span class="lbl">Map</span>
      <label class="inline">Move the rooms of map <input class="num" inputmode="numeric" aria-label="Map whose rooms move" bind:value={fromMap} /></label>
      <label class="inline">to map <input class="num" inputmode="numeric" aria-label="Map they move to" bind:value={toMap} /></label>
    </div>
    <div class="checks">
      <label class="check"><input type="checkbox" bind:checked={exits} /> Map change exits into it lead to the new map</label>
      <label class="check"><input type="checkbox" bind:checked={normal} /> A map change exit that then leads to its own map becomes a normal exit</label>
      <label class="check"><input type="checkbox" bind:checked={scripts} /> Text blocks' teleport commands follow</label>
      <label class="check"><input type="checkbox" bind:checked={spells} /> Spells' Teleport Map abilities follow</label>
      <label class="check"><input type="checkbox" bind:checked={users} /> Players on the map, and their trails, follow too</label>
    </div>
    <p class="hint">The rooms keep their numbers. The new map must not have rooms with the same numbers.</p>
  {/if}
</div>

<Run run={(dry) => api.renumber(body, dry, label)} key={JSON.stringify(body)} {blocked} {done} onapplied={again} plain />

<style>
  .ws-form { display: flex; flex-direction: column; gap: 10px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  .inline { display: inline-flex; align-items: center; gap: 6px; font-weight: 800; font-size: 12.5px; color: var(--ink-2); }
  .num { width: 6.5em; font-family: var(--font-mono); }
  /* one grid for every line, so the columns line up under their heads */
  .lines { display: grid; grid-template-columns: max-content 1.5em max-content minmax(0, auto); align-items: center; gap: 6px 8px; justify-content: start; }
  .line { display: contents; }
  .line.head span { font: 800 10.5px/1.2 var(--font-ui); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink-3); }
  .add { grid-column: 1 / -1; }
  .span { display: inline-flex; align-items: center; gap: 6px; }
  .to { font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  .to b { font-family: var(--font-mono); color: var(--ink-2); }
  .arrow { text-align: center; font-weight: 800; color: var(--lilac-ink); }
  /* on a phone the heads go, and this says which number is new */
  .becomes { display: none; }
  .used { display: flex; flex-direction: column; gap: 6px; }
  .runs { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; max-height: 160px; overflow: auto; }
  .chip { min-height: 26px; padding: 0 9px; border-radius: 999px; font: 700 12px var(--font-mono); background: var(--surface-2); border: 1px solid var(--line); color: var(--ink-2); box-shadow: none; }
  .chip:hover { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); }
  .checks { display: flex; flex-direction: column; gap: 6px; }
  .check { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; font-size: 13px; }
  .check input { flex: none; }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 75ch; }
  .hint.bad { color: var(--bad); }
  @media (max-width: 560px) {
    .lbl { flex-basis: 100%; }
    /* a line a row of its own: the old numbers, then the new beside its button */
    .lines { grid-template-columns: minmax(0, 1fr) auto; }
    .line .arrow, .line.head { display: none; }
    .line > .span:first-child { grid-column: 1 / -1; }
    .becomes { display: inline; }
  }
</style>
