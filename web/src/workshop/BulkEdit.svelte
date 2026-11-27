<!--
  Bulk edit, NMR's Universal Modifier: a field (or an ability) changed on every record of a
  table that matches, with "only if" conditions, and every value it changes previewed first.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import { ABILITIES } from '../abilities';
  import { api } from '../api';
  import { kindOf } from '../shell/kinds';
  import { app } from '../shell/state.svelte';
  import { columnsOf, decimal, fieldName, hasAbilities, numberFields, rangeOf, RUN_TABLES, type Span } from './fields';
  import Run from './Run.svelte';
  import Seg from './Seg.svelte';
  import SpanPick from './SpanPick.svelte';

  type Kind = 'math' | 'change_ability' | 'give_ability' | 'take_ability';
  type Op = 'add' | 'sub' | 'mul' | 'div' | 'percent' | 'set';
  type Cmp = 'eq' | 'le' | 'ge' | 'ne';
  interface Condition {
    kind: 'column' | 'has_ability' | 'lacks_ability';
    column: string;
    ability: string;
    cmp: Cmp;
    value: string;
  }

  const OPS: [Op, string][] = [['add', '+'], ['sub', '−'], ['mul', '×'], ['div', '÷'], ['percent', '%'], ['set', '=']];
  const CMPS: [Cmp, string][] = [['eq', '='], ['le', '≤'], ['ge', '≥'], ['ne', '≠']];
  const ABILITY_LIST = Object.entries(ABILITIES)
    .filter(([n]) => n !== '0')
    .map(([n, [name]]) => [n, `${n}: ${name}`] as const);

  let table = $state('Items');
  let span = $state<Span>({ all: true, map: '', from: '', to: '' });
  let kind = $state<Kind>('math');
  let field = $state('');
  let op = $state<Op>('add');
  let value = $state('0');
  let limit = $state<'' | 'le' | 'ge'>('');
  let limitAt = $state('');
  let ability = $state('1');
  let clear = $state(false);
  let conditions = $state<Condition[]>([]);

  const tables = $derived(RUN_TABLES.filter((n) => app.tables.some((x) => x.name === n)));
  const t = $derived(app.tables.find((x) => x.name === table));
  const fields = $derived(t ? numberFields(t) : []);
  const plainFields = $derived(t ? numberFields(t, false) : []);
  const abilities = $derived(!!t && hasAbilities(t));
  const many = $derived(kindOf(table).many.toLowerCase());

  // a new table starts at its first field, and without what it cannot do
  $effect(() => {
    if (!fields.some(([v]) => v === field)) field = fields[0]?.[0] ?? '';
  });
  $effect(() => {
    if (!abilities && kind !== 'math') kind = 'math';
  });

  function pickTable(name: string) {
    table = name;
    conditions = [];
  }

  const add = () => conditions.push({ kind: 'column', column: plainFields[0]?.[0] ?? '', ability: '1', cmp: 'eq', value: '0' });

  const n = $derived(decimal(value));
  const lim = $derived(decimal(limitAt));
  const range = $derived(rangeOf(table, span));

  const blocked = $derived.by(() => {
    if (!t) return 'The world has no such table.';
    if ('error' in range) return range.error;
    if (kind === 'math' && !field) return 'Pick the field to change.';
    if (kind !== 'take_ability' && Number.isNaN(n)) return 'Give the value as a number.';
    if (kind === 'give_ability' && !Number.isInteger(n)) return 'An ability’s value is a whole number.';
    if (op === 'div' && n === 0 && (kind === 'math' || kind === 'change_ability')) return 'Dividing by 0 is no change anyone wants.';
    if (limit && Number.isNaN(lim)) return 'Give the limit as a number.';
    for (const c of conditions) if (Number.isNaN(decimal(c.value))) return 'Give each condition’s value as a number.';
    return '';
  });

  const body = $derived.by(() => {
    const held = limit && !Number.isNaN(lim) ? [limit, lim] : null;
    const action =
      kind === 'math'
        ? { kind, columns: t ? columnsOf(t, field) : [], op, value: n, limit: held }
        : kind === 'change_ability'
          ? { kind, ability: Number(ability), op, value: n, limit: held }
          : kind === 'give_ability'
            ? { kind, ability: Number(ability), value: n }
            : { kind, ability: Number(ability), clear_value: clear };
    const only_if = conditions.map((c) =>
      c.kind === 'column'
        ? { kind: c.kind, column: c.column, cmp: c.cmp, value: decimal(c.value) }
        : { kind: c.kind, ability: Number(c.ability), cmp: c.cmp, value: decimal(c.value) },
    );
    return { table, action, range: 'range' in range ? range.range : null, only_if };
  });

  const said = $derived(
    kind === 'math'
      ? fieldName(field)
      : kind === 'give_ability'
        ? `ability ${ability} given`
        : kind === 'take_ability'
          ? `ability ${ability} taken`
          : `ability ${ability}`,
  );
