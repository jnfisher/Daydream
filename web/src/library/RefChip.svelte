<!--
  What a number points at: the record's kind, number and name, opening it beside the page.
  A number the table has no record of says so.
-->
<svelte:options css="injected" />

<script lang="ts">
  import type { Cell } from '../api';
  import { kindOf } from '../shell/kinds';
  import { keyLabel } from '../shell/routes';
  import { inspect } from '../shell/state.svelte';
  import { nameOf } from './names.svelte';

  let { table, key, compact = false, why }: { table: string; key: Cell[]; compact?: boolean; why?: string } = $props();

  const k = $derived(kindOf(table));
  const name = $derived(nameOf(table, key));
  const label = $derived(keyLabel(key));
</script>

<button
  type="button"
  class="ref {k.cls}"
  class:missing={name === null}
  class:compact
  title={name === null ? `There is no ${k.one.toLowerCase()} ${label}` : `${k.one} ${label}${name ? `: ${name}` : ''}${why ? ` (${why})` : ''}`}
  onclick={() => inspect({ table, key, name: name ?? undefined })}
>
  <span class="tile"><k.icon size={12} /></span><span class="num">{label}</span>
  {#if name === null}<span class="nm bad">none such</span>{:else if name}<span class="nm">{name}</span>{/if}
</button>

<style>
  .ref { max-width: 100%; min-width: 0; }
  .ref.compact { max-width: 15rem; }
  .ref.missing { border-color: var(--bad); background: var(--bad-bg); }
  .bad { color: var(--bad); }
</style>
