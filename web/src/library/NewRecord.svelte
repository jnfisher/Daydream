<!--
  Makes a record: asks its key (the next free one offered), says what is wrong with a typed
  one, and hands it on. The Library writes the record: a copy of the one open, or a new one.
-->
<svelte:options css="injected" />

<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Popover } from 'bits-ui';
  import type { Cell, TableInfo } from '../api';
  import { kindOf } from '../shell/kinds';
  import { keyProblem } from './list';

  let {
    table,
    offer,
    taken,
    says,
    act,
    cls = 'btn sm',
    oncreate,
    children,
  }: {
    table: TableInfo;
    /** The key offered when it opens. */
    offer: () => Cell[];
    taken: (label: string) => boolean;
    /** What the popover says it makes: `Copy item 868 to a new item`. */
    says: string;
    /** Its button's word: `Copy`. */
    act: string;
    cls?: string;
    oncreate: (key: Cell[]) => void;
    children: Snippet;
  } = $props();

  let open = $state(false);
  let parts = $state<string[]>([]);
  const numeric = $derived(table.key.map((c) => table.columns.find((x) => x.name === c)?.type !== 'text'));
  const one = $derived(kindOf(table.name).one);
  const problem = $derived(keyProblem(parts, numeric, taken, one));

  function opened(o: boolean) {
    if (o) parts = offer().map((v) => String(v ?? ''));
  }

  function create(e: SubmitEvent) {
    e.preventDefault();
    if (problem) return;
    open = false;
    oncreate(parts.map((p, i) => (numeric[i] ? Number(p) : p.trim())));
  }
</script>

<Popover.Root bind:open onOpenChange={opened}>
  <Popover.Trigger class={cls}>{@render children()}</Popover.Trigger>
  <Popover.Portal>
    <Popover.Content class="pop new-rec" sideOffset={6} align="end" collisionPadding={8}>
      <form onsubmit={create}>
        <p class="says">{says}</p>
        <div class="parts">
          {#each table.key as col, i (col)}
            <label>
              <span>{col}</span>
              <!-- svelte-ignore a11y_autofocus -->
              <input
                type="text"
                class:num={numeric[i]}
                inputmode={numeric[i] ? 'numeric' : undefined}
                spellcheck="false"
                bind:value={parts[i]}
                autofocus={i === table.key.length - 1}
              />
            </label>
          {/each}
        </div>
        {#if problem}<p class="problem">{problem}</p>{/if}
        <div class="go">
          <button type="button" class="btn sm ghost" onclick={() => (open = false)}>Cancel</button>
          <button type="submit" class="btn sm primary" disabled={!!problem}>{act}</button>
        </div>
      </form>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.new-rec) { width: min(320px, calc(100vw - 16px)); padding: 12px; }
  :global(.new-rec) form { display: grid; gap: 10px; }
  :global(.new-rec) .says { margin: 0; font-weight: 800; }
  :global(.new-rec) .parts { display: flex; gap: 8px; flex-wrap: wrap; }
  :global(.new-rec) label { display: grid; gap: 4px; font-size: 12px; font-weight: 800; color: var(--ink-2); }
  :global(.new-rec) input { width: 8rem; }
  :global(.new-rec) input.num { font: 13px var(--font-mono); font-variant-numeric: tabular-nums; }
  :global(.new-rec) .problem { margin: 0; color: var(--bad); font-size: 12.5px; font-weight: 700; }
  :global(.new-rec) .go { display: flex; justify-content: flex-end; gap: 6px; }
</style>
