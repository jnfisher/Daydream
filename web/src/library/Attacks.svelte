<!--
  A monster's attacks as a fight plays them: how often each one is picked (the Attack % add up
  as they go), how many blows its Energy buys, each slot's fields with what they mean for its
  type, the lines each outcome sends (to the player hit, or to the room), and a round drawn as
  the game draws one, again at a press.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Dices from '@lucide/svelte/icons/dices';
  import Plus from '@lucide/svelte/icons/plus';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import WandSparkles from '@lucide/svelte/icons/wand-sparkles';
  import X from '@lucide/svelte/icons/x';
  import { renderAnsi } from '../ansi';
  import { pickFor } from '../layouts';
  import {
    attackProblems,
    attacksOf,
    blankLine,
    blowLines,
    blowsOf,
    holdsNumbers,
    knownType,
    liveSlot,
    NORMAL,
    ROB,
    roundOf,
    SAMPLE_VICTIM,
    shares,
    SPELL,
    spellsOf,
    verbsOf,
    type Attack,
    type Outcome,
    type Texts,
  } from './combat';
  import type { Draft } from './draft.svelte';
  import FieldInput from './FieldInput.svelte';
  import { nameOf } from './names.svelte';
  import { linesOf, recordOf } from './records.svelte';

  let { draft }: { draft: Draft } = $props();

  const COLOURS = ['var(--peach)', 'var(--sky)', 'var(--lilac)', 'var(--mint)', 'var(--butter)'];
  /** Each slot's columns, with NMR's label. */
  const COLS: { template: string; label: string }[] = [
    { template: 'Attack % #', label: 'Attack%' },
    { template: 'Attack Accu/Spell #', label: 'Accu/Spell' },
    { template: 'Attack Min Hit/Cast % #', label: 'MinH/Cast%' },
    { template: 'Attack Max Hit/Cast LVL #', label: 'MaxH/CastLvL' },
    { template: 'Attack Energy #', label: 'Energy' },
    { template: 'Attack Hit Msg #', label: 'Hit Msg' },
    { template: 'Attack Dodge Msg #', label: 'Dodge Msg' },
    { template: 'Attack Miss Msg #', label: 'Miss Msg' },
    { template: 'Attack Hit Spell #', label: 'Hit Spell' },
  ];
  /** What a column means for an attack of each type; a column a type does not use is left out. */
  const MEANS: Record<number, Record<string, string>> = {
    [NORMAL]: {
      'Attack % #': 'chance, added up',
      'Attack Accu/Spell #': 'accuracy',
      'Attack Min Hit/Cast % #': 'least damage',
      'Attack Max Hit/Cast LVL #': 'most damage',
      'Attack Energy #': 'a blow costs',
      'Attack Hit Msg #': 'hit lines',
      'Attack Dodge Msg #': 'armour and dodge',
      'Attack Miss Msg #': 'miss lines',
      'Attack Hit Spell #': 'cast on a hit',
    },
    [SPELL]: {
      'Attack % #': 'chance, added up',
      'Attack Accu/Spell #': 'the spell',
      'Attack Min Hit/Cast % #': 'chance to cast',
      'Attack Max Hit/Cast LVL #': 'cast at level',
      'Attack Energy #': 'a cast costs',
    },
    [ROB]: { 'Attack % #': 'chance, added up' },
  };
  const TARGET: Record<string, string> = {
    'Attack Hit Msg #': 'Messages',
    'Attack Dodge Msg #': 'Messages',
    'Attack Miss Msg #': 'Messages',
    'Attack Hit Spell #': 'Spells',
  };
  const OUTCOMES: [Outcome, string][] = [
    ['hit', 'hit'],
    ['armour', 'armour'],
    ['dodge', 'dodge'],
    ['miss', 'miss'],
  ];

  const col = (template: string, i: number) => template.replace('#', String(i));
  /** The table a column names for an attack of a type, when it names one. */
  const linkTo = (template: string, type: number) => (type === SPELL && template === 'Attack Accu/Spell #' ? 'Spells' : TARGET[template]);
  const num = (v: unknown) => Number(v) || 0;
  const text = (v: unknown) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);

  const attacks = $derived(attacksOf(draft.get));
  const energy = $derived(num(draft.get('Energy')));
  const odds = $derived(shares(attacks));
  const mon = $derived(text(draft.get('Name')) || 'it');

  // its weapon names the words its messages leave to the weapon
  const weaponNo = $derived(num(draft.get('Weapon Number')));
  const weapon = $derived(weaponNo ? recordOf('Items', [weaponNo]) : null);
  const verbs = $derived(
    verbsOf(weapon ? { name: text(weapon['Name']), hit: linesOf(num(weapon['Hit Msg'])) ?? null, miss: linesOf(num(weapon['Miss Msg'])) ?? null } : null),
  );

  /** An attack's messages, or undefined while one is read. */
  function textsOf(a: Attack): Texts | undefined {
    const [hit, def, miss] = [linesOf(a.hit), linesOf(a.dodge), linesOf(a.miss)];
    if (hit === undefined || def === undefined || miss === undefined) return undefined;
    return { hit, def, miss };
  }

  const opened = (i: number) => !!draft.opened[`attack:${i}`];
  /** The slots a person works with: an attack, one a blow can pick, or one opened to fill. */
  const live = $derived(attacks.map((a, i) => liveSlot(a, odds.share[i]!) || opened(i)));
  /** Empty slots that still hold an old attack's numbers. */
  const left = $derived(attacks.map((a, i) => !live[i] && holdsNumbers(a)));
  const issues = $derived(attackProblems(attacks, energy));
  const free = $derived.by(() => {
    const clean = attacks.findIndex((_, i) => !live[i] && !left[i]);
    return clean >= 0 ? clean : attacks.findIndex((_, i) => !live[i]);
  });

  function clear(i: number) {
    for (const c of COLS) draft.set(col(c.template, i), '0');
    draft.set(col('Attack Type #', i), '0');
    draft.opened[`attack:${i}`] = false;
  }

  function add() {
    if (free < 0) return;
    draft.set(col('Attack Type #', free), String(NORMAL));
    draft.opened[`attack:${free}`] = true;
  }

  // ---- the lines -------------------------------------------------------------------------

  let view = $state<'you' | 'room'>('you');
  /** Whether a slot reads as lines, so the choice of reader matters. */
  const reads = $derived(attacks.some((a, i) => a.type === NORMAL && live[i]));
  const picked = $derived(odds.share.some((s) => s > 0));


  function ansi(node: HTMLElement, line: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(line);
    return { update: draw };
  }

  const sample = (a: Attack) => Math.max(1, Math.round((a.min + Math.max(a.min, a.max)) / 2));

  // ---- a round ---------------------------------------------------------------------------

  let seed = $state(Math.floor(Math.random() * 2 ** 31));

  /** A small seeded random source, so a round stays as drawn while other fields change. */
  function mulberry(s: number) {
    return () => {
      s = (s + 0x6d2b79f5) | 0;
      let t = Math.imul(s ^ (s >>> 15), 1 | s);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  const round = $derived(roundOf(attacks, energy, mulberry(seed), spellsOf(draft.get)));
  const tally = $derived.by(() => {
    const tries = round.steps.length;
    const blows = round.steps.filter((s) => s.kind === 'blow' || s.kind === 'cast').length;
    const of = `out of ${energy.toLocaleString()} energy`;
    if (blows === tries) return `${blows} ${blows === 1 ? 'blow' : 'blows'} ${of}`;
    return `${tries} tries, ${blows} of them ${blows === 1 ? 'a blow' : 'blows'}, ${of}`;
  });
  const spellName = (n: number) => nameOf('Spells', [n]) || `spell ${n}`;
</script>

<div class="atks">
  {#if picked || reads}
    <div class="odds">
      {#if picked}
        <div class="bar" role="img" aria-label="How often each attack is picked">
          {#each odds.share as s, i (i)}
            {#if s > 0}<span class="seg" class:none={!knownType(attacks[i]!.type)} style:flex-grow={s} style:--c={COLOURS[i]} title="Attack {i + 1}: {s}% of blows"></span>{/if}
          {/each}
          {#if odds.rest > 0}<span class="seg rest" style:flex-grow={odds.rest} title="{odds.rest}%: the attack the last blow used"></span>{/if}
        </div>
      {/if}
      <div class="odds-row">
        {#if picked}
          <ul class="legend">
            {#each odds.share as s, i (i)}
              {#if s > 0}<li><span class="sw" style:--c={COLOURS[i]}></span>Attack {i + 1} <b>{s}%</b></li>{/if}
            {/each}
            {#if odds.rest > 0}<li><span class="sw rest"></span>the last one again <b>{odds.rest}%</b></li>{/if}
          </ul>
        {/if}
        {#if reads}
          <span class="seen" role="group" aria-label="Lines as read by">
            <button type="button" class="btn sm" class:on={view === 'you'} aria-pressed={view === 'you'} onclick={() => (view = 'you')}>To the player hit</button>
            <button type="button" class="btn sm" class:on={view === 'room'} aria-pressed={view === 'room'} onclick={() => (view = 'room')}>To the room</button>
          </span>
        {/if}
      </div>
    </div>
  {/if}

  {#each attacks as a, i (i)}
    {#if live[i]}
      {@const share = odds.share[i]!}
      {@const means = MEANS[a.type] ?? {}}
      {@const texts = a.type === NORMAL ? textsOf(a) : undefined}
      {@const shownCols = COLS.filter((c) => c.template in means || num(draft.get(col(c.template, i))) !== 0)}
      {@const nums = shownCols.filter((c) => !linkTo(c.template, a.type))}
      {@const links = shownCols.filter((c) => !!linkTo(c.template, a.type))}
      <section class="atk" class:off={share === 0} aria-label="Attack {i + 1}">
        <header class="atk-h">
          <span class="sw" style:--c={COLOURS[i]}></span>
          <span class="eyebrow">Attack {i + 1}</span>
          <FieldInput {draft} col={col('Attack Type #', i)} pick={pickFor('Monsters', 'Attack Type #')} label="Attack {i + 1} Type" compact />
          <span class="facts">
            {#if share > 0}<span class="pill">{share}% of blows</span>{/if}
            {#if a.type === NORMAL && share > 0}
              {@const b = blowsOf(energy, a.energy)}
              {#if b > 0}<span class="pill">{b === 6 ? 'up to 6 blows' : b === 1 ? '1 blow' : `${b} blows`} a round</span>{/if}
            {/if}
          </span>
          <button type="button" class="btn icon sm ghost" aria-label="Clear attack {i + 1}" title="Clear this attack" onclick={() => clear(i)}><X size={14} /></button>
        </header>
        {#each issues[i]! as p (p)}
          <p class="callout warn"><TriangleAlert size={15} /><span>{p}</span></p>
        {/each}
        {#if a.type === ROB}
          <p class="faint note">A rob never steals in 1.11p: the blows that pick it strike with attack 1 instead.</p>
        {/if}
        {#if nums.length}
          <div class="nums">
            {#each nums as c (c.template)}
              {@const used = c.template in means}
              <span class="nf" class:unused={!used} title={used ? undefined : `A ${a.type === SPELL ? 'spell' : a.type === ROB ? 'rob' : 'typeless'} attack does not use it`}>
                <span class="lbl">{c.label}{#if used}<small>{means[c.template]}</small>{/if}</span>
                <FieldInput {draft} col={col(c.template, i)} label="Attack {i + 1} {c.label}" compact />
              </span>
            {/each}
          </div>
        {/if}
        {#if links.length}
          <div class="links">
            {#each links as c (c.template)}
              {@const used = c.template in means}
              <span class="pair" class:unused={!used} title={used ? undefined : `A ${a.type === SPELL ? 'spell' : a.type === ROB ? 'rob' : 'typeless'} attack does not use it`}>
                <span class="lbl">{c.label}{#if used}<small>{means[c.template]}</small>{/if}</span>
                <FieldInput {draft} col={col(c.template, i)} target={linkTo(c.template, a.type)} label="Attack {i + 1} {c.label}" compact />
              </span>
            {/each}
          </div>
        {/if}
        {#if a.type === NORMAL}
          {#if texts}
            <div class="term" aria-label="What each outcome reads as">
              {#each OUTCOMES as [o, label] (o)}
                {@const l = blowLines(mon, verbs, texts, SAMPLE_VICTIM, o, sample(a))}
                {@const line = view === 'you' ? l.you : l.room}
                <div class="tl">
                  <span class="why">{label}</span>
                  {#if blankLine(line)}<span class="say aside">(an empty line)</span>{:else}<span class="say" use:ansi={line}></span>{/if}
                </div>
              {/each}
            </div>
          {:else}
            <p class="faint note">Reading its messages…</p>
          {/if}
        {:else if a.type === SPELL && a.acc}
          <p class="faint note">Casts {spellName(a.acc)} at level {a.max}, {a.min}% of the time; the spell's own messages say how it went.</p>
        {/if}
      </section>
    {:else if left[i]}
      {@const msgs = [a.hit, a.dodge, a.miss].filter((m) => m)}
      <div class="leftover">
        <span class="eyebrow">Attack {i + 1}</span>
        <span class="faint">
          {a.type ? `Not used (type ${a.type} is no attack type, and no blow picks it)` : 'Empty'}, but it still holds an old attack's numbers{msgs.length ? ` (${msgs.length === 1 ? 'message' : 'messages'} ${msgs.join(', ')})` : ''}.
        </span>
        <button type="button" class="btn sm" onclick={() => clear(i)}><WandSparkles size={14} /> Clear</button>
      </div>
    {/if}
  {/each}

  {#if free >= 0}
    <div class="row"><button type="button" class="btn sm" onclick={add}><Plus size={14} /> Add an attack</button></div>
  {/if}

  {#if attacks.some((a) => a.type !== 0)}
    <div class="round">
      <div class="round-h">
        <span class="eyebrow">A round</span>
        <span class="faint">{tally}</span>
      </div>
      <div class="term" aria-live="polite">
        {#each round.steps as s, k (k)}
          {#if s.kind === 'blow'}
            {@const a = attacks[s.slot]!}
            {@const t = textsOf(a)}
            {#if t}
              {@const l = blowLines(mon, verbs, t, SAMPLE_VICTIM, s.outcome, s.damage)}
              {@const line = view === 'you' ? l.you : l.room}
              <div class="tl">
                <span class="why" style:--c={COLOURS[s.slot]}>{s.outcome}</span>
                {#if blankLine(line)}<span class="say aside">(an empty line)</span>{:else}<span class="say" use:ansi={line}></span>{/if}
              </div>
              {#if s.spell}<div class="tl"><span class="why">spell</span><span class="say aside">and casts {spellName(s.spell)}</span></div>{/if}
            {/if}
          {:else if s.kind === 'cast'}
            <div class="tl"><span class="why" style:--c={COLOURS[s.slot]}>spell</span><span class="say aside">casts {spellName(s.spell)} ({s.chance}% to cast, level {s.level})</span></div>
          {:else if s.kind === 'tired'}
            <div class="tl"><span class="why">spell</span><span class="say aside">too tired to cast {spellName(attacks[s.slot]!.acc)}</span></div>
          {:else}
            <div class="tl"><span class="why">nothing</span><span class="say aside">attack {s.slot + 1} {s.rob ? 'is a rob, and attack 1 is no normal attack' : 'has no type'}: nothing happens</span></div>
          {/if}
        {:else}
          <div class="tl"><span class="why">no blow</span><span class="say aside">the attack it picked costs more than its {energy.toLocaleString()} energy</span></div>
        {/each}
        {#if round.after}
          <div class="tl"><span class="why">after</span><span class="say aside">casts {spellName(round.after.spell)} at level {round.after.level}</span></div>
        {/if}
      </div>
      <div class="row">
        <button type="button" class="btn sm" onclick={() => (seed = Math.floor(Math.random() * 2 ** 31))}><Dices size={14} /> Roll another round</button>
        <span class="faint small">The attack each blow uses and how many its energy buys are the game's. Hits and misses are drawn at random here: in the game they depend on who it fights.</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .atks { display: grid; gap: 12px; }
  .odds { display: grid; gap: 8px; }
  .bar { display: flex; gap: 2px; height: 14px; }
  .seg { background: var(--c); border-radius: 4px; min-width: 4px; }
  .seg.none { background: repeating-linear-gradient(45deg, var(--c) 0 3px, transparent 3px 6px); }
  .seg.rest, .sw.rest { background: repeating-linear-gradient(135deg, var(--line-2) 0 3px, transparent 3px 6px); }
  .odds-row { display: flex; align-items: center; gap: 8px 14px; flex-wrap: wrap; }
  .legend { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px 14px; font-size: 12.5px; color: var(--ink-2); font-weight: 700; }
  .legend li { display: inline-flex; align-items: center; gap: 6px; }
  .legend b { color: var(--ink); }
  .sw { width: 10px; height: 10px; border-radius: 3px; background: var(--c); flex: none; display: inline-block; }
  .seen { display: inline-flex; gap: 4px; margin-left: auto; }
  .seen .on { background: var(--lilac-2); color: var(--lilac-ink); border-color: transparent; }
  .atk { border: 1px solid var(--line); border-radius: var(--r); padding: 8px 10px 10px; background: var(--surface-2); display: grid; gap: 8px; }
  .atk.off { border-style: dashed; }
  .atk-h { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .atk-h .facts { display: inline-flex; gap: 6px; flex-wrap: wrap; margin-left: auto; }
  .callout { margin: 0; }
  .note { margin: 0; font-size: 12.5px; font-weight: 700; }
  .lbl { color: var(--ink-2); font-size: 12px; font-weight: 800; display: grid; line-height: 1.15; }
  .lbl small { color: var(--ink-3); font-weight: 700; font-size: 11px; }
  .nums { display: flex; flex-wrap: wrap; align-items: end; gap: 8px 12px; }
  .nf { display: grid; gap: 3px; }
  .links { display: grid; gap: 6px; }
  .pair { display: grid; grid-template-columns: 6.6rem minmax(0, 1fr); align-items: center; gap: 8px; }
  .unused { opacity: .6; }
  .term { background: var(--term-bg); color: var(--term-fg); border: var(--stroke) solid var(--term-frame); border-radius: var(--r); padding: 8px 10px; display: grid; gap: 3px; font: 13px/1.3 var(--font-mono); overflow-x: auto; }
  .tl { display: grid; grid-template-columns: 4.6rem minmax(0, 1fr); gap: 8px; align-items: baseline; }
  .why { font: 800 10.5px/1.3 var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: #9b97c4; display: inline-flex; align-items: center; gap: 5px; }
  .why[style]::before { content: ''; width: 7px; height: 7px; border-radius: 2px; background: var(--c); }
  .say { white-space: pre-wrap; overflow-wrap: anywhere; }
  .say.aside { color: #9b97c4; font-style: italic; }
  .leftover { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; font-size: 12.5px; font-weight: 700; padding: 6px 10px; border: 1px dashed var(--line); border-radius: var(--r); }
  .leftover .faint { flex: 1 1 14rem; }
  .row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .round { display: grid; gap: 8px; border-top: 1px dashed var(--line); padding-top: 12px; }
  .round-h { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
  .round-h .faint, .small { font-size: 12px; font-weight: 700; }
  @container (max-width: 420px) {
    .pair { grid-template-columns: minmax(0, 1fr); gap: 2px; }
    .tl { grid-template-columns: minmax(0, 1fr); gap: 0; }
    .seen { margin-left: 0; }
  }
</style>
