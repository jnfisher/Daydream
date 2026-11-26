<!-- What just happened, at the bottom of the screen, with a way to take it back. -->
<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import { dismiss, toasts } from './toasts.svelte';
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast" class:error={t.kind === 'error'}>
      <span>{t.text}</span>
      {#if t.action}
        <button type="button" onclick={() => { dismiss(t.id); t.onAction?.(); }}>{t.action}</button>
      {/if}
      <button type="button" class="x" aria-label="Dismiss" onclick={() => dismiss(t.id)}><X size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .x { padding: 0 6px; }
</style>
