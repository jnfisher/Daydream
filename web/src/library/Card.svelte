<!--
  A record's card: who it is (its kind, number and name, the words for what it is, and what
  changed since the world was last published), then its fields grouped by what they are for
  (cards.ts), or NMR's own tabs ("All fields"). Save, Discard, copy it to a new record or
  delete it: the Library (Library.svelte) does the writing.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy';
  import GitCompare from '@lucide/svelte/icons/git-compare';
  import History from '@lucide/svelte/icons/history';
  import List from '@lucide/svelte/icons/list';
  import MapIcon from '@lucide/svelte/icons/map';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import { Popover } from 'bits-ui';
  import { tick } from 'svelte';
  import { titleOf } from '../ansi';
  import type { Cell, TableInfo } from '../api';
  import { exitParam, layoutFor, targetOf } from '../layouts';
  import { kindOf } from '../shell/kinds';
  import { read, write } from '../shell/prefs.svelte';
  import { app, takeBack } from '../shell/state.svelte';
  import CardSection from './CardSection.svelte';
  import { CARDS } from './cards';
  import Compare from './Compare.svelte';
  import { shown, type Draft } from './draft.svelte';
  import { keyLabel, nextKey, type ListRow } from './list';
  import { nameOf } from './names.svelte';
  import NewRecord from './NewRecord.svelte';
  import { byLevel, clampTo, labelsOf, leftoverProblem, refOf, storable, trailingSpaces, TWO_SPACES, type Problem, type Targets } from './problems';
  import Problems from './Problems.svelte';
  import { linesOf } from './records.svelte';
  import { complete, fromLayout, type Section } from './sections';

  let {
    draft,
    table,
    rows,
    version,
    saving,
    onsave,
    oncopy,
    ondelete,
    onbrowse,
  }: {
    draft: Draft;
    table: TableInfo;
    /** The table's list, for the records a number is drawn among and the keys taken. */
    rows: ListRow[];
    /** Bumped by every write, so where the record is used is read again. */
    version: number;
    saving: boolean;
    onsave: () => void;
    oncopy: (key: Cell[]) => void;
    ondelete: () => void;
    /** On a phone: back to the list. */
    onbrowse: () => void;
  } = $props();

  const def = $derived(CARDS[table.name]);
  const k = $derived(kindOf(table.name));
  const one = $derived(k.one.toLowerCase());
  const key = $derived(draft.row?.key ?? []);
  const label = $derived(keyLabel(key));
  const name = $derived(table.title ? titleOf(shown(draft.get(table.title))) : '');
  const tags = $derived(def?.tags?.(draft.get).filter(Boolean) ?? []);

  let view = $state<'card' | 'all'>(read('dd-lib-view', 'card') === 'all' ? 'all' : 'card');
  const setView = (v: 'card' | 'all') => {
    view = v;
    write('dd-lib-view', v);
  };

  const WHERE: Section = { id: 'where', title: 'Where it is in the world', side: true, where: true };
  const columns = $derived(table.columns.map((c) => c.name));
  const numeric = $derived(table.key.map((c) => table.columns.find((x) => x.name === c)?.type !== 'text'));
  // records are named by number: a record keyed by a word (an action) is named by nothing
  const sections = $derived(
    (def && view === 'card' ? complete(def.sections, columns, table.key, draft.get) : [...fromLayout(layoutFor(table), columns, table.key), WHERE]).filter(
      (s) => !s.where || numeric.every(Boolean),
    ),
  );
  const main = $derived(sections.filter((s) => !s.side));
  const side = $derived(sections.filter((s) => s.side));

  // the records a number is drawn among: its peers' saved values, as the list read them
  const peerRows = $derived.by(() => {
    const p = def?.peers;
    if (!p || !draft.row) return [];
    return rows.filter((r) => r.label !== label && p.same(draft.get, r.get));
  });
  const listed = $derived(new Set(Object.keys(rows[0]?.vals ?? {})));
  const peers = (col: string) => (listed.has(col) ? peerRows.map((r) => Number(r.vals[col]) || 0) : null);
  const peersSay = $derived(def?.peers?.says(draft.get) ?? '');

  // what changed since publishing, and taking it back
  const was = $derived(draft.row?.published && 'was' in draft.row.published ? draft.row.published.was : {});
  const mine = $derived((app.changes?.changes ?? []).filter((c) => c.nmr === table.name && c.key && keyLabel(c.key) === label));
  let wasOpen = $state(false);
  function takeItBack() {
    wasOpen = false;
    void takeBack(
      mine.map(({ stem, id }) => ({ stem, id })),
      `${one} ${label}`,
    );
  }

  let deleting = $state(false);
  let comparing = $state(false);
  const taken = (l: string) => rows.some((r) => r.label === l);

  // ---- what is wrong with it -------------------------------------------------------------

  const TARGETS: Targets = { targetOf, exitLinks: (ty, para) => exitParam(ty, para)?.links };
  /** The sections its problems are read from: its card's (whichever view is open), or NMR's tabs. */
  const checked = $derived(def ? def.sections : fromLayout(layoutFor(table), columns, table.key));
  const labels = $derived(labelsOf(checked));
  const labelOf = (col: string) => labels.get(col) ?? col;
  const fmt = (n: number) => n.toLocaleString('en-US');

  const problems = $derived.by((): Problem[] => {
    if (!draft.row) return [];
    const out: Problem[] = [];
    // what an input holds that will not do
    for (const col of draft.invalid) {
      const info = draft.column(col)!;
      const typed = draft.typed(col);
      let fix: Problem['fix'];
      if (info.range && Number.isFinite(Number(typed))) {
        const v = clampTo(info.range, Number(typed));
        fix = { label: `Make it ${fmt(v)}`, set: { [col]: String(v) } };
      } else if (!draft.isNumber(col)) {
        fix = { label: 'Take them out', set: { [col]: storable(typed, table.name === 'Textblocks' && col === 'Data') } };
      }
      out.push({ level: 'error', text: `${labelOf(col)}: ${draft.check(col)}`, cols: [col], fix });
    }
    // numbers that name a record the world does not have
    for (const c of table.columns) {
      if (draft.isKey(c.name) || draft.check(c.name)) continue;
      const r = refOf(table.name, c.name, draft.get, TARGETS);
      if (!r || nameOf(r.table, r.key) !== null) continue;
      // the game takes such a number (it shows nothing, or falls back): one to look at
      const kind = kindOf(r.table).one.toLowerCase();
      const n = Number(draft.get(c.name));
      out.push({
        level: 'warn',
        text: `${labelOf(c.name)} names ${kind} ${r.table === 'Textblocks' ? n : keyLabel(r.key)}, which the world does not have${n === TWO_SPACES ? ` (${n} is two spaces an old tool left)` : ''}.`,
        cols: [c.name],
        fix: { label: 'Set it to 0', set: { [c.name]: '0' } },
      });
    }
    // empty slots that still hold an old entry
    for (const s of checked) {
      if (!s.grid || s.custom) continue;
      const p = leftoverProblem(s.grid, s.title.toLowerCase(), draft.get, (c) => draft.has(c));
      if (p) out.push(p);
    }
    // what the card's own table knows
    out.push(...(def?.problems?.(draft.get, { lines: linesOf }) ?? []));
    if (table.name === 'Textblocks' && draft.has('Data')) {
      const p = trailingSpaces('Data', 'The text', draft.typed('Data'));
      if (p) out.push(p);
    }
    return byLevel(out);
  });

  function fix(set: Record<string, string>) {
    for (const [col, v] of Object.entries(set)) if (draft.has(col)) draft.set(col, v);
  }

  /** Brings a column's field into view: its section opened first, when it is folded. */
  function show(col: string): boolean {
    window.dispatchEvent(new CustomEvent('dd:reveal', { detail: col }));
    void tick().then(() => {
      const el = document.querySelector<HTMLElement>(`.rcard [data-col="${CSS.escape(col)}"]`);
      if (!el) return;
      el.scrollIntoView({ block: 'center', behavior: 'smooth' });
      el.querySelector<HTMLElement>('input, select, textarea')?.focus({ preventScroll: true });
    });
    return true;
  }
