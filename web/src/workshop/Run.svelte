<!--
  A Workshop run, previewed: read again as its settings change, it lists what would change,
  record by record, before and after. Apply writes it as one step of the history, so the
  toast's Undo (or Ctrl+Z) takes it all back until you publish.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import Info from '@lucide/svelte/icons/info';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import Trash from '@lucide/svelte/icons/trash';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import type { Snippet } from 'svelte';
  import { toShown } from '../ansi';
  import type { Cell, Plan, PlanChange } from '../api';
  import RefChip from '../library/RefChip.svelte';
  import { kindOf } from '../shell/kinds';
  import { keyLabel } from '../shell/routes';
  import { sfx } from '../shell/prefs.svelte';
  import { realmChanged, undo } from '../shell/state.svelte';
  import { messageOf, toast } from '../shell/toasts.svelte';

  interface Props {
    /** The run: previewed (`dry`), or written. */
    run: (dry: boolean) => Promise<Plan>;
    /** The settings as text: the preview is read again whenever they change. */
    key: string;
    /** Why it cannot run yet (a box left empty), when it cannot. */
    blocked?: string;
    /** The button's words, before how many records it changes: "Apply to", "Delete". */
    verb?: string;
    /** What the records counted are doing: "changing", "going". */
    changing?: string;
    /** What the toast says once it has run, given the records it changed. */
    done: (records: number) => string;
    danger?: boolean;
    /** What is listed in place of the changes, value by value. */
    list?: Snippet<[Plan]>;
  }

  let { run, key, blocked = '', verb = 'Apply to', changing = 'changing', done, danger = false, list }: Props = $props();

  const FIRST = 60;

  let plan = $state.raw<Plan | null>(null);
  /** The settings the preview shown was read for. */
  let read = $state('');
  let loading = $state(false);
  let busy = $state(false);
  let error = $state('');
  let shown = $state(FIRST);
  /** What the last Apply did, until the settings change. */
  let applied = $state<{ key: string; text: string } | null>(null);
  /** Bumped when the world changes under the page (an undo), so the preview is read again. */
  let world = $state(0);
  let asked = 0;

  $effect(() => {
    const on = () => {
      world++;
      applied = null;
    };
    window.addEventListener('dd:realm', on);
    return () => window.removeEventListener('dd:realm', on);
  });

  $effect(() => {
    const k = key;
    void world;
    if (blocked) {
      asked++;
      plan = null;
      error = '';
      loading = false;
      return;
    }
    loading = true;
    const my = ++asked;
    const t = setTimeout(async () => {
      try {
        const p = await run(true);
        if (my !== asked) return;
        plan = p;
        read = k;
        error = '';
        shown = FIRST;
      } catch (e) {
        if (my !== asked) return;
        plan = null;
        error = messageOf(e);
      } finally {
        if (my === asked) loading = false;
      }
    }, 260);
    return () => clearTimeout(t);
  });

  const k = $derived(kindOf(plan?.table));
  const noun = (n: number) => (n === 1 ? k.one : k.many).toLowerCase();
  const fresh = $derived(!!plan && read === key && !loading);

  /** The changes, a record at a time. */
  const records = $derived.by(() => {
    const out: { key: Cell[]; changes: PlanChange[] }[] = [];
    for (const c of plan?.changes.slice(0, shown) ?? []) {
      const last = out.at(-1);
      if (last && keyLabel(last.key) === keyLabel(c.key)) last.changes.push(c);
      else out.push({ key: c.key, changes: [c] });
    }
    return out;
  });

  const value = (v: Cell) => (typeof v === 'number' ? v.toLocaleString() : v === null ? '' : toShown(v.split('\0')[0]!));

  async function apply() {
    if (!plan || busy || !fresh) return;
    busy = true;
    try {
      const r = await run(false);
      const step = r.step;
      sfx('tada');
      const text = done(r.records);
      toast(text, step != null ? { action: 'Undo', onAction: () => void undo(step) } : {});
      realmChanged();
      // after the world's change is told, which clears what an undo makes stale
      applied = { key, text };
    } catch (e) {
      sfx('nope');
      toast(messageOf(e), { kind: 'error' });
    } finally {
      busy = false;
    }
  }
</script>

