<!--
  The Library: the world's records, a table at a time. The list on the left (found by name
  or number, narrowed by chips, what changed since publishing dotted), the open record's card
  on the right (Card.svelte). The frame routes here with `show`, and `[` and `]` step through
  the list as NMR's arrows do. Users, gangs and bank books are the Players place's.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import SearchIcon from '@lucide/svelte/icons/search';
  import { onMount, tick } from 'svelte';
  import { titleOf } from '../ansi';
  import { api, type Cell, type Row, type TableInfo } from '../api';
  import { kindOf, TABLE_ORDER } from '../shell/kinds';
  import Mascot from '../shell/Mascot.svelte';
  import { sfx } from '../shell/prefs.svelte';
  import { PLAYER_TABLES, recordHref, type Route } from '../shell/routes';
  import { addGuard, app, remember, undo } from '../shell/state.svelte';
  import { messageOf, toast } from '../shell/toasts.svelte';
  import Card from './Card.svelte';
  import { CARDS } from './cards';
  import { Draft } from './draft.svelte';
  import { found, keyLabel, listRow, nextKey, type ListRow } from './list';
  import NewRecord from './NewRecord.svelte';

  /** The chip that shows only what changed since publishing. */
  const CHANGED = 'Changed';

  /** The Library's tables, in NMR's menu order. */
  const tables = $derived(
    app.tables
      .filter((t) => !PLAYER_TABLES.includes(t.name))
      .sort((a, b) => (TABLE_ORDER.indexOf(a.name) + 1 || 99) - (TABLE_ORDER.indexOf(b.name) + 1 || 99)),
  );

  let table = $state.raw<TableInfo | null>(null);
  let rows = $state.raw<ListRow[]>([]);
  let reading = $state(false);
  let failed = $state('');
  let find = $state('');
  let chip = $state<string | null>(null);
  /** The record asked for, and its card's draft once it is read. */
  let key = $state.raw<Cell[] | null>(null);
  let draft = $state.raw<Draft | null>(null);
  let missing = $state('');
  let saving = $state(false);
  /** Bumped by every write, so the card reads again where the record is used. */
  let version = $state(0);
  /** On a phone: the list shown over the card. */
  let browsing = $state(false);
  /** Our own writes: the list need not be read again for them. */
  let writing = false;
  /** Something else wrote: the list is read again when next shown. */
  let stale = false;
  /** The latest route shown: an older one's reads arrive too late to show. */
  let seq = 0;

  const def = $derived(table ? CARDS[table.name] : undefined);
  const k = $derived(kindOf(table?.name));
  const one = $derived(k.one.toLowerCase());
  const many = $derived(k.many.toLowerCase());
  const numeric = $derived(table ? table.key.map((c) => table!.columns.find((x) => x.name === c)?.type !== 'text') : []);
  const open = $derived(key ? keyLabel(key) : '');
  /** A table keyed by a word (an action): the word is its name in the list. */
  const worded = $derived(numeric.length === 1 && !numeric[0]);

  const filters = $derived(def?.filters && rows.length ? def.filters(rows) : []);
  const changed = $derived(new Set((app.changes?.changes ?? []).filter((c) => c.nmr === table?.name && c.key).map((c) => keyLabel(c.key!))));
  const shown = $derived.by(() => {
    const f = filters.find((x) => x.label === chip);
    return rows.filter((r) => found(r, find) && (chip === CHANGED ? changed.has(r.label) : !f || f.test(r.get, r.key)));
  });
  const counted = $derived(
    shown.length === rows.length ? `${rows.length.toLocaleString()} ${rows.length === 1 ? one : many}` : `${shown.length.toLocaleString()} of ${rows.length.toLocaleString()}`,
  );
  const article = $derived(/^[aeiou]/i.test(one) ? 'an' : 'a');

  // ---- the list: only the rows in sight are drawn ------------------------------------------

  let listEl = $state<HTMLElement>();
  let top = $state(0);
  let tall = $state(600);
  const ROW = $derived(def?.summary || worded ? 46 : 36);
  const first = $derived(Math.max(0, Math.floor(top / ROW) - 10));
  const visible = $derived(shown.slice(first, Math.min(shown.length, Math.ceil((top + tall) / ROW) + 10)));

  // another search or chip: the list starts at its top
  $effect(() => {
    void find;
    void chip;
    if (listEl) listEl.scrollTop = top = 0;
  });

  /** Scrolls the list to the open record when it is out of sight. */
  async function reveal() {
    await tick();
    const el = listEl;
    const i = shown.findIndex((r) => r.label === open);
    if (!el || i < 0) return;
    const y = i * ROW;
    if (y < el.scrollTop) el.scrollTop = y;
    else if (y + ROW > el.scrollTop + el.clientHeight) el.scrollTop = y + ROW - el.clientHeight + 6;
    top = el.scrollTop;
  }

  // ---- reading ---------------------------------------------------------------------------

  const nameOf = (title: Cell) => (title === null ? '' : titleOf(String(title)));

  function byKey(a: ListRow, b: ListRow): number {
    for (let i = 0; i < a.key.length; i++) {
      const x = a.key[i] ?? '';
      const y = b.key[i] ?? '';
      if (x === y) continue;
      return typeof x === 'number' && typeof y === 'number' ? x - y : String(x).localeCompare(String(y));
    }
    return 0;
  }

  async function readRows(t: TableInfo): Promise<void> {
    const cols = (CARDS[t.name]?.listCols ?? []).filter((c) => t.columns.some((x) => x.name === c));
    reading = true;
    failed = '';
    try {
      const next = cols.length
        ? (await api.rowsWith(t.name, cols)).rows.map(([key, title, vals]) => listRow(key, nameOf(title), cols, vals))
        : (await api.rows(t.name)).rows.map(([key, title]) => listRow(key, nameOf(title), [], []));
      if (table === t) rows = next.sort(byKey);
    } catch (e) {
      if (table === t) failed = messageOf(e);
    } finally {
      if (table === t) reading = false;
    }
  }

  async function readRecord(t: TableInfo, k: Cell[], gen: number): Promise<void> {
    missing = '';
    try {
      const row = await api.row(t.name, k);
      if (gen !== seq) return;
      const d = new Draft(t);
      d.load(row);
      draft = d;
      remember({ table: t.name, key: k, name: (t.title && nameOf(row.values[t.title] ?? null)) || undefined });
    } catch (e) {
      if (gen !== seq) return;
      draft = null;
      missing = messageOf(e);
    }
  }

  // ---- what the frame calls ----------------------------------------------------------------

  /** Shows the table and record a route names. Answers false when the person kept unsaved
   *  edits instead, and the page stays where it was. */
  export async function show(r: Route): Promise<boolean> {
    const t = tables.find((x) => x.name === r.table) ?? table ?? tables[0];
    if (!t) return true;
    const k = r.key ?? null;
    const same = t === table && !!k && keyLabel(k) === open;
    if (same) {
      // back on the record open: as it was left (unless it was written meanwhile)
      browsing = false;
      if (stale) {
        stale = false;
        void readRows(t);
        if (!draft?.dirty) void readRecord(t, k, ++seq);
      }
      return true;
    }
    if (draft?.dirty && !confirm('Discard the changes to this record?')) return false;
    const gen = ++seq;
    if (t !== table) {
      table = t;
      rows = [];
      find = '';
      chip = null;
      key = null;
      draft = null;
      stale = false;
      await readRows(t);
    } else if (stale) {
      stale = false;
      await readRows(t);
    }
    if (gen !== seq) return true;
    key = k;
    browsing = false;
    if (!k) {
      draft = null;
      missing = '';
      return true;
    }
    await readRecord(t, k, gen);
    void reveal();
    return true;
  }

  /** The link of what the Library shows now. */
  export const here = (): string => (table ? recordHref(table.name, key) : '#/library');

  /** The previous (-1) or next (1) record in the list, as NMR's arrows step. */
  export function step(by: number): void {
    if (!table || !shown.length) return;
    const at = shown.findIndex((r) => r.label === open);
    const next = shown[Math.min(shown.length - 1, Math.max(0, at < 0 ? 0 : at + by))];
    if (next && next.label !== open) location.hash = recordHref(table.name, next.key);
  }

  // ---- writing ---------------------------------------------------------------------------

  /** A saved record's name and listed values, in the list. */
  function patch(t: TableInfo, row: Row) {
    const l = keyLabel(row.key);
    const i = rows.findIndex((r) => r.label === l);
    if (i < 0) return;
    const cols = Object.keys(rows[i]!.vals);
    const next = rows.slice();
    next[i] = listRow(row.key, t.title ? nameOf(row.values[t.title] ?? null) : rows[i]!.name, cols, cols.map((c) => row.values[c] ?? null));
    rows = next;
  }

  const failedWith = (e: unknown) => {
    sfx('nope');
    toast(messageOf(e), { kind: 'error' });
  };

  /** Writes the card's changes. Answers whether nothing is left unsaved. */
  async function save(): Promise<boolean> {
    const t = table;
    const d = draft;
    if (!t || !d?.row) return true;
    const values = d.changed();
    if (!Object.keys(values).length) return true;
    if (d.invalid.length) {
      sfx('nope');
      toast(`Fix ${d.invalid.length === 1 ? 'the field' : `the ${d.invalid.length} fields`} marked in red first: ${d.invalid.join(', ')}.`, { kind: 'error' });
      return false;
    }
    saving = writing = true;
    try {
      const report = await api.save(t.name, d.row.key, values);
      const row = await api.row(t.name, d.row.key);
      if (draft === d) d.load(row);
      patch(t, row);
      version++;
      if (report.errors.length) toast(report.errors.join('; '), { kind: 'error' });
      else if (report.updated) {
        sfx('pop');
        toast(`Saved ${kindOf(t.name).one.toLowerCase()} ${keyLabel(row.key)}`, { action: 'Undo', onAction: () => void undo() });
      } else toast('Nothing changed in the record.');
      return true;
    } catch (e) {
      failedWith(e);
      return false;
    } finally {
      saving = writing = false;
    }
  }

  /** A new record under `k`: a copy of the one open (its changes saved first), or a new one. */
  async function create(k: Cell[], copy: boolean) {
    const t = table;
    if (!t) return;
    if (copy && draft?.dirty && !(await save())) return;
    const values: Record<string, Cell> = copy && draft?.row ? { ...draft.row.values } : {};
    t.key.forEach((c, i) => (values[c] = k[i] ?? null));
    writing = true;
    try {
      const report = await api.insert(t.name, values);
      if (report.errors.length) {
        toast(report.errors.join('; '), { kind: 'error' });
        return;
      }
      await readRows(t);
      sfx('pop');
      toast(`${copy ? 'Copied to' : 'Added'} ${kindOf(t.name).one.toLowerCase()} ${keyLabel(k)}`, { action: 'Undo', onAction: () => void undo() });
      location.hash = recordHref(t.name, k);
    } catch (e) {
      failedWith(e);
    } finally {
      writing = false;
    }
  }

  async function remove() {
    const t = table;
    const d = draft;
    if (!t || !d?.row) return;
    const l = keyLabel(d.row.key);
    const at = shown.findIndex((r) => r.label === l);
    const next = shown[at + 1] ?? shown[at - 1];
    writing = true;
    try {
      await api.remove(t.name, d.row.key);
      rows = rows.filter((r) => r.label !== l);
      draft = null;
      key = null;
      toast(`Deleted ${kindOf(t.name).one.toLowerCase()} ${l}`, { action: 'Undo', onAction: () => void undo() });
      location.hash = next ? recordHref(t.name, next.key) : recordHref(t.name);
    } catch (e) {
      failedWith(e);
    } finally {
      writing = false;
    }
  }

  /** The map a new room is offered on: the open room's, or the map chip's. */
  function near(): Cell[] | null {
    if (key) return key;
    const m = /^Map (\d+)$/.exec(chip ?? '');
    return m ? [Number(m[1]), 0] : null;
  }

  onMount(() => {
    addGuard(() => !draft?.dirty || confirm('Discard the changes to this record?'));
    const keys = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 's' && app.route.place === 'library' && draft) {
        e.preventDefault();
        void save();
      }
    };
    const unload = (e: BeforeUnloadEvent) => {
      if (draft?.dirty) e.preventDefault();
    };
    // an undo or a take-back changed records under the Library: read them again
    const realm = () => {
      if (!table) return;
      if (app.route.place !== 'library') {
        stale = true;
        return;
      }
      void readRows(table);
      if (key) void readRecord(table, key, ++seq);
      version++;
    };
    const wrote = () => {
      if (!writing) stale = true;
    };
    document.addEventListener('keydown', keys);
    window.addEventListener('beforeunload', unload);
    window.addEventListener('dd:realm', realm);
    window.addEventListener('dd:wrote', wrote);
    return () => {
      document.removeEventListener('keydown', keys);
      window.removeEventListener('beforeunload', unload);
      window.removeEventListener('dd:realm', realm);
      window.removeEventListener('dd:wrote', wrote);
    };
  });
