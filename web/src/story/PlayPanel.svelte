<!--
  Play it: a pretend player in a story (play.ts). The player greets the monster and asks it
  about its topics, or types in the room (its exits' words, its commands), and reads what the
  game would send back, in its colours; notes say what the game does not (coins paid, an item
  given, a door unlocked, a test failed and why, with what would have passed it). The map
  lights up the way each turn went.

  On the World's map it walks (`walking`): from a room, the arrow keys (`walk`) and the ways
  under the game's words take the player through the exits, refused as the game refuses them,
  and the map follows them (`onmove`). What they type is typed where they are.
-->
<svelte:options css="injected" />

<script lang="ts">
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import Coins from '@lucide/svelte/icons/coins';
  import CornerDownLeft from '@lucide/svelte/icons/corner-down-left';
  import Footprints from '@lucide/svelte/icons/footprints';
  import Info from '@lucide/svelte/icons/info';
  import MapPin from '@lucide/svelte/icons/map-pin';
  import PlayIcon from '@lucide/svelte/icons/play';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import UserRound from '@lucide/svelte/icons/user-round';
  import Users from '@lucide/svelte/icons/users';
  import X from '@lucide/svelte/icons/x';
  import { onMount, tick } from 'svelte';
  import { ABILITIES } from '../abilities';
  import { PEN, renderAnsi, runsOf, type Pen } from '../ansi';
  import { api, type Cell } from '../api';
  import { DIRS, exitsOf, REMOTE, TEXT } from '../library/exits';
  import { readRecord, recordOf } from '../library/records.svelte';
  import { alternatives, coinsOf, patternLines, type Reading } from '../library/script';
  import { Play, PLAYER, walkWord, type Fix, type Pretend, type Shown } from './play';
  import { nodeId } from './story';

  interface Props {
    /** The story's start, read as the story reads it. */
    start: { table: string; key: Cell[]; reading?: Reading; answer?: boolean };
    /** What the last turn lit: the records it reached and the ways between them (`from>to`). */
    played?: { nodes: Set<string>; went: Set<string> } | null;
    onclose: () => void;
    /** After each turn, the records it reached. */
    onturn?: (nodes: Set<string>) => void;
    /** How high the panel is. */
    high?: number;
    /** Walking a map from the start room: the room played is the one the player is in. */
    walking?: boolean;
    /** Walking: where the player is after each turn. */
    onmove?: (map: number, room: number) => void;
  }
  let { start, played = $bindable(null), onclose, onturn, high = $bindable(0), walking = false, onmove }: Props = $props();

  const text = (v: Cell | undefined) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);
  const num = (v: Cell | undefined) => Number(v) || 0;
  const abilityName = (n: number) => ABILITIES[n]?.[0] ?? `ability ${n}`;

  // ---- what is played ----------------------------------------------------------------------

  /** A monster's talk (or a talk block's), a room's commands (or a block of them), or a
   * block run as its story reads it. */
  type Mode = 'talk' | 'room' | 'run';
  const mode = $derived.by((): Mode | null => {
    if (start.table === 'Monsters') return 'talk';
    if (start.table === 'Rooms') return 'room';
    if (start.reading === 'topics') return 'talk';
    if (start.reading === 'commands') return 'room';
    if (start.reading === 'script' || start.reading === 'chances' || start.reading === 'text') return 'run';
    return null;
  });
  /** The record played: when walking, the room the player is in. */
  const at = $derived.by((): Cell[] => (walking ? [me.map, me.room] : start.key));
  const values = $derived(recordOf(start.table, at));
  const n0 = $derived(num(start.key[0]));
  /** The block the turns start from: the talk block, the room's commands, the block run. */
  const block = $derived(start.table === 'Monsters' ? num(values?.['Greet Txt']) : start.table === 'Rooms' ? num(values?.['Command Text']) : n0);
  /** Who is asked: the monster, or someone when a talk block is played alone. */
  const who = $derived(start.table === 'Monsters' ? text(values?.['Name']) || `monster ${n0}` : 'someone');
  const blockValues = $derived(block ? recordOf('Textblocks', [block, 0]) : null);

  /** The words of the room's exits, which the game tries before its commands: each one's
   * first words. */
  const exitWays = $derived.by(() => {
    const v = values;
    if (start.table !== 'Rooms' || !v) return [];
    const out: string[] = [];
    for (const kind of [TEXT, REMOTE])
      for (const e of exitsOf((c) => v[c])) {
        if (e.type !== kind || !e.to || !e.p1) continue;
        const m = recordOf('Messages', [e.p1]);
        const w = m ? ['Line 1', 'Line 2', 'Line 3'].map((c) => text(m[c]).trim()).find(Boolean) : undefined;
        if (w && !out.includes(w)) out.push(w);
      }
    return out;
  });

  /** The words a player can ask about, or type: each topic; each exit's words, then each
   * command's first way. */
  const ways = $derived.by(() => {
    if (mode !== 'talk' && mode !== 'room') return [];
    const out: string[] = mode === 'room' ? [...exitWays] : [];
    const data = text(blockValues?.['Data']);
    for (const pl of data ? patternLines(data) : []) {
      const w = (mode === 'talk' ? pl.pattern : (alternatives(pl.pattern)[0] ?? '').replaceAll('*', '')).trim();
      if (w && !out.includes(w)) out.push(w);
    }
    return out.slice(0, 12);
  });

  /** Walking: the ways out of the player's room that a direction takes, as stored (a Text or
   * Remote Action exit's words are among the ways typed). */
  const dirs = $derived.by(() => {
    const v = values;
    if (!walking || !v) return [];
    return exitsOf((c) => v[c])
      .filter((e) => e.to && e.type !== TEXT && e.type !== REMOTE)
      .map((e) => e.dir);
  });

  const TITLE: Record<Mode, string> = { talk: 'Try the conversation', room: 'Try typing here', run: 'Try it' };
  const title = $derived(walking ? 'Walking' : mode ? TITLE[mode] : 'Play it');
  const runLabel = $derived(start.reading === 'chances' ? 'Roll it' : start.reading === 'text' ? 'Show it' : `Run block ${n0}`);

  // ---- the pretend player -------------------------------------------------------------------

  let classes = $state<[Cell[], Cell][]>([]);
  let races = $state<[Cell[], Cell][]>([]);

  /** A player as the turns start: by the monster that talks, or in the room. */
  function fresh(): Pretend {
    const me = structuredClone(PLAYER);
    if (start.table === 'Monsters') me.monsters = [n0];
    if (start.table === 'Rooms') [me.map, me.room] = [n0, num(start.key[1])];
    me.cls = num(classes[0]?.[0][0]);
    me.race = num(races[0]?.[0][0]);
    return me;
  }
  let me = $state<Pretend>(fresh());

  onMount(() => {
    // a player is of some class and race: the world's first, until another is picked
    void Promise.all([api.rows('Classes').catch(() => null), api.rows('Races').catch(() => null)]).then(([c, r]) => {
      classes = c?.rows ?? [];
      races = r?.rows ?? [];
      if (!me.cls) me.cls = num(classes[0]?.[0][0]);
      if (!me.race) me.race = num(races[0]?.[0][0]);
    });
    played = { nodes: new Set([nodeId(start.table, start.key)]), went: new Set() };
    if (walking) void run({ kind: 'look' });
    return () => (played = null);
  });

  let drawer = $state(false);
  const nameOf = (table: string, n: number) => (n < 0 ? 'a monster' : text(recordOf(table, [n])?.['Name']) || `${table === 'Items' ? 'item' : table === 'Spells' ? 'spell' : 'monster'} ${n}`);
  const named = (rows: [Cell[], Cell][], n: number) => text(rows.find(([k]) => num(k[0]) === n)?.[1] ?? null) || (n ? `number ${n}` : 'none');

  // ---- the turns ------------------------------------------------------------------------------

  /** A turn: what the player does. Walking, `go` a direction and `look` at the room (`at`
   * something, which is not played; `put` there first, as a sysop puts a player). */
  type Way =
    | { kind: 'greet' }
    | { kind: 'ask' | 'type'; words: string }
    | { kind: 'run' }
    | { kind: 'go'; dir: number; words?: string }
    | { kind: 'look'; words?: string; at?: boolean; put?: [number, number] };

  interface Turn {
    /** What the player typed, as the game's prompt would show it (nothing when they were put
     * somewhere). */
    said: string;
    way: Way;
    shown: Shown[];
    /** The pen each line the player reads starts with: a terminal's colours carry on. */
    pens: Pen[];
    /** The player as the turn found them. */
    before: Pretend;
  }
  let turns = $state.raw<Turn[]>([]);
  let busy = $state(false);
  let typed = $state('');
  let term = $state<HTMLElement>();

  function saidOf(way: Way): string {
    if (way.kind === 'go') return way.words ?? DIRS[way.dir]!;
    if (way.kind === 'look') return way.put ? '' : (way.words ?? 'look');
    if (way.kind === 'greet') return `greet ${who}`;
    if (way.kind === 'ask') return `ask ${who} ${way.words}`;
    if (way.kind === 'type') return way.words;
    return runLabel;
  }

  async function turn(p: Play, way: Way) {
    if (way.kind === 'go') await p.go(p.me.map, p.me.room, way.dir);
    else if (way.kind === 'look') {
      if (way.at) p.shown.push({ to: 'note', text: 'LOOK at something is not played here: LOOK alone shows the room.' });
      else if (way.put) {
        const [m, r] = way.put;
        const moved = m !== p.me.map || r !== p.me.room;
        p.shown.push({ to: 'note', text: `Put in room ${m}/${r}.` });
        // the room left keeps its monsters and what lies there
        if ((await p.look(m, r)) === 'ok' && moved) Object.assign(p.me, { floor: [], monsters: [] });
      } else await p.look(p.me.map, p.me.room);
    } else if (way.kind === 'run') {
      if (start.reading === 'chances') await p.chances(n0);
      else if (start.reading === 'text') await p.show(n0, !!start.answer);
      else await p.runBlock(n0);
    } else if (mode === 'talk') {
      const q = way.kind === 'ask' ? way.words : null;
      if (start.table === 'Monsters') await p.ask(n0, q);
      else await p.askBlock(n0, who, q);
    } else if (way.kind === 'type') {
      if (walking) await p.type(p.me.map, p.me.room, way.words);
      else if (start.table === 'Rooms') await p.type(n0, num(start.key[1]), way.words);
      else if ((await p.command(n0, way.words)) === 'nothing') p.shown.push({ to: 'note', text: 'None of the commands takes it.' });
    }
  }

  async function run(way: Way, before: Pretend = $state.snapshot(me) as Pretend) {
    if (busy) return;
    busy = true;
    const p = new Play(readRecord, structuredClone(before), { ability: abilityName });
    try {
      await turn(p, way);
    } catch {
      p.shown.push({ to: 'note', text: 'A record the turn needed could not be read, so it stopped here. Try it again.', bad: true });
    }
    me = p.me;
    // a turn under way when the classes came keeps them
    if (!me.cls) me.cls = num(classes[0]?.[0][0]);
    if (!me.race) me.race = num(races[0]?.[0][0]);
    let pen = PEN;
    const pens = p.shown.map((s) => {
      if (s.to !== 'you' && s.to !== 'all') return PEN;
      const at = pen;
      pen = runsOf(s.text, pen).pen;
      return at;
    });
    turns = [...turns, { said: saidOf(way), way, shown: p.shown, pens, before }];
    played = { nodes: p.touched, went: p.went };
    busy = false;
    onturn?.(p.touched);
    if (walking) onmove?.(me.map, me.room);
    await tick();
    term?.scrollTo({ top: term.scrollHeight, behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
  }

  /** The last turn again, from the player as it found them with the change made. */
  function retry(t: Turn, fix: Fix) {
    const before = structuredClone(t.before);
    fix.apply(before);
    me = before;
    void run(t.way, before);
  }

  function send(e: SubmitEvent) {
    e.preventDefault();
    const words = typed.trimEnd();
    if (!words.trim()) return;
    typed = '';
    // the game's own commands come before the room's
    const w = walking ? walkWord(words) : null;
    if (typeof w === 'number') void run({ kind: 'go', dir: w, words });
    else if (w) void run({ kind: 'look', words, at: w === 'look at' });
    else void run(mode === 'talk' ? { kind: 'ask', words } : { kind: 'type', words });
  }

  function again() {
    turns = [];
    me = fresh();
    played = { nodes: new Set([nodeId(start.table, start.key)]), went: new Set() };
    if (walking) void run({ kind: 'look' });
  }

  /** Walking: the player goes `dir` from where they are, as an arrow key on the map does. */
  export function walk(dir: number) {
    void run({ kind: 'go', dir });
  }

  /** Walking: the player is put in a room, as a sysop puts a player, and reads it. */
  export function put(map: number, room: number) {
    void run({ kind: 'look', put: [map, room] });
  }

  /** Walking, with nothing typed: the arrow keys walk, and Esc goes back to the map. */
  const ARROWS: Record<string, number> = { ArrowUp: 0, ArrowDown: 1, ArrowRight: 2, ArrowLeft: 3 };
  function typing(e: KeyboardEvent) {
    if (!walking || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      if (typed) typed = '';
      else onclose();
      return;
    }
    const d = ARROWS[e.key];
    if (d === undefined || typed) return;
    e.preventDefault();
    walk(d);
  }

  function ansi(node: HTMLElement, [line, pen]: [string, Pen]) {
    const draw = ([t, p]: [string, Pen]) => node.replaceChildren(renderAnsi(t.replace(/\n+$/, ''), p));
    draw([line, pen]);
    return { update: draw };
  }
</script>

{#snippet list(label: string, table: string, nums: number[], set: (next: number[]) => void)}
  <div class="list">
    <span class="lab">{label}</span>
    {#each nums as v, i (i)}
      <span class="chip">{nameOf(table, v)}<button type="button" class="x" aria-label="Take away {nameOf(table, v)}" onclick={() => set(nums.filter((_, j) => j !== i))}><X size={11} /></button></span>
    {/each}
    <input
      class="add"
      type="number"
      min="1"
      placeholder="+ number"
      aria-label="Add to {label.toLowerCase()} by number"
      onkeydown={(e) => {
        if (e.key !== 'Enter') return;
        e.preventDefault();
        const n = Math.trunc(Number(e.currentTarget.value));
        if (n > 0) set([...nums, n]);
        e.currentTarget.value = '';
      }}
    />
  </div>
{/snippet}

<section class="play" class:walk={walking} aria-label={title} bind:offsetHeight={high}>
  <header class="ph">
    <span class="tile">{#if walking}<Footprints size={14} />{:else}<PlayIcon size={14} />{/if}</span>
    <b class="title">{title}</b>
    <span class="purse" title="Coins: {me.copper.toLocaleString('en-US')} copper"><Coins size={14} />{coinsOf(me.copper)}</span>
    <span class="end">
      <button type="button" class="btn sm ghost" aria-expanded={drawer} onclick={() => (drawer = !drawer)}><UserRound size={14} />{me.name}, level {me.level}</button>
      <button type="button" class="btn sm icon ghost" title="Start over" aria-label="Start over" onclick={again}><RotateCcw size={14} /></button>
      <button type="button" class="btn sm icon ghost" title={walking ? 'Back to editing (Esc)' : 'Close'} aria-label={walking ? 'Back to editing' : 'Close'} onclick={onclose}><X size={15} /></button>
    </span>
  </header>

  {#if drawer}
    <div class="drawer">
      <div class="fields">
        <label><span>Name</span><input bind:value={me.name} maxlength="20" /></label>
        <label><span>Level</span><input type="number" min="1" bind:value={me.level} /></label>
        <label
          ><span>Class</span><select bind:value={me.cls}
            >{#if !classes.some(([k]) => num(k[0]) === me.cls)}<option value={me.cls}>{named(classes, me.cls)}</option>{/if}{#each classes as [k, t] (num(k[0]))}<option value={num(k[0])}>{text(t) || `class ${num(k[0])}`}</option>{/each}</select
          ></label
        >
        <label
          ><span>Race</span><select bind:value={me.race}
            >{#if !races.some(([k]) => num(k[0]) === me.race)}<option value={me.race}>{named(races, me.race)}</option>{/if}{#each races as [k, t] (num(k[0]))}<option value={num(k[0])}>{text(t) || `race ${num(k[0])}`}</option>{/each}</select
          ></label
        >
        <label title="At -51 or under a player is good; from 40, evil"><span>Evil points</span><input type="number" bind:value={me.evil} /></label>
        <label title="What every stat and skill a script tests is"><span>Stats</span><input type="number" min="0" bind:value={me.stats} /></label>
        <label><span>Copper</span><input type="number" min="0" bind:value={me.copper} /></label>
        <label class="switchy"><input class="switch" type="checkbox" role="switch" bind:checked={me.tournament} /><span>Tournament play</span></label>
      </div>
      {@render list('Carrying', 'Items', me.items, (next) => (me.items = next))}
      {@render list('Spells on them', 'Spells', me.spells, (next) => (me.spells = next))}
      {@render list('Monsters here', 'Monsters', me.monsters, (next) => (me.monsters = next))}
      {@render list('Lying in the room', 'Items', me.floor, (next) => (me.floor = next))}
      {#if Object.keys(me.abilities).length}
        <div class="list">
          <span class="lab">Abilities</span>
          {#each Object.entries(me.abilities) as [a, v] (a)}
            <span class="chip">{abilityName(Number(a))} {v}<button type="button" class="x" aria-label="Take away {abilityName(Number(a))}" onclick={() => delete me.abilities[Number(a)]}><X size={11} /></button></span>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <div class="term" role="log" aria-label="What the player reads" bind:this={term}>
    {#if !turns.length}
      <p class="hint">
        {#if walking}Walk the exits with the arrow keys or the ways below, and read what the game would send back. The map follows you.
        {:else}{#if mode === 'talk'}Greet {who} or ask about something, and read what the game would send back.{:else if mode === 'room'}Type what a player would here, an exit's words or a command, and read what the game would send back.{:else}Run it as the game would, and read what it sends back.{/if}
          The map lights up the way it goes.{/if}
      </p>
    {/if}
    {#each turns as t, ti (ti)}
      {#if t.said}<div class="said"><span class="prompt">&gt;</span> {t.said}</div>{/if}
      {#each t.shown as s, si (si)}
        {#if s.to === 'note'}
          <div class="note" class:bad={s.bad}>
            {#if s.bad}<CircleAlert size={13} />{:else}<Info size={13} />{/if}
            <span class="nt"
              >{s.text}{#if s.fix && ti === turns.length - 1}{@const fix = s.fix}<button type="button" class="fix" disabled={busy} onclick={() => retry(t, fix)}>{fix.label}, and again</button>{/if}</span
            >
          </div>
        {:else if s.to === 'room'}
          <div class="room"><span class="who"><Users size={12} />Others here read</span><span class="line" use:ansi={[s.text, PEN]}></span></div>
        {:else if s.to === 'there'}
          <div class="room"><span class="who there" title="Those in {s.where} read it"><MapPin size={12} /><span class="cut">In {s.where}</span></span><span class="line" use:ansi={[s.text, PEN]}></span></div>
        {:else}
          <div class="line" use:ansi={[s.text, t.pens[si] ?? PEN]}></div>
          {#if s.to === 'all'}<div class="room"><span class="who"><Users size={12} />Everyone here reads it</span></div>{/if}
        {/if}
      {/each}
      {#if !t.shown.length}<div class="note"><Info size={13} /><span>Nothing is shown.</span></div>{/if}
    {/each}
  </div>

  <div class="pb">
    {#if mode === 'talk' || mode === 'room'}
      {#if ways.length || dirs.length || mode === 'talk'}
        <div class="ways" aria-label={mode === 'talk' ? 'Topics' : walking ? 'Ways out and commands' : 'Commands'}>
          {#if mode === 'talk'}<button type="button" class="btn sm" disabled={busy} onclick={() => run({ kind: 'greet' })}>Greet</button>{/if}
          {#each dirs as d (d)}
            <button type="button" class="btn sm dir" disabled={busy} title="Walk {DIRS[d]}" onclick={() => walk(d)}>{DIRS[d]}</button>
          {/each}
          {#each ways as w (w)}
            <button type="button" class="btn sm way" disabled={busy} title={mode === 'talk' ? `ask ${who} ${w}` : w} onclick={() => run(mode === 'talk' ? { kind: 'ask', words: w } : { kind: 'type', words: w })}>{mode === 'talk' ? `about ${w}` : w}</button>
          {/each}
        </div>
      {/if}
      <form class="type" onsubmit={send}>
        <input
          bind:value={typed}
          placeholder={mode === 'talk' ? `Ask ${who} about anything` : walking ? 'Type what a player would: a direction, LOOK, a command' : 'Type what a player would'}
          aria-label={mode === 'talk' ? `Ask ${who} about` : 'Type as a player'}
          autocomplete="off"
          spellcheck="false"
          onkeydown={typing}
        />
        <button type="submit" class="btn sm primary sticker" disabled={busy || !typed.trim()}><CornerDownLeft size={14} />{mode === 'talk' ? 'Ask' : 'Type'}</button>
      </form>
      {#if walking}
        <p class="keys"><span class="kbd">←↑→↓</span> walk <span class="kbd">U</span><span class="kbd">D</span> up and down <span class="kbd">Esc</span> back to editing at the room you reach</p>
      {/if}
    {:else if mode === 'run'}
      <button type="button" class="btn sm primary sticker" disabled={busy} onclick={() => run({ kind: 'run' })}><PlayIcon size={14} />{runLabel}</button>
    {/if}
  </div>
</section>

<style>
  .play {
    position: absolute; left: 14px; bottom: 56px; z-index: 6; width: min(640px, calc(100% - 28px)); max-height: calc(100% - 128px);
    container-type: inline-size; display: flex; flex-direction: column; overflow: hidden;
    background: var(--surface); border: var(--stroke) solid var(--outline); border-radius: 18px; box-shadow: var(--lift), var(--float);
    animation: pop-in .25s var(--pop);
  }
  @media (prefers-reduced-motion: reduce) { .play { animation: none; } }
  .ph { display: flex; align-items: center; gap: 8px; padding: 8px 8px 8px 12px; border-bottom: 1px solid var(--line); flex-wrap: wrap; }
  .ph .tile { width: 26px; height: 26px; border-radius: 9px; display: grid; place-items: center; background: var(--mint-2); color: var(--mint-ink); flex: none; }
  .title { font: 600 16px var(--font-display); white-space: nowrap; }
  .purse { display: inline-flex; align-items: center; gap: 5px; font-size: 12.5px; font-weight: 800; color: var(--butter-ink); background: var(--butter-2); border-radius: 999px; padding: 2px 9px; white-space: nowrap; }
  .end { margin-left: auto; display: flex; gap: 2px; align-items: center; }

  .drawer { display: grid; gap: 8px; padding: 10px 12px; border-bottom: 1px solid var(--line); background: var(--surface-2); max-height: 38cqh; overflow: auto; font-size: 12px; font-weight: 800; color: var(--ink-2); }
  .fields { display: grid; grid-template-columns: repeat(auto-fill, minmax(118px, 1fr)); gap: 8px; }
  .fields label { display: grid; gap: 3px; }
  .fields input:not(.switch), .fields select { width: 100%; box-sizing: border-box; font: 700 12.5px var(--font-ui); padding: 4px 6px; }
  .fields .switchy { display: flex; align-items: center; gap: 7px; align-self: end; padding-bottom: 4px; }
  .list { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; }
  .lab { min-width: 108px; }
  .chip { display: inline-flex; align-items: center; gap: 3px; background: var(--surface); border: 1px solid var(--line-2); border-radius: 999px; padding: 1px 2px 1px 9px; font-weight: 800; color: var(--ink); }
  .chip .x { min-height: 0; width: 18px; height: 18px; padding: 0; border: 0; border-radius: 50%; background: none; color: var(--ink-3); }
  .chip .x:hover { background: var(--surface-3); color: var(--ink); }
  .add { width: 86px; font: 700 12px var(--font-ui); padding: 2px 8px; border-radius: 999px; }

  /* the game's text, 80 columns wide when the panel has room */
  .term {
    flex: 1; min-height: 140px; overflow: auto; padding: 10px 12px; background: var(--term-bg); color: var(--term-fg);
    font: clamp(10px, calc((100cqw - 26px) / 48.5), 13px)/1.35 var(--font-mono); overflow-wrap: anywhere;
  }
  .hint { margin: 0; font: 700 12.5px/1.45 var(--font-ui); color: #a9a3cc; white-space: normal; }
  /* the game's lines keep their spaces (the blanks between the blocks are not lines) */
  .said, .line { white-space: pre-wrap; }
  .said { color: #ffffff; font-weight: 700; margin-top: 8px; }
  .said:first-child { margin-top: 0; }
  .prompt { color: #8a84b0; }
  .line { min-height: 1.35em; }
  .room { display: flex; gap: 8px; align-items: baseline; }
  .room .who { display: inline-flex; align-items: center; gap: 4px; flex: none; font: 800 11px var(--font-ui); color: #8a84b0; white-space: nowrap; }
  /* another room's name gives way to its line */
  .room .who.there { flex: 0 1 auto; max-width: 40%; min-width: 0; }
  .who .cut { overflow: hidden; text-overflow: ellipsis; }
  .note {
    display: flex; align-items: flex-start; gap: 6px; margin: 3px 0; padding: 4px 8px; border-radius: 8px; width: fit-content; max-width: 100%; box-sizing: border-box;
    background: #1b1834; color: #cdc7f1; font: 700 12px/1.45 var(--font-ui); white-space: normal;
  }
  .note :global(svg) { flex: none; margin-top: 3px; }
  .nt { min-width: 0; }
  .note.bad { background: #2c1424; color: #ffb3cf; }
  .fix { min-height: 22px; margin: 2px 0 2px 8px; padding: 0 9px; vertical-align: middle; border-radius: 999px; border: 1px solid #6a5fb0; background: #2b2652; color: #ffffff; font: 800 11.5px var(--font-ui); }
  .fix:hover:not(:disabled) { background: #3b3470; border-color: #a99cf0; }

  .pb { display: grid; gap: 8px; padding: 10px 12px; border-top: 1px solid var(--line); }
  .ways { display: flex; flex-wrap: wrap; gap: 6px; max-height: 76px; overflow: auto; }
  .way { max-width: 220px; overflow: hidden; text-overflow: ellipsis; font-family: var(--font-mono); font-weight: 700; }
  .type { display: flex; gap: 6px; }
  .type input { flex: 1; font: 700 13px var(--font-mono); }
  .dir { font-weight: 800; background: var(--mint-2); color: var(--mint-ink); }
  .keys { margin: -2px 0 0; font: 700 11.5px var(--font-ui); color: var(--ink-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .keys .kbd { height: 18px; font-size: 10.5px; margin-left: 6px; }
  .keys .kbd:first-child { margin-left: 0; }
  @media (pointer: coarse) { .keys { display: none; } }

  /* walking a map: in its corner at a steady height, clear of its overview */
  .play.walk {
    left: var(--edge, 12px); bottom: var(--edge, 12px); max-height: none;
    width: min(600px, calc(100% - var(--card, 0px) - 214px - 3 * var(--edge, 12px)));
    height: min(calc(100% - 124px), max(330px, 46%));
  }

  @media (max-width: 760px) {
    .play { left: 8px; right: 8px; bottom: 8px; width: auto; max-height: min(72%, calc(100% - 72px)); }
    .ph .title { display: none; }
    /* the topics in one row that scrolls, so the game's words keep the room */
    .ways { flex-wrap: nowrap; overflow-x: auto; max-height: none; padding-bottom: 2px; }
    .way, .ways .btn { flex: none; }
    .play.walk { left: 8px; right: 8px; bottom: 8px; width: auto; height: min(50%, calc(100% - 124px)); }
  }
</style>
