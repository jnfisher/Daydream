<!--
  One column's control on a record card: a list's choices by name, a record or an ability
  picked by name or typed by number (with a chip of what it points at), text, or a number.
  The record's key is shown, not edited.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { ABILITIES } from '../abilities';
  import type { Pick } from '../layouts';
  import { kindOf } from '../shell/kinds';
  import { abilityChoices, recordChoices, refKey } from './choices';
  import type { Draft } from './draft.svelte';
  import Picker from './Picker.svelte';
  import RefChip from './RefChip.svelte';

  let {
    draft,
    col,
    pick,
    target,
    map,
    label,
    hint,
    compact = false,
  }: {
    draft: Draft;
    col: string;
    pick?: Pick;
    /** The table its number names, when it names one. */
    target?: string;
    /** For a room it names: the map it is on (the record's own, or a Map Change's). */
    map?: number;
    label: string;
    /** What the value means, as a tooltip. */
    hint?: string;
    compact?: boolean;
  } = $props();

  const info = $derived(draft.column(col));
  const value = $derived(draft.typed(col));
  const n = $derived(Number(value) || 0);
  const options = $derived(pick && 'options' in pick ? pick.options : null);
  const ability = $derived(!!pick && 'abilities' in pick);
  const isNum = $derived(draft.isNumber(col));

  const set = (v: string) => draft.set(col, v);
</script>

<span class="fi" class:compact>
  {#if draft.isKey(col)}
    <input type="text" class="num" value={value} disabled aria-label={label} title="The record's key: Copy it to a new one to use another" />
  {:else if options}
    <select class:changed={draft.edited(col)} aria-label={label} title={hint} value={value} onchange={(e) => set(e.currentTarget.value)}>
      {#if value !== '' && !(n in options)}<option value={value}>{value} (not in the list)</option>{/if}
      {#each Object.entries(options) as [v, name] (v)}
        <option value={v}>{name}{compact ? '' : ` · ${v}`}</option>
      {/each}
    </select>
  {:else if info?.type === 'memo'}
    <textarea rows="4" spellcheck="false" class:changed={draft.edited(col)} aria-label={label} value={value} oninput={(e) => set(e.currentTarget.value)}></textarea>
  {:else}
    <input
      type="text"
      class:num={isNum}
      class:wide={!isNum}
      class:changed={draft.edited(col)}
      inputmode={isNum ? 'numeric' : undefined}
      spellcheck="false"
      maxlength={info?.type === 'text' && info.size ? info.size : undefined}
      aria-label={label}
      title={hint}
      value={value}
      oninput={(e) => set(e.currentTarget.value)}
    />
    {#if ability}
      {#if n}
        <span class="ab" title={ABILITIES[n]?.[1] ?? ''}>{ABILITIES[n]?.[0] ?? 'unknown ability'}</span>
      {/if}
      <Picker what="an ability" choices={abilityChoices} onpick={(v) => set(String(v))} />
    {:else if target}
      {#if n > 0}<RefChip table={target} key={refKey(target, n, map)} compact={compact} />{/if}
      <Picker what="a {kindOf(target).one.toLowerCase()}" choices={() => recordChoices(target!, map)} onpick={(v) => set(String(v))} />
    {/if}
  {/if}
</span>

<style>
  .fi { display: inline-flex; align-items: center; gap: 6px; min-width: 0; max-width: 100%; }
  .fi input.num { width: 6.2rem; flex: none; font: 13px var(--font-mono); font-variant-numeric: tabular-nums; }
  .fi.compact input.num { width: 5rem; }
  .fi :global(.ref) { flex: 0 1 auto; min-width: 0; }
  .fi input.wide { width: min(26rem, 100%); }
  .fi select { max-width: 15rem; }
  .fi.compact select { max-width: 10rem; }
  .fi textarea { width: 100%; font: 13px/1.35 var(--font-mono); }
  .ab { font-weight: 800; font-size: 12.5px; color: var(--ink-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 12rem; }
  :global(.fi .pick) { flex: none; }
</style>
