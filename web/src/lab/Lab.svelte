<!--
  The Combat lab: a character made from the world's own classes, races and items meets one of
  its monsters, and they fight as 1.11p fights (fight.ts, rules.ts). Both sides' numbers stand
  side by side; a round rolls at a press, with the lines the player reads; a fight plays to the
  end; a thousand fights give the odds. Nothing here writes to the world.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Dices from '@lucide/svelte/icons/dices';
  import FlaskConical from '@lucide/svelte/icons/flask-conical';
  import Heart from '@lucide/svelte/icons/heart';
  import Info from '@lucide/svelte/icons/info';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Shield from '@lucide/svelte/icons/shield';
  import Skull from '@lucide/svelte/icons/skull';
  import Swords from '@lucide/svelte/icons/swords';
  import Trophy from '@lucide/svelte/icons/trophy';
  import X from '@lucide/svelte/icons/x';
  import { titleOf, renderAnsi } from '../ansi';
  import { api, type Cell } from '../api';
  import { WORN } from '../layouts';
  import { recordChoices } from '../library/choices';
  import type { Lines } from '../library/combat';
  import Picker from '../library/Picker.svelte';
  import { readRecord } from '../library/records.svelte';
  import type { Choice } from '../library/sections';
  import Seg from '../workshop/Seg.svelte';
  import {
    averageRandomHp,
    classOf,
    itemOf,
    kitOf,
    monsterOf,
    movesOf,
    oddsOf,
    raceOf,
    roundOf,
    sheetOf,
    spellOf,
    startOf,
    type Fight,
    type Line,
    type Odds,
    type Player,
    type State,
  } from './fight';
  import { effectiveCrit, expNeeded, seeded, type ItemRec, type MonsterRec, type SpellRec } from './rules';

  let { start = [] }: { start?: string[] } = $props();

  // ---- the character, kept in this browser -------------------------------------------------

  const KEY = 'dd-lab';
  const STATS = [
    ['str', 'STR', 'Min STR'],
    ['intel', 'INT', 'Min INT'],
    ['wil', 'WIL', 'Min WIL'],
    ['agi', 'AGL', 'Min AGL'],
    ['hea', 'HEA', 'Min HEA'],
    ['cha', 'CHM', 'Min CHM'],
  ] as const;
  type Stat = (typeof STATS)[number][0];
  type Align = 'good' | 'neutral' | 'evil';
  interface Saved {
    name: string;
    cls: number;
    race: number;
    level: number;
    stats: Record<Stat, number>;
    /** Hit points rolled in training; null for the average. */
    randomhp: number | null;
    encumpct: number;
    align: Align;
    weapon: number;
    armour: number[];
    move: number;
    monster: number;
  }
  const DEFAULT: Saved = {
    name: 'Rowan',
    cls: 1,
    race: 1,
    level: 10,
    stats: { str: 80, intel: 50, wil: 50, agi: 70, hea: 75, cha: 50 },
    randomhp: null,
    encumpct: 20,
    align: 'neutral',
    weapon: 64,
    armour: [],
    move: 5,
    monster: 1,
  };
  function load(): Saved {
    try {
      const s = JSON.parse(localStorage.getItem(KEY) ?? 'null');
      if (s && typeof s === 'object') return { ...DEFAULT, ...s, stats: { ...DEFAULT.stats, ...s.stats } };
    } catch {
      /* a private window, or nothing kept */
    }
    return structuredClone(DEFAULT);
  }
  /** The character kept, facing the monster the link names (`#/workshop/fight/381`). */
  function first(): Saved {
    const s = load();
    const routed = Number(start[0]);
    if (routed > 0) s.monster = routed;
    return s;
  }
  const me = $state<Saved>(first());
  $effect(() => {
    const s = JSON.stringify(me);
    try {
      localStorage.setItem(KEY, s);
    } catch {
      /* not kept: fine */
    }
  });

  const EVIL: Record<Align, number> = { good: -100, neutral: 0, evil: 100 };

  // ---- the records it reads ----------------------------------------------------------------

  type Values = Record<string, Cell>;
  const getter = (v: Values) => (c: string) => v[c];
  const text = (v: Cell | undefined) => (v === null || v === undefined ? '' : titleOf(String(v)));

  let itemTypes = $state<Map<number, [number, string, number]> | null>(null);
  void api.rowsWith('Items', ['Type', 'Worn On']).then(({ rows }) => {
    itemTypes = new Map(rows.map(([k, t, v]) => [Number(k[0]), [Number(v[0]), text(t), Number(v[1])]]));
  });
  const itemChoices = (type: number) => (): Choice[] =>
    [...(itemTypes ?? new Map()).entries()].filter(([, [t]]) => t === type).map(([n, [, name]]) => ({ value: n, label: name }));

  let clsV = $state<Values | null>(null);
  let raceV = $state<Values | null>(null);
  let weapon = $state<ItemRec | null>(null);
  let armour = $state<ItemRec[]>([]);
  let monsterV = $state<Values | null>(null);
  let monster = $state<MonsterRec | null>(null);
  let classes = $state<Choice[]>([]);
  let races = $state<Choice[]>([]);
  void recordChoices('Classes').then((c) => (classes = c));
  void recordChoices('Races').then((c) => (races = c));

  $effect(() => {
    void readRecord('Classes', [me.cls]).then((v) => (clsV = v));
  });
  $effect(() => {
    void readRecord('Races', [me.race]).then((v) => (raceV = v));
  });
  $effect(() => {
    const n = me.weapon;
    if (!n) weapon = null;
    else void readRecord('Items', [n]).then((v) => (weapon = v ? itemOf(getter(v)) : null));
  });
  $effect(() => {
    const ns = [...me.armour];
    void Promise.all(ns.map((n) => readRecord('Items', [n]))).then((vs) => (armour = vs.flatMap((v) => (v ? [itemOf(getter(v))] : []))));
  });

  // the monster, and everything its fight reads: messages, spells, its weapon
  const lines = new Map<number, Lines | null>();
  const spells = new Map<number, SpellRec | null>();
  const weapons = new Map<number, { name: string; hit: Lines | null; miss: Lines | null } | null>();
  let wordsFor = $state(0);
  const linesFrom = (v: Values | null): Lines | null => (v ? [text(v['Line 1']), text(v['Line 2']), text(v['Line 3'])] : null);
  async function readLines(ns: number[]) {
    await Promise.all(ns.filter((n) => n && !lines.has(n)).map(async (n) => lines.set(n, linesFrom(await readRecord('Messages', [n]).catch(() => null)))));
  }
  async function readSpell(n: number) {
    if (!n || spells.has(n)) return;
    const v = await readRecord('Spells', [n]).catch(() => null);
    const s = v ? spellOf(getter(v)) : null;
    spells.set(n, s);
    if (s?.msg) await readLines([s.msg]);
  }
  $effect(() => {
    const n = me.monster;
    void (async () => {
      const v = await readRecord('Monsters', [n]).catch(() => null);
      if (n !== me.monster) return;
      monsterV = v;
      if (!v) {
        monster = null;
        return;
      }
      const wn = Number(v['Weapon Number']) || 0;
      const wv = wn ? await readRecord('Items', [wn]).catch(() => null) : null;
      const w = wv ? itemOf(getter(wv)) : null;
      const m = monsterOf(getter(v), w);
      const msgs: number[] = [];
      if (w) {
        msgs.push(w.hitmsg, w.missmsg);
      }
      for (const a of m.attacks) msgs.push(a.hitmsg, a.defmsg, a.missmsg);
      await readLines(msgs);
      if (w) weapons.set(wn, { name: w.name, hit: lines.get(w.hitmsg) ?? null, miss: lines.get(w.missmsg) ?? null });
      await Promise.all([...m.attacks.filter((a) => a.type === 2).map((a) => a.acc), ...m.attacks.map((a) => a.hitspell), ...m.spells.map((s) => s.spell)].map(readSpell));
      if (n === me.monster) {
        monster = m;
        wordsFor++;
      }
    })();
  });
  $effect(() => {
    const w = weapon;
    if (w) void readLines([w.hitmsg, w.missmsg]).then(() => wordsFor++);
  });

  // ---- the fight ---------------------------------------------------------------------------

  const cls = $derived(clsV ? classOf(getter(clsV)) : null);
  const race = $derived(raceV ? raceOf(getter(raceV)) : null);
  const player = $derived.by((): Player | null => {
    if (!cls || !race) return null;
    return {
      name: me.name.trim() || 'Rowan',
      c: { level: me.level, ...me.stats, randomhp: me.randomhp ?? averageRandomHp(me.level, cls.hplevel), encumpct: me.encumpct, evil: EVIL[me.align] },
      cls,
      race,
      weapon,
      armour,
      move: me.move,
    };
  });
  const moves = $derived(player ? movesOf(player) : []);
  $effect(() => {
    if (moves.length && !moves.some((m) => m.move === me.move)) me.move = 5;
  });
  const fight = $derived.by((): Fight | null => {
    void wordsFor;
    if (!player || !monster) return null;
    return { player, monster, world: { lines: (n) => lines.get(n) ?? null, spell: (n) => spells.get(n) ?? null, weapon: (n) => weapons.get(n) ?? null } };
  });
  const sheet = $derived(fight ? sheetOf(fight.player, fight.monster) : null);
  const kit = $derived(player ? kitOf(player) : null);

  let seed = Math.floor(Math.random() * 2 ** 31);
  let rng = seeded(seed);
  let now = $state<State | null>(null);
  let log = $state<{ round: number; lines: Line[] }[]>([]);
  let odds = $state<Odds | null>(null);
  let oddsFor = $state('');
  let running = $state(false);
  const signature = $derived(fight ? JSON.stringify([fight.player, fight.monster.number]) : '');

  // a new matchup starts over
  $effect(() => {
    void signature;
    now = fight ? startOf(fight) : null;
    log = [];
  });

  const over = $derived(!!now && (now.hp <= 0 || now.monHp <= 0));
  function roll() {
    if (!fight || !now || over) return;
    const r = roundOf(fight, now, rng);
    now = r.state;
    log = [...log, { round: r.state.round, lines: r.lines }];
  }
  function toTheEnd() {
    if (!fight || !now) return;
    let st = now;
    const more: typeof log = [];
    while (st.hp > 0 && st.monHp > 0 && st.round < 200) {
      const r = roundOf(fight, st, rng);
      st = r.state;
      more.push({ round: st.round, lines: r.lines });
    }
    now = st;
    log = [...log, ...more];
  }
  function again() {
    if (!fight) return;
    seed = Math.floor(Math.random() * 2 ** 31);
    rng = seeded(seed);
    now = startOf(fight);
    log = [];
  }
  function runOdds() {
    if (!fight) return;
    running = true;
    const f = fight;
    const sig = signature;
    setTimeout(() => {
      odds = oddsOf(f, 1000, seed);
      oddsFor = sig;
      running = false;
    }, 20);
  }
  const freshOdds = $derived(odds && oddsFor === signature ? odds : null);

  let term = $state<HTMLDivElement | undefined>();
  $effect(() => {
    void log.length;
    if (term) term.scrollTop = term.scrollHeight;
  });

  function ansi(node: HTMLElement, line: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(line);
    return { update: draw };
  }

  // ---- what the page shows ----------------------------------------------------------------

  const pct = (n: number) => `${Math.round(n)}%`;
  const fmt = (n: number) => n.toLocaleString('en-US');
  const fmt1 = (n: number) => n.toLocaleString('en-US', { maximumFractionDigits: 1 });
  const nameOfItem = (n: number) => itemTypes?.get(n)?.[1] || `item ${n}`;
  const wornOn = (n: number) => (WORN[itemTypes?.get(n)?.[2] ?? -1] ?? '').toLowerCase();
  const raceMin = (col: string) => Number(raceV?.[col]) || 0;
  const raceMax = (col: string) => Number(raceV?.[col.replace('Min', 'Max')]) || 255;
  const expMod = $derived((cls?.exp ?? 0) + (race?.exp ?? 0));
  const expRows = $derived(Array.from({ length: 6 }, (_, i) => me.level + i).map((l) => ({ level: l, need: expNeeded(l, expMod), next: expNeeded(l + 1, expMod) - expNeeded(l, expMod) })));
  const ATTACK_TYPE = ['', 'normal', 'spell', 'rob'];
  const killsIn = (hp: number, dmg: number) => (dmg > 0 ? Math.ceil(hp / dmg) : Infinity);
  /** Hit points of each side as a share of its most. */
  const youShare = $derived(now && sheet ? Math.max(0, now.hp) / Math.max(1, sheet.you.maxhp) : 1);
  const monShare = $derived(now && monster ? Math.max(0, now.monHp) / Math.max(1, monster.hp) : 1);
