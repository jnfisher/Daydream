<!--
  A record beside the page: what a chip, a change or a search points at, read without leaving
  what is open. Its references open on top (Back steps down again); "Open full page" goes to
  its editor. Read-only: editing happens on the record's own page.
-->
<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import MapPin from '@lucide/svelte/icons/map-pin';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import X from '@lucide/svelte/icons/x';
  import { ABILITIES } from '../abilities';
  import { api, type Cell, type Row } from '../api';
  import { renderAnsi, toShown } from '../ansi';
  import { columnsShown, layoutFor, type Field, type Grid, type Layout, type Pick } from '../layouts';
  import { kindOf } from './kinds';
  import { keyLabel, recordHref } from './routes';
  import { app, closeInspector, inspect, remember, rowsOf } from './state.svelte';
  import { messageOf } from './toasts.svelte';

  const top = $derived(app.inspecting.at(-1));
  const back = $derived(app.inspecting.at(-2));
  const info = $derived(top ? app.tables.find((t) => t.name === top.table) : undefined);
  const k = $derived(kindOf(top?.table));
  const layout: Layout = $derived.by(() => {
    if (!info) return [];
    const l = layoutFor(info);
    const rest = info.columns.map((c) => c.name).filter((c) => !columnsShown(l).has(c));
    return rest.length ? [...l, { title: 'More', fields: rest.map((col) => ({ col })) }] : l;
  });

  let row = $state<Row | null>(null);
  let error = $state('');
  let tab = $state(0);
  /** Bumped when the realm changes under the card, so it reads the record again. */
  let version = $state(0);
  let names: Record<string, Map<number, string>> = $state({});

  $effect(() => {
    const bump = () => version++;
    window.addEventListener('dd:wrote', bump);
    window.addEventListener('dd:realm', bump);
    return () => {
      window.removeEventListener('dd:wrote', bump);
      window.removeEventListener('dd:realm', bump);
    };
  });

  // another record: its first tab
  $effect(() => {
    void top;
    tab = 0;
  });

  $effect(() => {
    const ref = top;
    void version;
    if (!ref) return;
    let gone = false;
    error = '';
    api
      .row(ref.table, ref.key)
      .then((r) => {
        if (gone) return;
        row = r;
        const name = titleOf(r);
        remember({ table: ref.table, key: ref.key, name: name || undefined });
      })
      .catch((e) => {
        if (gone) return;
        row = null;
        error = messageOf(e);
      });
    return () => (gone = true);
  });

  const shown = $derived(row && top && row.table === top.table && keyLabel(row.key) === keyLabel(top.key) ? row : null);
  const current = $derived(layout[tab] ?? layout[0]);

  // names for the references the tab shows
  $effect(() => {
    const refs = new Set<string>();
    for (const f of current?.fields ?? []) if (f.pick && 'ref' in f.pick) refs.add(f.pick.ref);
    for (const c of current?.grid?.columns ?? []) if (c.pick && 'ref' in c.pick) refs.add(c.pick.ref);
    for (const t of refs) {
      if (names[t]) continue;
      void rowsOf(t)
        .then((rows) => {
          const m = new Map<number, string>();
          for (const [key, title] of rows) if (typeof key[0] === 'number') m.set(key[0], title === null ? '' : String(title));
          names[t] = m;
        })
        .catch(() => {});
    }
  });
  // names change with writes
  $effect(() => {
    if (version) names = {};
  });

  function titleOf(r: Row): string {
    const col = info?.title;
    const v = col ? r.values[col] : null;
    return v === null || v === undefined ? '' : toShown(String(v)).split(/[\r\n]/)[0]!.trim().slice(0, 80);
  }

  const value = (col: string): Cell => shown?.values[col] ?? null;
  const text = (v: Cell) => (v === null ? '' : toShown(String(v)));

  const set = (col: string) => {
    const v = value(col);
    return v !== null && v !== 0 && v !== '';
  };

  /**
   * A grid's rows that hold something: the abilities a record has, not the 20 blank slots. A row
   * is in use when its first column is set (a drop slot with no item is empty, whatever its
   * chance), or any column of a grid of separate lists.
   */
  function gridRows(g: Grid) {
    const out: { n: number; label: string; cells: { col: string; pick?: Pick }[] }[] = [];
    for (let n = g.from; n <= g.to; n++) {
      const cells = g.columns.map((c) => ({ col: c.template.replace('#', String(n)), pick: c.pick }));
      if (g.separate ? cells.some(({ col }) => set(col)) : set(cells[0]!.col)) {
        out.push({ n, label: g.across ? `${g.across} ${n - g.from + 1}` : (g.names?.[n - g.from] ?? String(n)), cells });
      }
    }
    return out;
  }

  /**
   * A tab's fields as the card lists them. A wide field's unlabelled lines join it, so a
   * description's lines read as one text, without the blank lines at its ends; a text with
   * nothing in it is left out.
   */
  function entries(fields: Field[]) {
    const out: { field: Field; lines?: string[] }[] = [];
    for (const field of fields) {
      if (!info?.columns.some((c) => c.name === field.col)) continue;
      const last = out.at(-1);
      if (field.wide && field.label === '' && last?.lines) last.lines.push(text(value(field.col)));
      else out.push(field.wide ? { field, lines: [text(value(field.col))] } : { field });
    }
    return out.filter(({ lines }) => {
      if (!lines) return true;
      while (lines.length && !lines.at(-1)!.trim()) lines.pop();
      while (lines.length && !lines[0]!.trim()) lines.shift();
      return lines.length > 0;
    });
  }

  /**
   * A text's lines run together, as the card is narrower than the game's lines: a line goes on
   * after a space (straight on after a hyphen), and a blank line still breaks.
   */
  function flow(lines: string[]): string {
    let out = '';
    for (const raw of lines) {
      const line = raw.trimEnd();
      if (!line.trim()) out += '\n';
      else if (!out || out.endsWith('\n')) out += line;
      else out += (out.endsWith('-') ? '' : ' ') + line.trimStart();
    }
    return out;
  }

  function terminal(node: HTMLElement, cols: string[]) {
    const draw = (cs: string[]) => node.replaceChildren(...cs.flatMap((c, i) => (i ? ['\n', renderAnsi(text(value(c)))] : [renderAnsi(text(value(c)))])));
    draw(cols);
    return { update: draw };
  }
