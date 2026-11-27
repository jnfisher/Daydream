<!--
  The stamp drawer, beside the map in Build mode (a sheet along the bottom of a small screen): the
  shapes to put down (keys 1 to 9 pick the first nine), then the stamps saved from the map with
  Select, and for the one held, its size, turn and mirror, a fresh roll of a random one, whether
  it joins the rooms it touches, leaves the rooms already there alone (Shift) or replaces the
  exits in its way (Alt), the room its new rooms copy, and what putting it down where it is would
  do. A saved one held can be deleted (Ctrl+Z brings it back).
-->
<svelte:options css="injected" />

<script lang="ts">
  import ArrowLeftRight from '@lucide/svelte/icons/arrow-left-right';
  import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down';
  import Dices from '@lucide/svelte/icons/dices';
  import Minus from '@lucide/svelte/icons/minus';
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import X from '@lucide/svelte/icons/x';
  import { kindOf, saved } from './saved.svelte';
  import type { Stamper } from './stamper.svelte';
  import { STAMPS, type Shape, type ShapeLink, type StampKind } from './stamps';

  let {
    stamper: st,
    colours,
    onclose,
  }: {
    /** The stamp held, and what it would do where it is aimed. */
    stamper: Stamper;
    /** The map's colours for a stamp's cells and links. */
    colours: { fresh: string; join: string; clash: string; replace: string; kept: string };
    onclose: () => void;
  } = $props();

  const kind = $derived(st.kind);
  const sizes = $derived(st.sizes);
  /** Leave the rooms already there alone, and replace the exits in the way: switched on or held down. */
  const keep = $derived(st.keep || st.mods.shift);
  const overwrite = $derived(st.overwrite || st.mods.alt);
  /** A finger, not a mouse: tap to aim, tap again to put it down, and no keys to show. */
  const touch = $derived(st.host.coarse);
  /** The room new rooms copy, for a person. */
  const brush = $derived(st.brush === null ? 'the room picked' : st.host.called(st.brush));

  /**
   * A shape as a little drawing, 40 by 28: its cells, its first (its way in) filled, and lines for
   * its links (dashed up and down). A big one (a stamp saved from a map) is drawn as its outline.
   */
  function preview(s: Shape) {
    const xs = s.cells.map(([x]) => x);
    const ys = s.cells.map(([, y]) => y);
    const [x0, y0] = [Math.min(...xs), Math.min(...ys)];
    const w = Math.max(...xs) - x0 + 1;
    const h = Math.max(...ys) - y0 + 1;
    const at = (i: number) => [(s.cells[i]![0] - x0) * 10 + 5, (s.cells[i]![1] - y0) * 10 + 5] as const;
    const path = (links: ShapeLink[]) =>
      links
        .map((l) => {
          const [[x1, y1], [x2, y2]] = [at(l.a), at(l.b)];
          return `M${x1} ${y1}L${x2} ${y2}`;
        })
        .join('');
    // a pixel, in the drawing's units, once it is fitted in
    const px = Math.max((w * 10) / 40, (h * 10) / 28);
    const big = px > 2.2;
    const box = ([x, y]: readonly [number, number]) => (big ? `M${x - 3} ${y - 3}h6v6h-6z` : `M${x - 1.4} ${y - 3}h2.8a1.6 1.6 0 0 1 1.6 1.6v2.8a1.6 1.6 0 0 1-1.6 1.6h-2.8a1.6 1.6 0 0 1-1.6-1.6v-2.8a1.6 1.6 0 0 1 1.6-1.6z`);
    const [fx, fy] = at(0);
    const r = 2.6 * px;
    return {
      box: `0 0 ${w * 10} ${h * 10}`,
      big,
      lines: path(s.links.filter((l) => l.dir < 8)),
      stairs: path(s.links.filter((l) => l.dir > 7)),
      cells: s.cells.slice(1).map((_, i) => box(at(i + 1))).join(''),
      first: big ? `M${fx - r} ${fy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0z` : box([fx, fy]),
    };
  }
  const PREVIEWS = new Map(STAMPS.map((k) => [k, preview(k.make(k.sizes.map((z) => z.start), 1))]));
  const drawn = new WeakMap<StampKind, ReturnType<typeof preview>>();
  /** A shape's drawing, drawn once. */
  function previewOf(k: StampKind) {
    let p = PREVIEWS.get(k) ?? drawn.get(k);
    if (!p) drawn.set(k, (p = preview(k.make([], 1))));
    return p;
  }

  /** The stamps saved from the map, after the drawer's own. */
  const mine = $derived(saved.list.map(kindOf));

  // a saved stamp held, deleted (or its save undone): put away
  $effect(() => {
    const id = kind?.saved;
    if (id && saved.loaded && !saved.list.some((s) => s.id === id)) st.pick(null);
  });

  // the shape held, in view (Put it down, in Select, picks a saved one, at the end of the row on a phone)
  let shapes = $state<HTMLElement | null>(null);
  $effect(() => {
    if (kind) shapes?.querySelector('.shape.on')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  });