</script>

<div class="ws-form">
  <div class="row">
    <span class="lbl">Table</span>
    <select value={table} onchange={(e) => pickTable(e.currentTarget.value)} aria-label="Table">
      {#each tables as n (n)}<option value={n}>{kindOf(n).many}</option>{/each}
    </select>
  </div>
  <SpanPick bind:span {table} label="Which {many}" />

  <div class="row">
    <span class="lbl">Change</span>
    <Seg
      label="Change"
      options={abilities
        ? [['math', 'A field'], ['change_ability', 'An ability’s value'], ['give_ability', 'Give an ability'], ['take_ability', 'Take an ability']]
        : [['math', 'A field']]}
      bind:value={kind}
    />
  </div>

  <div class="row">
    <span class="lbl"></span>
    {#if kind === 'math'}
      <select bind:value={field} aria-label="Field">
        {#each fields as [v, l] (v)}<option value={v}>{l}</option>{/each}
      </select>
    {:else}
      <select bind:value={ability} aria-label="Ability" class="ability">
        {#each ABILITY_LIST as [v, l] (v)}<option value={v}>{l}</option>{/each}
      </select>
    {/if}
    {#if kind === 'math' || kind === 'change_ability'}
      <Seg label="How" options={OPS} bind:value={op} />
    {/if}
    {#if kind !== 'take_ability'}
      <input class="num" inputmode="decimal" aria-label="Value" bind:value />
    {:else}
      <label class="check"><input type="checkbox" bind:checked={clear} /> Clear its value too</label>
    {/if}
  </div>

  {#if kind === 'math' || kind === 'change_ability'}
    <div class="row">
      <span class="lbl">Hold it</span>
      <select bind:value={limit} aria-label="Hold it">
        <option value="">as it comes</option>
        <option value="le">at most</option>
        <option value="ge">at least</option>
      </select>
      {#if limit}<input class="num" inputmode="decimal" aria-label="Limit" bind:value={limitAt} />{/if}
    </div>
  {/if}

  <div class="conds">
    {#each conditions as c, i (i)}
      <div class="row cond">
        <span class="lbl">{i ? 'and only if' : 'Only if'}</span>
        <select bind:value={c.kind} aria-label="Condition">
          <option value="column">a field</option>
          {#if abilities}<option value="has_ability">it has an ability</option><option value="lacks_ability">it lacks an ability</option>{/if}
        </select>
        {#if c.kind === 'column'}
          <select bind:value={c.column} aria-label="Field">
            {#each plainFields as [v, l] (v)}<option value={v}>{l}</option>{/each}
          </select>
        {:else}
          <select bind:value={c.ability} aria-label="Ability" class="ability">
            {#each ABILITY_LIST as [v, l] (v)}<option value={v}>{l}</option>{/each}
          </select>
          <span class="faint">with a value</span>
        {/if}
        <select bind:value={c.cmp} aria-label="Compared">
          {#each CMPS as [v, l] (v)}<option value={v}>{l}</option>{/each}
        </select>
        <input class="num" inputmode="decimal" aria-label="Value" bind:value={c.value} />
        <button type="button" class="btn icon sm ghost" aria-label="Take this condition away" onclick={() => conditions.splice(i, 1)}><X size={14} /></button>
      </div>
    {/each}
    <button type="button" class="btn sm" onclick={add}><Plus size={14} />{conditions.length ? 'And only if…' : 'Only if…'}</button>
  </div>
</div>

<Run run={(dry) => api.modify(body, dry)} key={JSON.stringify(body)} {blocked} done={(k) => `Changed ${said} on ${k.toLocaleString()} ${k === 1 ? kindOf(table).one.toLowerCase() : many}.`} />

<style>
  .ws-form { display: flex; flex-direction: column; gap: 10px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  @media (max-width: 560px) { .lbl { flex-basis: 100%; } }
  .check { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; font-size: 13px; }
  .check input { flex: none; }
  .num { width: 6.5em; font-family: var(--font-mono); }
  .ability { max-width: min(22em, 100%); }
  .conds { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
  .cond { padding: 6px 8px; margin-left: -8px; border-radius: 12px; background: var(--surface-2); }
  .faint { font-size: 12.5px; font-weight: 700; }
</style>
