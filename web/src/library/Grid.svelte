<!--
  A table as a spreadsheet (grid.ts): a row for each record the list shows (its search and
  chips narrow it), the columns picked (under the card's headings, remembered in this
  browser), sorted by a header's click, and the rows in sight drawn. CSV out writes what it
  shows. A row's click opens its card.
-->
<svelte:options css="injected" />

<script lang="ts">
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import Columns from '@lucide/svelte/icons/columns-3-cog';
  import FileDown from '@lucide/svelte/icons/file-down';
  import { Popover } from 'bits-ui';
  import { untrack } from 'svelte';
  import { ABILITIES } from '../abilities';
  import { api, type Cell, type TableInfo } from '../api';
  import { layoutFor, pickFor, targetOf, type Pick } from '../layouts';
  import { kindOf } from '../shell/kinds';
  import { read, sfx, write } from '../shell/prefs.svelte';
  import { recordHref } from '../shell/routes';
  import { messageOf, toast } from '../shell/toasts.svelte';
  import { CARDS } from './cards';
  import { plainText } from './compare';
  import { columnGroups, compareCells, defaultColumns, inOrder, toCsv } from './grid';
  import type { ListRow } from './list';
  import { labelsOf } from './problems';
  import { fromLayout, slotCol } from './sections';

  let { table, rows, changed }: { table: TableInfo; rows: ListRow[]; changed: Set<string> } = $props();

  const def = $derived(CARDS[table.name]);
  const k = $derived(kindOf(table.name));
  const names = $derived(table.columns.map((c) => c.name));
  const sections = $derived(def ? def.sections : fromLayout(layoutFor(table), names, table.key));
  const groups = $derived(columnGroups(sections, names, table.key));
  const labels = $derived(labelsOf(sections));
  const types = $derived(new Map(table.columns.map((c) => [c.name, c])));

  /** What a column picks from: its card field's, its grid's, or fields.json's. */
  const picks = $derived.by(() => {
    const out = new Map<string, Pick>();
    for (const s of sections) {
      for (const f of s.fields ?? []) {
        const p = f.pick ?? pickFor(table.name, f.col);
        if (p) out.set(f.col, p);
      }
      const g = s.grid;
      if (g)
        for (const c of g.columns) {
          const p = c.pick ?? pickFor(table.name, c.template);
          if (p) for (let n = g.from; n <= g.to; n++) out.set(slotCol(c.template, n), p);
        }
    }
    return out;
  });

  // ---- the columns picked, remembered in this browser ----------------------------------

  const STORE = $derived(`dd-grid:${table.name}`);
  const usual = $derived(defaultColumns(sections, names, table.key, table.title, def?.listCols));
  function stored(): string[] | null {
    try {
      const v: unknown = JSON.parse(read(STORE, 'null'));
      return Array.isArray(v) ? v.filter((c): c is string => typeof c === 'string' && names.includes(c)) : null;
    } catch {
      return null;
    }
  }
  // a grid is drawn for one table (the Library keys it by the table): what was picked for it,
  // or the usual ones
  let picked = $state(new Set(untrack(() => stored() ?? usual)));
  // the name first, beside the number; the rest in the card's order
  const cols = $derived.by(() => {
    const all = inOrder(picked, groups);
    const t = table.title;
    return t && all.includes(t) ? [t, ...all.filter((c) => c !== t)] : all;
  });
  /** The name stays in sight beside the number as the columns scroll. */
  const pinned = $derived(!!table.title && cols[0] === table.title);
  function setPicked(next: Set<string>) {
    picked = next;
    write(STORE, JSON.stringify([...next]));
  }
  const toggle = (c: string) => {
    const next = new Set(picked);
    if (!next.delete(c)) next.add(c);
    setPicked(next);
  };

  // ---- the values: each column read once, all over again after a write ------------------

  let values = $state.raw<Record<string, Map<string, Cell>>>({});
  let reading = $state(false);
  let gen = 0;
  async function fill(want: string[], fresh = false) {
    const missing = fresh ? want : want.filter((c) => !(c in values));
    if (!missing.length) return;
    const g = ++gen;
    reading = true;
    try {
      const r = await api.rowsWith(table.name, missing);
      if (g !== gen) return;
      const next: Record<string, Map<string, Cell>> = fresh ? {} : { ...values };
      missing.forEach((c, i) => (next[c] = new Map(r.rows.map(([key, , vals]) => [key.map((x) => String(x ?? '')).join('/'), vals[i] ?? null]))));
      values = next;
    } catch (e) {
      if (g === gen) toast(messageOf(e), { kind: 'error' });
    } finally {
      if (g === gen) reading = false;
    }
  }
  $effect(() => {
    void fill(cols);
  });
  $effect(() => {
    const again = () => void fill(cols, true);
    window.addEventListener('dd:wrote', again);
    window.addEventListener('dd:realm', again);
    return () => {
      window.removeEventListener('dd:wrote', again);
      window.removeEventListener('dd:realm', again);
    };
  });
  const valueOf = (r: ListRow, c: string): Cell | undefined => values[c]?.get(r.label);

  // ---- sorted by a header's click: up, down, then as the list has them -------------------

  let sort = $state<{ col: string | null; down: boolean } | null>(null);
  function sortBy(col: string | null) {
    sort = sort?.col !== col ? { col, down: false } : sort.down ? null : { col, down: true };
  }
  const sorted = $derived.by(() => {
    const s = sort;
    if (!s) return rows;
    // the list is in key order already
    const out = s.col === null ? rows.slice() : rows.slice().sort((a, b) => compareCells(sortKey(a, s.col!), sortKey(b, s.col!)));
    return s.down ? out.reverse() : out;
  });
  const sortIs = (col: string | null) => (sort?.col === col ? (sort.down ? 'descending' : 'ascending') : 'none');
  /** A list's number sorts by its word, as the grid shows it. */
  const worded = (c: string) => {
    const p = picks.get(c);
    return !!p && ('options' in p || 'abilities' in p);
  };
  const sortKey = (r: ListRow, c: string): Cell | undefined => {
    const v = valueOf(r, c);
    return worded(c) && typeof v === 'number' ? text(c, v) : v;
  };
  const sortSays = $derived.by(() => {
    const s = sort;
    if (!s) return 'in number order';
    if (s.col === null) return `in number order, ${s.down ? 'highest' : 'lowest'} first`;
    const by = labels.get(s.col) ?? s.col;
    return isNumber(s.col) && !worded(s.col) ? `by ${by}, ${s.down ? 'highest' : 'lowest'} first` : `by ${by}, ${s.down ? 'Z to A' : 'A to Z'}`;
  });

  // ---- a cell as the card reads it -------------------------------------------------------

  const isNumber = (c: string) => ['long', 'double'].includes(types.get(c)?.type ?? '');
  /** Columns that name a record by number read as the number alone. */
  const named = (c: string) => !!targetOf(table.name, c) || /^(Exit|Para\d) \d+$/.test(c);
  function text(c: string, v: Cell | undefined): string {
    if (v === null || v === undefined) return '';
    if (typeof v === 'number') {
      const p = picks.get(c);
      if (p && 'options' in p) return p.options[v] ?? String(v);
      if (p && 'abilities' in p) return v ? (ABILITIES[v]?.[0] ?? String(v)) : '';
      return named(c) ? String(v) : v.toLocaleString();
    }
    return plainText(v).replace(/\n/g, ' ⏎ ');
  }
  const width = (c: string) => {
    if (c === table.title) return 220;
    const p = picks.get(c);
    if (p && ('options' in p || 'abilities' in p)) return 140;
    if (isNumber(c)) return 96;
    const size = types.get(c)?.size ?? 0;
    return size && size <= 16 ? 140 : 240;
  };
  const right = (c: string) => isNumber(c) && !worded(c);

  // ---- only the rows in sight are drawn --------------------------------------------------

  const RH = 32;
  let scroller = $state<HTMLElement>();
  let top = $state(0);
  let tall = $state(600);
  const first = $derived(Math.max(0, Math.floor(top / RH) - 8));
  const last = $derived(Math.min(sorted.length, Math.ceil((top + tall) / RH) + 8));
  const visible = $derived(sorted.slice(first, last));
  // another search, chip or sort: back to the top
  $effect(() => {
    void rows;
    void sort;
    if (scroller) scroller.scrollTop = top = 0;
  });

  // ---- out ---------------------------------------------------------------------------------

  async function csv() {
    await fill(cols);
    const header = [...table.key, ...cols];
    const body = sorted.map((r) => [
      ...r.key,
      ...cols.map((c) => {
        const v = valueOf(r, c);
        return typeof v === 'string' ? plainText(v) : (v ?? null);
      }),
    ]);
    const url = URL.createObjectURL(new Blob([toCsv(header, body)], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${table.name.toLowerCase()}.csv`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    sfx('pop');
    toast(`Wrote ${body.length.toLocaleString()} ${body.length === 1 ? k.one.toLowerCase() : k.many.toLowerCase()} to ${a.download}`);
  }

  let finding = $state('');
  const findIn = (c: string) => !finding.trim() || `${c} ${labels.get(c) ?? ''}`.toLowerCase().includes(finding.trim().toLowerCase());
</script>

<div class="grid-page">
  <div class="gbar">
    <span class="faint gsays" role="status">
      {cols.length} {cols.length === 1 ? 'column' : 'columns'} · {sortSays}{reading ? ' · reading…' : ''}
    </span>
    <Popover.Root onOpenChange={(o) => !o && (finding = '')}>
      <Popover.Trigger class="btn sm"><Columns size={14} /> Columns</Popover.Trigger>
      <Popover.Portal>
        <Popover.Content class="pop gcols" sideOffset={6} align="end" collisionPadding={8}>
          <div class="gcols-h">
            <input type="search" placeholder="Find a column" aria-label="Find a column" bind:value={finding} />
            <button type="button" class="btn sm ghost" onclick={() => setPicked(new Set(usual))}>The usual ones</button>
          </div>
          <div class="gcols-list">
            {#each groups as g (g.title)}
              {@const shownCols = g.cols.filter(findIn)}
              {#if shownCols.length}
                <fieldset>
                  <legend>{g.title}</legend>
                  {#each shownCols as c (c)}
                    <label title={c}><input type="checkbox" checked={picked.has(c)} onchange={() => toggle(c)} /> {labels.get(c) ?? c}</label>
                  {/each}
                </fieldset>
              {/if}
            {/each}
          </div>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
    <button type="button" class="btn sm" onclick={() => void csv()} title="Write the rows shown, sorted as shown, to a CSV file"><FileDown size={14} /> CSV</button>
  </div>
  <div class="gscroll" bind:this={scroller} bind:clientHeight={tall} onscroll={() => (top = scroller!.scrollTop)}>
    <table class="gt" aria-rowcount={sorted.length + 1}>
      <colgroup>
        <col style:width="92px" />
        {#each cols as c (c)}<col style:width="{width(c)}px" />{/each}
      </colgroup>
      <thead>
        <tr aria-rowindex={1}>
          <th class="key" scope="col" aria-sort={sortIs(null)} title={table.key.join(' / ')}>
            <button type="button" onclick={() => sortBy(null)}>{k.one}{#if sort?.col === null}{#if sort.down}<ArrowDown size={12} />{:else}<ArrowUp size={12} />{/if}{/if}</button>
          </th>
          {#each cols as c, j (c)}
            <th scope="col" class:r={right(c)} class:pin={pinned && j === 0} aria-sort={sortIs(c)} title={c}>
              <button type="button" onclick={() => sortBy(c)}>
                <span class="hl">{labels.get(c) ?? c}</span>{#if sort?.col === c}{#if sort.down}<ArrowDown size={12} />{:else}<ArrowUp size={12} />{/if}{/if}
              </button>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#if first > 0}<tr class="gap" style:height="{first * RH}px" aria-hidden="true"><td colspan={cols.length + 1}></td></tr>{/if}
        {#each visible as r, i (r.label)}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (the record's number is its link for the keyboard) -->
          <tr aria-rowindex={first + i + 2} onclick={(e) => !(e.target as Element).closest('a') && (location.hash = recordHref(table.name, r.key))}>
            <td class="key">
              <a href={recordHref(table.name, r.key)}>{r.label}</a>
              {#if changed.has(r.label)}<span class="dot cdot" title="Changed since publishing"></span>{/if}
            </td>
            {#each cols as c, j (c)}
              {@const t = text(c, valueOf(r, c))}
              <td class:r={right(c)} class:pin={pinned && j === 0} title={t.length > 24 ? t : undefined}>{t}</td>
            {/each}
          </tr>
        {/each}
        {#if last < sorted.length}<tr class="gap" style:height="{(sorted.length - last) * RH}px" aria-hidden="true"><td colspan={cols.length + 1}></td></tr>{/if}
      </tbody>
    </table>
  </div>
</div>

<style>
  .grid-page { display: flex; flex-direction: column; height: 100%; min-height: 0; padding: 10px 14px 14px; gap: 8px; }
  .gbar { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .gsays { flex: 1; font-size: 12.5px; font-weight: 800; }
  .gscroll { flex: 1; min-height: 0; overflow: auto; border: 1px solid var(--line); border-radius: 12px; background: var(--surface); }
  .gt { border-collapse: separate; border-spacing: 0; table-layout: fixed; width: max-content; min-width: 100%; font-size: 12.5px; }
  .gt th, .gt td { height: 32px; padding: 0 10px; border-bottom: 1px solid var(--line); border-right: 1px solid var(--line); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: left; }
  .gt td { font: 12.5px var(--font-mono); font-variant-numeric: tabular-nums; color: var(--ink); }
  .gt thead th { position: sticky; top: 0; z-index: 2; background: var(--surface-2); padding: 0; }
  .gt thead th button {
    all: unset; box-sizing: border-box; display: flex; align-items: center; gap: 4px; width: 100%; height: 32px; padding: 0 10px; cursor: pointer;
    font: 800 12px var(--font-ui); color: var(--ink-2);
  }
  .gt thead th button:hover { color: var(--ink); background: var(--surface-3); }
  .gt thead th button:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  .gt thead th.r button { justify-content: flex-end; }
  .hl { overflow: hidden; text-overflow: ellipsis; }
  .gt .r { text-align: right; }
  .gt .key { position: sticky; left: 0; z-index: 1; background: var(--surface); }
  .gt .pin { position: sticky; left: 92px; z-index: 1; background: var(--surface); box-shadow: 4px 0 6px -5px var(--shadow); }
  .gt thead .key, .gt thead .pin { z-index: 3; background: var(--surface-2); }
  .gt td.key a { color: var(--link); font-weight: 700; text-decoration: none; margin-right: 6px; }
  .gt td.key a:hover { text-decoration: underline; }
  .gt tbody tr:not(.gap) { cursor: pointer; }
  .gt tbody tr:not(.gap):hover td { background: var(--surface-2); }
  @media (max-width: 760px) {
    .grid-page { padding: 8px; }
    /* on a phone the number alone stays in sight */
    .gt .pin { position: static; box-shadow: none; }
  }
  .gt tr.gap td { padding: 0; border: 0; height: auto; }
  .cdot { color: var(--butter); width: 7px; height: 7px; }
  :global(.gcols) { width: min(560px, calc(100vw - 16px)); padding: 10px 12px; display: grid; gap: 8px; }
  :global(.gcols) .gcols-h { display: flex; gap: 8px; align-items: center; }
  :global(.gcols) .gcols-h input { flex: 1; }
  :global(.gcols) .gcols-list { max-height: min(60vh, 460px); overflow: auto; display: grid; gap: 6px; }
  :global(.gcols) fieldset { border: 0; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 2px 10px; }
  :global(.gcols) legend { font: 800 11px/1.2 var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: var(--ink-3); padding: 6px 0 4px; }
  :global(.gcols) label { display: flex; align-items: center; gap: 6px; font-size: 12.5px; font-weight: 700; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
