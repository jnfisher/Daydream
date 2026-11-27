<!--
  A story as a map (story.ts): the record it starts at on the left, then a column for each step
  on: a monster's talk block and its topics, the text each answer shows, the script it then
  runs, the messages, items and rooms the script's steps name. Each arrow leaves by the row of
  its node that takes it, a topic or a step, so the map reads as the game runs it. Hovering a
  node lights up what it connects to; clicking one opens it beside the map.
-->
<svelte:options css="injected" />

<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Ban from '@lucide/svelte/icons/ban';
  import CornerDownRight from '@lucide/svelte/icons/corner-down-right';
  import Dices from '@lucide/svelte/icons/dices';
  import Footprints from '@lucide/svelte/icons/footprints';
  import Funnel from '@lucide/svelte/icons/funnel';
  import Gift from '@lucide/svelte/icons/gift';
  import HandCoins from '@lucide/svelte/icons/hand-coins';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import MessageSquare from '@lucide/svelte/icons/message-square';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import PackageMinus from '@lucide/svelte/icons/package-minus';
  import PlayIcon from '@lucide/svelte/icons/play';
  import Zap from '@lucide/svelte/icons/zap';
  import type { Component } from 'svelte';
  import { ABILITIES } from '../abilities';
  import { plainOf, renderAnsi } from '../ansi';
  import type { Cell } from '../api';
  import { talkOdds } from '../library/combat';
  import { recordOf } from '../library/records.svelte';
  import { alternatives, coinsOf, linesOf, readingOf, type Bit, type Line, type Reading, type Step, type StepKind } from '../library/script';
  import { kindOf } from '../shell/kinds';
  import Mascot from '../shell/Mascot.svelte';
  import { keyLabel } from '../shell/routes';
  import { app, inspect } from '../shell/state.svelte';
  import { edgePath, layout, SPACING, storyOf, type EdgeKind, type StoryEdge, type StoryNode, type Values } from './story';

  let { start }: { start: { table: string; key: Cell[] } } = $props();

  const text = (v: Cell | undefined) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);
  const num = (v: Cell | undefined) => Number(v) || 0;
  /** Text as it reads without its colours, on one line. */
  const plain = (s: string) => plainOf(s).replace(/\s+/g, ' ').trim();
  const cut = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);

  // ---- the story ---------------------------------------------------------------------------

  /** How the start is read: a text block as its shape says, and as an answer when it links on. */
  const root = $derived.by(() => {
    if (start.table !== 'Textblocks') return start;
    const v = recordOf(start.table, start.key);
    if (!v) return start;
    const reading = readingOf(text(v['Data']));
    return { ...start, reading, answer: reading === 'text' && num(v['Link To']) > 0 };
  });
  const story = $derived(storyOf(root, recordOf));
  const first = $derived(story.nodes[0]);
  const lookFirst = $derived(recordOf(start.table, start.key));

  /** A node's record's values. */
  const valuesOf = (n: StoryNode): Values | null | undefined => recordOf(n.table, n.key);

  const byId = $derived(new Map(story.nodes.map((n) => [n.id, n])));

  /** The words players ask about to get each answer, and the greetings. */
  const asked = $derived.by(() => {
    const out = new Map<string, string[]>();
    for (const e of story.edges) {
      const from = byId.get(e.from);
      if (e.kind !== 'asks' || !from) continue;
      const v = valuesOf(from);
      const pattern = v ? linesOf(text(v['Data']), 'topics')[Number(e.port.slice(1))]?.pattern?.trim() : undefined;
      const said = out.get(e.to) ?? [];
      const word = pattern || 'anything';
      if (!said.includes(word)) said.push(word);
      out.set(e.to, said);
    }
    return out;
  });
  const greetings = $derived(new Set(story.edges.filter((e) => e.kind === 'greets').map((e) => e.to)));

  const READING: Record<Reading, string> = {
    topics: 'Talk block',
    text: 'Text',
    script: 'Script',
    commands: 'Commands',
    chances: 'Random',
    talk: 'Chatter',
    names: 'Names',
  };

  function titleOf(n: StoryNode): string {
    const v = valuesOf(n);
    if (n.state === 'missing') return 'None such';
    if (!v) return 'Reading…';
    if (n.table === 'Textblocks') {
      const words = asked.get(n.id);
      if (words) return words.map((w) => `“${w}”`).join(', ');
      if (greetings.has(n.id)) return 'Greeting';
      return READING[n.reading ?? 'text'];
    }
    if (n.table === 'Messages') return 'Message';
    return text(v['Name']) || kindOf(n.table).one;
  }

  const numOf = (n: StoryNode) => (n.table === 'Textblocks' && !num(n.key[1]) ? String(n.key[0]) : keyLabel(n.key));

  // ---- the rows a node shows -------------------------------------------------------------

  const ICONS: Record<StepKind, [Component<{ size?: number | string; class?: string }>, string]> = {
    test: [Funnel, 't-other'],
    pay: [HandCoins, 't-item'],
    take: [PackageMinus, 't-monster'],
    give: [Gift, 't-room'],
    say: [MessageSquare, 't-msg'],
    move: [Footprints, 't-shop'],
    run: [Dices, 't-text'],
    other: [Zap, 't-spell'],
    skip: [Ban, 't-other'],
  };

  /** The most steps and topics a node lists before the rest are one row. */
  const ROWS = 14;

  interface StepRow {
    s: Step;
    /** How many times in a row the same clause is written. */
    n: number;
    ports: string[];
  }
  interface LineRows {
    l: Line;
    rows: StepRow[];
  }

  /** A block's lines, each step a row, the same clause again and again one row; past the
   * most a node lists, the rest of the steps one row. */
  function lineRows(data: string, reading: Reading): { lines: LineRows[]; rest: string[]; restSteps: number } {
    const lines: LineRows[] = [];
    const rest: string[] = [];
    let shown = 0;
    let restSteps = 0;
    linesOf(data, reading).forEach((l, li) => {
      if (shown >= ROWS) {
        if (l.never) return;
        l.steps.forEach((_, si) => rest.push(`${li}.${si}`));
        restSteps += l.steps.length;
        return;
      }
      const rows: StepRow[] = [];
      if (l.never) shown++;
      else
        l.steps.forEach((s, si) => {
          const port = `${li}.${si}`;
          const last = rows.at(-1);
          if (last && last.s.clause.text.trim() === s.clause.text.trim()) {
            last.n++;
            last.ports.push(port);
          } else if (shown >= ROWS) {
            rest.push(port);
            restSteps++;
          } else {
            rows.push({ s, n: 1, ports: [port] });
            shown++;
          }
        });
      lines.push({ l, rows });
    });
    return { lines, rest, restSteps };
  }

  /** The map a room in a step is on, when the step does not say. */
  const map = $derived(start.table === 'Rooms' ? num(start.key[0]) : undefined);

  function refKeyOf(t: string, n: number, m?: number): Cell[] {
    if (t === 'Rooms') return [m ?? 0, n];
    if (t === 'Textblocks') return [n, 0];
    return [n];
  }

  /** What a step's record is called in its words: an item's name, a message's first line. */
  function refLabel(t: string, n: number, m?: number): string {
    const one = kindOf(t).one.toLowerCase();
    if (n <= 0 || t === 'Textblocks') return `${one} ${n}`;
    if (t === 'Rooms' && m === undefined) return `room ${n}`;
    const v = recordOf(t, refKeyOf(t, n, m));
    if (t === 'Messages') {
      const line = v ? plain(text(v['Line 1'])) : '';
      return line ? `“${cut(line, 40)}”` : `message ${n}`;
    }
    return (v && text(v['Name'])) || `${one} ${n}`;
  }

  const pct = (p: number) => (p >= 10 || Number.isInteger(p) ? `${Math.round(p)}%` : `${p.toFixed(1)}%`);

  function ansi(node: HTMLElement, line: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(line);
    return { update: draw };
  }

  // ---- where everything goes ------------------------------------------------------------

  /** Each node's height and how far down it each row it leaves by sits, as measured. */
  let sizes = $state<Record<string, { h: number; ports: Record<string, number> }>>({});

  const placed = $derived(
    layout(
      story,
      (id) => sizes[id]?.h ?? 44,
      (e) => sizes[e.from]?.ports[e.port],
    ),
  );

  function measure(el: HTMLElement, id: string) {
    let at = id;
    const read = () => {
      const ports: Record<string, number> = {};
      for (const p of el.querySelectorAll<HTMLElement>('[data-port]'))
        for (const name of p.dataset.port!.split(' ')) ports[name] = Math.round(el.clientTop + p.offsetTop + p.offsetHeight / 2);
      const h = el.offsetHeight;
      const had = sizes[at];
      if (had && had.h === h && Object.keys(ports).length === Object.keys(had.ports).length && Object.entries(ports).every(([k, v]) => had.ports[k] === v)) return;
      sizes[at] = { h, ports };
    };
    const seen = new ResizeObserver(read);
    seen.observe(el);
    return {
      update(next: string) {
        at = next;
        read();
      },
      destroy: () => seen.disconnect(),
    };
  }

  const downOf = (e: StoryEdge) => sizes[e.from]?.ports[e.port] ?? SPACING.entry;

  // ---- how the map is looked at -----------------------------------------------------------

  let fit = $state(true);
  let wide = $state(0);
  /** Fitted to the page's width, never so small its words cannot be read. */
  const zoom = $derived(fit && placed.width ? Math.max(0.8, Math.min(1, (wide - 4) / (placed.width + 40))) : 1);

  // the nodes go where they are measured to fit without gliding there first
  let settled = $state(false);
  $effect(() => {
    let id = requestAnimationFrame(() => (id = requestAnimationFrame(() => (settled = true))));
    return () => cancelAnimationFrame(id);
  });

  /** The node hovered or focused: it and what it connects to are lit, the rest dimmed. */
  let lit = $state<string | null>(null);
  const near = $derived.by(() => {
    if (!lit) return null;
    const s = new Set([lit]);
    for (const e of story.edges) {
      if (e.from === lit) s.add(e.to);
      if (e.to === lit) s.add(e.from);
    }
    return s;
  });
  const open = $derived(app.inspecting.at(-1));
  const isOpen = (n: StoryNode) => !!open && open.table === n.table && keyLabel(open.key) === keyLabel(n.key);
  /** How much of the right of the page the open record covers (Inspector.svelte). */
  const cover = $derived(app.inspecting.length ? Math.min(420, wide - 24) + 24 : 0);

  let canvas = $state<HTMLElement>();

  // ---- playing it ----------------------------------------------------------------------------

  /** Whether a pretend player can try the story: a monster with a talk block, a room with
   * commands, or a text block the game runs or shows. */
  const playable = $derived.by(() => {
    if (!lookFirst) return false;
    if (start.table === 'Monsters') return num(lookFirst['Greet Txt']) > 0;
    if (start.table === 'Rooms') return num(lookFirst['Command Text']) > 0;
    return root.table === 'Textblocks' && 'reading' in root && root.reading !== undefined && root.reading !== 'talk' && root.reading !== 'names';
  });
  let playing = $state(false);
  /** The panel's code, read the first time it is opened. */
  let panel: Promise<typeof import('./PlayPanel.svelte')> | undefined;
  const panelCode = () => (panel ??= import('./PlayPanel.svelte'));
  /** What the last turn lit. */
  let played = $state<{ nodes: Set<string>; went: Set<string> } | null>(null);
  /** How much of the bottom of the page the panel covers. */
  let panelHigh = $state(0);
  const under = $derived(playing ? panelHigh + 70 : 0);

  /** After a turn, the furthest record it reached comes into view. */
  function shown(nodes: Set<string>) {
    let far: string | null = null;
    for (const id of nodes) {
      const b = placed.at.get(id);
      if (b && (!far || b.x > placed.at.get(far)!.x || (b.x === placed.at.get(far)!.x && b.y > placed.at.get(far)!.y))) far = id;
    }
    if (far) reveal(far);
  }

  $effect(() => {
    if (!playing) return;
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !e.defaultPrevented) playing = false;
    };
    window.addEventListener('keydown', esc);
    return () => window.removeEventListener('keydown', esc);
  });

  /** Scrolls a node out from under the open record and the play panel, or into view. */
  function reveal(id: string) {
    requestAnimationFrame(() => {
      const b = placed.at.get(id);
      if (!b || !canvas) return;
      const [left, right, top, bottom] = [b.x * zoom, (b.x + b.w) * zoom, b.y * zoom, (b.y + b.h) * zoom];
      const seen = canvas.clientWidth - cover;
      const high = canvas.clientHeight - under;
      const to: ScrollToOptions = { behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' };
      if (right > canvas.scrollLeft + seen) to.left = right - seen + 24;
      else if (left < canvas.scrollLeft) to.left = left - 24;
      if (bottom > canvas.scrollTop + high) to.top = Math.min(top - 24, bottom - high + 24);
      else if (top < canvas.scrollTop) to.top = top - 24;
      if (to.left !== undefined || to.top !== undefined) canvas.scrollTo(to);
    });
  }

  function go(n: StoryNode) {
    const v = valuesOf(n);
    inspect({ table: n.table, key: n.key, name: v && n.table !== 'Textblocks' && n.table !== 'Messages' ? text(v['Name']) || undefined : undefined });
    reveal(n.id);
  }

  // ---- what the arrows are --------------------------------------------------------------

  /** An arrow's family: the story leading on, what it says, the way out when a test fails,
   * and what a step does with a record. */
  type Family = 'on' | 'says' | 'else' | 'does';
  const FAMILY: Record<EdgeKind, Family> = {
    talks: 'on',
    chatter: 'on',
    greets: 'on',
    asks: 'on',
    then: 'on',
    types: 'on',
    runs: 'on',
    says: 'says',
    else: 'else',
    gives: 'does',
    takes: 'does',
    needs: 'does',
    moves: 'does',
    works: 'does',
    casts: 'does',
    summons: 'does',
  };
  const LEGEND: [Family, string][] = [
    ['on', 'leads on'],
    ['says', 'says'],
    ['else', 'if not'],
    ['does', 'gives, takes, needs, moves'],
  ];
  const families = $derived(new Set(story.edges.map((e) => FAMILY[e.kind])));

  /** How many records of each kind the story holds. */
  const counts = $derived.by(() => {
    const by = new Map<string, number>();
    for (const n of story.nodes.slice(1)) by.set(n.table, (by.get(n.table) ?? 0) + 1);
    return [...by].map(([t, c]) => `${c} ${(c === 1 ? kindOf(t).one : kindOf(t).many).toLowerCase()}`);
  });

  const k = $derived(kindOf(start.table));
  const name = $derived(first ? titleOf(first) : '');
  const edgeKey = (e: StoryEdge) => `${e.from}>${e.to}>${e.kind}>${e.port}`;
</script>

<!-- one line, so no space comes between a word and a record that the words do not have -->
{#snippet said(bits: Bit[])}{#each bits as b, i (i)}{#if typeof b === 'string'}{b}{:else if 'ref' in b}<b class="rf {kindOf(b.ref.t).cls}">{refLabel(b.ref.t, b.ref.n, b.ref.map ?? map)}</b>{:else if 'ability' in b}<b class="rf">{ABILITIES[b.ability]?.[0] ?? `ability ${b.ability}`}</b>{:else}<b class="coin">{coinsOf(b.copper)}</b>{/if}{/each}{/snippet}

{#snippet steps(rows: StepRow[])}
  {#each rows as r (r.ports[0])}
    {@const [Icon, cls] = ICONS[r.s.kind]}
    <div class="row step" class:skip={r.s.kind === 'skip'} data-port={r.ports.join(' ')}>
      <span class="gi {cls}"><Icon size={12} /></span>
      <span class="said">{@render said(r.s.bits)}{#if r.n > 1}<b class="times"> ×{r.n}</b>{/if}</span>
    </div>
  {/each}
{/snippet}

{#snippet body(n: StoryNode)}
  {@const v = valuesOf(n)}
  {#if n.state === 'missing'}
    <p class="note bad">There is no {kindOf(n.table).one.toLowerCase()} {numOf(n)}.</p>
  {:else if !v}
    <p class="note">Reading…</p>
  {:else if n.table === 'Monsters' && n === first}
    {@const greet = num(v['Greet Txt'])}
    {@const talk = num(v['Talk Txt'])}
    {#if greet}<div class="row" data-port="talks"><span class="gi t-text"><MessageSquare size={12} /></span><span>Greeted or asked: talk block <b>{greet}</b></span></div>{/if}
    {#if talk}<div class="row" data-port="chatter"><span class="gi t-text"><MessageSquare size={12} /></span><span>Says now and then: block <b>{talk}</b></span></div>{/if}
    {#if !greet && !talk}<p class="note">Says nothing: it has no talk block.</p>{/if}
  {:else if n.table === 'Rooms' && n === first}
    {@const cmd = num(v['Command Text'])}
    {#if cmd}<div class="row" data-port="types"><span class="gi t-text"><CornerDownRight size={12} /></span><span>Players can type: block <b>{cmd}</b></span></div>{:else}<p class="note">Nothing to type here: it has no commands.</p>{/if}
  {:else if n.table === 'Textblocks'}
    {@const data = text(v['Data'])}
    {@const link = num(v['Link To'])}
    {#if n.reading === 'topics'}
      {@const topics = linesOf(data, 'topics')}
      {#if link}<div class="row" data-port="greet"><span class="gi t-text"><MessageSquare size={12} /></span><span>Greeted: shows block <b>{link}</b></span></div>{/if}
      {#each topics.slice(0, ROWS) as t, i (i)}
        <div class="row topic" data-port="t{i}"><span class="faint">Asked</span><span class="tag">{t.pattern?.trim() || 'anything'}</span><span class="to">{t.block || 'nothing'}</span></div>
      {/each}
      {#if topics.length > ROWS}
        <div class="row more" data-port={topics.slice(ROWS).map((_, i) => `t${i + ROWS}`).join(' ')}>And {topics.length - ROWS} topics more</div>
      {/if}
      {#if !topics.length && !link}<p class="note">Nothing to ask about, and no greeting.</p>{/if}
    {:else if n.reading === 'script' || n.reading === 'commands' || n.reading === 'chances'}
      {@const all = lineRows(data, n.reading)}
      {#each all.lines as { l, rows } (l.no)}
        {#if n.reading === 'commands'}
          <div class="typed">{#each alternatives(l.pattern ?? '').slice(0, 4) as a, j (j)}{#if j}<span class="faint">or</span>{/if}<code>{plain(a) || ' '}</code>{/each}</div>
        {:else if n.reading === 'chances'}
          <div class="typed"><span class="pill" class:never={l.never}>{pct(l.pct ?? 0)}</span><span class="faint">of rolls</span></div>
        {:else if all.lines.length > 1}
          <div class="typed"><span class="faint">Line {l.no}{l.no > 1 ? ', if the one before stops' : ''}</span></div>
        {/if}
        {#if l.never}<p class="note">Never run: {l.never}.</p>{:else if rows.length}{@render steps(rows)}{:else}<p class="note">Does nothing.</p>{/if}
      {/each}
      {#if all.rest.length}<div class="row more" data-port={all.rest.join(' ')}>And {all.restSteps} steps more</div>{/if}
      {#if !all.lines.length}<p class="note">Nothing the game runs.</p>{/if}
    {:else if n.reading === 'talk'}
      {@const odds = talkOdds(data).slice(0, 4)}
      {#each odds as o, i (i)}<div class="row chat"><span class="pill">{pct(o.pct)}</span><span class="mini line"><span class="clamp" use:ansi={o.text}></span></span></div>{/each}
      {#if !odds.length}<p class="note">Never says anything: no line has a chance.</p>{/if}
    {:else}
      <div class="mini"><div class="clamp" use:ansi={data.replace(/\n\s*\n/g, '\n').trim()}></div></div>
      {#if n.answer && link}<div class="row" data-port="then"><span class="gi t-text"><Dices size={12} /></span><span>Then runs block <b>{link}</b></span></div>{/if}
    {/if}
  {:else if n.table === 'Messages'}
    {@const lines = ['Line 1', 'Line 2', 'Line 3'].map((c) => text(v[c])).filter((l) => l.trim())}
    {#if lines.length}<div class="mini"><div class="clamp" use:ansi={lines.join('\n')}></div></div>{:else}<p class="note">Empty: it says nothing.</p>{/if}
  {/if}
{/snippet}

<div class="sg">
  <div class="bar" style:padding-right="{16 + cover}px">
    <a class="btn sm ghost back" href="#/story" aria-label="Story"><ArrowLeft size={15} /><span class="wide">Story</span></a>
    <span class="who {k.cls}">
      <span class="tile"><k.icon size={15} /></span>
      <h1>{name || `${k.one} ${keyLabel(start.key)}`}</h1>
      <span class="num">{k.one} {first ? numOf(first) : keyLabel(start.key)}</span>
    </span>
    <span class="stats">
      {#each counts as c (c)}<span class="tag">{c}</span>{/each}
      {#if story.more}<span class="tag warn" title="A story shows at most {story.nodes.length} records">and {story.more} more not shown</span>{/if}
    </span>
    <span class="end">
      {#if playable}
        <button type="button" class="btn sm sticker" class:primary={!playing} aria-pressed={playing} onclick={() => (playing = !playing)}><PlayIcon size={14} />Play it</button>
      {/if}
      <button type="button" class="btn sm icon" title={fit ? 'Show it at its own size' : 'Fit it to the page'} aria-label={fit ? 'Show it at its own size' : 'Fit it to the page'} aria-pressed={fit} onclick={() => (fit = !fit)}>
        {#if fit}<Maximize2 size={15} />{:else}<Minimize2 size={15} />{/if}
      </button>
    </span>
  </div>

  {#if lookFirst === null}
    <div class="empty">
      <Mascot mood="worry" size={72} />
      <p>There is no {k.one.toLowerCase()} {keyLabel(start.key)} in this world.</p>
      <a class="btn" href="#/story">Pick another story</a>
    </div>
  {:else}
    <div class="canvas" bind:clientWidth={wide} bind:this={canvas}>
      <!-- room to scroll the map's right out from under the open record -->
      <div class="sizer" style:width="{(placed.width + 40) * zoom + cover}px" style:height="{placed.height * zoom + under}px">
        <div class="graph" style:width="{placed.width + 40}px" style:height="{placed.height}px" style:transform="scale({zoom})">
          <svg class="edges" width={placed.width + 40} height={placed.height} aria-hidden="true">
            <defs>
              {#each LEGEND as [f] (f)}
                <marker id="sg-{f}" class="f-{f}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" /></marker>
              {/each}
            </defs>
            {#each story.edges as e (edgeKey(e))}
              {@const a = placed.at.get(e.from)}
              {@const b = placed.at.get(e.to)}
              {#if a && b}
                {@const went = !near && !!played?.went.has(`${e.from}>${e.to}@${e.port}`)}
                <path
                  class="edge f-{FAMILY[e.kind]}"
                  class:dim={near ? !(near.has(e.from) && near.has(e.to) && (e.from === lit || e.to === lit)) : !!played && !went}
                  class:went
                  d={edgePath(a, downOf(e), b)}
                  marker-end="url(#sg-{FAMILY[e.kind]})"
                />
              {/if}
            {/each}
          </svg>
          {#each story.nodes as n (n.id)}
            {@const at = placed.at.get(n.id)}
            {@const nk = kindOf(n.table)}
            <div
              class="node {nk.cls}"
              class:missing={n.state === 'missing'}
              class:run={n.table === 'Textblocks' && (n.reading === 'script' || n.reading === 'commands' || n.reading === 'chances')}
              class:dim={near ? !near.has(n.id) : !!played && !played.nodes.has(n.id)}
              class:trail={!near && !!played?.nodes.has(n.id)}
              class:lit={lit === n.id}
              class:open={isOpen(n)}
              class:calm={!settled}
              style:transform="translate({at?.x ?? 0}px, {at?.y ?? 0}px)"
              style:width="{SPACING.w}px"
              role="button"
              tabindex="0"
              use:measure={n.id}
              onclick={() => go(n)}
              onkeydown={(ev) => {
                if (ev.key === 'Enter' || ev.key === ' ') {
                  ev.preventDefault();
                  go(n);
                }
              }}
              onpointerenter={() => (lit = n.id)}
              onpointerleave={() => (lit = null)}
              onfocus={() => (lit = n.id)}
              onblur={() => (lit = null)}
            >
              <div class="nh">
                <span class="tile"><nk.icon size={13} /></span>
                <b title={titleOf(n)}>{titleOf(n)}</b>
                <span class="nn" title="{nk.one} {keyLabel(n.key)}">{numOf(n)}</span>
              </div>
              {#if n.table === 'Textblocks' || n.table === 'Messages' || n === first || n.state !== 'ok'}
                <div class="nb">{@render body(n)}</div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>
    {#if playing}
      {#await panelCode() then { default: PlayPanel }}
        <PlayPanel start={root} bind:played bind:high={panelHigh} onclose={() => (playing = false)} onturn={shown} />
      {/await}
    {/if}
    {#if families.size}
      <div class="legend" aria-label="What the arrows mean">
        {#each LEGEND.filter(([f]) => families.has(f)) as [f, label] (f)}
          <span><svg width="22" height="6" aria-hidden="true"><line class="edge f-{f}" x1="1" y1="3" x2="21" y2="3" /></svg>{label}</span>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .sg { position: absolute; inset: 0; display: flex; flex-direction: column; background: var(--bg); }
  /* the arrows' colours: validated apart for colour blindness on each theme's surface */
  .sg { --e-on: var(--ink-3); --e-says: #19679f; --e-else: #c0245e; --e-does: #7d5800; }
  @media (prefers-color-scheme: dark) {
    :global(:root:not([data-dd])) .sg { --e-says: #3d98d8; --e-else: #e8709c; --e-does: #b88f22; }
  }
  :global(:root[data-dd='starlight']) .sg { --e-says: #3d98d8; --e-else: #e8709c; --e-does: #b88f22; }

  .bar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; padding: 10px 16px; border-bottom: 1px solid var(--line); background: var(--surface); }
  .who { display: inline-flex; align-items: center; gap: 8px; min-width: 0; }
  .who .tile { width: 28px; height: 28px; border-radius: 9px; }
  .who h1 { margin: 0; font-size: 19px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .who .num { font: 600 12px var(--font-mono); color: var(--ink-3); white-space: nowrap; }
  .stats { display: flex; gap: 6px; flex-wrap: wrap; }
  .stats .warn { background: var(--warn-bg); color: var(--warn); }
  .end { margin-left: auto; display: flex; gap: 6px; }

  .canvas { flex: 1; overflow: auto; position: relative; background-image: radial-gradient(var(--line) 1.2px, transparent 1.3px); background-size: 22px 22px; }
  .sizer { position: relative; }
  .graph { position: absolute; left: 0; top: 0; transform-origin: 0 0; }
  .edges { position: absolute; left: 0; top: 0; overflow: visible; pointer-events: none; }
  .edge { fill: none; stroke-width: 2; transition: opacity .2s; }
  .f-on { stroke: var(--e-on); fill: var(--e-on); }
  .f-says { stroke: var(--e-says); fill: var(--e-says); }
  .f-else { stroke: var(--e-else); fill: var(--e-else); }
  .f-does { stroke: var(--e-does); fill: var(--e-does); }
  path.edge, line.edge { fill: none; }
  .edge.f-else { stroke-dasharray: 6 4; }
  marker path { stroke: none; }
  .edge.dim { opacity: .12; }
  .edge.went { stroke-width: 3.5; }

  .node {
    position: absolute; left: 0; top: 0; box-sizing: border-box;
    background: var(--surface); border: var(--stroke) solid var(--line-2); border-radius: 14px; box-shadow: 0 3px 0 var(--shadow);
    cursor: pointer; transition: transform .25s var(--ease), opacity .2s, border-color .15s, box-shadow .15s;
  }
  .node:hover, .node.lit { border-color: var(--outline); }
  .node:focus-visible { outline: none; box-shadow: 0 0 0 4px color-mix(in srgb, var(--focus) 40%, transparent), 0 3px 0 var(--shadow); }
  .node.open { border-color: var(--outline); box-shadow: 0 0 0 4px color-mix(in srgb, var(--t-bg) 90%, transparent), 0 3px 0 var(--shadow); }
  .node.dim { opacity: .35; }
  .node.trail { border-color: var(--outline); box-shadow: 0 0 0 4px color-mix(in srgb, var(--mint) 55%, transparent), 0 3px 0 var(--shadow); }
  .node.run { border-style: dashed; }
  .node.missing { border-color: var(--bad); background: var(--bad-bg); }
  .node.calm { transition: none; }
  @media (prefers-reduced-motion: reduce) { .node { transition: none; } }

  .nh { display: flex; align-items: center; gap: 7px; padding: 7px 10px; min-height: 36px; box-sizing: border-box; }
  .nh .tile { width: 22px; height: 22px; border-radius: 7px; }
  .nh b { font: 600 14.5px/1.15 var(--font-display); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .nn { margin-left: auto; font: 600 11px var(--font-mono); color: var(--ink-3); flex: none; }
  .nb { padding: 0 8px 9px; display: grid; gap: 2px; font-size: 12px; font-weight: 700; color: var(--ink-2); }
  .row { display: flex; align-items: flex-start; gap: 6px; padding: 3px 4px; border-radius: 7px; line-height: 1.35; min-width: 0; }
  .row b { color: var(--ink); }
  .gi { width: 18px; height: 18px; border-radius: 6px; display: grid; place-items: center; flex: none; background: var(--t-bg, var(--surface-3)); color: var(--t-ink, var(--ink-2)); }
  .said { min-width: 0; overflow-wrap: anywhere; }
  .rf { font-weight: 800; color: var(--t-ink, var(--ink)); }
  .coin { color: var(--butter-ink); }
  .times { color: var(--ink-3); }
  .step.skip { color: var(--ink-3); }
  .topic { align-items: center; }
  .topic .tag { height: 20px; font-size: 11.5px; max-width: 150px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .to { margin-left: auto; font: 600 11px var(--font-mono); color: var(--ink-3); }
  .more { color: var(--ink-3); }
  .typed { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; margin-top: 4px; padding: 0 4px; font-size: 11.5px; }
  .typed:first-child { margin-top: 0; }
  .typed code { font: 700 11.5px var(--font-mono); color: var(--pink-ink); background: var(--pink-2); border-radius: 6px; padding: 0 5px; white-space: pre; max-width: 100%; overflow: hidden; text-overflow: ellipsis; }
  .pill { font: 700 11px var(--font-mono); background: var(--lilac-2); color: var(--lilac-ink); border-radius: 999px; padding: 0 6px; flex: none; }
  .pill.never { background: var(--surface-3); color: var(--ink-3); }
  .note { margin: 0; padding: 2px 4px; font-size: 12px; color: var(--ink-3); }
  .note.bad { color: var(--bad); }
  .mini {
    font: 12px/1.3 var(--font-mono); background: var(--term-bg); color: var(--term-fg); border-radius: 9px; padding: 6px 8px;
    border: var(--stroke) solid var(--term-frame); white-space: pre-wrap; overflow-wrap: anywhere;
  }
  /* the first lines, as many as a glance takes in */
  .clamp { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 4; line-clamp: 4; overflow: hidden; }
  .mini.line { padding: 2px 6px; border-radius: 7px; flex: 1; min-width: 0; }
  .mini.line .clamp { -webkit-line-clamp: 2; line-clamp: 2; }
  .chat { align-items: center; }

  .legend {
    position: absolute; left: 14px; bottom: 14px; z-index: 2; display: flex; gap: 12px; flex-wrap: wrap; align-items: center;
    background: color-mix(in srgb, var(--surface) 92%, transparent); border: 1px solid var(--line); border-radius: 12px; padding: 6px 12px;
    font-size: 11.5px; font-weight: 800; color: var(--ink-2); pointer-events: none;
  }
  .legend span { display: inline-flex; align-items: center; gap: 5px; }
  .legend line { stroke-width: 2.5; }

  .empty { flex: 1; display: grid; place-content: center; justify-items: center; gap: 10px; text-align: center; font-weight: 700; color: var(--ink-2); }

  @media (max-width: 760px) {
    .stats { display: none; }
    .legend { display: none; }
    /* one line: the name gives way first */
    .bar { flex-wrap: nowrap; padding-left: 8px; }
    .who { flex: 1 1 auto; }
    .who .num, .wide { display: none; }
    .back { padding: 0 8px; }
  }
</style>
