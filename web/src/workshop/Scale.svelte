<!--
  Balance › Scale a field: one number multiplied (or divided) on every record that matches,
  such as the experience monsters give. NMR's Multiply All Exp, Divide All Exp and Multiply
  Boss Exp, with any factor and a preview of every value it changes.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { api } from '../api';
  import { kindOf } from '../shell/kinds';
  import { app } from '../shell/state.svelte';
  import { columnsOf, fieldName, hasColumn, numberFields, rangeOf, whole, type Span } from './fields';
  import Run from './Run.svelte';
  import Seg from './Seg.svelte';
  import SpanPick from './SpanPick.svelte';

  /** The tables worth scaling, and the field each starts at. */
  const TABLES: [string, string][] = [
    ['Monsters', 'Experience'],
    ['Items', 'Cost'],
    ['Spells', 'Mana'],
    ['Shops', 'MarkUp'],
    ['Classes', 'EXP %'],
    ['Races', 'EXP %'],
  ];
  const CHIPS = [0.5, 1.5, 2, 3];

  let table = $state('Monsters');
  let field = $state('Experience');
  let factor = $state<number | null>(1.5);
  let who = $state<'all' | 'bosses' | 'span'>('all');
  let span = $state<Span>({ all: false, map: '', from: '', to: '' });
  let capOn = $state(false);
  let cap = $state('');

  const t = $derived(app.tables.find((x) => x.name === table));
  const fields = $derived(t ? numberFields(t) : []);
  const bosses = $derived(hasColumn(t, 'Game Limit'));
  const many = $derived(kindOf(table).many.toLowerCase());

  function pickTable(name: string) {
    table = name;
    field = TABLES.find(([n]) => n === name)?.[1] ?? '';
    if (who === 'bosses' && !hasColumn(app.tables.find((x) => x.name === name), 'Game Limit')) who = 'all';
  }

  const f = $derived(typeof factor === 'number' && Number.isFinite(factor) ? factor : NaN);
  const range = $derived(who === 'span' ? rangeOf(table, span) : { range: null });
  const capN = $derived(whole(cap));

  const blocked = $derived(
    !t
      ? 'The world has no such table.'
      : !(f > 0)
        ? 'Give a factor above 0: 2 doubles, 0.5 halves.'
        : 'error' in range
          ? range.error
          : capOn && Number.isNaN(capN)
            ? 'Give the most it may be as a whole number.'
            : '',
  );

  const body = $derived({
    table,
    action: { kind: 'math', columns: t ? columnsOf(t, field) : [], op: 'mul', value: f, limit: capOn && !Number.isNaN(capN) ? ['le', capN] : null },
    range: 'range' in range ? range.range : null,
    only_if: who === 'bosses' ? [{ kind: 'column', column: 'Game Limit', cmp: 'eq', value: 1 }] : [],
  });

  const label = $derived(
    `${fieldName(field)} × ${f} on ${who === 'bosses' ? `${many} with Game Limit 1` : who === 'span' ? `${many} ${span.from} to ${span.to}` : many}`,
  );
</script>

<div class="ws-form">
  <div class="row">
    <span class="lbl">Field</span>
    <select value={table} onchange={(e) => pickTable(e.currentTarget.value)} aria-label="Table">
      {#each TABLES.filter(([n]) => app.tables.some((x) => x.name === n)) as [n] (n)}<option value={n}>{kindOf(n).many}</option>{/each}
    </select>
    <select bind:value={field} aria-label="Field">
      {#each fields as [v, l] (v)}<option value={v}>{l}</option>{/each}
    </select>
  </div>

  <div class="row">
    <span class="lbl">Which {many}</span>
    <Seg
      label="Which {many}"
      options={bosses ? [['all', 'All'], ['bosses', 'Game Limit 1'], ['span', 'A run of numbers']] : [['all', 'All'], ['span', 'A run of numbers']]}
      bind:value={who}
    />
  </div>
  {#if who === 'span'}<SpanPick bind:span {table} label="Numbers" />{/if}
  {#if who === 'bosses'}<p class="hint">Only one of each can be in the world at a time: most are bosses.</p>{/if}

  <div class="row">
    <span class="lbl">Times</span>
    <input class="num factor" type="number" min="0.01" step="0.05" aria-label="Factor" bind:value={factor} />
    <input class="slide" type="range" min="0.25" max="4" step="0.25" aria-label="Factor" bind:value={factor} />
    {#each CHIPS as c (c)}
      <button type="button" class="btn sm" class:on={f === c} onclick={() => (factor = c)}>×{c}</button>
    {/each}
  </div>

  <div class="row">
    <label class="check"><input type="checkbox" bind:checked={capOn} /> Keep it at most</label>
    <input class="num" inputmode="numeric" aria-label="The most it may be" disabled={!capOn} bind:value={cap} />
  </div>
  <p class="hint">
    Values are rounded as NMR rounds them (halves to even) and held inside what the field can store.
    {#if table === 'Monsters' && field === 'Experience'}A monster gives its Experience times its Exp Multiplier, so this scales what it gives.{/if}
  </p>
</div>

<Run run={(dry) => api.modify(body, dry, label)} key={JSON.stringify(body)} {blocked} done={(n) => `Scaled ${fieldName(field)} on ${n.toLocaleString()} ${n === 1 ? kindOf(table).one.toLowerCase() : many}.`} />

<style>
  .ws-form { display: flex; flex-direction: column; gap: 10px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  @media (max-width: 560px) { .lbl { flex-basis: 100%; } }
  .check { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; font-size: 13px; }
  .check input { flex: none; }
  .num { width: 6.5em; font-family: var(--font-mono); }
  .factor { font-size: 16px; font-weight: 700; }
  .slide { width: min(220px, 100%); accent-color: var(--lilac-ink); padding: 0; border: 0; background: none; }
  .btn.on { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 70ch; }
</style>
