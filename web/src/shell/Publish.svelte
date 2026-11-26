<!--
  Publish: what goes out, the checks, and how it gets into the game, as the open world's
  engine allows (the server's plan, /api/publish). Holding the button starts the publish on
  the server; the sheet over the page follows it to the end.
-->
<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import CloudUpload from '@lucide/svelte/icons/cloud-upload';
  import Info from '@lucide/svelte/icons/info';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import Megaphone from '@lucide/svelte/icons/megaphone';
  import Server from '@lucide/svelte/icons/server';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import type { PublishCheck, PublishWayId } from '../api';
  import ChangeRow from './ChangeRow.svelte';
  import HoldButton from './HoldButton.svelte';
  import Mascot from './Mascot.svelte';
  import { read, write } from './prefs.svelte';
  import { DEFAULT_MESSAGE, loadPlan, pub, publish, stepIcon, WARN_MINUTES, warning } from './publishing.svelte';
  import ServerCard from './ServerCard.svelte';
  import Sky from './Sky.svelte';
  import { app, takeBack } from './state.svelte';
  import { ago } from './time';

  const plan = $derived(pub.plan);
  const changes = $derived(app.changes?.changes ?? []);
  const since = $derived(app.changes?.since);
  const n = $derived(changes.length);
  const s = (k: number) => (k === 1 ? '' : 's');
  const counts = $derived.by(() => {
    const c = { added: 0, changed: 0, deleted: 0 };
    for (const ch of changes) c[ch.how]++;
    return c;
  });

  /** The way picked on the page; the plan's own choice until then. */
  let chosen: PublishWayId | null = $state(null);
  const way = $derived(plan?.ways.find((w) => w.id === (chosen ?? plan.way)) ?? plan?.ways[0] ?? null);
  const other = $derived(plan?.ways.find((w) => w.id !== way?.id && w.ready) ?? null);
  const busy = $derived(pub.job?.state === 'running');
  const wcc = $derived(plan?.engine === 'worldgroup' || plan?.engine === 'majorbbs');
  const db = $derived(plan?.engine === 'mbbsemu' || plan?.engine === 'db');
  const blocks = $derived(plan?.checks.filter((c) => c.level === 'block') ?? []);
  const last = $derived(plan?.job && plan.job.state !== 'running' ? plan.job : null);

  // the plan read afresh on coming here, and whenever the changes or the game's state change
  let seen = '';
  $effect(() => {
    const sig = JSON.stringify([app.changes?.since, changes.map((c) => [c.stem, c.id, c.how, c.members]), app.running]);
    if (app.route.place !== 'publish') {
      seen = '';
      return;
    }
    if (sig === seen) return;
    seen = sig;
    void loadPlan();
  });

  const title = $derived.by(() => {
    if (!plan) return 'Publish';
    if (plan.engine === 'files') return 'Nothing to publish';
    if (since === null) return `Send this world to ${plan.name}`;
    if (!n) return 'Nothing is waiting to publish';
    return `Send ${n} change${s(n)} to ${plan.name}`;
  });

  const lede = $derived.by(() => {
    if (!plan) return pub.error ? `Daydream could not read the plan: ${pub.error}` : 'Reading the plan…';
    if (plan.engine === 'files') return plan.checks[0]?.text ?? '';
    if (wcc) {
      return plan.ways.find((w) => w.id === 'update')?.ready
        ? `${plan.name} has no remote control, so the gentle way hands MajorMUD an update file it merges by itself, and nobody has to stop the BBS.`
        : `Daydream writes your changes into the game's files while ${plan.name} is stopped. It waits while you stop it, and carries on by itself.`;
    }
    if (db) return `${plan.name} writes its .db files as it runs, so you edit a copy while it plays. To publish, Daydream waits while you stop it, then writes each change onto the game's record as play left it.`;
    if (warns) return 'silvermere warns its players and closes, Daydream writes its world from your working copy, and what play changed stays as it is.';
    return 'Daydream waits while silvermere stops, then writes its world from your working copy, keeping what play changed.';
  });

  const mood = $derived(pub.error && !plan ? 'worry' : busy ? 'dream' : n ? 'happy' : 'dream');

  // silvermere's warning: how long the players get, and why, as this browser last had it
  const control = $derived(plan?.control?.engine === 'silvermere' ? plan.control : null);
  const players = $derived(control?.players ?? null);
  const warns = $derived(way?.steps.some((st) => st.id === 'warn') ?? false);
  const stored = Number(read('dd-warn-in', '5'));
  let warnIn = $state(WARN_MINUTES.includes(stored) ? stored : 5);
  let message = $state(read('dd-warn-why', DEFAULT_MESSAGE));
  const why = $derived(message.trim() || DEFAULT_MESSAGE);
  function setWarnIn(m: number) {
    warnIn = m;
    write('dd-warn-in', String(m));
  }
  function keepWhy() {
    write('dd-warn-why', message.trim() || DEFAULT_MESSAGE);
  }
  const initials = (name: string) =>
    name.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]!.toUpperCase()).join('') || '?';

  const running = $derived.by(() => {
    const r = plan?.running;
    if (!r || r.running === null) return { text: 'cannot tell whether it runs', color: 'var(--ink-3)', pulse: false };
    if (r.running) return { text: `running${r.by ? ` (${r.by})` : ''}`, color: 'var(--mint)', pulse: true };
    return { text: 'stopped', color: 'var(--ink-3)', pulse: false };
  });

  const button = $derived.by(() => {
    if (!way) return '';
    if (way.id === 'update') return way.button;
    if (since === null) return 'Hold to publish the world';
    return n ? `Hold to publish ${n} change${s(n)}` : way.button;
  });

  /** What stays as play left it, for the way chosen. */
  const kept = $derived.by(() => {
    if (!plan) return '';
    if (plan.engine === 'silvermere') return 'The world’s tables are written from your working copy. Players and what play changed stay exactly as they are.';
    if (way?.id === 'update') return 'Only these records go into the update file. MajorMUD keeps what play changed: players, what lies on the floors, shop stock and kill timers.';
    return 'Only these records are written, each change onto the game’s record as play left it. Players, what lies on the floors, shop stock and kill timers stay as they are.';
  });

  let nudge = $state(false);
  let nudged: ReturnType<typeof setTimeout> | null = null;
  function short() {
    nudge = true;
    if (nudged) clearTimeout(nudged);
    nudged = setTimeout(() => (nudge = false), 2600);
  }

  const help = $derived.by(() => {
    if (!way) return '';
    if (busy) return 'One publish at a time. This one carries on wherever you go in Daydream.';
    // a way that cannot be taken says why above it
    if (!way.ready) return blocks[0]?.text ?? (way.why ? '' : !n && since !== null ? 'Nothing is waiting to publish.' : '');
    if (nudge) return 'Keep holding until the button fills.';
    if (warns && warnIn && players?.length !== 0)
      return `Press and hold, or press Enter. silvermere closes ${warnIn} minute${s(warnIn)} later, and you can call it off until then.`;
    return 'Press and hold, or press Enter. Daydream backs up what it replaces first.';
  });

  function takeAll() {
    if (!confirm(`Take back all ${n} changes? Every record goes back to how the game has it. You can undo this.`)) return;
    void takeBack('all', `all ${n} changes`);
  }

  const NOTE: Record<PublishCheck['level'], { cls: string; icon: typeof Info }> = {
    ok: { cls: 'ok', icon: CircleCheck },
    note: { cls: 'info', icon: Info },
    warn: { cls: 'warn', icon: TriangleAlert },
    block: { cls: 'bad', icon: CircleX },
  };
