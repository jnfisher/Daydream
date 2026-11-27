<!--
  Two numbers of a record against its peers' (scatter.ts): a dot for each peer, opening it on a
  click, and this record in pink, named, where its inputs put it now (a dashed ring where it
  was saved, when they moved it). A weapon's Max Hit against its Speed, among every weapon of
  its type.
-->
<svelte:options css="injected" />

<script lang="ts">
  import { titleOf } from '../ansi';
  import type { Cell } from '../api';
  import { kindOf } from '../shell/kinds';
  import { recordHref } from '../shell/routes';
  import { shown, type Draft } from './draft.svelte';
  import type { ListRow } from './list';
  import { niceAxis, spots } from './scatter';

  let {
    draft,
    table,
    rows,
    x,
    y,
    xLabel = x,
    yLabel = y,
    note = '',
    says,
  }: {
    draft: Draft;
    table: string;
    /** The peers, as the list read them (x and y among its columns). */
    rows: ListRow[];
    x: string;
    y: string;
    xLabel?: string;
    yLabel?: string;
    /** How to read an axis: `lower speed is faster`. */
    note?: string;
    /** Who the dots are: `every 1H Sharp weapon`. */
    says: string;
  } = $props();

  const H = 230;
  const M = { l: 44, r: 16, t: 24, b: 40 };
  let w = $state(0);

  const num = (v: Cell | undefined) => Number(v) || 0;
  const one = $derived(kindOf(table).one.toLowerCase());
  const nameOf = (r: ListRow) => (r.name ? titleOf(r.name) : `${one} ${r.label}`);
  const points = $derived(rows.map((r) => ({ x: num(r.vals[x]), y: num(r.vals[y]), name: nameOf(r), key: r.key, label: r.label })));
  const dots = $derived(spots(points));
  const me = $derived({ x: num(draft.get(x)), y: num(draft.get(y)) });
  const was = $derived(draft.edited(x) || draft.edited(y) ? { x: num(draft.saved(x)), y: num(draft.saved(y)) } : null);
  const moved = $derived(!!was && (was.x !== me.x || was.y !== me.y));
  const myName = $derived((draft.table.title ? titleOf(shown(draft.get(draft.table.title))) : '') || 'this one');

  const all = $derived([...points, me, ...(was ? [was] : [])]);
  const ax = $derived(niceAxis(Math.min(...all.map((p) => p.x)), Math.max(...all.map((p) => p.x)), w < 420 ? 3 : 5));
  const ay = $derived(niceAxis(Math.min(...all.map((p) => p.y)), Math.max(...all.map((p) => p.y)), 4));
  const X = (v: number) => M.l + ((v - ax.lo) / (ax.hi - ax.lo || 1)) * (w - M.l - M.r);
  const Y = (v: number) => H - M.b - ((v - ay.lo) / (ay.hi - ay.lo || 1)) * (H - M.t - M.b);

  /** The dot the pointer or focus is on. */
  let hot = $state<number | null>(null);
  let svg = $state<SVGSVGElement>();

  // the pointer finds the nearest dot, so crowded dots are each easy to reach
  function near(e: PointerEvent) {
    if (!svg) return;
    const r = svg.getBoundingClientRect();
    const px = e.clientX - r.left;
    const py = e.clientY - r.top;
    let best: number | null = null;
    let bestD = 24 * 24;
    dots.forEach((d, i) => {
      const dd = (X(d.x) - px) ** 2 + (Y(d.y) - py) ** 2;
      if (dd < bestD) [best, bestD] = [i, dd];
    });
    hot = best;
  }
  function open() {
    if (hot !== null) location.hash = recordHref(table, dots[hot]!.all[0]!.key);
  }

  // one dot takes Tab; the arrow keys walk them left to right
  let links = $state<(HTMLOrSVGElement | undefined)[]>([]);
  const order = $derived(dots.map((_, i) => i).sort((a, b) => dots[a]!.x - dots[b]!.x || dots[b]!.y - dots[a]!.y));
  let tabAt = $state(-1);
  const tabbable = $derived(tabAt >= 0 && tabAt < dots.length ? tabAt : (order[0] ?? 0));
  function walk(e: KeyboardEvent, i: number) {
    const at = order.indexOf(i);
    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1, Home: -at, End: order.length - 1 - at }[e.key];
    const to = step === undefined ? undefined : order[at + step];
    if (to === undefined) return;
    e.preventDefault();
    tabAt = to;
    links[to]?.focus();
  }
  const tip = $derived(hot === null ? null : dots[hot]);
  /** A name label beside this record's dot: away from where it was saved, where there's room. */
  const labelLeft = $derived(X(me.x) > w - 140 || (moved && !!was && was.x > me.x && X(me.x) > M.l + 120));
  const fmt = (v: number) => v.toLocaleString();
  const many = (n: number) => (n === 1 ? '' : ` and ${n - 1} more`);
</script>