<section class="run" aria-live="polite" aria-busy={loading}>
  <div class="head">
    <h3>Preview</h3>
    {#if loading}<span class="faint spin"><LoaderCircle size={14} />Reading the world…</span>{/if}
  </div>

  {#if applied && applied.key === key}
    <div class="callout ok"><CircleCheck size={16} /><span>{applied.text} It is one step in Changes, and Undo takes it back until you publish.</span></div>
  {/if}

  {#if blocked}
    <p class="quiet">{blocked}</p>
  {:else if error}
    <div class="callout bad"><TriangleAlert size={16} /><span>{error}</span></div>
  {:else if plan}
    {#if plan.records}
      <div class="tiles">
        <div><small>{plan.records === 1 ? k.one : k.many} {changing}</small><b>{plan.records.toLocaleString()}</b></div>
        {#if !list}<div><small>Values</small><b>{plan.changed.toLocaleString()}</b></div>{/if}
        {#if plan.unchanged}<div><small>Already so</small><b>{plan.unchanged.toLocaleString()}</b></div>{/if}
        {#if plan.skipped}<div><small>Left out</small><b>{plan.skipped.toLocaleString()}</b></div>{/if}
      </div>
    {:else}
      <p class="quiet good">
        <CircleCheck size={16} />
        Nothing to change{plan.unchanged ? `: ${plan.unchanged.toLocaleString()} ${noun(plan.unchanged)} already so` : ''}{plan.skipped
          ? `${plan.unchanged ? ',' : ':'} ${plan.skipped.toLocaleString()} left out`
          : ''}.
      </p>
    {/if}

    {#if plan.notes.length}
      <div class="callout info notes">
        <Info size={16} />
        <div>
          <b>Worth a look, left as they are:</b>
          <ul>{#each plan.notes.slice(0, 12) as n, i (i)}<li>{n}</li>{/each}</ul>
          {#if plan.notes.length > 12}<span>and {(plan.notes.length - 12).toLocaleString()} more.</span>{/if}
        </div>
      </div>
    {/if}

    {#if list}
      {@render list(plan)}
    {:else if records.length}
      <div class="table-wrap">
        <table class="changes">
          <thead><tr><th>{k.one}</th><th>Field</th><th class="num">Before</th><th class="num">After</th></tr></thead>
          <tbody>
            {#each records as r (keyLabel(r.key))}
              {#each r.changes as c, i (c.column)}
                <tr class:first={i === 0}>
                  <td>{#if i === 0}<RefChip table={plan.table} key={r.key} compact />{/if}</td>
                  <td class="fname">{c.column === '(new)' ? 'New' : c.column === '(deleted)' ? 'Deleted' : c.column}</td>
                  <td class="num old">{value(c.before)}</td>
                  {#if c.column === '(deleted)'}<td class="num gone">gone</td>{:else}<td class="num new">{value(c.after)}</td>{/if}
                </tr>
              {/each}
            {/each}
          </tbody>
        </table>
      </div>
      {#if plan.changes.length > shown}
        <div class="more">
          <span class="faint">{shown.toLocaleString()} of {plan.changed.toLocaleString()} values</span>
          <button type="button" class="btn sm" onclick={() => (shown += 200)}>Show more</button>
        </div>
      {:else if plan.changed > plan.changes.length}
        <p class="faint more">The first {plan.changes.length.toLocaleString()} of {plan.changed.toLocaleString()} values are listed.</p>
      {/if}
    {/if}
  {:else if loading}
    <p class="quiet">Working out what would change…</p>
  {/if}

  <div class="go">
    <button type="button" class="btn sticker {danger ? 'danger-go' : 'primary'}" disabled={!fresh || !plan?.records || busy} onclick={() => void apply()}>
      {#if danger}<Trash size={16} />{:else}<Check size={16} />{/if}
      {busy ? 'Writing…' : plan?.records && fresh ? `${verb} ${plan.records.toLocaleString()} ${noun(plan.records)}` : verb.replace(/ to$/, '')}
    </button>
    <span class="faint">One step in Changes: Undo takes it all back until you publish.</span>
  </div>
</section>

<style>
  .run { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .head { display: flex; align-items: baseline; gap: 10px; }
  .head h3 { margin: 0; font-size: 16px; }
  .spin { display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; font-weight: 700; }
  .spin :global(svg) { animation: turn 1s linear infinite; }
  @keyframes turn { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spin :global(svg) { animation: none; } }
  .quiet { margin: 0; color: var(--ink-2); font-weight: 700; display: flex; align-items: center; gap: 6px; }
  .quiet.good { color: var(--ok); }
  .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(110px, 1fr)); gap: 8px; }
  .tiles div { background: var(--surface-2); border-radius: 12px; padding: 8px 10px; min-width: 0; }
  .tiles small { display: block; font: 800 10.5px/1.2 var(--font-ui); letter-spacing: .06em; text-transform: uppercase; color: var(--ink-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tiles b { font: 700 18px/1.4 var(--font-mono); }
  .notes ul { margin: 4px 0 0; padding-left: 18px; }
  .notes li { margin: 1px 0; }
  .table-wrap { overflow: auto; max-height: min(60vh, 560px); border: 1px solid var(--line); border-radius: 14px; }
  .changes { width: 100%; border-collapse: collapse; font-size: 13px; }
  .changes th { position: sticky; top: 0; z-index: 1; background: var(--surface); text-align: left; font: 800 11px/1.2 var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: var(--ink-3); padding: 8px 10px; border-bottom: 1px solid var(--line); }
  .changes td { padding: 3px 10px; vertical-align: middle; }
  .changes tr.first td { border-top: 1px dashed var(--line); padding-top: 6px; }
  .changes tbody tr:first-child td { border-top: 0; }
  .fname { font-weight: 700; color: var(--ink-2); white-space: nowrap; }
  .num { text-align: right; font-family: var(--font-mono); white-space: nowrap; }
  .old { color: var(--ink-3); text-decoration: line-through; }
  .new { color: var(--ok); font-weight: 800; }
  .gone { color: var(--bad); font-weight: 800; }
  .more { display: flex; align-items: center; justify-content: flex-end; gap: 10px; margin: 0; font-size: 12.5px; font-weight: 700; }
  .go { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; padding-top: 4px; }
  .go .faint { font-size: 12px; font-weight: 700; }
  .danger-go:not(:disabled) { background: var(--bad); color: #fff; }
  .danger-go:hover:not(:disabled) { background: var(--bad); filter: brightness(1.05); }
</style>
