<!--
  Picks a number by name: a search over a table's records (or the abilities), the closest
  first. Typing a number still works in the field beside it.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import Search from '@lucide/svelte/icons/search';
  import { Popover } from 'bits-ui';
  import type { Choice } from './sections';

  let {
    what,
    choices,
    onpick,
    label,
  }: {
    what: string;
    choices: () => Promise<Choice[]> | Choice[];
    onpick: (v: number) => void;
    /** A word on the button (`Add`), which then adds rather than finds. */
    label?: string;
  } = $props();

  let open = $state(false);
  let find = $state('');
  let all = $state<Choice[] | null>(null);

  $effect(() => {
    if (!open || all) return;
    void Promise.resolve(choices()).then((c) => (all = c));
  });

  const MAX = 60;
  const shown = $derived.by(() => {
    if (!all) return [];
    const q = find.trim().toLowerCase();
    if (!q) return all.slice(0, MAX);
    const n = Number(q);
    const scored: [number, Choice][] = [];
    for (const c of all) {
      const name = c.label.toLowerCase();
      const score = c.value === n ? 0 : name.startsWith(q) ? 1 : name.includes(q) ? 2 : String(c.value).startsWith(q) ? 3 : -1;
      if (score >= 0) scored.push([score, c]);
    }
    return scored
      .sort((a, b) => a[0] - b[0])
      .slice(0, MAX)
      .map(([, c]) => c);
  });

  function pick(v: number) {
    onpick(v);
    open = false;
    find = '';
  }
</script>

<Popover.Root bind:open>
  {#if label}
    <Popover.Trigger class="btn sm ghost pick" title="Add {what}, found by name or number"><Plus size={14} /> {label}</Popover.Trigger>
  {:else}
    <Popover.Trigger class="btn icon sm ghost pick" aria-label="Pick {what} by name" title="Pick {what} by name"><Search size={14} /></Popover.Trigger>
  {/if}
  <Popover.Portal>
    <Popover.Content class="pop picker" sideOffset={6} align="start" collisionPadding={8}>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="search" placeholder="Name or number" aria-label="Find {what}" bind:value={find} autofocus />
      <div class="list">
        {#if !all}
          <p class="faint">Reading…</p>
        {:else if !shown.length}
          <p class="faint">Nothing matches.</p>
        {:else}
          {#each shown as c (c.value)}
            <button type="button" class="menu-item" onclick={() => pick(c.value)}>
              <span class="n tnum">{c.value}</span>
              <span class="lbl">{c.label}{#if c.sub}<span class="sub">{c.sub}</span>{/if}</span>
            </button>
          {/each}
        {/if}
      </div>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.picker) { width: min(360px, calc(100vw - 16px)); display: flex; flex-direction: column; gap: 6px; }
  :global(.picker) input { width: 100%; }
  :global(.picker) .list { max-height: 320px; overflow: auto; }
  :global(.picker) .n { font: 600 11.5px/1 var(--font-mono); color: var(--ink-3); min-width: 3.2em; text-align: right; }
  :global(.picker) .lbl { min-width: 0; }
  :global(.picker) .sub { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  :global(.picker) p { margin: 8px; }
</style>