</script>

<article class="rcard" aria-label="{k.one} {label}">
  <header class="hero sky">
    <button type="button" class="btn sm browse" onclick={onbrowse}><List size={14} /> All {k.many.toLowerCase()}</button>
    <div class="hero-in">
      <span class="big {k.cls}"><k.icon size={34} /></span>
      <div class="who">
        <div class="chips">
          <span class="num-badge">{k.one} {label}</span>
          {#each tags as t, i (i)}<span class="pill">{t}</span>{/each}
          {#if draft.added}
            <Popover.Root bind:open={wasOpen}>
              <Popover.Trigger class="pill warn as-btn"><History size={12} /> New since publishing</Popover.Trigger>
              <Popover.Portal>
                <Popover.Content class="pop was-pop" sideOffset={6} align="start" collisionPadding={8}>
                  <p>This {one} was added since the world was last published.</p>
                  {#if mine.length}<button type="button" class="btn sm" onclick={takeItBack}><Undo2 size={14} /> Take it back</button>{/if}
                </Popover.Content>
              </Popover.Portal>
            </Popover.Root>
          {:else if draft.unpublished}
            <Popover.Root bind:open={wasOpen}>
              <Popover.Trigger class="pill warn as-btn">
                <History size={12} />
                {draft.unpublished} unpublished change{draft.unpublished === 1 ? '' : 's'}
              </Popover.Trigger>
              <Popover.Portal>
                <Popover.Content class="pop was-pop" sideOffset={6} align="start" collisionPadding={8}>
                  <p>Changed since the world was last published:</p>
                  <ul>
                    {#each Object.entries(was) as [col, v] (col)}
                      <li><b>{col}</b> was <code>{shown(v) || '(blank)'}</code></li>
                    {/each}
                  </ul>
                  {#if mine.length}<button type="button" class="btn sm" onclick={takeItBack}><Undo2 size={14} /> Take back to as published</button>{/if}
                </Popover.Content>
              </Popover.Portal>
            </Popover.Root>
          {/if}
          <Problems {problems} onfix={fix} onshow={show} />
        </div>
        <h1>{name || `${k.one} ${label}`}</h1>
      </div>
      <div class="actions">
        {#if draft.dirty}
          <button type="button" class="btn sm ghost" onclick={() => draft.discard()} title="Put the fields back as saved">Discard</button>
        {/if}
        <button
          type="button"
          class="btn sm primary"
          disabled={!draft.dirty || saving || draft.invalid.length > 0}
          onclick={onsave}
          title={draft.invalid.length ? 'Fix the fields marked in red first' : 'Save the changes (Ctrl+S)'}
        >
          {saving ? 'Saving…' : 'Save'}
        </button>
        <NewRecord
          {table}
          offer={() => nextKey(table.name, numeric, rows, key)}
          {taken}
          says="Copy {one} {label} to a new {one}{draft.dirty ? ' (its changes saved first)' : ''}"
          act="Copy"
          cls="btn sm"
          oncreate={oncopy}
        >
          <Copy size={14} /> Copy
        </NewRecord>
        {#if numeric.every(Boolean) && rows.length > 1 && table.name !== 'Textblocks'}
          <button type="button" class="btn sm" onclick={() => (comparing = true)} title="Put this {one} beside the most like it, the differences marked">
            <GitCompare size={14} /> Compare
          </button>
        {/if}
        {#if table.name === 'Rooms'}
          <a class="btn sm" href="#/world/{key[0]}/{key[1]}" title="Open the map at this room"><MapIcon size={14} /> On the map</a>
        {/if}
        <Popover.Root bind:open={deleting}>
          <Popover.Trigger class="btn sm icon ghost danger" aria-label="Delete {one} {label}" title="Delete {one} {label}"><Trash2 size={15} /></Popover.Trigger>
          <Popover.Portal>
            <Popover.Content class="pop was-pop" sideOffset={6} align="end" collisionPadding={8}>
              <p>Delete {one} {label}{name ? `, ${name}` : ''}? Undo puts it back.</p>
              <div class="go">
                <button type="button" class="btn sm ghost" onclick={() => (deleting = false)}>Keep it</button>
                <button
                  type="button"
                  class="btn sm danger"
                  onclick={() => {
                    deleting = false;
                    ondelete();
                  }}><Trash2 size={14} /> Delete</button
                >
              </div>
            </Popover.Content>
          </Popover.Portal>
        </Popover.Root>
      </div>
    </div>
    {#if def}
      <div class="views" role="group" aria-label="Show">
        <button type="button" class:on={view === 'card'} aria-pressed={view === 'card'} onclick={() => setView('card')}>Card</button>
        <button type="button" class:on={view === 'all'} aria-pressed={view === 'all'} onclick={() => setView('all')} title="Every field, as Nightmare Redux's tabs lay them out">All fields</button>
      </div>
    {/if}
  </header>

  {#if comparing}
    <Compare bind:open={comparing} {draft} {table} {rows} sections={checked} {def} />
  {/if}

  <div class="cols" class:single={!side.length}>
    <div class="col">
      {#each main as s (s.id)}
        <CardSection {draft} section={s} table={table.name} rowKey={key} {version} {peers} {peerRows} {peersSay} />
      {/each}
    </div>
    {#if side.length}
      <div class="col">
        {#each side as s (s.id)}
          <CardSection {draft} section={s} table={table.name} rowKey={key} {version} {peers} {peerRows} {peersSay} />
        {/each}
      </div>
    {/if}
  </div>
</article>

<style>
  .rcard { container-type: inline-size; min-height: 100%; }
  .hero { padding: 20px clamp(14px, 3vw, 30px) 14px; border-bottom: 1px solid var(--line); }
  .hero-in { display: flex; gap: 16px; align-items: center; flex-wrap: wrap; max-width: 1240px; margin: 0 auto; }
  .big {
    width: 64px; height: 64px; border-radius: 20px; display: grid; place-items: center; flex: none;
    background: var(--t-big, var(--surface-3)); color: var(--on-pastel);
    border: var(--stroke) solid var(--outline); box-shadow: 0 4px 0 var(--shadow); transform: rotate(-4deg);
  }
  .big.t-item { --t-big: var(--butter); }
  .big.t-monster { --t-big: var(--peach); }
  .big.t-room { --t-big: var(--mint); }
  .big.t-spell { --t-big: var(--pink); }
  .big.t-text { --t-big: var(--lilac); }
  .big.t-msg { --t-big: var(--sky); }
  .big.t-shop { --t-big: var(--teal); }
  .big.t-other { --t-big: var(--lilac-2); color: var(--lilac-ink); }
  .who { min-width: 0; flex: 1 1 16rem; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; margin-bottom: 6px; }
  .num-badge { font: 700 12px/1 var(--font-mono); padding: 5px 7px; border-radius: 8px; background: var(--surface-3); color: var(--ink-2); }
  :global(.pill.as-btn) { border: 0; cursor: pointer; min-height: 0; box-shadow: none; }
  h1 { margin: 0; font-size: clamp(22px, 3vw, 30px); line-height: 1.1; overflow-wrap: anywhere; }
  .actions { margin-left: auto; display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
  .browse { display: none; }
  .views {
    display: flex; gap: 3px; padding: 3px; margin: 12px auto 0; width: fit-content; border-radius: 12px;
    background: var(--surface-2); border: 1px solid var(--line);
  }
  .views button { min-height: 26px; padding: 0 12px; border: 0; border-radius: 9px; background: transparent; color: var(--ink-2); font-size: 12.5px; box-shadow: none; }
  .views button.on { background: var(--surface); color: var(--ink); box-shadow: 0 0 0 1px var(--line-2), 0 1px 0 var(--line-2); }
  .cols { display: grid; gap: 16px; padding: 16px clamp(14px, 3vw, 30px) 60px; max-width: 1240px; margin: 0 auto; align-items: start; }
  .col { display: grid; gap: 16px; min-width: 0; }
  @container (min-width: 900px) {
    .cols:not(.single) { grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr); }
  }
  :global(.was-pop) { width: min(340px, calc(100vw - 16px)); padding: 12px; display: grid; gap: 8px; }
  :global(.was-pop) p { margin: 0; font-weight: 700; }
  :global(.was-pop) ul { margin: 0; padding-left: 18px; display: grid; gap: 3px; max-height: 240px; overflow: auto; }
  :global(.was-pop) code { font: 12px var(--font-mono); background: var(--surface-2); padding: 1px 5px; border-radius: 6px; }
  :global(.was-pop) .go { display: flex; justify-content: flex-end; gap: 6px; }
  :global(.was-pop > .btn) { justify-self: start; }
  @media (max-width: 760px) {
    .browse { display: inline-flex; margin-bottom: 12px; }
    .hero-in { gap: 12px; }
    .big { width: 48px; height: 48px; border-radius: 15px; }
    .big :global(svg) { width: 26px; height: 26px; }
  }
</style>
