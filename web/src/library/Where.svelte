<!--
  Where a record is used: every row of the world that names it, grouped by how ("Dropped by
  3 monsters", "Exits lead here from 2 rooms"), each a chip that opens it beside the card.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { api, type Cell } from '../api';
  import { kindOf } from '../shell/kinds';
  import { messageOf } from '../shell/toasts.svelte';
  import RefChip from './RefChip.svelte';
  import { groupReferences, type Group } from './where';

  let { table, rowKey, version }: { table: string; rowKey: Cell[]; version: number } = $props();

  let groups = $state<Group[] | null>(null);
  let more = $state(false);
  let error = $state('');
  let opened = $state<Record<string, boolean>>({});
  const SHOWN = 12;

  const counted = (t: string, n: number) => {
    const k = kindOf(t);
    return `${n.toLocaleString()} ${(n === 1 ? k.one : k.many).toLowerCase()}`;
  };

  $effect(() => {
    const [t, key] = [table, rowKey];
    void version;
    let gone = false;
    error = '';
    api
      .references(t, key)
      .then((r) => {
        if (gone) return;
        groups = groupReferences(r.references, counted);
        more = r.more;
      })
      .catch((e) => {
        if (!gone) error = messageOf(e);
      });
    return () => (gone = true);
  });

  // another record: its groups start folded
  $effect(() => {
    void rowKey;
    opened = {};
  });
</script>

{#if error}
  <p class="faint">{error}</p>
{:else if !groups}
  <p class="faint">Looking through the world…</p>
{:else if !groups.length}
  <p class="faint">Nothing in the world names it.</p>
{:else}
  <ul class="where">
    {#each groups as g (g.says + g.table)}
      {@const k = kindOf(g.table)}
      {@const all = opened[g.says + g.table]}
      <li>
        <span class="tile {k.cls} big"><k.icon size={16} /></span>
        <div class="body">
          <div class="says">{g.says}</div>
          <div class="chips">
            {#each all ? g.rows : g.rows.slice(0, SHOWN) as r (r.key.join('/'))}
              <RefChip table={r.table} key={r.key} compact why={r.by.join(', ')} />
            {/each}
            {#if !all && g.rows.length > SHOWN}
              <button type="button" class="btn sm" onclick={() => (opened[g.says + g.table] = true)}>+{(g.rows.length - SHOWN).toLocaleString()} more</button>
            {/if}
          </div>
        </div>
      </li>
    {/each}
  </ul>
  {#if more}<p class="faint">And more: only the first 500 rows are listed.</p>{/if}
{/if}

<style>
  .where { list-style: none; margin: 0; padding: 0; display: grid; gap: 12px; }
  .where li { display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 10px; align-items: start; }
  .where li + li { padding-top: 12px; border-top: 1px dashed var(--line); }
  .tile.big { width: 34px; height: 34px; border-radius: 11px; }
  .says { font-weight: 800; margin: 6px 0 6px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  p { margin: 0; }
</style>
