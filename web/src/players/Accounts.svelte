<!--
  Whose a record is, among the players: a user's gang (and whether they lead it) and their bank
  books, each opening beside the card; or, on a bank book, the user it belongs to and the bank
  that keeps it.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Crown from '@lucide/svelte/icons/crown';
  import { api, type Cell } from '../api';
  import type { Draft } from '../library/draft.svelte';
  import FieldInput from '../library/FieldInput.svelte';
  import RefChip from '../library/RefChip.svelte';
  import { messageOf } from '../shell/toasts.svelte';

  let { draft, table, rowKey, version }: { draft: Draft; table: string; rowKey: Cell[]; version: number } = $props();

  const user = $derived(table === 'Users');
  const bbs = $derived(String(rowKey[0] ?? ''));
  const gang = $derived(user ? String(draft.get('Gang') ?? '').trim() : '');
  const fmt = (n: number) => n.toLocaleString();

  let books = $state<{ key: Cell[]; amount: number; loan: number }[] | null>(null);
  let leader = $state<string | null>(null);
  let error = $state('');

  // a user's bank books, read when the card opens and after each write
  $effect(() => {
    if (!user) return;
    const who = bbs;
    void version;
    let gone = false;
    error = '';
    api
      .rowsWith('BankBooks', ['Amount', 'Loan'])
      .then((r) => {
        if (gone) return;
        books = r.rows.filter(([k]) => String(k[0] ?? '') === who).map(([k, , [a, l]]) => ({ key: k, amount: Number(a) || 0, loan: Number(l) || 0 }));
      })
      .catch((e) => {
        if (!gone) error = messageOf(e);
      });
    return () => (gone = true);
  });

  // who leads their gang
  $effect(() => {
    const g = gang;
    void version;
    leader = null;
    if (!g) return;
    let gone = false;
    api
      .rowIfAny('Gangs', [g])
      .then((r) => {
        if (!gone) leader = r.values ? String(r.values['Leader'] ?? '').trim() : null;
      })
      .catch(() => {});
    return () => (gone = true);
  });

  const banked = $derived((books ?? []).reduce((n, b) => n + b.amount, 0));
  const owed = $derived((books ?? []).reduce((n, b) => n + b.loan, 0));
</script>

{#if user}
  <div class="frow">
    <span class="flabel">Gang</span>
    <span class="fctl"><FieldInput {draft} col="Gang" label="Gang" /></span>
  </div>
  {#if gang}
    <div class="chips">
      <RefChip table="Gangs" key={[gang]} />
      {#if leader !== null && leader === bbs}<span class="pill lead"><Crown size={12} /> Leads it</span>{/if}
    </div>
  {/if}
  <div class="eyebrow">Bank books</div>
  {#if error}
    <p class="faint">{error}</p>
  {:else if !books}
    <p class="faint">Looking in the banks…</p>
  {:else if !books.length}
    <p class="faint">No bank books.</p>
  {:else}
    <ul class="books">
      {#each books as b (b.key.join('/'))}
        <li>
          <RefChip table="Shops" key={[b.key[1] ?? 0]} compact why="the bank" />
          <span class="amt"><b>{fmt(b.amount)}</b> copper{#if b.loan}<span class="owes">&nbsp;· owes {fmt(b.loan)}</span>{/if}</span>
          <a class="btn sm ghost" href="#/bankbooks/{encodeURIComponent(bbs)}/{b.key[1]}" title="Open this bank book">Open</a>
        </li>
      {/each}
    </ul>
    {#if books.length > 1}<p class="faint small">{fmt(banked)} copper in all{owed ? `, ${fmt(owed)} owed` : ''}.</p>{/if}
  {/if}
{:else}
  <div class="chips col">
    <span class="faint small">Belongs to</span>
    <RefChip table="Users" key={[rowKey[0] ?? '']} />
    <span class="faint small">Kept by the bank</span>
    <RefChip table="Shops" key={[rowKey[1] ?? 0]} />
  </div>
{/if}

<style>
  .frow { display: grid; grid-template-columns: 7.5rem minmax(0, 1fr); align-items: center; gap: 4px 12px; padding: 6px 0; }
  .flabel { color: var(--ink-2); font-size: 13px; font-weight: 800; }
  .fctl { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chips.col { flex-direction: column; align-items: flex-start; }
  .lead { color: var(--butter-ink); background: var(--butter-2); display: inline-flex; gap: 4px; align-items: center; }
  .eyebrow { margin-top: 6px; }
  .books { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .books li { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .amt { font-size: 13.5px; }
  .owes { color: var(--warn); font-weight: 700; }
  .small { font-size: 12.5px; margin: 0; }
</style>
