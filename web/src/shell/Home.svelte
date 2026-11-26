<!--
  Home: the open world at a glance. A greeting with what needs doing, the changes waiting to
  publish, the saves that can be undone, the records opened lately, and what is in the world.
-->
<script lang="ts">
  import CloudUpload from '@lucide/svelte/icons/cloud-upload';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import MapIcon from '@lucide/svelte/icons/map';
  import Redo2 from '@lucide/svelte/icons/redo-2';
  import Search from '@lucide/svelte/icons/search';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import ChangeRow from './ChangeRow.svelte';
  import { kindOf, TABLE_ORDER } from './kinds';
  import Mascot from './Mascot.svelte';
  import { keyLabel, recordHref } from './routes';
  import Sky from './Sky.svelte';
  import { app, redo, rowsOf, undo } from './state.svelte';
  import { ago, partOfDay } from './time';

  const mod = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent) ? '⌘' : 'Ctrl';
  const c = $derived(app.current);
  const name = $derived(c ? (c.label ?? c.realm_dir.split(/[\\/]/).filter(Boolean).pop() ?? c.realm_dir) : '');
  const changes = $derived(app.changes?.changes ?? []);
  const steps = $derived(app.history?.steps.slice(0, 7) ?? []);

  const WHAT = { wcc: 'a MajorMUD game in WCC .DAT files', db: 'a MajorMUD game in MBBSEmu .db files', silvermere: 'a silvermere server' };

  const sentence = $derived.by(() => {
    if (!c) return 'Pick a world to start: Daydream finds the games on this computer.';
    const bits = [`You are working on ${name}, ${c.kind ? WHAT[c.kind] : 'a realm folder'}.`];
    if (app.running === true) bits.push(`The game is running${app.runningBy ? ` (${app.runningBy})` : ''}${c.kind === 'db' ? '' : ', so stop it before you publish'}.`);
    else if (app.running === false) bits.push('The game is stopped.');
    bits.push(changes.length ? `You have ${changes.length} change${changes.length === 1 ? '' : 's'} waiting to publish.` : 'Nothing is waiting to publish.');
    return bits.join(' ');
  });

  // how many of each record the world has, read once the tables are known
  let counts: Record<string, number> = $state({});
  $effect(() => {
    const tables = TABLE_ORDER.filter((t) => app.tables.some((x) => x.name === t) && !['Users', 'BankBooks', 'Gangs'].includes(t));
    void app.changes; // a save can add or remove records
    for (const t of tables) {
      void rowsOf(t)
        .then((rows) => (counts[t] = rows.length))
        .catch(() => {});
    }
  });

  const TIPS = [
    [`${mod} K`, 'finds any record, place or command. Try a name, a number, or "item axe".'],
    ['g then a letter', 'jumps to a place: g w is the World map, g l the Library, g i Items, g m Monsters.'],
    [`${mod} Z`, 'undoes the last save, wherever it was made. Every save waits in Changes until you publish.'],
    ['[ and ]', 'step to the previous and next record in an editor, like the old Previous and Next buttons.'],
    ['Esc', 'closes the record open beside the page. Its Open full page button goes to its editor.'],
  ];
  let tip = $state(Math.floor(Math.random() * TIPS.length));
</script>

