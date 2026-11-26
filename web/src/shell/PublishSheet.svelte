<!--
  Publishing, step by step, over the page: what it does now, what it waits for, and how it
  ended. It follows the job wherever it was started from; closing it leaves the job running,
  and a toast says when it ends.
-->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import Hand from '@lucide/svelte/icons/hand';
  import House from '@lucide/svelte/icons/house';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import X from '@lucide/svelte/icons/x';
  import Mascot from './Mascot.svelte';
  import { cancelPublish, GAME, pub, stepIcon } from './publishing.svelte';
  import { ago } from './time';

  const job = $derived(pub.job);
  const open = $derived(pub.sheet && job !== null);
  const game = $derived(job ? (GAME[job.engine] ?? 'the game') : '');
  const current = $derived(job?.steps.find((s) => s.state === 'running') ?? null);
  const waiting = $derived(job?.state === 'running' && current?.id === 'stop');

  /** How far along, out of 100: a step under way counts part done. */
  const pct = $derived.by(() => {
    if (!job) return 0;
    if (job.state === 'done') return 100;
    const done = job.steps.filter((s) => s.state === 'done' || s.state === 'yours').length;
    return Math.round((100 * (done + (current ? 0.4 : 0))) / Math.max(1, job.steps.length));
  });

  const mood = $derived(job?.state === 'done' ? 'joy' : job?.state === 'failed' ? 'worry' : job?.state === 'cancelled' ? 'happy' : 'dream');

  const heading = $derived.by(() => {
    if (!job) return '';
    switch (job.state) {
      case 'done':
        return job.way === 'update' ? 'Update file written' : `Published to ${game}`;
      case 'failed':
        return 'Publishing stopped';
      case 'cancelled':
        return 'Publishing cancelled';
    }
    switch (current?.id) {
      case 'check':
        return 'Checking the changes';
      case 'stop':
        return `Waiting for ${game} to stop`;
      case 'backup':
        return 'Backing up';
      case 'write':
        return job.way === 'update' ? 'Writing the update file' : 'Writing your changes';
      default:
        return 'Working on it';
    }
  });

  /** What a step left for the sysop is called. */
  const yours = (id: string) => (id === 'live' ? 'at the next start' : 'your turn');

  let sheet: HTMLElement | undefined = $state();
  let before: Element | null = null;
  $effect(() => {
    if (!open) return;
    before = document.activeElement;
    sheet?.focus();
    return () => {
      if (before instanceof HTMLElement && before.isConnected) before.focus();
    };
  });

  // confetti, once, when a publish ends well with the sheet open
  let pieces: { left: number; delay: number; turn: number; color: string }[] = $state([]);
  let celebrated = -1;
  $effect(() => {
    if (!open || !job || job.state !== 'done' || celebrated === job.id) return;
    celebrated = job.id;
    const colors = ['var(--pink)', 'var(--butter)', 'var(--mint)', 'var(--sky)', 'var(--lilac)', 'var(--peach)'];
    pieces = Array.from({ length: 60 }, (_, i) => ({
      left: Math.random() * 100,
      delay: Math.random() * 0.5,
      turn: Math.random() * 180,
      color: colors[i % colors.length],
    }));
  });

  function close() {
    pub.sheet = false;
    pieces = [];
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }
</script>

