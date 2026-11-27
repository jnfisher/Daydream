<!--
  A Workshop run that brings a file into the world or sends the world out. Preview runs it dry
  and shows what it would do; the button then does just that. A run into the world is one step
  in Changes, which Undo takes back until you publish. A run out of it, into a game's files or
  database, cannot be taken back here, so its button is held down until it fills, as
  publishing's is. A download only reads the world, and needs no preview first.
-->
<svelte:options css="injected" />

<script lang="ts" generics="T">
  import Check from '@lucide/svelte/icons/check';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CloudUpload from '@lucide/svelte/icons/cloud-upload';
  import Download from '@lucide/svelte/icons/download';
  import Eye from '@lucide/svelte/icons/eye';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import type { Snippet } from 'svelte';
  import HoldButton from '../shell/HoldButton.svelte';
  import { sfx } from '../shell/prefs.svelte';
  import { realmChanged, undo } from '../shell/state.svelte';
  import { messageOf, toast } from '../shell/toasts.svelte';

  interface Props {
    /** The run: previewed (`dry`), or done. */
    run: (dry: boolean) => Promise<T>;
    /** The settings as text: a preview read for other settings is out of date. */
    key: string;
    /** Why it cannot run yet (no file picked), when it cannot. */
    blocked?: string;
    /**
     * `in`: it changes the world, as one step of its history. `out`: it writes outside
     * Daydream, so it is held to go. `read`: it only reads the world (a download).
     */
    way: 'in' | 'out' | 'read';
    /** The button's words for what a preview found ("Import 1,204 records"); '' for nothing to do. */
    act: (preview: T) => string;
    /** The button's words before a preview. */
    verb: string;
    /** What it says once it has run. */
    done: (result: T) => string;
    /** What a preview found (`dry`), or what the run did. */
    result: Snippet<[T, boolean]>;
  }

  let { run, key, blocked = '', way, act, verb, done, result }: Props = $props();

  /** The last preview or run, and the settings and world it was read for. */
  let shown = $state.raw<{ r: T; dry: boolean; key: string; world: number; text: string } | null>(null);
  let busy = $state<'' | 'preview' | 'go'>('');
  let error = $state('');
  /** Bumped when the world changes under the page (an undo), which dates a preview. */
  let world = $state(0);
  /** A press too short for the held button: say how it goes. */
  let nudge = $state(false);
  let nudged: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const on = () => world++;
    window.addEventListener('dd:realm', on);
    return () => window.removeEventListener('dd:realm', on);
  });

  const fresh = $derived(!!shown && shown.dry && shown.key === key && shown.world === world);
  /** What was done stays until the world changes under it (an undo takes it back). */
  const visible = $derived(shown && (shown.dry || shown.world === world) ? shown : null);
  const words = $derived(fresh && shown ? act(shown.r) : '');
  /** A download goes without a preview; anything else waits for one that found work. */
  const ready = $derived(!blocked && !busy && (way === 'read' ? !(fresh && shown && !words) : !!words));
  const label = $derived(words || verb);
  const stepOf = (r: T) => {
    const s = (r as { step?: unknown } | null)?.step;
    return typeof s === 'number' ? s : null;
  };

  async function preview() {
    if (blocked || busy) return;
    busy = 'preview';
    error = '';
    const [k, w] = [key, world];
    try {
      const r = await run(true);
      shown = { r, dry: true, key: k, world: w, text: '' };
    } catch (e) {
      shown = null;
      error = messageOf(e);
    } finally {
      busy = '';
    }
  }

  async function go() {
    if (!ready) return;
    busy = 'go';
    error = '';
    const k = key;
    try {
      const r = await run(false);
      sfx('tada');
      const text = done(r);
      const step = stepOf(r);
      toast(text, step !== null ? { action: 'Undo', onAction: () => void undo(step) } : {});
      if (way === 'in') realmChanged();
      // after the world's change is told, so the run's own change does not date it
      shown = { r, dry: false, key: k, world, text };
    } catch (e) {
      sfx('nope');
      error = messageOf(e);
    } finally {
      busy = '';
    }
  }

  function short() {
    nudge = true;
    clearTimeout(nudged);
    nudged = setTimeout(() => (nudge = false), 2600);
  }
</script>

<section class="run" aria-live="polite" aria-busy={busy !== ''}>
  <div class="head">
    <h3>{visible && !visible.dry ? 'Done' : 'Preview'}</h3>
    {#if busy}
      <span class="faint spin"><LoaderCircle size={14} />{busy === 'preview' ? 'Working out what it would do…' : way === 'read' ? 'Making the file…' : 'Writing…'}</span>
    {/if}
  </div>

  {#if error}
    <div class="callout bad"><TriangleAlert size={16} /><span>{error}</span></div>
  {/if}

  {#if visible}
    {#if !visible.dry}
      <div class="callout ok">
        <CircleCheck size={16} /><span>{visible.text}{way === 'in' ? ' It is one step in Changes: Undo takes it back until you publish.' : ''}</span>
      </div>
    {:else if !fresh}
      <p class="quiet">Something changed since this preview. Preview again to see what it would do now.</p>
    {/if}
    <div class="found" class:dated={visible.dry && !fresh}>{@render result(visible.r, visible.dry)}</div>
  {:else if blocked}
    <p class="quiet">{blocked}</p>
  {:else if !busy && !error}
    <p class="quiet">
      {way === 'read' ? 'Preview lists what the file would hold.' : 'Preview shows what it would do before anything is written.'}
    </p>
  {/if}

  <div class="go">
    <button type="button" class="btn sticker" disabled={!!blocked || !!busy} onclick={() => void preview()}>
      <Eye size={16} />{fresh ? 'Preview again' : 'Preview'}
    </button>
    {#if way === 'out'}
      <HoldButton onfire={() => void go()} onshort={short} disabled={!ready} describedby="transfer-note">
        <CloudUpload size={18} /><span>{busy === 'go' ? 'Writing…' : label}</span>
      </HoldButton>
    {:else}
      <button type="button" class="btn sticker primary" disabled={!ready} onclick={() => void go()}>
        {#if way === 'read'}<Download size={16} />{:else}<Check size={16} />{/if}
        {busy === 'go' ? (way === 'read' ? 'Making the file…' : 'Writing…') : label}
      </button>
    {/if}
    <span class="faint note" class:nudge id="transfer-note">
      {#if way === 'in'}
        One step in Changes: Undo takes it back until you publish.
      {:else if way === 'out'}
        This writes outside Daydream and can't be undone here. {fresh && words ? 'Hold the button until it fills.' : 'Preview first.'}
      {:else}
        It only reads the world.
      {/if}
    </span>
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
  .quiet { margin: 0; color: var(--ink-2); font-weight: 700; }
  .found { display: flex; flex-direction: column; gap: 10px; min-width: 0; transition: opacity .2s; }
  .found.dated { opacity: .55; }
  .go { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; padding-top: 4px; }
  .note { font-size: 12px; font-weight: 700; flex: 1 1 220px; }
  .note.nudge { color: var(--ink); animation: shake .4s; }
  @keyframes shake { 25% { transform: translateX(-3px); } 75% { transform: translateX(3px); } }
  @media (prefers-reduced-motion: reduce) { .note.nudge { animation: none; } }
</style>