</script>

{#snippet tile(k: StampKind, i: number)}
  {@const p = previewOf(k)}
  <button type="button" role="option" aria-selected={kind?.id === k.id} class="shape" class:on={kind?.id === k.id} title="{k.name}: {k.about}" onclick={() => st.pick(kind?.id === k.id ? null : k)}>
    {#if i < 9 && !touch}<span class="key">{i + 1}</span>{/if}
    <svg viewBox={p.box} class:big={p.big} aria-hidden="true">
      {#if p.lines}<path class="ln" d={p.lines} />{/if}
      {#if p.stairs}<path class="ln ud" d={p.stairs} />{/if}
      {#if p.cells}<path class="cl" d={p.cells} />{/if}
      <path class="cl first" d={p.first} />
    </svg>
    <span class="name">{k.name}</span>
  </button>
{/snippet}

<section class="drawer" aria-label="Stamps" style:--fresh={colours.fresh} style:--join={colours.join} style:--clash={colours.clash} style:--replace={colours.replace} style:--kept={colours.kept}>
  <header>
    <b>Stamps</b>
    <span class="faint">{kind ? (touch ? 'Tap to aim, tap again to put it down' : 'Click the map to put it down') : touch ? 'Pick a shape' : 'Pick a shape (1 to 9)'}</span>
    <button type="button" class="btn icon sm ghost" onclick={onclose} title="Put the stamps away (S)" aria-label="Put the stamps away"><X size={15} /></button>
  </header>
  <div class="shapes" role="listbox" aria-label="Shapes" bind:this={shapes}>
    {#each STAMPS as k, i (k.id)}{@render tile(k, i)}{/each}
    {#if mine.length}
      <span class="group" role="presentation">Saved</span>
      {#each mine as k (k.id)}{@render tile(k, Infinity)}{/each}
    {/if}
  </div>
  {#if saved.loaded && !mine.length}
    <p class="faint hint">Your own: pick rooms with Select{touch ? '' : ' (V)'}, then save their shape as a stamp.</p>
  {/if}
  {#if kind}
    <div class="held">
      <p class="about"><b>{kind.name}</b> {kind.about.toLowerCase()}.</p>
      <p class="said" class:faint={!st.said} aria-live="polite">{st.said || (touch ? 'Tap the map to see what it would do there.' : 'Point at the map to see what it would do there.')}</p>
      {#each kind.sizes as z, i (z.word)}
        <div class="size">
          {#if !touch}<span class="faint">{i ? '{ }' : '[ ]'}</span>{/if}
          <button type="button" class="btn icon sm" disabled={sizes[i]! <= z.min} onclick={() => st.size(i, -1)} aria-label="Smaller"><Minus size={14} /></button>
          <span class="n tnum">{sizes[i]}</span>
          <button type="button" class="btn icon sm" disabled={sizes[i]! >= z.max} onclick={() => st.size(i, 1)} aria-label="Bigger"><Plus size={14} /></button>
          <span>{z.word}</span>
        </div>
      {/each}
      <div class="acts">
        <button type="button" class="btn sm" onclick={() => st.rotate()} title="Turn it a quarter clockwise (R)"><RotateCw size={14} />Turn</button>
        <button type="button" class="btn sm" onclick={() => st.flip('x')} title="Mirror it left to right (X)"><ArrowLeftRight size={14} />X</button>
        <button type="button" class="btn sm" onclick={() => st.flip('y')} title="Mirror it top to bottom (Y)"><ArrowUpDown size={14} />Y</button>
        {#if kind.random}<button type="button" class="btn sm" onclick={() => st.roll()} title="Another one (Space)"><Dices size={14} />Roll</button>{/if}
        {#if kind.saved}<button type="button" class="btn sm ghost" onclick={() => void st.drop()} title="Delete this stamp{touch ? '' : ' (Ctrl+Z brings it back)'}"><Trash2 size={14} />Delete</button>{/if}
      </div>
      <div class="switches">
        <label>
          <input class="switch" type="checkbox" role="switch" checked={st.seam} onchange={(e) => st.setSeam(e.currentTarget.checked)} />
          <span>Join the rooms it touches</span>{#if !touch}<span class="kbd">J</span>{/if}
        </label>
        <label>
          <input class="switch" type="checkbox" role="switch" checked={keep} onchange={(e) => st.setKeep(e.currentTarget.checked)} />
          <span>Leave rooms under it alone</span>{#if !touch}<span class="kbd">Shift</span>{/if}
        </label>
        <label>
          <input class="switch" type="checkbox" role="switch" checked={overwrite} onchange={(e) => st.setOverwrite(e.currentTarget.checked)} />
          <span>Replace the exits in its way</span>{#if !touch}<span class="kbd">Alt</span>{/if}
        </label>
      </div>
      <p class="brush">New rooms copy <b>{brush}</b>, without its exits, items, coins and NPC.</p>
      <ul class="keys faint">
        <li><span class="sw fresh"></span>new room or exit</li>
        <li><span class="sw join"></span>a room it joins</li>
        {#if overwrite}
          <li><span class="sw replace"></span>an exit in the way: replaced</li>
        {:else}
          <li><span class="sw clash"></span>an exit in the way: left</li>
        {/if}
        <li><span class="sw kept"></span>left as it is</li>
      </ul>
      {#if !touch}<p class="faint tip">Shift and Alt held down switch those on for a moment. Arrows move it, Enter puts it down, Esc puts it away.</p>{/if}
    </div>
  {/if}
</section>

<style>
  .drawer {
    position: absolute; z-index: 6; left: var(--edge); top: calc(var(--edge) + 50px);
    width: 284px; max-height: calc(100% - 2 * var(--edge) - 120px); overflow: auto;
    border-radius: 18px; padding: 8px; display: flex; flex-direction: column; gap: 8px;
    background: color-mix(in srgb, var(--surface) 92%, transparent); backdrop-filter: blur(8px);
    border: 1px solid var(--line-2); box-shadow: 0 2px 0 color-mix(in srgb, var(--shadow) 12%, transparent), var(--float);
  }
  header { display: flex; align-items: center; gap: 8px; padding: 0 0 0 6px; }
  header b { font-size: 14px; }
  header .faint { flex: 1; font-size: 11.5px; font-weight: 700; }
  /* the drawer scrolls as a whole: its parts keep their height */
  header, .shapes, .hint, .held { flex: none; }
  .shapes { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 4px; }
  .shape {
    position: relative; display: flex; flex-direction: column; align-items: center; gap: 2px;
    min-height: 0; padding: 6px 2px 4px; border-radius: 11px; border: 1px solid transparent;
    background: transparent; box-shadow: none; color: var(--ink-2);
  }
  .shape:hover { background: var(--surface-3); }
  .shape.on { background: var(--mint-2); border-color: var(--outline); color: var(--ink); }
  .shape svg { width: 40px; height: 28px; overflow: visible; }
  .shape .ln { fill: none; stroke: var(--ink-3); stroke-width: 1.6; stroke-linecap: round; }
  .shape .ln.ud { stroke-dasharray: 2 2; }
  .shape .cl { fill: var(--surface); stroke: var(--ink-2); stroke-width: 1.1; }
  .shape .cl.first { fill: var(--mint-ink); stroke: var(--mint-ink); }
  /* a big one, fitted in: lines a pixel wide, and its cells a shadow under them */
  .shape svg.big path { vector-effect: non-scaling-stroke; }
  .shape svg.big .ln { stroke-width: 1; }
  .shape svg.big .cl { fill: var(--ink-3); stroke: none; opacity: 0.45; }
  .shape svg.big .cl.first { fill: var(--mint-ink); opacity: 1; }
  .group { grid-column: 1 / -1; padding: 6px 6px 0; font-size: 10.5px; font-weight: 800; letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink-3); }
  .hint { margin: 0 6px; font-size: 11.5px; font-weight: 700; line-height: 1.35; }
  .shape .name { font-size: 10.5px; font-weight: 800; line-height: 1.1; text-align: center; text-wrap: balance; overflow-wrap: anywhere; }
  .shape .key {
    position: absolute; top: 2px; left: 3px; font: 700 9.5px var(--font-mono); color: var(--ink-3);
  }
  .held { display: flex; flex-direction: column; gap: 7px; padding: 8px 6px 4px; border-top: 1px dashed var(--line); }
  .about { margin: 0; font-size: 12.5px; line-height: 1.35; color: var(--ink-2); }
  .about b { color: var(--ink); }
  .size { display: flex; align-items: center; gap: 6px; font-size: 12.5px; font-weight: 700; }
  .size .faint { font: 700 10.5px var(--font-mono); min-width: 22px; }
  .size .n { min-width: 22px; text-align: center; font-weight: 800; }
  .acts { display: flex; flex-wrap: wrap; gap: 5px; }
  .acts .btn { gap: 4px; }
  .switches { display: flex; flex-direction: column; gap: 6px; }
  .switches label { display: flex; align-items: center; gap: 8px; font-size: 12.5px; font-weight: 700; }
  .switches span:not(.kbd) { flex: 1; }
  .switches .kbd { height: 18px; font-size: 10px; color: var(--ink-3); }
  .brush, .said, .tip { margin: 0; font-size: 12px; line-height: 1.35; }
  .brush { color: var(--ink-2); }
  /* two lines kept for it, so the controls below stay put as it changes */
  .said { padding: 6px 8px; border-radius: 10px; background: var(--surface-2); border: 1px solid var(--line); font-weight: 700; min-height: calc(2 * 1.35em + 12px); }
  .keys { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 3px 8px; font-size: 11px; font-weight: 700; }
  .keys li { display: flex; align-items: center; gap: 5px; }
  .sw { width: 11px; height: 11px; border-radius: 3px; flex: none; border: 2px solid; }
  .sw.fresh { border-color: var(--fresh); background: color-mix(in srgb, var(--fresh) 40%, transparent); }
  .sw.join { border-color: var(--join); }
  .sw.clash { border-color: var(--clash); border-style: dashed; }
  .sw.replace { border-color: var(--replace); background: color-mix(in srgb, var(--replace) 30%, transparent); }
  .sw.kept { border-color: var(--kept); border-style: dotted; }
  @media (max-width: 700px) {
    .drawer { top: auto; bottom: calc(var(--edge) + 56px); left: var(--edge); right: var(--edge); width: auto; max-height: 46%; }
    /* one row of shapes to swipe along, so the one held has room below */
    .shapes { grid-template-columns: none; grid-auto-flow: column; grid-auto-columns: max-content; overflow-x: auto; overscroll-behavior-x: contain; scrollbar-width: thin; }
    .shape { width: 66px; }
    /* in the one row, a narrow mark before the saved ones */
    .group { grid-column: auto; padding: 0 2px; align-self: center; writing-mode: vertical-rl; transform: rotate(180deg); }
    .about { display: none; }
  }
</style>
