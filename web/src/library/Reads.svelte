<!--
  How the game reads a message, on its card: each way it is used (a monster's hit lines, its
  death line, a script's words), the lines that use reads and for whom, filled in as the game
  fills them: for the first few records when their own names go in, else once for them all.
  The lines are the ones typed on the card, so an edit shows what each record will say; a
  message more than one record uses says that a change reaches them all.
-->
<svelte:options css="injected" />

<script lang="ts">
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import Users from '@lucide/svelte/icons/users';
  import { renderAnsi } from '../ansi';
  import { api, type Cell } from '../api';
  import { kindOf } from '../shell/kinds';
  import { messageOf } from '../shell/toasts.svelte';
  import type { Lines } from './combat';
  import type { Draft } from './draft.svelte';
  import { countedUsers, filledFor, groupUses, namesIt, SAMPLES_SAID, whenSaid, type UseGroup, type User } from './msguse';
  import type { MessageRead } from './where';
  import RefChip from './RefChip.svelte';

  let { draft, rowKey, version }: { draft: Draft; rowKey: Cell[]; version: number } = $props();

  /** The records whose lines are filled in, in each group; the rest are chips. */
  const FILLED = 3;
  const CHIPS = 12;
  let groups = $state<UseGroup[] | null>(null);
  let more = $state(false);
  let error = $state('');
  let opened = $state<Record<number, boolean>>({});

  $effect(() => {
    const key = rowKey;
    void version;
    let gone = false;
    error = '';
    api
      .references('Messages', key)
      .then((r) => {
        if (gone) return;
        groups = groupUses(r.references);
        more = r.more;
      })
      .catch((e) => {
        if (!gone) error = messageOf(e);
      });
    return () => (gone = true);
  });
  // another message: its groups start folded
  $effect(() => {
    void rowKey;
    opened = {};
  });

  const counted = (t: string, n: number) => {
    const k = kindOf(t);
    return `${n.toLocaleString()} ${(n === 1 ? k.one : k.many).toLowerCase()}`;
  };
  /** Every record that uses it, once. */
  const users = $derived.by(() => {
    const seen = new Map<string, { table: string }>();
    for (const g of groups ?? []) for (const u of g.users) seen.set(`${u.table}/${u.key.join('/')}`, u);
    return [...seen.values()];
  });
  const lines = $derived<Lines>([1, 2, 3].map((i) => String(draft.get(`Line ${i}`) ?? '')) as Lines);
  const changed = $derived(['Line 1', 'Line 2', 'Line 3'].some((c) => draft.edited(c)));

  function ansi(node: HTMLElement, line: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(line);
    return { update: draw };
  }
  const upper = (s: string) => (s ? s[0]!.toUpperCase() + s.slice(1) : s);
</script>

{#snippet filled(reads: MessageRead[], name: string)}
  <div class="lines">
    {#each filledFor(lines, reads, name) as l (l.line + l.to)}
      <div class="l">
        <span class="who" title="When {l.when}">Line {l.line}, to {l.to}</span>
        {#if l.text.trim()}<span class="mini" use:ansi={l.text}></span>{:else}<span class="faint">empty: {l.to} reads nothing</span>{/if}
      </div>
    {/each}
  </div>
{/snippet}

{#snippet chips(gi: number, users: User[], lead: string)}
  <div class="chips">
    {#if lead}<span class="faint">{lead}</span>{/if}
    {#each opened[gi] ? users : users.slice(0, CHIPS) as u (u.key.join('/'))}<RefChip table={u.table} key={u.key} compact why={u.by.join(', ')} />{/each}
    {#if !opened[gi] && users.length > CHIPS}
      <button type="button" class="btn sm" onclick={() => (opened[gi] = true)}>+{(users.length - CHIPS).toLocaleString()} more</button>
    {/if}
  </div>
{/snippet}

{#if error}
  <p class="faint">{error}</p>
{:else if !groups}
  <p class="faint">Looking through the world…</p>
{:else if !groups.length}
  <p class="faint">Nothing in the world uses it, so no one reads it.</p>
{:else}
  <div class="reads">
    {#if users.length > 1}
      <p class="callout" class:warn={changed}>
        {#if changed}<TriangleAlert size={15} />{:else}<Users size={15} />{/if}
        <span>
          {#if changed}Saving changes what {countedUsers(users, counted)} say{users.length === 1 ? 's' : ''}, as below.
          {:else}Used by {countedUsers(users, counted)}: a change to its lines changes what each of them says. A record's message field can give it a copy of its own.{/if}
        </span>
      </p>
    {/if}
    {#each groups as g, gi (gi)}
      {@const k = kindOf(g.table)}
      {@const each = !!g.reads && namesIt(g.reads)}
      <section class="use">
        <div class="head">
          <span class="tile {k.cls}"><k.icon size={14} /></span>
          <b>{g.reads ? whenSaid(g.reads) : 'Named, but not read line by line'}</b>
          <span class="faint">{counted(g.table, g.users.length)}</span>
        </div>
        {#if g.reads && each}
          <!-- its name is in what they read: the first few filled in, each with its own -->
          <ul class="filled">
            {#each g.users.slice(0, FILLED) as u (u.key.join('/'))}
              <li>
                <RefChip table={u.table} key={u.key} compact why={u.by.join(', ')} />
                {@render filled(g.reads, u.title)}
              </li>
            {/each}
          </ul>
          {#if g.users.length > FILLED}{@render chips(gi, g.users.slice(FILLED), 'and')}{/if}
        {:else if g.reads}
          <!-- the same words for every one of them -->
          {@render filled(g.reads, g.users[0]?.title ?? '')}
          {@render chips(gi, g.users, g.users.length === 1 ? 'for' : 'for each of')}
        {:else}
          {@render chips(gi, g.users, '')}
          <p class="faint">{upper(g.users[0]?.by[0] ?? '')}: the game names it here, but does not read its lines one by one.</p>
        {/if}
      </section>
    {/each}
    <p class="faint">{SAMPLES_SAID}</p>
    {#if more}<p class="faint">And more: only the first 500 uses are listed.</p>{/if}
  </div>
{/if}

<style>
  .reads { display: grid; gap: 12px; }
  .callout { margin: 0; }
  .use { display: grid; gap: 8px; }
  .use + .use { padding-top: 12px; border-top: 1px dashed var(--line); }
  .head { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-size: 13.5px; }
  .head .faint { font-size: 12px; font-weight: 700; }
  .filled { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .filled li { display: grid; gap: 4px; justify-items: start; }
  .lines { display: grid; gap: 3px; width: 100%; }
  .l { display: grid; grid-template-columns: 9.5rem minmax(0, 1fr); gap: 8px; align-items: baseline; }
  .who { color: var(--ink-3); font-size: 11.5px; font-weight: 800; }
  .mini { font: 12.5px/1.3 var(--font-mono); background: var(--term-bg); color: var(--term-fg); border-radius: 8px; padding: 4px 8px; white-space: pre-wrap; overflow-wrap: anywhere; border: var(--stroke) solid var(--term-frame); }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; }
  .faint { margin: 0; font-size: 12px; }
  @container (max-width: 520px) {
    .l { grid-template-columns: minmax(0, 1fr); gap: 2px; }
  }
</style>
