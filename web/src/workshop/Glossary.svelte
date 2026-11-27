<!-- Find and list › Ability glossary: every ability by number, its name and what it does. -->
<svelte:options css="injected" />

<script lang="ts">
  import Search from '@lucide/svelte/icons/search';
  import { ABILITIES } from '../abilities';

  let q = $state('');

  const all = Object.entries(ABILITIES)
    .filter(([n]) => n !== '0')
    .map(([n, [name, note]]) => ({ n: Number(n), name, note }));

  const shown = $derived.by(() => {
    const words = q.toLowerCase().split(/\s+/).filter(Boolean);
    return words.length ? all.filter((a) => words.every((w) => `${a.n} ${a.name} ${a.note}`.toLowerCase().includes(w))) : all;
  });
</script>

<div class="find">
  <Search size={16} />
  <input type="search" placeholder="Find an ability by number, name or what it does" aria-label="Find an ability" bind:value={q} />
  <span class="faint">{shown.length.toLocaleString()} of {all.length.toLocaleString()}</span>
</div>

<ul class="list">
  {#each shown as a (a.n)}
    <li>
      <span class="n">{a.n}</span>
      <span class="what"><b>{a.name}</b>{#if a.note}<span class="note">{a.note}</span>{/if}</span>
    </li>
  {:else}
    <li class="none">No ability goes by “{q.trim()}”.</li>
  {/each}
</ul>

<style>
  .find { display: flex; align-items: center; gap: 8px; color: var(--ink-3); }
  .find input { flex: 1; min-width: 0; max-width: 420px; height: 36px; }
  .find .faint { font-size: 12.5px; font-weight: 700; }
  .list { list-style: none; margin: 10px 0 0; padding: 0; border: 1px solid var(--line); border-radius: 14px; overflow: hidden; }
  .list li { display: grid; grid-template-columns: 3.5em minmax(0, 1fr); gap: 10px; padding: 7px 12px; border-top: 1px dashed var(--line); align-items: baseline; }
  .list li:first-child { border-top: 0; }
  .list li:hover { background: var(--surface-2); }
  .n { font: 700 13px var(--font-mono); color: var(--ink-3); text-align: right; }
  .what { display: flex; flex-direction: column; min-width: 0; }
  .what b { font-weight: 800; }
  .note { font-size: 12.5px; font-weight: 600; color: var(--ink-2); line-height: 1.45; }
  .none { color: var(--ink-2); font-weight: 700; display: block !important; }
</style>
