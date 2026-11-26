<!--
  Publish: the changes waiting, and writing them into the game. For now the write itself is
  today's card for the world's engine (WCC files, silvermere); the guided steps per engine
  come next.
-->
<script lang="ts">
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import { how } from '../games';
  import { silvermere, writeDat } from '../tools';
  import ChangeRow from './ChangeRow.svelte';
  import Mascot from './Mascot.svelte';
  import { app, realmChanged, takeBack } from './state.svelte';
  import { ago } from './time';

  const c = $derived(app.current);
  const changes = $derived(app.changes?.changes ?? []);
  const since = $derived(app.changes?.since);
  const counts = $derived.by(() => {
    const n = { added: 0, changed: 0, deleted: 0 };
    for (const ch of changes) n[ch.how]++;
    return n;
  });

  function takeAll() {
    if (!confirm(`Take back all ${changes.length} changes? Every record goes back to how the game has it. You can undo this.`)) return;
    void takeBack('all', `all ${changes.length} changes`);
  }

  /** Mounts one of today's cards. */
  function card(node: HTMLElement, make: () => HTMLElement) {
    node.replaceChildren(make());
  }
</script>

<div class="publish">
  <header>
    <Mascot mood={changes.length ? 'happy' : 'dream'} size={64} />
    <div>
      <h1>Publish</h1>
      <p class="muted">{c?.kind ? how({ kind: c.kind, live: !!c.live }) : 'Write the changes into the game.'}</p>
    </div>
  </header>

  <section class="card">
    <div class="card-h">
      <span>{changes.length ? `${changes.length} change${changes.length === 1 ? '' : 's'} waiting` : 'Nothing waiting to publish'}</span>
      {#if changes.length}
        <span class="pills">
          {#if counts.changed}<span class="pill warn">{counts.changed} changed</span>{/if}
          {#if counts.added}<span class="pill ok">{counts.added} added</span>{/if}
          {#if counts.deleted}<span class="pill bad">{counts.deleted} deleted</span>{/if}
        </span>
        <button type="button" class="btn sm danger take-all" onclick={takeAll}><Undo2 size={14} />Take back all</button>
      {/if}
    </div>
    {#if since}
      <p class="faint since">Since the world was read from its game or last published, {ago(since)}.</p>
    {:else if app.changes && since === null}
      <p class="faint since">This world was edited before Daydream kept a list of changes, so they are not known: Publish writes the whole world back once.</p>
    {/if}
    {#if changes.length}
      <ul class="list">
        {#each changes as ch (`${ch.stem}:${ch.id}`)}<ChangeRow c={ch} />{/each}
      </ul>
    {/if}
  </section>

  {#if c?.kind === 'db'}
    <section class="card">
      <h3>Nothing to write</h3>
      <p class="muted">{c.live ? 'This world is the game’s own .db files: every save is already in the game.' : 'This realm folder is edited where it is.'}</p>
    </section>
  {:else}
    <div class="cards">
      {#if c?.kind !== 'silvermere'}<div use:card={writeDat}></div>{/if}
      {#if c?.kind !== 'wcc'}<div use:card={() => silvermere(realmChanged)}></div>{/if}
    </div>
  {/if}
</div>

<style>
  .publish { display: flex; flex-direction: column; gap: 16px; max-width: 1100px; margin: 0 auto; }
  header { display: flex; align-items: center; gap: 16px; }
  h1 { margin: 0; font-size: 30px; }
  header p { margin: 2px 0 0; font-weight: 700; }
  .card-h { flex-wrap: wrap; font: 600 17px var(--font-display); }
  .pills { display: flex; gap: 6px; font-family: var(--font-ui); }
  .take-all { margin-left: auto; font-family: var(--font-ui); }
  .since { margin: -4px 0 8px; font-size: 12.5px; font-weight: 700; }
  .list { list-style: none; margin: 0 -6px; padding: 0; max-height: 50vh; overflow: auto; }
  h3 { margin: 0 0 6px; }
  .cards { display: flex; flex-direction: column; gap: 16px; }
</style>