</script>

<div class="lab">
  <div class="sides">
    <!-- the character -->
    <section class="card side you" aria-labelledby="lab-you">
      <header class="side-h">
        <span class="badge peach"><Heart size={18} /></span>
        <div>
          <h2 id="lab-you">Your character</h2>
          <p class="faint">Made from this world’s own classes, races and items.</p>
        </div>
      </header>
      <div class="grid2">
        <label class="f"><span>Name</span><input bind:value={me.name} maxlength="20" /></label>
        <label class="f"><span>Level</span><input type="number" min="1" max="255" bind:value={me.level} class="num" /></label>
        <label class="f">
          <span>Class</span>
          <select bind:value={me.cls}>{#each classes as c (c.value)}<option value={c.value}>{c.label}</option>{/each}</select>
        </label>
        <label class="f">
          <span>Race</span>
          <select bind:value={me.race}>{#each races as c (c.value)}<option value={c.value}>{c.label}</option>{/each}</select>
        </label>
      </div>
      <div class="stats" role="group" aria-label="Stats">
        {#each STATS as [k, label, min] (k)}
          <label class="stat" title="{label}: {raceMin(min)} to {raceMax(min)} for this race at the start">
            <span>{label}</span>
            <input type="number" min="0" max="999" bind:value={me.stats[k]} class="num" />
          </label>
        {/each}
      </div>
      <p class="hint">Stats as they stand, with what worn items add. This race starts between {raceMin('Min STR')} and {raceMax('Min STR')} strength.</p>

      <div class="kit">
        <div class="kit-row">
          <span class="lbl">Weapon</span>
          {#if me.weapon}
            <a class="chip" href="#/library/items/{me.weapon}">{nameOfItem(me.weapon)}</a>
            <button type="button" class="btn icon sm ghost" aria-label="Fight with fists" title="Fight with fists" onclick={() => (me.weapon = 0)}><X size={14} /></button>
          {:else}<span class="faint">fists</span>{/if}
          <Picker what="a weapon" label={me.weapon ? 'Change' : 'Pick'} choices={itemChoices(1)} onpick={(v) => (me.weapon = v)} />
        </div>
        <div class="kit-row">
          <span class="lbl">Armour</span>
          {#each me.armour as a, i (a)}
            <span class="chip">
              <a href="#/library/items/{a}">{nameOfItem(a)}</a><small>{wornOn(a)}</small>
              <button type="button" class="x" aria-label="Take off {nameOfItem(a)}" onclick={() => me.armour.splice(i, 1)}><X size={12} /></button>
            </span>
          {:else}<span class="faint">none</span>{/each}
          <Picker what="armour" label="Wear" choices={itemChoices(0)} onpick={(v) => !me.armour.includes(v) && me.armour.push(v)} />
        </div>
        {#if moves.length > 1}
          <div class="kit-row">
            <span class="lbl">Fights with</span>
            <select bind:value={me.move} aria-label="Fights with">{#each moves as m (m.move)}<option value={m.move}>{m.name}</option>{/each}</select>
          </div>
        {/if}
        <div class="kit-row">
          <span class="lbl">Carrying</span>
          <input type="range" min="0" max="100" step="1" bind:value={me.encumpct} aria-label="Encumbrance, percent" class="slide" />
          <span class="tnum small">{me.encumpct}% of the most</span>
        </div>
        <div class="kit-row">
          <span class="lbl">Alignment</span>
          <Seg label="Alignment" options={[['good', 'Good'], ['neutral', 'Neutral'], ['evil', 'Evil']]} bind:value={me.align} />
        </div>
        <div class="kit-row">
          <span class="lbl">Rolled HP</span>
          <label class="check"><input type="checkbox" checked={me.randomhp === null} onchange={(e) => (me.randomhp = e.currentTarget.checked ? null : averageRandomHp(me.level, cls?.hplevel ?? 0))} /> the average</label>
          {#if me.randomhp !== null}<input type="number" class="num" min="0" bind:value={me.randomhp} aria-label="Hit points rolled in training" />{/if}
          <span class="faint small">0 to {cls?.hplevel ?? 0} a level after the first</span>
        </div>
      </div>
    </section>

    <!-- the monster -->
    <section class="card side mon" aria-labelledby="lab-mon">
      <header class="side-h">
        <span class="badge lilac"><Skull size={18} /></span>
        <div>
          <h2 id="lab-mon">
            {#if monsterV}<a href="#/library/monsters/{me.monster}">{text(monsterV['Name']) || `monster ${me.monster}`}</a>{:else}A monster{/if}
          </h2>
          <p class="faint">Monster {me.monster}{#if monster}: {fmt(monster.hp)} HP, {fmt(monster.exp * monster.level)} exp{/if}</p>
        </div>
        <span class="push"><Picker what="a monster" label="Change" choices={() => recordChoices('Monsters')} onpick={(v) => (me.monster = v)} /></span>
      </header>
      {#if monster === null && monsterV === null}
        <p class="callout warn">The world has no monster {me.monster}. Pick another.</p>
      {:else if sheet}
        <div class="nums">
          <span><b>{sheet.mon.dv}</b>AC</span>
          <span><b>{fmt1(sheet.mon.dr / 10)}</b>DR</span>
          <span><b>{sheet.mon.mr}</b>MR</span>
          <span><b>{sheet.mon.dodge}</b>Dodge</span>
          <span><b>{fmt(monster?.energy ?? 0)}</b>Energy</span>
        </div>
        <ul class="atks">
          {#each sheet.mon.attacks as a (a.slot)}
            <li>
              <span class="eyebrow">Attack {a.slot + 1}</span>
              <span class="pill">{a.share}% of blows</span>
              {#if a.type === 1}
                <span>hits you <b>{pct(a.hit)}</b>{#if a.dodge}, you dodge <b>{pct(a.dodge)}</b>{/if}</span>
                <span class="faint">{a.min}–{a.max}, {a.blows} a round</span>
              {:else if a.type === 2}
                {@const s = spells.get(monster?.attacks[a.slot]?.acc ?? 0)}
                <span>casts <b>{s?.name || `spell ${monster?.attacks[a.slot]?.acc}`}</b></span>
                <span class="faint">{monster?.attacks[a.slot]?.min}% to cast, level {monster?.attacks[a.slot]?.max}</span>
              {:else}
                <span class="faint">a {ATTACK_TYPE[a.type] || 'typeless'} attack</span>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="faint">Reading…</p>
      {/if}
    </section>
  </div>

  {#if sheet && kit && fight}
    <!-- the tale of the tape -->
    <section class="card tape" aria-label="Both sides’ numbers">
      <div class="tape-row">
        <div class="t">
          <span class="eyebrow">You hit it</span>
          <b class="big">{sheet.you.canHit ? pct(sheet.you.hit) : '—'}</b>
          <span class="faint">{#if !sheet.you.canHit}your blows have no effect on it{:else if sheet.you.monDodge}and it dodges {pct(sheet.you.monDodge)}{:else}of your blows{/if}</span>
        </div>
        <div class="t">
          <span class="eyebrow">Blows a round</span>
          <b class="big">{fmt1(sheet.you.blows)}</b>
          <span class="faint">{fmt(sheet.you.energy)} energy each</span>
        </div>
        <div class="t">
          <span class="eyebrow">Damage</span>
          <b class="big">{sheet.you.mindmg}–{sheet.you.maxdmg}</b>
          <span class="faint">less its {fmt1(sheet.mon.dr / 10)} DR</span>
        </div>
        <div class="t">
          <span class="eyebrow">Critical</span>
          <b class="big">{pct(effectiveCrit(sheet.you.crit))}</b>
          <span class="faint">{sheet.you.fast ? 'with the fast-blow bonus' : 'of hits'}</span>
        </div>
      </div>
      <div class="sheet">
        <span><b>{fmt(sheet.you.maxhp)}</b>HP</span>
        <span><b>{sheet.you.av}</b>AV</span>
        <span><b>{sheet.you.dv}</b>DV</span>
        <span><b>{fmt1(sheet.you.dr / 10)}</b>DR</span>
        <span><b>{sheet.you.dodge}</b>Dodge</span>
        <span><b>{sheet.you.mr}</b>MR</span>
        <span><b>{fmt(expNeeded(me.level + 1, expMod) - expNeeded(me.level, expMod))}</b>exp to {me.level + 1}</span>
      </div>
    </section>

    <!-- the arena -->
    <section class="card arena" aria-label="The fight">
      <div class="bars">
        <div class="bar-side">
          <span class="who"><Heart size={15} />{fight.player.name}</span>
          <span class="hp" role="meter" aria-label="{fight.player.name}’s hit points" aria-valuemin="0" aria-valuemax={sheet.you.maxhp} aria-valuenow={Math.max(0, now?.hp ?? 0)}>
            <span class="fill you-fill" style:width="{youShare * 100}%"></span>
          </span>
          <span class="tnum small">{fmt(Math.max(0, now?.hp ?? 0))} / {fmt(sheet.you.maxhp)}</span>
        </div>
        <span class="vs" aria-hidden="true"><Swords size={20} /></span>
        <div class="bar-side right">
          <span class="who"><Skull size={15} />{fight.monster.name}</span>
          <span class="hp" role="meter" aria-label="{fight.monster.name}’s hit points" aria-valuemin="0" aria-valuemax={fight.monster.hp} aria-valuenow={Math.max(0, now?.monHp ?? 0)}>
            <span class="fill mon-fill" style:width="{monShare * 100}%"></span>
          </span>
          <span class="tnum small">{fmt(Math.max(0, now?.monHp ?? 0))} / {fmt(fight.monster.hp)}</span>
        </div>
      </div>

      <div class="term" bind:this={term} aria-live="polite">
        {#each log as r (r.round)}
          <div class="rh">Round {r.round}</div>
          {#each r.lines as l, k (k)}
            <div class="tl {l.who}"><span class="say" use:ansi={l.text}></span>{#if l.note}<span class="aside"> ({l.note})</span>{/if}</div>
          {/each}
        {:else}
          <p class="aside">Roll a round to see it play, as {fight.player.name} reads it.</p>
        {/each}
      </div>

      <div class="row">
        <button type="button" class="btn sticker" onclick={roll} disabled={over}><Dices size={16} /> Roll a round</button>
        <button type="button" class="btn" onclick={toTheEnd} disabled={over}><Swords size={16} /> Fight to the end</button>
        <button type="button" class="btn ghost" onclick={again} disabled={!log.length}><RotateCcw size={15} /> Start over</button>
        {#if over && now}
          <span class="pill {now.monHp <= 0 ? 'ok' : 'bad'}">
            {#if now.monHp <= 0}<Trophy size={13} /> Won in {now.round} {now.round === 1 ? 'round' : 'rounds'}{:else}<Skull size={13} /> Lost in {now.round} {now.round === 1 ? 'round' : 'rounds'}{/if}
          </span>
        {/if}
      </div>
    </section>

    <!-- the odds -->
    <section class="card odds" aria-labelledby="lab-odds">
      <header class="side-h">
        <span class="badge mint"><FlaskConical size={18} /></span>
        <div>
          <h2 id="lab-odds">A thousand fights</h2>
          <p class="faint">Each to the end, from full hit points, without resting between.</p>
        </div>
        <button type="button" class="btn sm push" onclick={runOdds} disabled={running}>{running ? 'Fighting…' : freshOdds ? 'Again' : 'Run them'}</button>
      </header>
      {#if freshOdds}
        {@const o = freshOdds}
        <div class="tape-row">
          <div class="t">
            <span class="eyebrow">Wins</span>
            <b class="big" class:good={o.wins / o.fights >= 0.9} class:bad={o.wins / o.fights < 0.5}>{pct((o.wins / o.fights) * 100)}</b>
            <span class="faint">{fmt(o.wins)} of {fmt(o.fights)}{#if o.stalled}, {o.stalled} still going at 200 rounds{/if}</span>
          </div>
          <div class="t">
            <span class="eyebrow">Rounds a fight</span>
            <b class="big">{fmt1(o.rounds)}</b>
            <span class="faint">on average</span>
          </div>
          <div class="t">
            <span class="eyebrow">You deal</span>
            <b class="big">{fmt1(o.dealt)}</b>
            <span class="faint">a round{#if o.dealt > 0}: {killsIn(fight.monster.hp, o.dealt)} rounds to kill it{/if}</span>
          </div>
          <div class="t">
            <span class="eyebrow">You take</span>
            <b class="big">{fmt1(o.taken)}</b>
            <span class="faint">a round{#if o.wins}, {fmt(Math.round(o.left))} HP left after a win{/if}</span>
          </div>
        </div>
      {:else}
        <p class="faint">How often {fight.player.name} wins, how long it takes, and what each side does a round.</p>
      {/if}
    </section>

    <!-- the calculators -->
    <section class="card calc" id="lab-calc" aria-labelledby="lab-calc-h">
      <header class="side-h">
        <span class="badge butter"><Shield size={18} /></span>
        <div>
          <h2 id="lab-calc-h">Experience to level</h2>
          <p class="faint">For a {race && raceV ? text(raceV['Name']) : 'race'} {clsV ? text(clsV['Name']) : 'class'}: {expMod >= 0 ? '+' : ''}{expMod}% on the table.</p>
        </div>
      </header>
      <table class="exp">
        <thead><tr><th>Level</th><th>Experience to be it</th><th>To the next</th></tr></thead>
        <tbody>
          {#each expRows as r (r.level)}
            <tr class:now={r.level === me.level}><td class="tnum">{r.level}</td><td class="tnum">{fmt(r.need)}</td><td class="tnum">{fmt(r.next)}</td></tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/if}

  <p class="callout info left-out">
    <Info size={16} />
    <span>
      The numbers are 1.11p’s, as silvermere plays them. Left out: resting and regeneration between rounds, a smash knocking the monster down, damage shields,
      a weapon’s own spells, what a spell does besides damage, and groups. An area spell is played as if aimed at you alone. Nothing here changes the world.
    </span>
  </p>
</div>

<style>
  .lab { display: grid; gap: 14px; container-type: inline-size; }
  .sides { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 22rem), 1fr)); gap: 14px; align-items: start; }
  .side { display: grid; gap: 12px; }
  .side-h { display: flex; align-items: center; gap: 10px; }
  .side-h h2 { margin: 0; font-size: 17px; }
  .side-h h2 a { color: inherit; }
  .side-h p { margin: 2px 0 0; font-size: 12.5px; font-weight: 700; }
  .push { margin-left: auto; }
  .badge { width: 34px; height: 34px; border-radius: 11px; display: grid; place-items: center; flex: none; border: var(--stroke) solid var(--outline); }
  .badge.peach { background: var(--peach-2); color: var(--peach-ink); }
  .badge.lilac { background: var(--lilac-2); color: var(--lilac-ink); }
  .badge.mint { background: var(--mint-2); color: var(--mint-ink); }
  .badge.butter { background: var(--butter-2); color: var(--butter-ink); }
  .grid2 { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px 10px; }
  .f { display: grid; gap: 3px; font: 800 12px var(--font-ui); color: var(--ink-2); }
  .f input, .f select { width: 100%; }
  .num { font-family: var(--font-mono); }
  .stats { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 6px; }
  @container (max-width: 520px) { .stats { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
  .stat { display: grid; gap: 2px; text-align: center; font: 800 11px var(--font-ui); letter-spacing: .06em; color: var(--ink-3); }
  .stat input { text-align: center; width: 100%; font-size: 15px; font-weight: 700; padding-inline: 2px; }
  .hint { margin: -4px 0 0; font-size: 12px; font-weight: 700; color: var(--ink-3); }
  .kit { display: grid; gap: 8px; }
  .kit-row { display: flex; align-items: center; gap: 6px 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 6.5em; }
  .chip { display: inline-flex; align-items: center; gap: 5px; padding: 3px 8px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); font-size: 13px; font-weight: 700; color: var(--ink); }
  .chip a { color: inherit; }
  .chip small { color: var(--ink-3); font-weight: 700; }
  .chip .x { border: 0; background: none; padding: 0; min-height: 0; width: 16px; height: 16px; display: grid; place-items: center; box-shadow: none; color: var(--ink-3); }
  .slide { width: min(180px, 100%); accent-color: var(--peach-ink); padding: 0; border: 0; background: none; }
  .check { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; font-size: 13px; }
  .small { font-size: 12px; font-weight: 700; }
  .nums, .sheet { display: flex; flex-wrap: wrap; gap: 6px; }
  .nums span, .sheet span { display: inline-flex; align-items: baseline; gap: 5px; padding: 4px 10px; border-radius: 10px; background: var(--surface-2); font: 800 11px var(--font-ui); letter-spacing: .05em; color: var(--ink-3); text-transform: uppercase; }
  .nums b, .sheet b { font: 700 15px var(--font-mono); color: var(--ink); letter-spacing: 0; text-transform: none; }
  .atks { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .atks li { display: flex; flex-wrap: wrap; align-items: baseline; gap: 4px 10px; padding: 7px 10px; border: 1px solid var(--line); border-radius: var(--r); background: var(--surface-2); font-size: 13px; font-weight: 700; }
  .tape { display: grid; gap: 12px; background: linear-gradient(135deg, var(--peach-2), var(--surface) 45%, var(--lilac-2)); }
  .tape-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(9.5rem, 1fr)); gap: 10px; }
  .t { display: grid; gap: 2px; padding: 10px 12px; border-radius: var(--r); background: var(--surface); border: 1px solid var(--line); }
  .t .faint { font-size: 12px; font-weight: 700; }
  .big { font: 700 26px/1.1 var(--font-display); color: var(--ink); }
  .big.good { color: var(--ok); }
  .big.bad { color: var(--bad); }
  .arena { display: grid; gap: 12px; }
  .bars { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); align-items: center; gap: 12px; }
  .bar-side { display: grid; gap: 4px; }
  .bar-side.right { text-align: right; }
  .bar-side.right .who { justify-content: flex-end; }
  .who { display: inline-flex; align-items: center; gap: 6px; font: 800 14px var(--font-ui); overflow-wrap: anywhere; }
  .hp { display: block; height: 14px; border-radius: 999px; background: var(--surface-3); border: var(--stroke) solid var(--outline); overflow: hidden; }
  .right .hp { transform: scaleX(-1); }
  .fill { display: block; height: 100%; transition: width .35s var(--ease); }
  .you-fill { background: linear-gradient(90deg, var(--peach), var(--pink)); }
  .mon-fill { background: linear-gradient(90deg, var(--lilac), var(--sky)); }
  .vs { width: 38px; height: 38px; border-radius: 50%; display: grid; place-items: center; background: var(--butter-2); color: var(--butter-ink); border: var(--stroke) solid var(--outline); }
  .term { background: var(--term-bg); color: var(--term-fg); border: var(--stroke) solid var(--term-frame); border-radius: var(--r); padding: 8px 12px; display: grid; gap: 2px; align-content: start; font: 13px/1.35 var(--font-mono); max-height: 22rem; min-height: 7rem; overflow: auto; }
  .rh { font: 800 10.5px/1.3 var(--font-ui); letter-spacing: .08em; text-transform: uppercase; color: #9b97c4; margin-top: 6px; }
  .rh:first-child { margin-top: 0; }
  .tl.mon { padding-left: 1.2em; }
  .say { white-space: pre-wrap; overflow-wrap: anywhere; }
  .aside { margin: 0; color: #9b97c4; font-style: italic; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .odds, .calc { display: grid; gap: 12px; }
  .exp { border-collapse: collapse; font-size: 13px; width: min(100%, 30rem); }
  .exp th { text-align: left; font: 800 11px var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: var(--ink-3); padding: 4px 10px 4px 0; }
  .exp td { padding: 4px 10px 4px 0; font-family: var(--font-mono); border-top: 1px solid var(--line); }
  .exp tr.now td { font-weight: 800; color: var(--peach-ink); }
  .left-out { margin: 0; }
  @container (max-width: 520px) {
    .bars { grid-template-columns: minmax(0, 1fr); }
    .vs { display: none; }
    .bar-side.right { text-align: left; }
    .bar-side.right .who { justify-content: flex-start; }
    .right .hp { transform: none; }
    .lbl { flex-basis: 100%; }
  }
</style>
