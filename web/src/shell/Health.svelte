<!--
  The health checks across the whole world (the server's /api/checks/health): what is wrong
  that has a sure fix, and counts worth knowing. A fix is one click and one step of the
  history, so the toast's Undo (or Ctrl+Z, or Changes) takes it back until you publish. On Home
  it is a card with a fix on each line; in the Workshop's Health group each line also lists
  its records. Read again after each save while its place is open.
-->
<svelte:options css="injected" />

<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import Crown from '@lucide/svelte/icons/crown';
  import DoorOpen from '@lucide/svelte/icons/door-open';
  import HeartPulse from '@lucide/svelte/icons/heart-pulse';
  import Info from '@lucide/svelte/icons/info';
  import MessageSquareText from '@lucide/svelte/icons/message-square-text';
  import Package from '@lucide/svelte/icons/package';
  import Scissors from '@lucide/svelte/icons/scissors';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import Sword from '@lucide/svelte/icons/sword';
  import type { Component } from 'svelte';
  import { api, type Health, type HealthFix, type HealthId } from '../api';
  import RefChip from '../library/RefChip.svelte';
  import { sfx } from './prefs.svelte';
  import type { Place } from './routes';
  import { app, realmChanged, undo } from './state.svelte';
  import { messageOf, toast } from './toasts.svelte';

  let { place, full = false }: { place: Place; full?: boolean } = $props();

  const n = (x: number) => x.toLocaleString();
  const count = (x: number, one: string, many = `${one}s`) => `${n(x)} ${x === 1 ? one : many}`;
  /** A verb that agrees with a count: `ends` for one, `end` for more. */
  const agree = (x: number, one: string, many: string) => (x === 1 ? one : many);

  interface Words {
    icon: Component<{ size?: number }>;
    /** What it found, in a line. */
    title: (h: Health) => string;
    /** Why it matters, and what the fix does. */
    about: (h: Health) => string;
    /** What a line says when nothing was found. */
    ok: string;
    /** The button: a fix, or where to look. */
    act?: string;
    href?: string;
    /** What the toast says once the fix has run. */
    done?: (r: HealthFix) => string;
  }

  const WORDS: Record<HealthId, Words> = {
    uses: {
      icon: Sword,
      title: (h) => `${count(h.records, 'item')} with uses to fix`,
      about: () => 'Items with 0 uses break the first time they are used, so they get −1 (never wears out). Items kept after their uses get their recharge.',
      ok: 'Every item’s uses are fine',
      act: 'Fix them',
      done: (r) => `Fixed the uses on ${count(r.records, 'item')}.`,
    },
    blank_blows: {
      icon: MessageSquareText,
      title: (h) => `${count(h.records, 'monster')} with blank attack messages`,
      about: (h) => `${count(h.values, 'blow')} ${agree(h.values, 'reads', 'read')} as an empty line. 0 tells them in the game’s own words.`,
      ok: 'No attack message is blank',
      act: 'Fix them',
      done: (r) => `Cleared ${count(r.values, 'blank attack message')} on ${count(r.records, 'monster')}.`,
    },
    exits: {
      icon: DoorOpen,
      title: (h) => `${count(h.records, 'room')} with exits to nowhere`,
      about: (h) => `${count(h.values, 'exit')} ${agree(h.values, 'leads', 'lead')} to a room the world does not have: dig the room, or take the exit away.`,
      ok: 'Every exit leads to a room',
    },
    drop_uses: {
      icon: Package,
      title: (h) => `${count(h.records, 'monster')} ${agree(h.records, 'drops', 'drop')} items with other uses`,
      about: () => 'A drop’s uses should be its item’s, as NMR’s Fix Number of Uses on Item Drops sets them.',
      ok: 'Every drop’s uses match its item',
      act: 'Fix them',
      done: (r) => `Fixed the uses on the drops of ${count(r.records, 'monster')}.`,
    },
    leftovers: {
      icon: Sparkles,
      title: (h) => `${count(h.values, 'leftover value')} in empty ability slots`,
      about: (h) => `On ${count(h.records, 'record')}: slots with no ability still hold a value, most often 8224, two spaces an old tool left. The game skips them.`,
      ok: 'No empty ability slot holds a value',
      act: 'Clear them',
      done: (r) => `Cleared ${count(r.values, 'leftover value')} on ${count(r.records, 'record')}.`,
    },
    spaces: {
      icon: Scissors,
      title: (h) => `${count(h.records, 'text block')} ${agree(h.records, 'ends', 'end')} in stray spaces`,
      about: () => 'Harmless, but they count against the length limit. Spaces inside a text are left alone.',
      ok: 'No text block ends in stray spaces',
      act: 'Trim them',
      done: (r) => `Trimmed the ends of ${count(r.records, 'text block')}.`,
    },
    limited: {
      icon: Crown,
      title: (h) => count(h.records, 'limited item'),
      about: () => 'NMR’s Limited Item List: where each one’s copies are, and which are over their limit.',
      ok: 'No item has a game limit',
      act: 'Open',
      href: '#/workshop/limited',
    },
  };

  const ORDER = { warn: 0, note: 1, info: 2, ok: 3 } as const;

  let checks: Health[] = $state([]);
  let error = $state('');
  let loading = $state(true);
  let busy = $state<HealthId | null>(null);
  let open: Partial<Record<HealthId, boolean>> = $state({});

  const shown = $derived([...checks].sort((a, b) => ORDER[a.level] - ORDER[b.level]));
  const wrong = $derived(checks.filter((c) => c.level === 'warn' || c.level === 'note').length);

  let asked = 0;
  async function load() {
    const my = ++asked;
    try {
      const r = await api.health();
      if (my !== asked) return;
      checks = r.checks;
      error = '';
    } catch (e) {
      if (my === asked) error = messageOf(e);
    } finally {
      if (my === asked) loading = false;
    }
  }

  // read again when the world changes (a save, an undo, another world) and the place is open
  $effect(() => {
    void app.changes;
    void app.current;
    if (app.route.place === place) void load();
  });

  async function fix(h: Health) {
    const w = WORDS[h.id];
    if (busy || !w.done) return;
    busy = h.id;
    try {
      const r = await api.healthFix(h.id);
      const step = r.step;
      sfx('tada');
      toast(w.done(r), step != null ? { action: 'Undo', onAction: () => void undo(step) } : {});
      realmChanged();
    } catch (e) {
      sfx('nope');
      toast(messageOf(e), { kind: 'error' });
    } finally {
      busy = null;
    }
  }
</script>

<div class="health" class:full>
  {#if !full}
    <div class="head">
      <h3>Health</h3>
      {#if !loading && !error}<span class="faint">{wrong ? `${count(wrong, 'thing')} to look at` : 'All good'}</span>{/if}
      <a class="btn sm ghost" href="#/workshop/health"><HeartPulse size={14} />All checks</a>
    </div>
  {/if}

  {#if error}
    <p class="quiet bad">{error}</p>
  {:else if loading && !checks.length}
    <p class="quiet">Reading the world…</p>
  {:else}
    <ul class="rows">
      {#each shown as h (h.id)}
        {@const w = WORDS[h.id]}
        {@const I = h.level === 'ok' ? CircleCheck : w.icon}
        <li class="row {h.level}">
          <span class="gi"><I size={16} /></span>
          <span class="what">
            <b>{h.level === 'ok' ? w.ok : w.title(h)}</b>
            {#if h.level !== 'ok'}<span class="s" title={full ? undefined : w.about(h)}>{w.about(h)}</span>{/if}
          </span>
          <span class="acts">
            {#if h.level === 'ok'}
              <span class="pill ok">fine</span>
            {:else}
              {#if full && h.list.length && !w.href}
                <button type="button" class="btn sm ghost" aria-expanded={!!open[h.id]} onclick={() => (open[h.id] = !open[h.id])}>
                  {open[h.id] ? 'Hide them' : 'See them'}
                </button>
              {/if}
              {#if w.href}
                <a class="btn sm" href={w.href}>{w.act}</a>
              {:else if h.fix && w.act}
                <button type="button" class="btn sm" disabled={!!busy} onclick={() => void fix(h)}>{busy === h.id ? 'Fixing…' : w.act}</button>
              {:else if !full}
                <a class="btn sm" href="#/workshop/health">See them</a>
              {/if}
            {/if}
          </span>
          {#if full && open[h.id]}
            <div class="list">
              <div class="chips">
                {#each h.list as r (`${r.table}:${r.key.join('/')}`)}<RefChip table={r.table} key={r.key} compact />{/each}
                {#if h.records > h.list.length}<span class="faint more">and {n(h.records - h.list.length)} more</span>{/if}
              </div>
              {#if h.notes.length}
                <div class="callout info">
                  <Info size={16} />
                  <div>
                    <b>The fix leaves these as they are, worth a look:</b>
                    <ul class="notes">{#each h.notes.slice(0, 8) as note, i (i)}<li>{note}</li>{/each}</ul>
                    {#if h.notes.length > 8}<span>and {n(h.notes.length - 8)} more.</span>{/if}
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .health { display: flex; flex-direction: column; min-width: 0; }
  .head { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; margin-bottom: 6px; }
  .head h3 { margin: 0; font-size: 17px; }
  .head .faint { font-size: 12.5px; font-weight: 700; }
  .head .btn { margin-left: auto; align-self: center; }
  .quiet { margin: 6px 0; color: var(--ink-2); font-weight: 600; }
  .quiet.bad { color: var(--bad); }
  .rows { list-style: none; margin: 0 -6px; padding: 0; }
  .row { display: grid; grid-template-columns: 30px minmax(0, 1fr) auto; align-items: center; gap: 4px 10px; padding: 7px 6px; }
  .row + .row { border-top: 1px dashed var(--line); }
  .gi { width: 30px; height: 30px; border-radius: 10px; display: grid; place-items: center; background: var(--surface-2); color: var(--ink-2); }
  .warn .gi { background: var(--warn-bg); color: var(--warn); }
  .note .gi, .info .gi { background: var(--sky-2); color: var(--sky-ink); }
  .ok .gi { background: var(--ok-bg); color: var(--ok); }
  .what { display: flex; flex-direction: column; min-width: 0; }
  .what b { font-weight: 800; font-size: 13.5px; }
  .s { font-size: 12.5px; font-weight: 600; color: var(--ink-2); line-height: 1.45; }
  .ok .what b { font-weight: 700; color: var(--ink-2); }
  .acts { display: flex; gap: 6px; align-items: center; justify-content: flex-end; flex-wrap: wrap; }
  .list { grid-column: 2 / -1; display: flex; flex-direction: column; gap: 8px; padding: 4px 0 2px; }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; max-height: 220px; overflow: auto; }
  .more { font-size: 12px; font-weight: 700; }
  .notes { margin: 4px 0 0; padding-left: 18px; list-style: disc; }
  .full .row { padding: 10px 8px; }
  /* on Home, a line each: the whole of it is the line's tooltip */
  .health:not(.full) .s { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .health:not(.full) .row.ok { padding-top: 4px; padding-bottom: 4px; }
  .health:not(.full) .ok .gi { width: 24px; height: 24px; border-radius: 8px; margin-left: 3px; }
  .full .what b { font-size: 14px; }
  @media (max-width: 560px) {
    .row { grid-template-columns: 30px minmax(0, 1fr); }
    .acts { grid-column: 2; justify-content: flex-start; }
    /* a touch screen shows no tooltip, so Home's lines wrap there */
    .health:not(.full) .s { white-space: normal; }
  }
</style>
