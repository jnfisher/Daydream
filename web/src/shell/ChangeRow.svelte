<!-- One change waiting to publish: what it is, how it changed, and taking it back. -->
<script lang="ts">
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import type { Change } from '../api';
  import { kindOf } from './kinds';
  import { inspect, takeBack } from './state.svelte';

  let { c, onopen }: { c: Change; onopen?: () => void } = $props();

  const k = $derived(kindOf(c.nmr ?? c.table));
  const opens = $derived(c.how !== 'deleted' && c.nmr !== null && c.key !== null);
  const detail = $derived.by(() => {
    const bits = [c.what];
    if (c.how === 'changed' && c.members.length) {
      const shown = c.members.slice(0, 3).join(', ');
      bits.push(c.members.length > 3 ? `${shown} +${c.members.length - 3} more` : shown);
    }
    if (c.was) bits.push(`was ${c.was}`);
    return bits.join(' · ');
  });

  function open() {
    if (!opens) return;
    inspect({ table: c.nmr!, key: c.key!, name: c.name ?? undefined });
    onopen?.();
  }
</script>

<li class="change {k.cls}">
  <button type="button" class="what" disabled={!opens} onclick={open} title={opens ? 'Open it beside the page' : undefined}>
    <span class="tile"><k.icon size={14} /></span>
    <span class="txt">
      <b class:gone={c.how === 'deleted'}>{c.name || c.what}</b>
      <span class="sub">{detail}</span>
    </span>
  </button>
  <span class="pill {c.how === 'added' ? 'ok' : c.how === 'deleted' ? 'bad' : 'warn'}">{c.how}</span>
  <button type="button" class="btn icon sm ghost" title="Take back: put {c.what} as it was" aria-label="Take back {c.what}" onclick={() => void takeBack([{ stem: c.stem, id: c.id }], c.name || c.what)}>
    <Undo2 size={15} />
  </button>
</li>

<style>
  .change { display: flex; align-items: center; gap: 8px; padding: 3px 4px 3px 0; border-radius: 12px; min-width: 0; }
  .change:hover { background: var(--surface-2); }
  .what {
    flex: 1; min-width: 0; display: flex; align-items: center; justify-content: flex-start; gap: 10px; padding: 5px 8px; min-height: 0;
    border: 0; background: transparent; box-shadow: none; text-align: left; font-weight: 600; white-space: normal;
  }
  .what:hover:not(:disabled) { background: transparent; }
  .what:disabled { opacity: 1; cursor: default; }
  .tile { width: 26px; height: 26px; border-radius: 8px; }
  .txt { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
  .txt b { font-weight: 800; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .txt b.gone { text-decoration: line-through; color: var(--ink-2); }
  .sub { font-size: 12px; color: var(--ink-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pill { flex: none; }
</style>
