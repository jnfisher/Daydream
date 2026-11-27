<!--
  Who else uses the message a field names: a badge beside its chip saying how many other
  records read it. Its popover says who, that a change to the message changes them all, and
  gives the record a copy of its own (the Library makes it: `dd:own-message`).
-->
<svelte:options css="injected" />

<script lang="ts">
  import CopyPlus from '@lucide/svelte/icons/copy-plus';
  import Users from '@lucide/svelte/icons/users';
  import { Popover } from 'bits-ui';
  import { kindOf } from '../shell/kinds';
  import type { Draft } from './draft.svelte';
  import { countedTables } from './msguse';
  import { sharedWith } from './msgusers.svelte';
  import RefChip from './RefChip.svelte';

  let { draft, n }: { draft: Draft; n: number } = $props();

  const SHOWN = 8;
  let open = $state(false);
  const shared = $derived(draft.row ? sharedWith(n, draft.table.name, draft.row.key) : undefined);
  const others = $derived(shared?.others ?? 0);
  const one = $derived(kindOf(draft.table.name).one.toLowerCase());
  const counted = (t: string, k: number) => {
    const kind = kindOf(t);
    return `${k.toLocaleString()} ${(k === 1 ? kind.one : kind.many).toLowerCase()}`;
  };
  const said = $derived(shared ? countedTables(shared.tables, counted) : '');

  function own() {
    open = false;
    window.dispatchEvent(new CustomEvent('dd:own-message', { detail: { table: draft.table.name, key: draft.row?.key, number: n } }));
  }
</script>

{#if shared && others}
  <Popover.Root bind:open>
    <Popover.Trigger class="shared-n" title="Message {n} is also used by {said}" aria-label="Message {n} is also used by {said}">
      <Users size={12} />{others.toLocaleString()}
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content class="pop was-pop shared-pop" sideOffset={6} align="start" collisionPadding={8}>
        <p>Message {n} is also used by {said}.</p>
        <span class="faint">A change to its lines changes what each of them says too.</span>
        <div class="chips">
          {#each shared.some.slice(0, SHOWN) as u (`${u.table}/${u.key.join('/')}`)}<RefChip table={u.table} key={u.key} compact />{/each}
          {#if others > SHOWN}<span class="faint">+{(others - SHOWN).toLocaleString()} more</span>{/if}
        </div>
        <button type="button" class="btn sm" onclick={own}><CopyPlus size={14} /> Give this {one} its own copy</button>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
{/if}

<style>
  :global(.shared-n) {
    display: inline-flex; align-items: center; gap: 3px; height: 22px; min-height: 0; padding: 0 7px; flex: none;
    border-radius: 999px; border: 1px solid var(--line); background: var(--sky-2); color: var(--sky-ink);
    font: 800 11px/1 var(--font-ui); box-shadow: none; cursor: pointer;
  }
  :global(.shared-n:hover) { border-color: var(--ink-3); }
  :global(.shared-pop) .chips { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; }
  :global(.shared-pop) .faint { font-size: 12px; }
</style>