{#if open && job}
  <div class="run">
    <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="pub-sheet-h" tabindex="-1" bind:this={sheet} onkeydown={keydown}>
      {#if pieces.length}
        <div class="confetti" aria-hidden="true">
          {#each pieces as p, i (i)}
            <i style:left="{p.left}%" style:background={p.color} style:animation-delay="{p.delay}s" style:rotate="{p.turn}deg"></i>
          {/each}
        </div>
      {/if}
      <div class="top">
        <Mascot {mood} size={72} />
        <div class="say" aria-live="polite">
          <div class="eyebrow">Publishing to {job.game}</div>
          <h2 id="pub-sheet-h">{heading}{job.state === 'running' ? '…' : ''}</h2>
          {#if job.state === 'done' && job.result}
            <p>{job.result}</p>
          {:else if job.state === 'failed' && job.error}
            <p class="bad">{job.error}</p>
          {:else if job.state === 'cancelled'}
            <p>Nothing was written. Your changes are still waiting.</p>
          {:else if job.finished}
            <p>{ago(job.finished)}</p>
          {/if}
        </div>
      </div>

      <div class="bar" role="progressbar" aria-label="Progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct}>
        <span style:width="{pct}%" class:failed={job.state === 'failed'}></span>
      </div>

      <ol class="steps">
        {#each job.steps as st (st.id)}
          {@const Icon = stepIcon(st, job.way)}
          <li class="st {st.state}">
            <span class="bub">
              {#if st.state === 'done'}<Check size={17} />{:else if st.state === 'failed'}<X size={17} />{:else if st.state === 'yours'}<Hand size={17} />{:else}<Icon size={17} />{/if}
            </span>
            <div class="body">
              <h4>
                {st.title}
                {#if st.state === 'yours'}<span class="pill warn">{yours(st.id)}</span>{/if}
                {#if st.state === 'skipped' && job.state !== 'done'}<span class="pill">not done</span>{/if}
              </h4>
              {#if st.state === 'running' || st.state === 'yours'}<p>{st.detail}</p>{/if}
              {#if st.log.length}
                <div class="log">{#each st.log as line, i (i)}<div>{line}</div>{/each}</div>
              {/if}
            </div>
          </li>
        {/each}
      </ol>

      {#if job.skipped.length}
        <div class="callout warn skipped">
          <TriangleAlert size={16} />
          <span>
            <b>{job.skipped.length} edit{job.skipped.length === 1 ? '' : 's'} could not be written</b>, so {job.skipped.length === 1 ? 'it stays' : 'they stay'} among your changes:
            <ul>{#each job.skipped.slice(0, 8) as s, i (i)}<li>{s}</li>{/each}</ul>
            {#if job.skipped.length > 8}and {job.skipped.length - 8} more.{/if}
          </span>
        </div>
      {/if}

      <div class="acts">
        {#if job.state === 'running'}
          {#if waiting}
            <button type="button" class="btn" onclick={() => void cancelPublish()}><X size={15} />Cancel</button>
          {/if}
          <button type="button" class="btn sticker" onclick={close}>Keep it running in the background</button>
        {:else if job.state === 'done'}
          <button type="button" class="btn" onclick={close}>Close</button>
          <a class="btn primary" href="#/home" onclick={close}><House size={15} />Back home</a>
        {:else}
          <button type="button" class="btn sticker" onclick={close}>Close</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .run {
    position: fixed; inset: 0; z-index: 160; display: grid; place-items: center; padding: 16px;
    background: color-mix(in srgb, var(--ink) 30%, transparent); backdrop-filter: blur(4px); animation: fade-in .2s;
  }
  .sheet {
    position: relative; width: min(620px, 100%); max-height: 92vh; overflow: auto; padding: 22px;
    background: var(--surface); border: var(--stroke) solid var(--outline); border-radius: 26px;
    box-shadow: 0 6px 0 var(--shadow), var(--float); animation: pop-in .3s var(--pop);
  }
  .sheet:focus { outline: none; }
  .top { display: flex; gap: 14px; align-items: center; }
  .say { min-width: 0; }
  h2 { margin: 4px 0 0; font-size: 24px; line-height: 1.15; }
  .say p { margin: 4px 0 0; font-size: 13.5px; font-weight: 700; color: var(--ink-2); }
  .say p.bad { color: var(--bad); }
  .bar { height: 12px; border-radius: 999px; background: var(--surface-3); overflow: hidden; margin: 16px 0 6px; }
  .bar span {
    display: block; height: 100%; background: var(--rainbow); background-size: 200% 100%; border-radius: 999px;
    animation: rainbow-drift 3s linear infinite alternate; transition: width .4s var(--ease);
  }
  .bar span.failed { background: var(--bad); animation: none; }
  .steps { list-style: none; margin: 4px 0 0; padding: 0; }
  .st { display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 12px; padding: 5px 0; position: relative; }
  /* the dashed path from each step to the next */
  .st:not(:last-child)::before { content: ""; position: absolute; left: 16px; top: 39px; bottom: -5px; border-left: 2.5px dashed var(--line-2); }
  .st.done:not(:last-child)::before { border-left-style: solid; border-color: var(--mint); }
  .bub {
    width: 34px; height: 34px; border-radius: 12px; display: grid; place-items: center; position: relative; z-index: 1;
    background: var(--surface); border: var(--stroke) solid var(--line-2); color: var(--ink-2);
    transition: background .25s, border-color .25s, transform .3s var(--pop);
  }
  .st.running .bub { background: var(--sky-2); color: var(--sky-ink); border-color: var(--outline); transform: scale(1.08); animation: breathe 1.6s ease-in-out infinite; }
  .st.done .bub { background: var(--mint); color: var(--on-pastel); border-color: var(--outline); }
  .st.yours .bub { background: var(--butter-2); color: var(--butter-ink); border-color: var(--outline); }
  .st.failed .bub { background: var(--bad-bg); color: var(--bad); border-color: var(--bad); }
  .st.skipped { opacity: .55; }
  @keyframes breathe { 50% { transform: scale(1.14); } }
  h4 { margin: 6px 0 2px; font: 600 15px/1.2 var(--font-display); display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .body p { margin: 0; font-size: 12.5px; font-weight: 600; color: var(--ink-2); line-height: 1.45; }
  .log { margin-top: 4px; font: 600 12px/1.55 var(--font-mono); color: var(--ink-2); overflow-wrap: anywhere; }
  .skipped { margin-top: 12px; }
  .skipped ul { margin: 4px 0 0; padding-left: 18px; font-family: var(--font-mono); font-weight: 600; }
  .acts { display: flex; justify-content: flex-end; flex-wrap: wrap; gap: 8px; margin-top: 16px; }
  .confetti { position: absolute; inset: 0; pointer-events: none; overflow: hidden; border-radius: 26px; }
  .confetti i { position: absolute; top: -12px; width: 9px; height: 14px; border-radius: 3px; animation: fall 1.8s cubic-bezier(.2, .7, .4, 1) forwards; }
  @keyframes fall { to { transform: translateY(560px) rotate(540deg); opacity: 0; } }
  @media (prefers-reduced-motion: reduce) {
    .confetti { display: none; }
    .st.running .bub { animation: none; }
  }
</style>
