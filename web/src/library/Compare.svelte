<!--
  A record beside others of its table (compare.ts): its card's fields under its headings, a
  column for each record, and the values that differ from the open one marked. A click on one
  types it into the open record's field, so Save keeps it. It starts with the closest of its
  peers (or the records beside it in the list), and any record of the table can be added.
-->
<svelte:options css="injected" />

<script lang="ts">
  import GitCompare from '@lucide/svelte/icons/git-compare';
  import X from '@lucide/svelte/icons/x';
  import { Dialog } from 'bits-ui';
  import { ABILITIES } from '../abilities';
  import { titleOf } from '../ansi';
  import { api, type Cell, type TableInfo } from '../api';
  import { pickFor, targetOf, type Pick } from '../layouts';
  import { kindOf } from '../shell/kinds';
  import { sfx } from '../shell/prefs.svelte';
  import { messageOf } from '../shell/toasts.svelte';
  import type { CardDef } from './cards';
  import { recordChoices } from './choices';
  import { around, closest, compareFields, plainText, same } from './compare';
  import { shown, type Draft } from './draft.svelte';
  import { keyLabel, type ListRow } from './list';
  import { nameOf } from './names.svelte';
  import Picker from './Picker.svelte';
  import { refKey } from './problems';
  import type { Section } from './sections';

  let {
    open = $bindable(false),
    draft,
    table,
    rows,
    sections,
    def,
  }: {
    open: boolean;
    draft: Draft;
    table: TableInfo;
    /** The table's list: who the closest are found among. */
    rows: ListRow[];
    /** The card's sections, whose fields are compared. */
    sections: Section[];
    def: CardDef | undefined;
  } = $props();

  /** The most records beside the open one. */
  const MAX = 3;

  interface Other {
    key: Cell[];
    label: string;
    name: string;
    values: Record<string, Cell> | null;
    error?: string;
  }
  let others = $state<Other[]>([]);
  let onlyDiff = $state(true);
  /** Fields a value was copied into, which stay in view. */
  let touched = $state<Record<string, boolean>>({});

  const k = $derived(kindOf(table.name));
  const one = $derived(k.one.toLowerCase());
  const mine = $derived(keyLabel(draft.row?.key ?? []));
  const nameOfRow = (values: Record<string, Cell>) => (table.title ? titleOf(shown(values[table.title] ?? null)) : '');
  const myName = $derived(table.title ? titleOf(shown(draft.get(table.title))) : '');
  const map = $derived(Number(draft.get('Map Number')) || 0);
  const groups = $derived(compareFields(sections, table.key, (c) => draft.has(c)));

  // the closest of its peers, when it opens
  $effect(() => {
    if (open && !others.length) start();
  });

  function start() {
    let pool = rows;
    const peers = def?.peers;
    if (peers) {
      const like = rows.filter((r) => r.label === mine || peers.same(draft.get, r.get));
      if (like.some((r) => r.label !== mine)) pool = like;
    }
    const at = pool.findIndex((r) => r.label === mine);
    const cols = (def?.measures ?? []).filter((c) => def?.listCols?.includes(c) && draft.isNumber(c));
    const picked = cols.length ? closest(draft.get, pool.filter((_, i) => i !== at), cols, MAX) : around(pool, at, MAX);
    for (const r of picked) add(r.key, r.name);
  }

  function add(key: Cell[], name = '') {
    const label = keyLabel(key);
    if (label === mine || others.length >= MAX || others.some((o) => o.label === label)) return;
    others.push({ key, label, name, values: null });
    const find = () => others.find((o) => o.label === label);
    api
      .row(table.name, key)
      .then((r) => {
        const o = find();
        if (!o) return;
        o.values = r.values;
        o.name ||= nameOfRow(r.values);
      })
      .catch((e) => {
        const o = find();
        if (o) o.error = messageOf(e);
      });
  }

  const drop = (label: string) => (others = others.filter((o) => o.label !== label));

  /** A value as the card reads it: a list's word, an ability's name, a record's number and name,
   * a text whole (its colours left out). */
  function read(col: string, pick: Pick | undefined, v: Cell | undefined): string {
    if (v === null || v === undefined) return '';
    const p = pick ?? pickFor(table.name, col);
    const n = Number(v) || 0;
    if (p && 'options' in p) return p.options[n] ?? String(v);
    if (p && 'abilities' in p) return n ? (ABILITIES[n]?.[0] ?? String(v)) : '0';
    const target = targetOf(table.name, col);
    if (target && n > 0) {
      const name = nameOf(target, refKey(target, n, map));
      return name ? `${n} · ${name}` : String(n);
    }
    return typeof v === 'string' ? plainText(v) : String(v);
  }

  const differs = (col: string) => others.some((o) => o.values && !same(o.values[col], draft.get(col)));
  const shownGroups = $derived(
    groups.map((g) => ({ ...g, fields: g.fields.filter((f) => !onlyDiff || differs(f.col) || touched[f.col]) })).filter((g) => g.fields.length),
  );
  const loading = $derived(others.some((o) => !o.values && !o.error));

  function take(col: string, v: Cell) {
    draft.set(col, shown(v));
    touched[col] = true;
    sfx('pop');
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(o) => {
    if (!o) {
      others = [];
      touched = {};
    }
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="scrim" />
    <Dialog.Content class="cmp-sheet">
      <header class="cmp-h">
        <Dialog.Title class="cmp-title"><GitCompare size={18} /> Compare</Dialog.Title>
        <Dialog.Description class="faint cmp-says">
          The values that differ from {myName || `${one} ${mine}`} are yellow: a click copies one into it, and Save keeps it.
        </Dialog.Description>
        <label class="only"><input type="checkbox" bind:checked={onlyDiff} /> Only what differs</label>
        <Dialog.Close class="btn icon sm ghost cmp-x" aria-label="Close"><X size={16} /></Dialog.Close>
      </header>
      <div class="cmp-scroll">
        <table class="cmp">
          <thead>
            <tr>
              <th class="f"></th>
              <th class="me" scope="col"><span class="who">{myName || `${k.one} ${mine}`}</span><span class="num">{mine} · this one</span></th>
              {#each others as o (o.label)}
                <th scope="col">
                  <div class="colh">
                    <span class="names"><span class="who">{o.name || `${k.one} ${o.label}`}</span><span class="num">{o.label}</span></span>
                    <button type="button" class="btn icon sm ghost rm" aria-label="Take {o.label} out of the comparison" onclick={() => drop(o.label)}><X size={13} /></button>
                  </div>
                </th>
              {/each}
              {#if others.length < MAX}
                <th class="add">
                  <Picker what="a {one} to compare" label="Add" choices={() => recordChoices(table.name, map)} onpick={(v) => add(refKey(table.name, v, map))} />
                </th>
              {/if}
            </tr>
          </thead>
          <tbody>
            {#each shownGroups as g (g.title)}
              <tr class="grp"><th colspan={others.length + 3} scope="colgroup">{g.title}</th></tr>
              {#each g.fields as f (f.col)}
                {@const mv = draft.get(f.col)}
                <tr>
                  <th class="f" scope="row" title={f.col}>{f.label}</th>
                  <td class="me" class:edited={draft.edited(f.col)}><span class="v">{read(f.col, f.pick, mv)}</span></td>
                  {#each others as o (o.label)}
                    <td>
                      {#if o.values}
                        {@const v = o.values[f.col]}
                        {#if same(v, mv)}
                          <span class="v">{read(f.col, f.pick, v)}</span>
                        {:else}
                          <button type="button" class="diff" title="Copy into {myName || `${one} ${mine}`}" onclick={() => take(f.col, v ?? null)}>{read(f.col, f.pick, v) || '(blank)'}</button>
                        {/if}
                      {:else if o.error}
                        <span class="faint">{o.error}</span>
                      {:else}
                        <span class="faint">…</span>
                      {/if}
                    </td>
                  {/each}
                  {#if others.length < MAX}<td></td>{/if}
                </tr>
              {/each}
            {:else}
              <tr>
                <td colspan={others.length + 3} class="none faint">
                  {#if !others.length}Add a {one} to compare it with.{:else if loading}Reading them…{:else}Nothing differs.{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.cmp-sheet) {
    position: fixed; z-index: 201; left: 50%; top: 7vh; transform: translateX(-50%);
    width: min(1040px, calc(100vw - 24px)); max-height: 86vh; display: flex; flex-direction: column; gap: 10px;
    background: var(--surface); color: var(--ink); border: var(--stroke) solid var(--outline); border-radius: 20px;
    box-shadow: 0 5px 0 var(--shadow), var(--float); padding: 14px 16px; animation: fade-in .15s;
  }
  .cmp-h { display: flex; align-items: center; gap: 8px 14px; flex-wrap: wrap; }
  :global(.cmp-title) { margin: 0; display: inline-flex; align-items: center; gap: 8px; font-size: 18px; }
  :global(.cmp-says) { margin: 0; flex: 1 1 18rem; font-size: 12.5px; font-weight: 700; }
  .only { display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; font-weight: 800; color: var(--ink-2); }
  .cmp-scroll { overflow: auto; min-height: 0; border-radius: 12px; border: 1px solid var(--line); }
  .cmp { border-collapse: separate; border-spacing: 0; width: 100%; font-size: 13px; }
  .cmp th, .cmp td { padding: 6px 10px; text-align: left; vertical-align: top; border-bottom: 1px solid var(--line); }
  .cmp thead th { position: sticky; top: 0; z-index: 2; background: var(--surface-2); vertical-align: bottom; min-width: 9rem; }
  .cmp .f { position: sticky; left: 0; z-index: 1; background: var(--surface); color: var(--ink-2); font-weight: 800; font-size: 12.5px; white-space: nowrap; min-width: 0; }
  .cmp thead .f { z-index: 3; background: var(--surface-2); }
  .who { display: block; font-weight: 800; max-width: 14rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .num { font: 700 11.5px var(--font-mono); color: var(--ink-3); }
  .colh { display: flex; align-items: flex-start; justify-content: space-between; gap: 4px; }
  .names { min-width: 0; }
  .rm { flex: none; margin: -3px -6px 0 0; }
  .add { vertical-align: middle !important; }
  .grp th { background: var(--surface-2); font: 800 11px/1.2 var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: var(--ink-3); padding-top: 9px; }
  td.me { background: var(--select); font-weight: 700; }
  td.me.edited { box-shadow: inset 3px 0 0 var(--butter); }
  .v, .diff { font: 13px/1.35 var(--font-mono); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; white-space: pre-wrap; }
  .diff {
    display: block; width: calc(100% + 12px); text-align: left; min-height: 0; padding: 2px 6px; margin: -2px -6px;
    border: 0; border-radius: 7px; background: var(--butter-2); color: var(--ink); font-weight: 700; cursor: copy; box-shadow: none;
  }
  .diff:hover:not(:disabled) { background: var(--butter); }
  .none { padding: 18px 10px; text-align: center; }
  @media (max-width: 760px) {
    :global(.cmp-sheet) { top: 8px; max-height: calc(100dvh - 16px); padding: 12px; }
    /* the close stays top right; the words and the switch go below */
    :global(.cmp-x) { order: 1; margin-left: auto; }
    :global(.cmp-says) { order: 2; flex-basis: 100%; }
    .only { order: 3; }
    .cmp thead th { min-width: 7.5rem; }
    /* this one stays beside its field as the others scroll past */
    .cmp .f { width: 6.5rem; min-width: 6.5rem; max-width: 6.5rem; white-space: normal; }
    .cmp .me { position: sticky; left: 6.5rem; z-index: 1; box-shadow: 4px 0 6px -4px var(--shadow); }
    .cmp thead .me { z-index: 3; }
    .cmp td.me.edited { box-shadow: inset 3px 0 0 var(--butter), 4px 0 6px -4px var(--shadow); }
  }
</style>
