<!-- A choice of a few, side by side: one is on. -->
<svelte:options css="injected" />

<script lang="ts" generics="T extends string">
  let { options, value = $bindable(), label }: { options: readonly (readonly [T, string])[]; value: T; label: string } = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as [id, text] (id)}
    <button type="button" role="radio" aria-checked={value === id} class:on={value === id} onclick={() => (value = id)}>{text}</button>
  {/each}
</div>

<style>
  .seg { display: inline-flex; flex-wrap: wrap; gap: 2px; padding: 3px; border-radius: 12px; background: var(--surface-2); border: 1px solid var(--line); max-width: 100%; }
  .seg button { flex: 1 1 auto; min-height: 30px; padding: 0 12px; border: 0; border-radius: 9px; background: transparent; font-size: 13px; color: var(--ink-2); box-shadow: none; }
  .seg button:hover:not(.on) { background: var(--surface-3); }
  .seg button.on { background: var(--lilac-2); color: var(--lilac-ink); box-shadow: 0 0 0 1px var(--line-2); }
</style>
