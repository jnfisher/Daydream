<!--
  All maps: every map as a disc, sized by its rooms and named by the name most of them share,
  and the map change exits between maps as lines, thicker for more exits, with an arrow where
  they go one way only. A disc opens its map, and a line the room its first exit is in, on the
  map nearer where it was clicked. Scrolling in over a disc opens it too. The arrow keys go from
  disc to disc; W, Esc or the X go back into the map. Loaded the first time it is opened (W on
  the map, All maps in its list, or zooming out past a map).
-->
<!-- loaded on its own, so its styles travel with its code -->
<svelte:options css="injected" />

<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Waypoints from '@lucide/svelte/icons/waypoints';
  import X from '@lucide/svelte/icons/x';
  import type { Palette } from './look';
  import { mapName, type MapLink, type MapName } from './model';
  import { worldLayout, type WorldEdge, type WorldNode } from './worldmap';

  interface Props {
    /** Every map with rooms, and how many. */
    maps: [number, number][];
    names: MapName[];
    links: MapLink[];
    /** The map open beneath. */
    current: number;
    pal: Palette;
    /** Opens a map, at a room or as it was last seen. */
    onopen: (map: number, room: number | null) => void;
    /** Back into the map beneath. */
    onclose: () => void;
  }
  let { maps, names, links, current, pal, onopen, onclose }: Props = $props();

  /** What the title along the top and the key along the bottom leave clear, in pixels. */
  const [TOP, BOTTOM, SIDE] = [76, 60, 24];

  let root: HTMLDivElement;
  let w = $state(0);
  let h = $state(0);
  /** The box's shape when it opened: the picture is turned to it once, and keeps still after. */
  let aspect = $state(0);
  $effect(() => {
    if (!aspect && w > 0 && h > 0) aspect = Math.max(1, w - 2 * SIDE) / Math.max(1, h - TOP - BOTTOM);
  });
  const layout = $derived(aspect ? worldLayout(maps, links, aspect) : null);
  const named = $derived(new Map(names.map((n) => [n.map, n])));
  const rooms = $derived(maps.reduce((s, [, n]) => s + n, 0));
  const exits = $derived(links.reduce((s, l) => s + l.exits, 0));

  // the picture's scale, and where its middle goes
  const view = $derived.by(() => {
    const b = layout?.bounds;
    if (!b || w <= 0 || h <= 0) return null;
    const k = Math.min(Math.max(1, w - 2 * SIDE) / Math.max(1, b.x1 - b.x0), Math.max(1, h - TOP - BOTTOM) / Math.max(1, b.y1 - b.y0), 1.5);
    return { k, ox: (w - (b.x1 - b.x0) * k) / 2 - b.x0 * k, oy: TOP + (h - TOP - BOTTOM - (b.y1 - b.y0) * k) / 2 - b.y0 * k };
  });

  interface Disc {
    node: WorldNode;
    x: number;
    y: number;
    r: number;
    name: string | null;
  }
  const discs = $derived.by((): Disc[] => {
    const v = view;
    if (!v || !layout) return [];
    return layout.nodes.map((n) => ({ node: n, x: v.ox + n.x * v.k, y: v.oy + n.y * v.k, r: Math.max(9, n.r * v.k), name: n.missing ? null : mapName(named, n.map, n.rooms) }));
  });
  const discOf = $derived(new Map(discs.map((d) => [d.node.map, d])));

  interface Line {
    edge: WorldEdge;
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    width: number;
    /** The way it goes when it goes one way only. */
    way: 'ab' | 'ba' | null;
    /** Its arrowhead, on the disc it leads to. */
    arrow: string;
  }
  const lines = $derived.by((): Line[] =>
    (layout?.edges ?? []).flatMap((edge) => {
      const [a, b] = [discOf.get(edge.a), discOf.get(edge.b)];
      if (!a || !b) return [];
      const [dx, dy] = [b.x - a.x, b.y - a.y];
      const d = Math.hypot(dx, dy) || 1;
      const [ux, uy] = [dx / d, dy / d];
      let [x1, y1, x2, y2] = [a.x + ux * (a.r + 3), a.y + uy * (a.r + 3), b.x - ux * (b.r + 3), b.y - uy * (b.r + 3)];
      const width = Math.min(7, 1.5 + 0.9 * Math.log2((edge.ab?.exits ?? 0) + (edge.ba?.exits ?? 0)));
      const way = edge.ab && edge.ba ? null : edge.ab ? 'ab' : 'ba';
      let arrow = '';
      if (way) {
        const s = 6 + width * 1.3;
        const [tx, ty, sx, sy] = way === 'ab' ? [x2, y2, -ux, -uy] : [x1, y1, ux, uy];
        const [bx, by] = [tx + sx * s, ty + sy * s];
        const [px, py] = [-sy * s * 0.55, sx * s * 0.55];
        arrow = `M${tx},${ty}L${bx + px},${by + py}L${bx - px},${by - py}Z`;
        if (way === 'ab') [x2, y2] = [tx + sx * s * 0.7, ty + sy * s * 0.7];
        else [x1, y1] = [tx + sx * s * 0.7, ty + sy * s * 0.7];
      }
      return [{ edge, x1, y1, x2, y2, width, way, arrow }];
    }),
  );

  // ---- what the pointer (or the keyboard) is on ----------------------------------------

  /** A disc, or a line and which of its maps the pointer is nearer. */
  type On = { map: number } | { edge: WorldEdge; near: 'a' | 'b' };
  let on = $state.raw<On | null>(null);
  /** Where the tip goes: the pointer, or beside a disc the keyboard is on. */
  let tipAt = $state<[number, number] | null>(null);
  const onMap = $derived(on && 'map' in on ? on.map : null);
  const onEdge = $derived(on && 'edge' in on ? on.edge : null);
  /** The maps a disc pointed at is joined to. */
  const near = $derived.by(() => {
    const out = new Set<number>();
    if (onMap === null) return out;
    for (const e of layout?.edges ?? []) {
      if (e.a === onMap) out.add(e.b);
      if (e.b === onMap) out.add(e.a);
    }
    return out;
  });
  const lit = (e: WorldEdge) => (onMap !== null ? e.a === onMap || e.b === onMap : onEdge === e);
  const faded = (m: number) => (onMap !== null ? m !== onMap && !near.has(m) : onEdge ? m !== onEdge.a && m !== onEdge.b : false);

  const plural = (n: number, one: string) => `${n.toLocaleString()} ${one}${n === 1 ? '' : 's'}`;
  const list = (ns: number[]) => (ns.length < 2 ? ns.join('') : `${ns.slice(0, -1).join(', ')} and ${ns.at(-1)}`);

  /** The exits a click on a line follows: those from the map the pointer is nearer, if any go that way. */
  function chosen(e: WorldEdge, nearer: 'a' | 'b'): MapLink {
    return (nearer === 'a' ? (e.ab ?? e.ba) : (e.ba ?? e.ab))!;
  }

  const tip = $derived.by((): { title: string; lines: string[] } | null => {
    if (onMap !== null) {
      const d = discOf.get(onMap);
      if (!d) return null;
      const n = d.node;
      if (n.missing) return { title: `Map ${n.map}`, lines: ['This world has no rooms on it: an add-on’s map, perhaps. Map changes lead here.'] };
      const common = named.get(n.map);
      const out = [common && d.name ? `${plural(n.rooms, 'room')}, ${common.rooms.toLocaleString()} of them ${common.name}.` : `${plural(n.rooms, 'room')}.`];
      const joined = [...near].sort((p, q) => p - q);
      out.push(joined.length ? `Map changes join it to ${joined.length === 1 ? 'map' : 'maps'} ${list(joined)}.` : 'No map change leads here or away.');
      if (n.map === current) out.push('The map open now.');
      return { title: d.name ? `Map ${n.map} · ${d.name}` : `Map ${n.map}`, lines: out };
    }
    if (onEdge && on && 'near' in on) {
      const e = onEdge;
      const way = (l: MapLink) => `Map ${l.from} to map ${l.to}: ${plural(l.exits, 'exit')}, the first in room ${l.from}/${l.room}.`;
      const out = [e.ab, e.ba].filter((l): l is MapLink => !!l).map(way);
      if (!e.ab || !e.ba) out.push('One way: no map change leads back.');
      const go = chosen(e, on.near);
      out.push(`Click for room ${go.from}/${go.room}.`);
      const title = e.ab && e.ba ? `Maps ${e.a} and ${e.b}` : `Map ${go.from} to map ${go.to}`;
      return { title, lines: out };
    }
    return null;
  });

  function pointed(e: PointerEvent, next: On) {
    const r = root.getBoundingClientRect();
    tipAt = [e.clientX - r.left, e.clientY - r.top];
    on = next;
  }

  function overLine(e: PointerEvent, l: Line) {
    const r = root.getBoundingClientRect();
    const [px, py] = [e.clientX - r.left, e.clientY - r.top];
    // which end of the line the pointer is nearer
    const t = ((px - l.x1) * (l.x2 - l.x1) + (py - l.y1) * (l.y2 - l.y1)) / ((l.x2 - l.x1) ** 2 + (l.y2 - l.y1) ** 2 || 1);
    const nearer = t < 0.5 ? 'a' : 'b';
    if (!(on && 'edge' in on && on.edge === l.edge && on.near === nearer)) on = { edge: l.edge, near: nearer };
    tipAt = [px, py];
  }

  function off() {
    on = null;
    tipAt = null;
  }

  // ---- going into a map --------------------------------------------------------------

  /** Where the picture zooms into as it goes: the disc picked. */
  let leaving = $state<[number, number] | null>(null);

  function open(m: number, room: number | null) {
    if (leaving) return;
    const d = discOf.get(m);
    leaving = d ? [d.x, d.y] : [w / 2, h / 2];
    off();
    onopen(m, room);
  }

  /** Back into the map beneath (W or Esc). */
  export function back(): void {
    if (leaving) return;
    const d = discOf.get(current);
    leaving = d ? [d.x, d.y] : [w / 2, h / 2];
    off();
    onclose();
  }

  // scrolling in over a disc opens it, once the wheel has gone in far enough
  let inward = 0;
  let inwardAt = 0;
  function wheel(e: WheelEvent) {
    e.preventDefault();
    const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? h : 1;
    const f = -e.deltaY * unit * (e.ctrlKey ? 0.01 : 0.0018);
    const now = performance.now();
    if (now - inwardAt > 500 || f <= 0) inward = 0;
    inwardAt = now;
    if (f <= 0) return;
    inward += f;
    if (inward < 0.35) return;
    inward = 0;
    const r = root.getBoundingClientRect();
    const [px, py] = [e.clientX - r.left, e.clientY - r.top];
    const d = discs.find((c) => Math.hypot(c.x - px, c.y - py) <= c.r + 4);
    if (d && !d.node.missing) open(d.node.map, null);
  }

  // ---- keys ----------------------------------------------------------------------------

  const ARROWS: Record<string, [number, number]> = { ArrowRight: [1, 0], ArrowLeft: [-1, 0], ArrowDown: [0, 1], ArrowUp: [0, -1] };

  /** The arrow keys go to the disc that way. */
  function keys(e: KeyboardEvent) {
    const v = ARROWS[e.key];
    if (!v || e.altKey || e.ctrlKey || e.metaKey) return;
    e.preventDefault();
    const from = discOf.get(Number((document.activeElement as HTMLElement | null)?.dataset?.map));
    // the first arrow starts at the map open beneath (or the first, if it has no rooms)
    if (!from) {
      focusDisc(discOf.has(current) ? current : (discs[0]?.node.map ?? current));
      return;
    }
    let best: Disc | null = null;
    let score = Infinity;
    for (const d of discs) {
      if (d === from) continue;
      const [dx, dy] = [d.x - from.x, d.y - from.y];
      const ahead = dx * v[0] + dy * v[1];
      if (ahead <= 0) continue;
      const dist = Math.hypot(dx, dy);
      // the nearest one most nearly that way
      const s = dist * (1 + 2 * (1 - ahead / dist));
      if (s < score) [best, score] = [d, s];
    }
    if (best) focusDisc(best.node.map);
  }

  function focusDisc(m: number) {
    root.querySelector<HTMLButtonElement>(`.disc[data-map="${m}"]`)?.focus();
  }

  /** A disc the keyboard is on is pointed at, as the pointer would. */
  function focused(e: FocusEvent, d: Disc) {
    if (!(e.currentTarget as HTMLElement).matches(':focus-visible')) return;
    on = { map: d.node.map };
    tipAt = [d.x + d.r * 0.75, d.y + d.r * 0.4];
  }

  onMount(() => {
    root.addEventListener('wheel', wheel, { passive: false });
    root.addEventListener('keydown', keys);
    // the keyboard comes here: the arrows start at the map open beneath
    void tick().then(() => root.focus({ preventScroll: true }));
    return () => {
      root.removeEventListener('wheel', wheel);
      root.removeEventListener('keydown', keys);
    };
  });