</script>

<div class="lib" class:has-card={!!key} class:browsing>
  <aside class="lib-side" aria-label={k.many}>
    <nav class="tables" aria-label="Tables">
      {#each tables as t (t.name)}
        {@const tk = kindOf(t.name)}
        <a class="tabchip {tk.cls}" class:on={t === table} href={t === table ? here() : recordHref(t.name)} aria-current={t === table ? 'page' : undefined}>
          <span class="tile"><tk.icon size={12} /></span>{tk.many}
        </a>
      {/each}
    </nav>
    {#if table}
      <div class="find">
        <label class="search">
          <SearchIcon size={15} />
          <input type="search" placeholder="Find {many} by name or number" aria-label="Find {many} by name or number" bind:value={find} />
        </label>
        {#if filters.length || changed.size}
          <div class="types" role="group" aria-label="Show only">
            <button type="button" class:on={!chip} aria-pressed={!chip} onclick={() => (chip = null)}>All</button>
            {#each filters as f (f.label)}
              <button type="button" class:on={chip === f.label} aria-pressed={chip === f.label} onclick={() => (chip = chip === f.label ? null : f.label)}>{f.label}</button>
            {/each}
            {#if changed.size}
              <button type="button" class:on={chip === CHANGED} aria-pressed={chip === CHANGED} title="Changed since the world was last published" onclick={() => (chip = chip === CHANGED ? null : CHANGED)}>
                <span class="dot cdot"></span>{CHANGED}
              </button>
            {/if}
          </div>
        {/if}
      </div>
      <div class="count">
        <span>{reading && !rows.length ? '' : counted}</span>
        <NewRecord
          {table}
          offer={() => nextKey(table!.name, numeric, rows, near())}
          taken={(l) => rows.some((r) => r.label === l)}
          says="Add a new {one}"
          act="Add"
          cls="btn sm ghost"
          oncreate={(nk) => void create(nk, false)}
        >
          <Plus size={14} /> New
        </NewRecord>
      </div>
      <div class="list" bind:this={listEl} bind:clientHeight={tall} onscroll={() => (top = listEl!.scrollTop)}>
        {#if failed}
          <p class="pad bad">{failed}</p>
        {:else if !rows.length && reading}
          <p class="pad faint">Reading the {many}…</p>
        {:else if !shown.length}
          <div class="none"><Mascot mood="worry" size={56} /><p>{rows.length ? `No ${many} match.` : `No ${many} yet.`}</p></div>
        {:else}
          <div class="rows" style:height="{shown.length * ROW}px">
            {#each visible as r, i (r.label)}
              <a
                class="li"
                class:on={r.label === open}
                href={recordHref(table.name, r.key)}
                style:top="{(first + i) * ROW}px"
                style:height="{ROW}px"
                aria-current={r.label === open ? 'true' : undefined}
                onclick={() => (browsing = false)}
              >
                <span class="tile ic {k.cls}"><k.icon size={14} /></span>
                <span class="t">
                  <b>
                    {#if worded}<span class="nm">{r.label}</span>{:else if r.name}<span class="nm">{r.name}</span>{:else}<i class="nm faint">no name</i>{/if}
                    {#if changed.has(r.label)}<span class="dot cdot" title="Changed since publishing"></span>{/if}
                  </b>
                  {#if worded}<span class="sum">{r.name}</span>{:else if def?.summary}<span class="sum">{def.summary(r.get)}</span>{/if}
                </span>
                {#if !worded}<span class="n">{r.label}</span>{/if}
              </a>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </aside>

  <main class="lib-main">
    {#if table && key && draft}
      <Card
        {draft}
        {table}
        {rows}
        {version}
        {saving}
        onsave={() => void save()}
        oncopy={(nk) => void create(nk, true)}
        ondelete={() => void remove()}
        onbrowse={() => (browsing = true)}
      />
    {:else if table && key && missing}
      <div class="empty">
        <Mascot mood="worry" size={84} />
        <p>There is no {one} {open} in this world.</p>
        <p class="faint small">{missing}</p>
        <a class="btn sm" href={recordHref(table.name)}>Back to the {many}</a>
      </div>
    {:else if table && key}
      <p class="pad faint">Reading {one} {open}…</p>
    {:else if table}
      <div class="empty">
        <Mascot mood="dream" size={84} />
        <p>Pick {article} {one} from the list, or add a new one.</p>
        <p class="faint small"><span class="kbd">[</span> and <span class="kbd">]</span> step through the list; <span class="kbd">Ctrl</span> <span class="kbd">S</span> saves.</p>
      </div>
    {/if}
  </main>
</div>

<style>
  .lib { display: grid; grid-template-columns: 300px minmax(0, 1fr); height: 100%; overflow: hidden; position: relative; }
  .lib-side { border-right: 1px solid var(--line); background: var(--surface); display: flex; flex-direction: column; min-height: 0; }
  .tables { display: flex; gap: 4px; flex-wrap: wrap; padding: 12px 12px 8px; }
  .tabchip {
    display: inline-flex; align-items: center; gap: 5px; height: 28px; padding: 0 10px 0 4px; border-radius: 999px;
    border: 1px solid var(--line); background: var(--surface-2); font: 800 12px var(--font-ui); color: var(--ink-2); text-decoration: none;
  }
  .tabchip:hover { border-color: var(--ink-3); color: var(--ink); }
  .tabchip.on { border-color: var(--outline); color: var(--ink); background: var(--surface); box-shadow: 0 2px 0 var(--shadow); }
  .find { padding: 0 12px 6px; display: flex; flex-direction: column; gap: 8px; }
  .search { display: flex; align-items: center; gap: 7px; padding: 0 10px; border-radius: 999px; border: 1px solid var(--line-2); background: var(--surface-2); color: var(--ink-3); }
  .search:focus-within { border-color: var(--focus); background: var(--surface); box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus) 22%, transparent); }
  .search input { flex: 1; border: 0; background: transparent; padding: 7px 0; box-shadow: none; }
  .search input:focus { box-shadow: none; background: transparent; }
  .types { display: flex; gap: 4px; flex-wrap: wrap; }
  .types button {
    min-height: 26px; padding: 0 9px; border-radius: 8px; border: 1px solid var(--line); background: transparent;
    color: var(--ink-2); font: 800 11.5px var(--font-ui); box-shadow: none; gap: 5px;
  }
  .types button.on { background: var(--lilac-2); color: var(--lilac-ink); border-color: transparent; }
  .cdot { color: var(--butter); width: 7px; height: 7px; }
  .count { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 0 8px 4px 14px; color: var(--ink-3); font-size: 12px; font-weight: 800; }
  .list { flex: 1; overflow: auto; padding: 0 6px 12px; min-height: 0; }
  .rows { position: relative; }
  .li {
    position: absolute; left: 0; right: 0; display: flex; align-items: center; gap: 9px; padding: 0 8px;
    border-radius: 11px; color: var(--ink); text-decoration: none;
  }
  .li:hover { background: var(--surface-2); }
  .li.on { background: var(--select); box-shadow: inset 0 0 0 1.5px var(--outline); }
  .li .ic { width: 28px; height: 28px; border-radius: 9px; }
  .li .t { display: flex; flex-direction: column; min-width: 0; line-height: 1.2; flex: 1; }
  .li .t b { font-size: 13px; display: flex; align-items: center; gap: 5px; min-width: 0; }
  .li .nm { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .li i.nm { font-weight: 700; }
  .li .sum { font-size: 11.5px; color: var(--ink-3); font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .li .n { font: 600 11px var(--font-mono); color: var(--ink-3); font-variant-numeric: tabular-nums; }
  .lib-main { overflow: auto; min-width: 0; min-height: 0; }
  .pad { padding: 14px; margin: 0; }
  .bad { color: var(--bad); font-weight: 700; }
  .none, .empty { display: grid; place-items: center; gap: 8px; padding: 32px 20px; text-align: center; color: var(--ink-2); font-weight: 700; }
  .none p, .empty p { margin: 0; }
  .empty { min-height: 60%; align-content: center; }
  .small { font-size: 12.5px; max-width: 34rem; overflow-wrap: anywhere; }
  @media (max-width: 760px) {
    .lib { grid-template-columns: minmax(0, 1fr); }
    .lib.has-card .lib-side { display: none; }
    .lib.has-card.browsing .lib-side { display: flex; position: absolute; inset: 0; z-index: 5; }
    .lib:not(.has-card) .lib-main { display: none; }
  }
</style>
