<!--
  A family of numbered columns on a record card: only the slots in use (a monster's drops, an
  item's abilities, a room's exits), each cleared with one button, and the next free slot
  opened with Add. Separate lists (who may use an item) are sets of chips; a slot of many
  columns (an exit, a shop's stock) is a line of labelled fields; a slot of many rows (a
  monster's attack) is a block. A room's exits name their parameters by the exit's type, and
  show only those the type uses (or that hold something).
-->
<svelte:options css="injected" />

<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import { exitParam, targetOf, type Grid } from '../layouts';
  import { kindOf } from '../shell/kinds';
  import { recordChoices, refKey } from './choices';
  import type { Draft } from './draft.svelte';
  import FieldInput from './FieldInput.svelte';
  import Picker from './Picker.svelte';
  import RefChip from './RefChip.svelte';
  import { slotCol, slotsInUse } from './sections';

  let { draft, grid, table }: { draft: Draft; grid: Grid; table: string } = $props();

  const MAP_CHANGE = 8;
  const mode = $derived(grid.separate ? 'sets' : grid.across ? 'blocks' : grid.names || grid.columns.length > 4 ? 'lines' : 'table');
  const id = (n: number) => `${grid.columns[0]!.template}:${n}`;
  const used = $derived(slotsInUse(grid, draft.get));
  const slots = $derived.by(() => {
    const out: number[] = [];
    for (let n = grid.from; n <= grid.to; n++) if (used.includes(n) || draft.opened[id(n)]) out.push(n);
    return out;
  });
  const next = $derived.by(() => {
    for (let n = grid.from; n <= grid.to; n++) if (!slots.includes(n)) return n;
    return null;
  });
  const exits = $derived(table === 'Rooms' && grid.columns[0]!.template === 'Exit #');
  const own = $derived(Number(draft.get('Map Number')) || 0);

  const slotName = (n: number) => grid.names?.[n - grid.from] ?? (grid.across ? `${grid.across} ${n - grid.from + 1}` : String(n));
  const num = (col: string) => Number(draft.get(col)) || 0;
  const paraOf = (template: string) => /^(Para\d) #$/.exec(template)?.[1];

  /** What an exit's parameter means for its type, and the table it names. */
  const param = (template: string, n: number) => {
    const p = paraOf(template);
    return exits && p ? exitParam(num(`Type ${n}`), p) : undefined;
  };

  /** The table a cell's number names: an exit's parameter by the exit's type. */
  function cellTarget(template: string, n: number): string | undefined {
    if (exits && paraOf(template)) return param(template, n)?.links;
    return targetOf(table, template);
  }

  /** Whether a cell shows: an exit's parameter only when its type uses it or it holds something. */
  const cellShown = (template: string, n: number) => !exits || !paraOf(template) || !!param(template, n) || num(slotCol(template, n)) !== 0;

  /** The map a room number of slot `n` is on: a Map Change exit's Para1, else the record's. */
  const mapOf = (n: number) => (exits && num(`Type ${n}`) === MAP_CHANGE ? num(`Para1 ${n}`) : own);

  function add() {
    if (next === null) return;
    draft.opened[id(next)] = true;
  }

  function clear(n: number) {
    for (const c of grid.columns) {
      const col = slotCol(c.template, n);
      if (draft.has(col)) draft.set(col, draft.isNumber(col) ? '0' : '');
    }
    draft.opened[id(n)] = false;
  }

  // ---- sets: each column its own list ----

  const setOf = (template: string) => {
    const out: number[] = [];
    for (let n = grid.from; n <= grid.to; n++) if (draft.has(slotCol(template, n)) && num(slotCol(template, n)) !== 0) out.push(n);
    return out;
  };
  const freeIn = (template: string) => {
    for (let n = grid.from; n <= grid.to; n++) if (draft.has(slotCol(template, n)) && num(slotCol(template, n)) === 0) return n;
    return null;
  };
</script>

{#if mode === 'sets'}
  <div class="sets">
    {#each grid.columns as c (c.template)}
      {@const target = targetOf(table, c.template)}
      {@const members = setOf(c.template)}
      {@const free = freeIn(c.template)}
      <div class="set">
        <span class="lbl">{c.label ?? c.template}</span>
        <div class="members">
          {#each members as n (n)}
            {@const col = slotCol(c.template, n)}
            <span class="member" class:changed={draft.edited(col)}>
              {#if target}<RefChip table={target} key={refKey(target, num(col))} compact />{:else}<span class="pill">{num(col)}</span>{/if}
              <button type="button" class="btn icon sm ghost" aria-label="Take {num(col)} off {c.label ?? c.template}" title="Take it off" onclick={() => draft.set(col, '0')}><X size={13} /></button>
            </span>
          {/each}
          {#if !members.length}<span class="faint none">none</span>{/if}
          {#if target && free !== null}
            <Picker what="a {kindOf(target).one.toLowerCase()}" label="Add" choices={() => recordChoices(target)} onpick={(v) => draft.set(slotCol(c.template, free), String(v))} />
          {/if}
        </div>
      </div>
    {/each}
  </div>
{:else if slots.length}
  {#if mode === 'blocks'}
    <div class="blocks">
      {#each slots as n (n)}
        <div class="block">
          <div class="block-h">
            <span class="eyebrow">{slotName(n)}</span>
            <button type="button" class="btn icon sm ghost" aria-label="Clear {slotName(n)}" title="Clear {slotName(n)}" onclick={() => clear(n)}><X size={14} /></button>
          </div>
          <div class="pairs">
            {#each grid.columns as c (c.template)}
              {@const col = slotCol(c.template, n)}
              {#if draft.has(col)}
                <span class="pair">
                  <span class="lbl">{c.label ?? c.template}</span>
                  <FieldInput {draft} {col} pick={c.pick} target={cellTarget(c.template, n)} map={mapOf(n)} label="{slotName(n)} {c.label ?? c.template}" compact />
                </span>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else if mode === 'lines'}
    <div class="lines">
      {#each slots as n (n)}
        <div class="line">
          <span class="n">{slotName(n)}</span>
          <div class="cells">
            {#each grid.columns as c, i (c.template)}
              {@const col = slotCol(c.template, n)}
              {@const p = param(c.template, n)}
              {#if draft.has(col) && cellShown(c.template, n)}
                <span class="cell">
                  {#if i > 0}<span class="lbl" title={p ? `${p.means} (${c.label ?? c.template})` : undefined}>{p?.means ?? c.label ?? c.template}</span>{/if}
                  <FieldInput
                    {draft}
                    {col}
                    pick={c.pick}
                    target={cellTarget(c.template, n)}
                    map={mapOf(n)}
                    hint={p?.means}
                    label="{slotName(n)} {c.label ?? c.template}"
                    compact
                  />
                </span>
              {/if}
            {/each}
          </div>
          <button type="button" class="btn icon sm ghost" aria-label="Clear {slotName(n)}" title="Clear this slot" onclick={() => clear(n)}><X size={14} /></button>
        </div>
      {/each}
    </div>
  {:else}
    <div class="scroll">
      <table class="slots">
        <thead>
          <tr>
            <th></th>
            {#each grid.columns as c (c.template)}<th>{c.label ?? c.template}</th>{/each}
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each slots as n (n)}
            <tr>
              <td class="n">{slotName(n)}</td>
              {#each grid.columns as c (c.template)}
                {@const col = slotCol(c.template, n)}
                <td>
                  {#if draft.has(col)}
                    <FieldInput {draft} {col} pick={c.pick} target={cellTarget(c.template, n)} map={mapOf(n)} label="{slotName(n)} {c.label ?? c.template}" compact />
                  {/if}
                </td>
              {/each}
              <td>
                <button type="button" class="btn icon sm ghost" aria-label="Clear {slotName(n)}" title="Clear this slot" onclick={() => clear(n)}><X size={14} /></button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{:else}
  <p class="faint none">None.</p>
{/if}
{#if mode !== 'sets' && next !== null}
  <button type="button" class="btn sm add" onclick={add}><Plus size={14} /> Add</button>
{/if}

<style>
  .scroll { overflow-x: auto; max-width: 100%; }
  .slots { border-collapse: collapse; }
  .slots th { text-align: left; font: 800 11px/1.2 var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: var(--ink-3); padding: 0 8px 6px 0; white-space: nowrap; }
  .slots td { padding: 3px 8px 3px 0; vertical-align: middle; }
  .slots td.n { color: var(--ink-3); font: 700 12px var(--font-mono); text-align: right; padding-right: 10px; white-space: nowrap; }
  .blocks { display: grid; gap: 10px; }
  .block { border: 1px solid var(--line); border-radius: var(--r); padding: 8px 10px 10px; background: var(--surface-2); }
  .block-h { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; }
  .pairs { display: grid; grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr)); gap: 6px 14px; }
  .pair { display: grid; grid-template-columns: 7.5rem minmax(0, 1fr); align-items: center; gap: 8px; }
  .lbl { color: var(--ink-2); font-size: 12px; font-weight: 800; }
  .lines { display: grid; }
  .line { display: grid; grid-template-columns: 2.6rem minmax(0, 1fr) auto; align-items: start; gap: 8px; padding: 7px 0; border-bottom: 1px dashed var(--line); }
  .line:last-child { border-bottom: 0; }
  .line .n { color: var(--ink-2); font: 800 12.5px/30px var(--font-mono); }
  .cells { display: flex; flex-wrap: wrap; gap: 6px 14px; align-items: center; min-width: 0; }
  .cell { display: inline-flex; align-items: center; gap: 6px; min-width: 0; max-width: 100%; }
  .cell .lbl { max-width: 13rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sets { display: grid; gap: 8px; }
  .set { display: grid; grid-template-columns: 8.5rem minmax(0, 1fr); align-items: center; gap: 10px; padding: 4px 0; }
  .members { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; min-width: 0; }
  .member { display: inline-flex; align-items: center; gap: 1px; border-radius: 999px; min-width: 0; max-width: 100%; }
  .member.changed { background: var(--changed); }
  .none { margin: 0 0 6px; }
  .members .none { margin: 0; }
  .add { margin-top: 4px; justify-self: start; }
  @container (max-width: 400px) {
    .set { grid-template-columns: minmax(0, 1fr); gap: 4px; }
    .line { grid-template-columns: 2rem minmax(0, 1fr) auto; }
  }
</style>