<figure class="sc">
  <figcaption class="sc-h">
    <span class="sc-t">{yLabel} against {xLabel}</span>
    <span class="key"><i class="k peer"></i>{says}</span>
    <span class="key"><i class="k me"></i>this one</span>
    {#if note}<span class="faint sc-note">{note}</span>{/if}
  </figcaption>
  <div class="plot" bind:clientWidth={w}>
    {#if w > 0}
      <svg bind:this={svg} width={w} height={H} role="group" aria-label="{rows.length} records at {dots.length} spots; the arrow keys walk them">
        {#each ay.ticks as t (t)}
          <line class="grid" x1={M.l} x2={w - M.r} y1={Y(t)} y2={Y(t)} />
          <text class="tk" x={M.l - 6} y={Y(t) + 4} text-anchor="end">{fmt(t)}</text>
        {/each}
        {#each ax.ticks as t (t)}
          <line class="grid" x1={X(t)} x2={X(t)} y1={M.t} y2={H - M.b} />
          <text class="tk" x={X(t)} y={H - M.b + 16} text-anchor="middle">{fmt(t)}</text>
        {/each}
        <text class="ax" x={w - M.r} y={H - 4} text-anchor="end">{xLabel} →</text>
        <text class="ax" x="4" y="11">↑ {yLabel}</text>

        {#each dots as d, i (`${d.x}/${d.y}`)}
          {@const first = d.all[0]!}
          <a
            bind:this={links[i]}
            href={recordHref(table, first.key)}
            class="pt"
            class:hot={hot === i}
            tabindex={i === tabbable ? 0 : -1}
            aria-label="{first.name}{many(d.all.length)}: {yLabel} {fmt(d.y)}, {xLabel} {fmt(d.x)}"
            onfocus={() => ((hot = i), (tabAt = i))}
            onblur={() => (hot = null)}
            onkeydown={(e) => walk(e, i)}
          >
            <circle class="dot" cx={X(d.x)} cy={Y(d.y)} r={hot === i ? 6 : d.all.length > 1 ? 5 : 4} />
          </a>
        {/each}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (each dot is a link for the keyboard) -->
        <rect
          class="catch"
          class:on={hot !== null}
          x={M.l - 12}
          y={M.t - 12}
          width={Math.max(0, w - M.l - M.r + 24)}
          height={H - M.t - M.b + 24}
          onpointermove={near}
          onpointerleave={() => (hot = null)}
          onclick={open}
        />

        {#if was && moved}
          <line class="moved" x1={X(was.x)} y1={Y(was.y)} x2={X(me.x)} y2={Y(me.y)} />
          <circle class="was" cx={X(was.x)} cy={Y(was.y)} r="6.5"><title>Where it was saved: {yLabel} {fmt(was.y)}, {xLabel} {fmt(was.x)}</title></circle>
        {/if}
        <circle class="me" cx={X(me.x)} cy={Y(me.y)} r="7"><title>{myName}: {yLabel} {fmt(me.y)}, {xLabel} {fmt(me.x)}</title></circle>
        <text class="me-l" x={X(me.x) + (labelLeft ? -12 : 12)} y={Y(me.y) + 4} text-anchor={labelLeft ? 'end' : 'start'}>{myName}</text>
      </svg>
      {#if tip}
        {@const left = X(tip.x)}
        <div class="tip" class:l={left < 110} class:r={left > w - 110} style:left="{left}px" style:top="{Y(tip.y) - 12}px" role="status">
          <strong>{yLabel} {fmt(tip.y)} · {xLabel} {fmt(tip.x)}</strong>
          {#each tip.all.slice(0, 4) as p (p.label)}<span>{p.name} <span class="faint">{p.label}</span></span>{/each}
          {#if tip.all.length > 4}<span class="faint">and {tip.all.length - 4} more</span>{/if}
        </div>
      {/if}
    {/if}
  </div>
</figure>

<style>
  .sc { margin: 12px 0 0; display: grid; gap: 6px; }
  .sc-h { display: flex; align-items: center; flex-wrap: wrap; gap: 4px 14px; font-size: 12.5px; font-weight: 800; color: var(--ink-2); }
  .sc-t { color: var(--ink); }
  .key { display: inline-flex; align-items: center; gap: 6px; }
  .k { width: 9px; height: 9px; border-radius: 50%; display: inline-block; }
  .k.peer { background: var(--dot); }
  .k.me { background: var(--dot-me); }
  .sc-note { font-weight: 700; }
  .plot { position: relative; min-width: 0; }
  svg { display: block; overflow: visible; }
  .grid { stroke: var(--line); stroke-width: 1; }
  .tk { font: 10.5px var(--font-mono); fill: var(--ink-3); }
  .ax { font: 800 11px var(--font-ui); fill: var(--ink-2); }
  .dot { fill: var(--dot); stroke: var(--surface); stroke-width: 2; transition: r .12s var(--ease); }
  .pt { pointer-events: none; outline: none; }
  .pt.hot .dot { stroke: var(--ink); stroke-width: 1.5; }
  .pt:focus-visible .dot { stroke: var(--focus); stroke-width: 3; }
  .catch { fill: transparent; }
  .catch.on { cursor: pointer; }
  .me, .me-l, .was, .moved { pointer-events: none; }
  .me { fill: var(--dot-me); stroke: var(--surface); stroke-width: 2; }
  .me-l { font: 800 12px var(--font-ui); fill: var(--ink); paint-order: stroke; stroke: var(--surface); stroke-width: 3px; stroke-linejoin: round; }
  .was { fill: none; stroke: var(--ink-3); stroke-width: 1.5; stroke-dasharray: 3 2; }
  .moved { stroke: var(--ink-3); stroke-width: 1.5; }
  .tip {
    position: absolute; z-index: 2; transform: translate(-50%, -100%); pointer-events: none; display: grid; gap: 1px;
    padding: 6px 9px; border-radius: 10px; background: var(--surface); border: var(--stroke) solid var(--outline); box-shadow: var(--float);
    font-size: 12px; font-weight: 700; white-space: nowrap; color: var(--ink);
  }
  .tip.l { transform: translate(-12px, -100%); }
  .tip.r { transform: translate(calc(-100% + 12px), -100%); }
  .tip strong { font-weight: 900; font-variant-numeric: tabular-nums; }
</style>
