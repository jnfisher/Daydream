<!--
  What is wrong with the record on its card (problems.ts): a pill in its header that counts
  them, opening the list. Each says what is wrong, with its fix (typed into the card, so it is
  one Save, and one undo) and a way to the field; a line borrowed from another monster, with a
  copy of the message of its own.
-->
<svelte:options css="injected" />

<script lang="ts">
  import BroomSparkles from '@lucide/svelte/icons/broom-sparkles';
  import CircleAlert from '@lucide/svelte/icons/circle-exclamation-point';
  import CopyPlus from '@lucide/svelte/icons/copy-plus';
  import Locate from '@lucide/svelte/icons/locate';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import WandSparkles from '@lucide/svelte/icons/wand-sparkles';
  import { Popover } from 'bits-ui';
  import { allFixes, type Problem } from './problems';
  import Said from './Said.svelte';

  let {
    problems,
    onfix,
    onshow,
    onaccept,
    onown,
  }: {
    problems: Problem[];
    /** Types a fix's values into the card. */
    onfix: (set: Record<string, string>) => void;
    /** Brings a column's field into view, answering whether the card shows it. */
    onshow: (col: string) => boolean;
    /** Tells spelling to leave a word alone. */
    onaccept?: (word: string) => void;
    /** Gives the record a copy of a message of its own. */
    onown?: (message: number) => void;
  } = $props();

  let open = $state(false);
  const worst = $derived(problems.some((p) => p.level === 'error') ? 'error' : problems.some((p) => p.level === 'warn') ? 'warn' : 'tidy');
  const says = $derived.by(() => {
    const n = problems.length;
    if (worst === 'error') return `${n} ${n === 1 ? 'problem' : 'problems'}`;
    return worst === 'warn' ? `${n} to look at` : `${n} to tidy`;
  });
  const fixable = $derived(problems.filter((p) => p.fix));
  const ICON = { error: CircleAlert, warn: TriangleAlert, tidy: BroomSparkles };

  function fix(set: Record<string, string>) {
    onfix(set);
    if (problems.length <= 1) open = false;
  }
  function show(col: string) {
    open = false;
    onshow(col);
  }
</script>

{#if problems.length}
  {@const Icon = ICON[worst]}
  <Popover.Root bind:open>
    <Popover.Trigger class="pill as-btn probs-pill {worst}"><Icon size={12} /> {says}</Popover.Trigger>
    <Popover.Portal>
      <Popover.Content class="pop probs-pop" sideOffset={6} align="start" collisionPadding={8}>
        <ul>
          {#each problems as p, i (i)}
            {@const PIcon = ICON[p.level]}
            <li class={p.level}>
              <span class="ic"><PIcon size={15} /></span>
              <span class="t">{#if p.bits}<Said bits={p.bits} />{:else}{p.text}{/if}</span>
              <span class="go">
                {#if p.fix}
                  {@const set = p.fix.set}
                  <button type="button" class="btn sm" onclick={() => fix(set)}><WandSparkles size={13} /> {p.fix.label}</button>
                {/if}
                {#if p.own && onown}
                  {@const m = p.own}
                  <button
                    type="button"
                    class="btn sm"
                    onclick={() => {
                      open = false;
                      onown(m);
                    }}
                    title="A copy of message {m} for this record alone, its name written in"><CopyPlus size={13} /> Give it its own copy</button
                  >
                {/if}
                {#if p.accept && onaccept}
                  {@const w = p.accept}
                  <button type="button" class="btn sm ghost" onclick={() => onaccept(w)} title="The world means it: spelling leaves it alone">Not a typo</button>
                {/if}
                {#if p.level !== 'tidy' && p.cols[0]}
                  {@const col = p.cols[0]}
                  <button type="button" class="btn sm ghost" onclick={() => show(col)} title="Go to the field"><Locate size={13} /> Show</button>
                {/if}
              </span>
            </li>
          {/each}
        </ul>
        <div class="foot">
          <span class="faint">A fix fills in the fields. Save keeps it, Undo takes it back.</span>
          {#if fixable.length > 1}
            <button type="button" class="btn sm" onclick={() => fix(allFixes(fixable))}><WandSparkles size={13} /> Fix all {fixable.length}</button>
          {/if}
        </div>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
{/if}

<style>
  :global(.probs-pill.error) { background: var(--bad-bg); color: var(--bad); }
  :global(.probs-pill.warn) { background: var(--warn-bg); color: var(--warn); }
  :global(.probs-pill.tidy) { background: var(--lilac-2); color: var(--lilac-ink); }
  :global(.probs-pop) { width: min(460px, calc(100vw - 16px)); padding: 10px 12px; display: grid; gap: 8px; }
  :global(.probs-pop) ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; max-height: min(60vh, 420px); overflow: auto; }
  :global(.probs-pop) li { display: grid; grid-template-columns: 20px minmax(0, 1fr); gap: 4px 8px; padding: 7px 0; border-bottom: 1px dashed var(--line); font-size: 13px; font-weight: 700; line-height: 1.35; }
  :global(.probs-pop) li:last-child { border-bottom: 0; }
  :global(.probs-pop) .ic { display: inline-flex; padding-top: 1px; }
  :global(.probs-pop) .error .ic { color: var(--bad); }
  :global(.probs-pop) .warn .ic { color: var(--warn); }
  :global(.probs-pop) .tidy .ic { color: var(--lilac-ink); }
  :global(.probs-pop) .go { grid-column: 2; display: flex; gap: 6px; flex-wrap: wrap; }
  :global(.probs-pop) .go:empty { display: none; }
  :global(.probs-pop) .foot { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; justify-content: space-between; font-size: 12px; font-weight: 700; }
</style>