</script>

<div
  bind:this={root}
  bind:clientWidth={w}
  bind:clientHeight={h}
  class="world-view"
  role="region"
  aria-label="All maps"
  tabindex="-1"
  style:--path={pal.path}
  style:--oneway={pal.oneway}
  style:--lit={pal.sel}
>
  {#if view}
    {@const from = leaving ?? (discOf.get(current) ? [discOf.get(current)!.x, discOf.get(current)!.y] : [w / 2, h / 2])}
    <div class="stage" class:leaving={!!leaving} style:transform-origin="{from[0]}px {from[1]}px">
      <svg class="lines" width={w} height={h} aria-hidden="true">
        {#each lines as l (`${l.edge.a}>${l.edge.b}`)}
          <g class="line" class:one={!!l.way} class:lit={lit(l.edge)} class:faded={!!on && !lit(l.edge)}>
            <line x1={l.x1} y1={l.y1} x2={l.x2} y2={l.y2} stroke-width={lit(l.edge) ? l.width + 1.5 : l.width} />
            {#if l.arrow}<path d={l.arrow} />{/if}
          </g>
        {/each}
        <!-- wide and unseen, to point at a line with -->
        {#each lines as l (`hit ${l.edge.a}>${l.edge.b}`)}
          <line
            class="hit"
            role="presentation"
            x1={l.x1}
            y1={l.y1}
            x2={l.x2}
            y2={l.y2}
            stroke-width={Math.max(14, l.width + 10)}
            onpointermove={(e) => overLine(e, l)}
            onpointerleave={off}
            onclick={() => {
              const go = chosen(l.edge, on && 'near' in on && on.edge === l.edge ? on.near : 'a');
              open(go.from, go.room);
            }}
          />
        {/each}
      </svg>
      {#each discs as d (d.node.map)}
        {@const n = d.node}
        <button
          type="button"
          class="disc"
          class:here={n.map === current}
          class:missing={n.missing}
          class:faded={faded(n.map)}
          class:lit={onMap === n.map}
          data-map={n.map}
          style:left="{d.x}px"
          style:top="{d.y}px"
          style:--r="{d.r}px"
          aria-label="Map {n.map}{d.name ? `, ${d.name}` : ''}, {n.missing ? 'not in this world' : plural(n.rooms, 'room')}{n.map === current ? ', open now' : ''}"
          aria-disabled={n.missing}
          onpointerenter={(e) => pointed(e, { map: n.map })}
          onpointermove={(e) => pointed(e, { map: n.map })}
          onpointerleave={off}
          onfocus={(e) => focused(e, d)}
          onblur={off}
          onclick={() => !n.missing && open(n.map, null)}
        >
          <b class="tnum">{n.map}</b>
          {#if d.name && d.r >= 30}<span class="name">{d.name}</span>{/if}
          {#if !n.missing && d.r >= 46}<small class="tnum">{plural(n.rooms, 'room')}</small>{/if}
          {#if n.missing && d.r >= 22}<small>not here</small>{/if}
        </button>
      {/each}
    </div>
  {/if}

  <div class="head">
    <button type="button" class="btn glass close" onclick={back} title="Back to map {current} (W or Esc)" aria-label="Back to map {current}"><X size={16} /></button>
    <div class="glass title">
      <span class="ic"><Waypoints size={15} /></span>
      <b>All maps</b>
      <small class="tnum">{plural(maps.length, 'map')}<span class="more"> · {plural(rooms, 'room')} · {plural(exits, 'map change')}</span></small>
    </div>
  </div>

  <div class="key glass" aria-hidden="true">
    <span><svg width="26" height="10"><line x1="1" y1="5" x2="25" y2="5" stroke={pal.path} stroke-width="3" /></svg>both ways</span>
    <span><svg width="26" height="10"><line x1="1" y1="5" x2="17" y2="5" stroke={pal.oneway} stroke-width="3" /><path d="M25,5L16,0.5L16,9.5Z" fill={pal.oneway} /></svg>one way</span>
    <span>thicker: more exits</span>
    <span class="wide-only"><span class="kbd">←↑→↓</span> go between maps <span class="kbd">Enter</span> open <span class="kbd">W</span> back</span>
  </div>

  {#if tip && tipAt}
    <div class="tip" class:left={tipAt[0] > w - 300} class:up={tipAt[1] > h - 130} style:left="{tipAt[0]}px" style:top="{tipAt[1]}px" role="tooltip">
      <b>{tip.title}</b>
      {#each tip.lines as t}<span>{t}</span>{/each}
    </div>
  {/if}
</div>

<style>
  .world-view {
    position: absolute; inset: 0; z-index: 9; overflow: hidden; background: var(--bg); outline: none;
    --edge: 12px;
  }
  .stage { position: absolute; inset: 0; animation: arrive 0.3s var(--ease) both; }
  .stage.leaving { animation: none; transition: transform 0.2s ease-in, opacity 0.2s ease-in; transform: scale(2.4); opacity: 0; pointer-events: none; }
  @keyframes arrive { from { transform: scale(2.2); opacity: 0; } }
  @media (prefers-reduced-motion: reduce) {
    .stage { animation: fade-in 0.12s both; }
    .stage.leaving { transform: none; }
    @keyframes fade-in { from { opacity: 0; } }
  }

  .lines { position: absolute; inset: 0; overflow: visible; }
  .line line { stroke: var(--path); stroke-linecap: round; transition: opacity 0.12s; }
  .line path { fill: var(--path); }
  .line.one line { stroke: var(--oneway); }
  .line.one path { fill: var(--oneway); }
  .line.lit line { stroke: var(--lit); }
  .line.lit path { fill: var(--lit); }
  .line.faded { opacity: 0.2; }
  .hit { stroke: transparent; cursor: pointer; pointer-events: stroke; }

  .disc {
    position: absolute; width: calc(var(--r) * 2); height: calc(var(--r) * 2); min-height: 0; transform: translate(-50%, -50%);
    border-radius: 50%; padding: 0 4px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1px;
    background: var(--surface); color: var(--ink); border: 1.5px solid var(--outline); box-shadow: 0 3px 0 var(--shadow);
    font: inherit; text-align: center; line-height: 1.1; cursor: pointer;
    transition: opacity 0.12s, transform 0.12s var(--pop), background 0.12s;
  }
  .disc b { font: 700 clamp(11px, calc(var(--r) * 0.42), 24px) var(--font-display); }
  .disc .name {
    max-width: calc(var(--r) * 1.55); font-size: clamp(10px, calc(var(--r) * 0.2), 12.5px); font-weight: 800; color: var(--ink-2); white-space: normal;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere;
  }
  .disc small { font-size: 10.5px; font-weight: 700; color: var(--ink-3); }
  .disc:hover:not(.missing), .disc.lit:not(.missing) { transform: translate(-50%, -50%) scale(1.05); background: var(--surface-2); }
  .disc:focus-visible { outline: 3px solid var(--focus); outline-offset: 3px; }
  .disc.here { background: var(--mint-2); box-shadow: 0 3px 0 var(--shadow), 0 0 0 4px var(--bg), 0 0 0 7px var(--mint); }
  .disc.here b, .disc.here .name { color: var(--mint-ink); }
  .disc.missing { background: transparent; border-style: dashed; border-color: var(--bad); box-shadow: none; color: var(--bad); cursor: default; }
  .disc.missing small { color: var(--bad); }
  .disc.faded { opacity: 0.45; }

  .head { position: absolute; z-index: 1; top: var(--edge); left: var(--edge); right: var(--edge); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; pointer-events: none; }
  .head > * { pointer-events: auto; }
  .glass {
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid var(--line-2);
    box-shadow: 0 2px 0 color-mix(in srgb, var(--shadow) 12%, transparent), var(--float);
  }
  .close { width: 36px; padding: 0 !important; justify-content: center; border-radius: 50% !important; }
  .title { height: 36px; display: flex; align-items: center; gap: 8px; padding: 0 14px 0 6px; border-radius: 999px; font-size: 14px; min-width: 0; }
  .title .ic { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; background: var(--lilac-2); color: var(--lilac-ink); flex: none; }
  .title small { font-weight: 700; color: var(--ink-3); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  @media (max-width: 560px) { .title .more { display: none; } }

  .key {
    position: absolute; z-index: 1; left: var(--edge); bottom: var(--edge); max-width: calc(100% - 2 * var(--edge));
    display: flex; align-items: center; flex-wrap: wrap; gap: 4px 14px; padding: 7px 12px; border-radius: 12px;
    font-size: 12px; font-weight: 700; color: var(--ink-2);
  }
  .key > span { display: inline-flex; align-items: center; gap: 6px; }
  .key .kbd { height: 18px; font-size: 10.5px; }
  @media (max-width: 760px), (pointer: coarse) { .key .wide-only { display: none; } }

  .tip {
    position: absolute; z-index: 2; pointer-events: none; transform: translate(14px, 14px);
    max-width: 280px; padding: 7px 10px; border-radius: 10px; display: flex; flex-direction: column; gap: 2px;
    background: var(--surface); border: 1.5px solid var(--outline); box-shadow: 0 3px 0 var(--shadow);
    font-size: 12.5px; line-height: 1.3;
  }
  .tip.left { transform: translate(calc(-100% - 14px), 14px); }
  .tip.up { transform: translate(14px, calc(-100% - 14px)); }
  .tip.left.up { transform: translate(calc(-100% - 14px), calc(-100% - 14px)); }
  .tip span { color: var(--ink-3); font-weight: 700; font-size: 11.5px; }
</style>