<div class="home">
  <section class="hero sky">
    <Sky clouds={4} stars={22} seed={5} />
    <div class="in">
      <Mascot mood={c ? 'joy' : 'dream'} size={112} />
      <div class="say">
        <h1>Good {partOfDay()}!</h1>
        <p>{sentence}</p>
        <div class="acts">
          {#if c}
            <a class="btn lg primary" href="#/publish"><CloudUpload size={18} />{changes.length ? `Publish ${changes.length} change${changes.length === 1 ? '' : 's'}` : 'Publish'}</a>
            <a class="btn lg sticker" href="#/world"><MapIcon size={18} />Open the map</a>
            <button type="button" class="btn lg" onclick={() => (app.paletteOpen = true)}><Search size={18} />Find anything <span class="kbd hide-narrow">{mod} K</span></button>
          {:else}
            <a class="btn lg primary" href="#/worlds"><FolderOpen size={18} />Pick a world</a>
          {/if}
        </div>
      </div>
    </div>
  </section>

  {#if c}
    <div class="grid">
      <section class="card span-7">
        <h3>Waiting to publish</h3>
        {#if changes.length}
          <ul class="changes">
            {#each changes.slice(0, 6) as ch (`${ch.stem}:${ch.id}`)}<ChangeRow c={ch} />{/each}
          </ul>
          <div class="row-end">
            {#if changes.length > 6}<span class="faint">and {changes.length - 6} more</span>{/if}
            <a class="btn sm" href="#/publish">Review all <ArrowRight size={14} /></a>
          </div>
        {:else}
          <p class="quiet">Nothing yet. Each save waits here until you publish it, and each can be taken back.</p>
        {/if}
      </section>

      <section class="card span-5">
        <h3>Recent saves</h3>
        {#if steps.length}
          <ol class="steps">
            {#each steps as s (s.step)}
              <li class:undone={s.state === 'undone'}>
                <span class="lbl">{s.label}</span>
                <span class="faint when">{s.records} record{s.records === 1 ? '' : 's'} · {ago(s.at)}{s.state === 'undone' ? ' · undone' : ''}</span>
                {#if app.history?.undo?.step === s.step}
                  <button type="button" class="btn sm" onclick={() => void undo(s.step)}><Undo2 size={14} />Undo</button>
                {:else if app.history?.redo?.step === s.step}
                  <button type="button" class="btn sm" onclick={() => void redo(s.step)}><Redo2 size={14} />Redo</button>
                {/if}
              </li>
            {/each}
          </ol>
        {:else}
          <p class="quiet">No saves yet in this world.</p>
        {/if}
      </section>

      <section class="card span-7">
        <h3>In this world</h3>
        <div class="tiles">
          {#each TABLE_ORDER.filter((t) => app.tables.some((x) => x.name === t) && !['Users', 'BankBooks', 'Gangs'].includes(t)) as t (t)}
            {@const k = kindOf(t)}
            <a class="stile {k.cls}" href={recordHref(t)}>
              <span class="tile"><k.icon size={16} /></span>
              <b class="tnum">{counts[t]?.toLocaleString() ?? '…'}</b>
              <span>{k.many}</span>
            </a>
          {/each}
        </div>
      </section>

      <section class="card span-5">
        <h3>Pick up where you left off</h3>
        {#if app.recent.length}
          <ul class="recent">
            {#each app.recent.slice(0, 6) as r (`${r.table}:${keyLabel(r.key)}`)}
              {@const k = kindOf(r.table)}
              <li class={k.cls}>
                <a href={recordHref(r.table, r.key)}>
                  <span class="tile"><k.icon size={15} /></span>
                  <span class="nm">{r.name || `${k.one} ${keyLabel(r.key)}`}</span>
                  <span class="faint num">{k.one} {keyLabel(r.key)}</span>
                </a>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="quiet">The records you open show up here.</p>
        {/if}
      </section>

      <section class="card span-12 tip">
        <span class="pill lilac"><Sparkles size={13} />Tip</span>
        <span><span class="kbd">{TIPS[tip]![0]}</span> {TIPS[tip]![1]}</span>
        <button type="button" class="btn sm ghost next" onclick={() => (tip = (tip + 1) % TIPS.length)}>Next tip <ArrowRight size={14} /></button>
      </section>
    </div>
  {/if}
</div>

<style>
  .home { padding: 18px clamp(12px, 2.5vw, 26px) 60px; display: flex; flex-direction: column; gap: 16px; max-width: 1240px; margin: 0 auto; }
  .hero { border-radius: 26px; border: var(--stroke) solid var(--outline); box-shadow: 0 5px 0 var(--shadow); padding: 26px clamp(16px, 3vw, 34px); }
  .in { position: relative; display: flex; align-items: center; gap: clamp(14px, 3vw, 30px); flex-wrap: wrap; }
  .say { flex: 1 1 360px; min-width: 0; }
  h1 { margin: 0 0 4px; font-size: clamp(26px, 4vw, 36px); line-height: 1.1; }
  .say p { margin: 0 0 14px; font-size: 15px; font-weight: 700; color: var(--ink-2); max-width: 62ch; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; }
  .btn.lg { min-height: 42px; padding: 0 16px; font-size: 14px; border-radius: 14px; }
  .grid { display: grid; grid-template-columns: repeat(12, minmax(0, 1fr)); gap: 16px; }
  .span-7 { grid-column: span 7; }
  .span-5 { grid-column: span 5; }
  .span-12 { grid-column: span 12; }
  @media (max-width: 980px) { .span-7, .span-5 { grid-column: span 12; } }
  h3 { margin: 0 0 10px; font-size: 17px; }
  .quiet { margin: 0; color: var(--ink-2); font-weight: 600; }
  .changes { list-style: none; margin: 0 -6px; padding: 0; }
  .row-end { display: flex; justify-content: flex-end; align-items: center; gap: 10px; margin-top: 8px; font-size: 12.5px; font-weight: 700; }
  .steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .steps li { display: grid; grid-template-columns: 1fr auto; grid-template-rows: auto auto; column-gap: 10px; padding: 6px 8px; border-radius: 12px; align-items: center; }
  .steps li:hover { background: var(--surface-2); }
  .steps .lbl { font-weight: 800; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .steps .when { font-size: 12px; font-weight: 700; grid-row: 2; }
  .steps .btn { grid-row: 1 / span 2; grid-column: 2; }
  .steps li.undone .lbl { color: var(--ink-3); text-decoration: line-through; }
  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 8px; }
  .stile { display: flex; flex-direction: column; gap: 2px; padding: 10px 12px; border-radius: 16px; text-decoration: none; color: var(--ink); background: color-mix(in srgb, var(--t-bg) 55%, var(--surface)); border: 1px solid var(--line); transition: transform .15s var(--pop); }
  .stile:hover { transform: translateY(-2px); border-color: var(--outline); }
  .stile .tile { width: 28px; height: 28px; border-radius: 9px; margin-bottom: 4px; }
  .stile b { font: 600 20px var(--font-display); }
  .stile span:last-child { font-size: 12px; font-weight: 800; color: var(--ink-2); }
  .recent { list-style: none; margin: 0 -6px; padding: 0; }
  .recent a { display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 12px; color: var(--ink); text-decoration: none; min-width: 0; }
  .recent a:hover { background: var(--surface-2); }
  .recent .tile { width: 26px; height: 26px; border-radius: 8px; }
  .recent .nm { font-weight: 800; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .recent .num { font: 600 11px var(--font-mono); flex: none; }
  .tip { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; font-weight: 700; font-size: 13.5px; }
  .tip .next { margin-left: auto; }
</style>
