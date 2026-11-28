<!--
  Who is in a gang: every user whose Gang names it, by level, its leader first and crowned; and
  whether the gang's own count of members (which the game keeps) agrees, with the fix.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Crown from '@lucide/svelte/icons/crown';
  import { api, type Cell } from '../api';
  import type { Draft } from '../library/draft.svelte';
  import RefChip from '../library/RefChip.svelte';
  import { messageOf } from '../shell/toasts.svelte';

  let { draft, rowKey, version }: { draft: Draft; rowKey: Cell[]; version: number } = $props();

  const gang = $derived(String(rowKey[0] ?? ''));
  const leader = $derived(String(draft.get('Leader') ?? '').trim());
  const counted = $derived(Number(draft.get('Members')) || 0);

  let members = $state<{ bbs: string; level: number }[] | null>(null);
  let error = $state('');

  $effect(() => {
    const g = gang;
    void version;
    let gone = false;
    error = '';
    api
      .rowsWith('Users', ['Gang', 'Level'])
      .then((r) => {
        if (gone) return;
        members = r.rows
          .filter(([, , [gg]]) => String(gg ?? '').trim() === g)
          .map(([k, , [, lv]]) => ({ bbs: String(k[0] ?? ''), level: Number(lv) || 0 }))
          .sort((a, b) => Number(b.bbs === leader) - Number(a.bbs === leader) || b.level - a.level || a.bbs.localeCompare(b.bbs));
      })
      .catch((e) => {
        if (!gone) error = messageOf(e);
      });
    return () => (gone = true);
  });

  const leads = $derived(!!members && members.some((m) => m.bbs === leader));
</script>

{#if error}
  <p class="faint">{error}</p>
{:else if !members}
  <p class="faint">Looking through the users…</p>
{:else}
  {#if !members.length}
    <p class="faint">No user names this gang.</p>
  {:else}
    <ul class="who">
      {#each members as m (m.bbs)}
        <li>
          <RefChip table="Users" key={[m.bbs]} compact />
          <span class="lv">level {m.level}</span>
          {#if m.bbs === leader}<span class="pill lead"><Crown size={12} /> Leader</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
  {#if members.length !== counted}
    <div class="off">
      <span>The gang counts {counted} member{counted === 1 ? '' : 's'}; {members.length} user{members.length === 1 ? ' names' : 's name'} it.</span>
      <button type="button" class="btn sm" onclick={() => draft.set('Members', String(members!.length))}>Make it {members.length}</button>
    </div>
  {/if}
  {#if leader && !leads}
    <p class="off">Its leader, {leader}, is not among them{members.length ? '' : ' (or is not a user here)'}.</p>
  {/if}
{/if}

<style>
  .who { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .who li { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lv { color: var(--ink-3); font-size: 12.5px; font-weight: 700; }
  .lead { color: var(--butter-ink); background: var(--butter-2); display: inline-flex; gap: 4px; align-items: center; }
  .off { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin: 0; padding: 8px 10px; border-radius: 12px; background: var(--warn-bg); color: var(--warn); font-size: 13px; font-weight: 700; }
</style>