</script>

<div class="publish">
  <section class="hero sky">
    <Sky clouds={3} stars={18} seed={11} />
    <div class="in">
      <Mascot {mood} size={84} />
      <div class="say">
        <div class="eyebrow">Publish</div>
        <h1>{title}</h1>
        <p>{lede}</p>
      </div>
      {#if plan && plan.engine !== 'files'}
        <div class="srv">
          <span class="eng" style:grid-row="span {players && plan.running.running ? 4 : 3}"><Server size={22} /></span>
          <b title={plan.game ?? plan.name}>{plan.game ?? plan.name}</b>
          <span class="sub"><span class="dot" class:pulse={running.pulse} style:color={running.color}></span>{running.text}</span>
          {#if plan.location}<span class="sub loc" title={plan.location}>{plan.location}</span>{/if}
          {#if players && plan.running.running}
            <span class="sub who">
              {#if players.length}
                <span class="faces" aria-hidden="true">
                  {#each players.slice(0, 5) as p, i (i)}<i title={p.name}>{initials(p.name)}</i>{/each}
                </span>
                <span title={players.map((p) => p.name).join(', ')}>{players.length} in the game</span>
              {:else}
                nobody in the game
              {/if}
            </span>
          {/if}
        </div>
      {/if}
    </div>
  </section>

  <div class="cols">
    <section class="card sticker flow">
      <div class="card-h">
        <h3>How it gets there</h3>
        {#if plan && plan.ways.length > 1}
          <span class="seg" role="tablist" aria-label="Ways to publish">
            {#each plan.ways as w (w.id)}
              <button type="button" role="tab" aria-selected={w.id === way?.id} class:on={w.id === way?.id} class:off={!w.ready} title={w.why ? `Not ready: ${w.why}` : undefined} onclick={() => (chosen = w.id)}>{w.title}</button>
            {/each}
          </span>
        {/if}
      </div>

      {#if !plan}
        {#if pub.error}
          <div class="callout bad"><CircleX size={16} /><span>{pub.error}</span></div>
        {:else}
          <p class="muted">Reading the plan…</p>
        {/if}
      {:else if !way}
        <p class="summary">{plan.checks[0]?.text ?? 'This world has nothing to publish.'}</p>
        <p class="faint">Exports and the other ways to write a world are in the <a href="#/workshop">Workshop</a>.</p>
      {:else}
        <p class="summary">{way.summary}</p>
        {#if way.why && !blocks.length}
          <div class="callout warn">
            <TriangleAlert size={16} />
            <span>
              <b>Not this way:</b> {way.why}.
              {#if way.not_carried.length}
                <ul class="nc">{#each way.not_carried.slice(0, 6) as line, i (i)}<li>{line}</li>{/each}</ul>
                {#if way.not_carried.length > 6}<span>and {way.not_carried.length - 6} more.</span>{/if}
              {/if}
              {#if other}
                <span class="row"><button type="button" class="btn sm" onclick={() => (chosen = other.id)}>{other.title} instead</button></span>
              {/if}
            </span>
          </div>
        {/if}
        <ol class="steps">
          {#each way.steps as st (st.id)}
            {@const Icon = stepIcon(st, way.id)}
            <li class="st">
              <span class="bub"><Icon size={19} /></span>
              <div>
                <h4>{st.title}</h4><p>{st.detail}</p>
                {#if st.id === 'warn'}
                  <div class="notice">
                    <span class="seg" role="radiogroup" aria-label="When silvermere closes">
                      {#each WARN_MINUTES as m (m)}
                        <button type="button" role="radio" aria-checked={warnIn === m} class:on={warnIn === m} disabled={busy} onclick={() => setWarnIn(m)}>{m ? `${m} min` : 'Now'}</button>
                      {/each}
                    </span>
                    <label class="why">
                      <span>Why, in a few words</span>
                      <input type="text" maxlength="200" bind:value={message} onchange={keepWhy} disabled={busy} placeholder={DEFAULT_MESSAGE} />
                    </label>
                    <p class="hear">
                      <Megaphone size={14} />
                      {#if players?.length === 0}
                        <span>Nobody is in the game, so silvermere closes at once and nobody needs telling.</span>
                      {:else}
                        <span>The players read <q>{warning(warnIn, why)}</q>{warnIn ? ', and again as the time comes closer.' : '.'}</span>
                      {/if}
                    </p>
                  </div>
                {/if}
              </div>
            </li>
          {/each}
        </ol>
        <div class="go">
          {#if busy}
            <button type="button" class="btn rainbow big busy" onclick={() => (pub.sheet = true)}><LoaderCircle size={19} />Publishing… show how it goes</button>
          {:else}
            {@const w = way}
            <HoldButton onfire={() => void publish(w.id, warns ? { in_secs: warnIn * 60, message: why } : undefined)} onshort={short} disabled={!w.ready} describedby={help ? 'pub-help' : undefined}>
              <CloudUpload size={19} /><span>{button}</span>
            </HoldButton>
          {/if}
        </div>
        {#if help}<p class="help" class:nudge id="pub-help">{help}</p>{/if}
      {/if}

      {#if last}
        <div class="last">
          {#if last.state === 'done'}<CircleCheck size={15} />{:else}<Info size={15} />{/if}
          <span>
            Last publish{last.finished ? `, ${ago(last.finished)}` : ''}:
            {last.state === 'done' ? 'it went through.' : last.state === 'cancelled' ? 'cancelled.' : `stopped: ${last.error ?? 'something went wrong'}.`}
          </span>
          <button type="button" class="btn sm ghost" onclick={() => (pub.sheet = true)}>See how it went</button>
        </div>
      {/if}
    </section>

    <div class="stack">
      <section class="card review">
        <div class="card-h">
          <h3>What goes out</h3>
          {#if n}
            <span class="pills">
              {#if counts.changed}<span class="pill warn">{counts.changed} changed</span>{/if}
              {#if counts.added}<span class="pill ok">{counts.added} added</span>{/if}
              {#if counts.deleted}<span class="pill bad">{counts.deleted} deleted</span>{/if}
            </span>
            <button type="button" class="btn sm ghost danger end" onclick={takeAll}><Undo2 size={14} />Take back all</button>
          {/if}
        </div>
        {#if since && n}
          <p class="faint since">Changed since the world was read from its game or last published, {ago(since)}.</p>
        {:else if since}
          <p class="faint since">The world was read from its game or last published {ago(since)}.</p>
        {:else if app.changes && since === null}
          <p class="faint since">This world was edited before Daydream kept a list of its changes, so they are not known: publishing writes the whole world, once.</p>
        {/if}
        {#if n}
          <ul class="list">
            {#each changes as ch (`${ch.stem}:${ch.id}`)}<ChangeRow c={ch} />{/each}
          </ul>
          {#if plan && plan.engine !== 'files'}
            <div class="callout ok kept"><ShieldCheck size={16} /><span>{kept}</span></div>
          {/if}
        {:else if since !== null}
          <p class="quiet">Every save you make in Daydream gathers here until you publish it.</p>
        {/if}
      </section>

      {#if plan}
        <section class="card checks">
          <div class="card-h">
            <h3>Checks</h3>
            <span class="end">
              {#if blocks.length}
                <span class="pill bad"><CircleX size={13} />{blocks.length} blocker{s(blocks.length)}</span>
              {:else}
                <span class="pill ok"><CircleCheck size={13} />No blockers</span>
              {/if}
            </span>
          </div>
          <div class="notes">
            {#each plan.checks as ck, i (i)}
              {@const k = NOTE[ck.level]}
              <div class="callout {k.cls}"><k.icon size={16} /><span>{ck.text}</span></div>
            {:else}
              <p class="quiet">All clear: nothing stands in the way.</p>
            {/each}
          </div>
        </section>
      {/if}

      {#if control}
        <ServerCard c={control} />
      {/if}
    </div>
  </div>
</div>

<style>
  .publish { padding: 18px clamp(12px, 2.5vw, 26px) 60px; display: flex; flex-direction: column; gap: 16px; max-width: 1240px; margin: 0 auto; }
  .hero { border-radius: 26px; border: var(--stroke) solid var(--outline); box-shadow: 0 5px 0 var(--shadow); padding: 22px clamp(16px, 3vw, 34px); }
  .in { position: relative; display: flex; align-items: center; gap: clamp(14px, 3vw, 24px); flex-wrap: wrap; }
  .say { flex: 1 1 320px; min-width: 0; }
  h1 { margin: 6px 0 0; font-size: clamp(24px, 3.4vw, 34px); line-height: 1.08; }
  .say p { margin: 6px 0 0; color: var(--ink-2); font-weight: 700; max-width: 60ch; }
  .srv {
    flex: 0 1 340px; min-width: 0; display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 3px 12px; align-items: center;
    background: var(--surface); border: var(--stroke) solid var(--outline); border-radius: 18px; padding: 12px 16px; box-shadow: 0 4px 0 var(--shadow);
  }
  .srv .eng { grid-row: span 3; width: 44px; height: 44px; border-radius: 14px; display: grid; place-items: center; background: var(--mint-2); color: var(--mint-ink); }
  .srv b { font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .srv .sub { font-size: 12px; color: var(--ink-2); font-weight: 700; display: flex; gap: 6px; align-items: center; min-width: 0; }
  .srv .loc { display: block; font: 600 11.5px var(--font-mono); color: var(--ink-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .faces { display: inline-flex; }
  .faces i {
    width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; margin-left: -6px;
    font: 800 9.5px/1 var(--font-ui); font-style: normal; background: var(--lilac-2); color: var(--lilac-ink); border: 2px solid var(--surface);
  }
  .faces i:first-child { margin-left: 0; }
  .faces i:nth-child(2n) { background: var(--peach-2); color: var(--peach-ink); }
  .faces i:nth-child(3n) { background: var(--sky-2); color: var(--sky-ink); }

  .cols { display: grid; grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  .stack { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  @media (max-width: 900px) { .cols { grid-template-columns: minmax(0, 1fr); } }

  .card-h { flex-wrap: wrap; }
  h3 { margin: 0; font-size: 17px; }
  .end { margin-left: auto; }
  .seg { margin-left: auto; display: inline-flex; flex-wrap: wrap; gap: 2px; padding: 3px; border-radius: 12px; background: var(--surface-2); border: 1px solid var(--line); }
  .seg button { min-height: 28px; padding: 0 10px; border: 0; border-radius: 9px; background: transparent; font-size: 12.5px; color: var(--ink-2); }
  .seg button:hover:not(.on) { background: var(--surface-3); }
  .seg button.on { background: var(--surface); color: var(--ink); box-shadow: 0 0 0 1px var(--line-2), 0 1px 0 var(--line-2); }
  .seg button.off:not(.on) { color: var(--ink-3); }
  .summary { margin: 0 0 4px; font-size: 13.5px; font-weight: 700; color: var(--ink-2); line-height: 1.5; }
  .flow .callout { margin-top: 10px; }
  .nc { margin: 4px 0 2px; padding-left: 18px; font: 600 12px/1.5 var(--font-mono); }
  .row { display: flex; gap: 6px; margin-top: 8px; }

  .steps { list-style: none; margin: 12px 0 0; padding: 0; }
  .st { display: grid; grid-template-columns: 40px minmax(0, 1fr); gap: 12px; padding: 8px 0; position: relative; }
  /* the dashed path from each step to the next */
  .st:not(:last-child)::before { content: ""; position: absolute; left: 19px; top: 48px; bottom: -8px; border-left: 2.5px dashed var(--line-2); }
  .bub { width: 40px; height: 40px; border-radius: 14px; display: grid; place-items: center; position: relative; z-index: 1; background: var(--surface); border: var(--stroke) solid var(--line-2); color: var(--ink-2); }
  h4 { margin: 2px 0 2px; font: 600 16.5px/1.2 var(--font-display); }
  .st p { margin: 0; font-size: 13px; font-weight: 600; color: var(--ink-2); line-height: 1.45; }
  .notice { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; padding: 10px 12px; border-radius: 14px; background: var(--surface-2); border: 1px solid var(--line); }
  .notice .seg { margin-left: 0; align-self: flex-start; background: var(--surface); }
  .notice .seg button.on { background: var(--butter-2); color: var(--butter-ink); }
  .why { display: flex; flex-direction: column; gap: 4px; font-size: 12px; font-weight: 800; color: var(--ink-2); }
  .why input { width: 100%; min-height: 34px; font: 700 13px var(--font-ui); background: var(--surface); }
  .notice .hear { display: flex; gap: 7px; align-items: flex-start; font-size: 12.5px; }
  .hear > :global(svg) { flex: none; margin-top: 2px; color: var(--butter-ink); }
  .hear q { font-weight: 800; color: var(--ink); quotes: '“' '”'; }

  .go { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 16px; }
  .big { min-height: 52px; padding: 0 22px; border-radius: 16px; font-size: 15.5px; }
  .busy :global(svg) { animation: spin 1.1s linear infinite; }
  @keyframes spin { to { rotate: 360deg; } }
  .help { margin: 8px 0 0; font-size: 12px; font-weight: 700; color: var(--ink-3); }
  .help.nudge { color: var(--pink-ink); animation: nudge .4s var(--ease); }
  @keyframes nudge { 25% { translate: -3px 0; } 75% { translate: 3px 0; } }
  @media (prefers-reduced-motion: reduce) { .help.nudge, .busy :global(svg) { animation: none; } }
  .last { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-top: 16px; padding-top: 12px; border-top: 1px dashed var(--line); font-size: 12.5px; font-weight: 700; color: var(--ink-2); }
  .last .btn { margin-left: auto; }

  .pills { display: flex; gap: 6px; }
  .since { margin: -2px 0 8px; font-size: 12.5px; font-weight: 700; }
  .list { list-style: none; margin: 0 -6px; padding: 0; max-height: 46vh; overflow: auto; }
  .kept { margin-top: 10px; }
  .quiet { margin: 0; color: var(--ink-2); font-weight: 600; }
  .notes { display: flex; flex-direction: column; gap: 8px; }

  @media (max-width: 600px) {
    .hero { padding: 18px 16px; }
    .in { gap: 12px; }
    .in > :global(.mascot) { width: 56px; height: 56px; }
    .say { flex-basis: 200px; }
    .srv { flex-basis: 100%; }
    .seg { width: 100%; }
    .seg button { flex: 1 1 0; white-space: normal; line-height: 1.2; padding: 6px 8px; }
    .notice .seg { align-self: stretch; }
  }
</style>