</script>

{#snippet cell(field: Field)}
  {@const v = value(field.col)}
  {@const n = typeof v === 'number' ? v : Number(v)}
  {#if field.pick && 'ref' in field.pick}
    {#if !n}
      <span class="none">none</span>
    {:else}
      {@const rk = kindOf(field.pick.ref)}
      {@const ref = field.pick.ref}
      <button type="button" class="ref {rk.cls}" title="{rk.one} {n}{names[ref]?.get(n) ? `: ${names[ref]?.get(n)}` : ''}" onclick={() => inspect({ table: ref, key: [n], name: names[ref]?.get(n) })}>
        <span class="tile"><rk.icon size={12} /></span><span class="num">{n}</span><span class="nm">{names[ref]?.get(n) ?? ''}</span>
      </button>
    {/if}
  {:else if field.pick && 'options' in field.pick}
    {field.pick.options[n] ?? text(v)}{#if field.pick.options[n] !== undefined}<span class="faint num opt-n">{text(v)}</span>{/if}
  {:else if field.pick && 'abilities' in field.pick}
    {n ? (ABILITIES[n]?.[0] ?? `ability ${n}`) : 'none'}
  {:else if text(v) === ''}
    <span class="none">—</span>
  {:else}
    {text(v)}
  {/if}
{/snippet}

{#if top}
  <aside class="inspector {k.cls}" aria-label="{k.one} {keyLabel(top.key)}">
    {#if back}
      <button type="button" class="back" onclick={() => (app.inspecting = app.inspecting.slice(0, -1))}>
        <ArrowLeft size={14} />Back to {back.name || `${kindOf(back.table).one} ${keyLabel(back.key)}`}
      </button>
    {/if}
    <header>
      <span class="tile big"><k.icon size={22} /></span>
      <div class="who">
        <span class="eyebrow">{k.one} {keyLabel(top.key)}</span>
        <h2>{(shown && titleOf(shown)) || top.name || `${k.one} ${keyLabel(top.key)}`}</h2>
      </div>
      <button type="button" class="btn icon sm ghost" title="Close (Esc)" aria-label="Close" onclick={closeInspector}><X size={17} /></button>
    </header>
    <div class="actions">
      <a class="btn sm sticker" href={recordHref(top.table, top.key)} onclick={closeInspector}><Maximize2 size={14} />Open full page</a>
      {#if top.table === 'Rooms'}
        <a class="btn sm" href="#/world/{top.key[0]}/{top.key[1]}" onclick={closeInspector}><MapPin size={14} />Show on the map</a>
      {/if}
    </div>
    {#if error}
      <p class="error">{error}</p>
    {:else if !shown}
      <p class="faint loading">Reading the record…</p>
    {:else}
      {#if layout.length > 1}
        <div class="tabs" role="tablist">
          {#each layout as t, i (t.title)}
            <button type="button" role="tab" class:active={i === tab} aria-selected={i === tab} onclick={() => (tab = i)}>{t.title}</button>
          {/each}
        </div>
      {/if}
      <div class="body">
        {#if current?.fields}
          <dl>
            {#each entries(current.fields) as { field, lines } (field.col)}
              <dt title={field.col}>{field.label ?? field.col}</dt>
              {#if lines}
                <dd class="wide">{flow(lines)}</dd>
              {:else}
                <dd>{@render cell(field)}</dd>
              {/if}
            {/each}
          </dl>
        {/if}
        {#if current?.matrix}
          <table class="mini">
            <thead><tr><th></th>{#each current.matrix.headers as hd (hd)}<th>{hd}</th>{/each}</tr></thead>
            <tbody>
              {#each current.matrix.rows as r (r.label)}
                <tr><th>{r.label}</th>{#each r.cols as c (c)}<td>{text(value(c))}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        {/if}
        {#if current?.grid}
          {@const g = current.grid}
          {@const rows = gridRows(g)}
          {#if !rows.length}
            <p class="faint">Nothing here.</p>
          {:else if g.across}
            <!-- many columns (an attack's ten): a row for each, a column for each attack in use -->
            <table class="mini across">
              <thead><tr><th></th>{#each rows as r (r.n)}<th>{r.label}</th>{/each}</tr></thead>
              <tbody>
                {#each g.columns as c, i (c.template)}
                  <tr><th>{c.label ?? c.template}</th>{#each rows as r (r.n)}<td>{@render cell(r.cells[i]!)}</td>{/each}</tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <table class="mini">
              <thead><tr><th>#</th>{#each g.columns as c (c.template)}<th>{c.label ?? c.template}</th>{/each}</tr></thead>
              <tbody>
                {#each rows as r (r.n)}
                  <tr><th>{r.label}</th>{#each r.cells as c (c.col)}<td>{@render cell(c)}</td>{/each}</tr>
                {/each}
              </tbody>
            </table>
          {/if}
        {/if}
        {#if current?.preview}
          <div class="eyebrow">As players see it</div>
          <pre class="terminal" use:terminal={current.preview}></pre>
        {/if}
      </div>
    {/if}
  </aside>
{/if}

<style>
  .inspector {
    position: absolute; top: 12px; right: 12px; bottom: 12px; z-index: 20;
    width: min(420px, calc(100% - 24px)); display: flex; flex-direction: column; gap: 10px;
    padding: 14px 16px; overflow: auto;
    background: var(--surface); border: var(--stroke) solid var(--outline); border-radius: var(--r-lg);
    box-shadow: 0 4px 0 var(--shadow), var(--float);
    animation: slide-in .25s var(--pop);
  }
  @keyframes slide-in { from { opacity: 0; transform: translateX(18px); } }
  .back { align-self: flex-start; min-height: 26px; padding: 0 10px; border-radius: 999px; font-size: 12px; background: var(--surface-2); box-shadow: none; }
  header { display: flex; align-items: center; gap: 12px; }
  .tile.big { width: 44px; height: 44px; border-radius: 14px; }
  .who { flex: 1; min-width: 0; }
  .who h2 { margin: 2px 0 0; font-size: 20px; line-height: 1.15; overflow-wrap: anywhere; }
  .actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .tabs { margin: 0; }
  .body { display: flex; flex-direction: column; gap: 10px; min-height: 0; }
  dl { display: grid; grid-template-columns: minmax(7rem, auto) 1fr; gap: 5px 12px; margin: 0; font-size: 13px; }
  dt { color: var(--ink-3); font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  dd { margin: 0; font-weight: 700; min-width: 0; overflow-wrap: anywhere; }
  dd.wide { grid-column: 1 / -1; font-weight: 600; white-space: pre-wrap; }
  .none { color: var(--ink-3); font-weight: 600; }
  .num { font: 600 11px var(--font-mono); }
  .opt-n { margin-left: 6px; }
  .mini { border-collapse: collapse; font-size: 12.5px; width: 100%; }
  .mini th, .mini td { text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--line); vertical-align: middle; }
  .mini thead th { color: var(--ink-3); font-weight: 800; font-size: 11.5px; }
  .mini tbody th { color: var(--ink-3); font-weight: 700; width: 1%; white-space: nowrap; }
  .mini .ref { max-width: 11rem; }
  /* the attacks share the width, and a long message chip is cut short in its cell */
  .mini.across { table-layout: fixed; }
  .mini.across thead th:first-child { width: 6.8rem; }
  .mini.across tbody th { overflow: hidden; text-overflow: ellipsis; }
  .mini.across .ref { max-width: 100%; }
  .terminal { margin: 0; }
  .error { color: var(--bad); font-weight: 700; }
  .loading { margin: 0; }
  @media (max-width: 760px) {
    .inspector { top: 8px; right: 8px; bottom: 8px; width: calc(100% - 16px); }
  }
</style>
