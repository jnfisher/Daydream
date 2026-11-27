<!--
  The stamp drawer, beside the map in Build mode (a sheet along the bottom of a small screen): the
  shapes to put down (keys 1 to 9 pick the first nine), and for the one held, its size, turn and
  mirror, a fresh roll of a random one, whether it joins the rooms it touches, leaves the rooms
  already there alone (Shift) or replaces the exits in its way (Alt), the room its new rooms copy,
  and what putting it down where it is would do.
-->
<svelte:options css="injected" />

<script lang="ts">
  import ArrowLeftRight from '@lucide/svelte/icons/arrow-left-right';
  import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down';
  import Dices from '@lucide/svelte/icons/dices';
  import Minus from '@lucide/svelte/icons/minus';
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import X from '@lucide/svelte/icons/x';
  import { STAMPS, type Shape, type StampKind } from './stamps';

  let {
    kind,
    sizes,
    seam,
    keep,
    overwrite,
    touch,
    brush,
    said,
    colours,
    onpick,
    onsize,
    onturn,
    onflip,
    onroll,
    onseam,
    onkeep,
    onoverwrite,
    onclose,
  }: {
    kind: StampKind | null;
    sizes: number[];
    seam: boolean;
    /** Leave the rooms already there alone, and replace the exits in the way: switched on or held down. */
    keep: boolean;
    overwrite: boolean;
    /** A finger, not a mouse: tap to aim, tap again to put it down, and no keys to show. */
    touch: boolean;
    /** The room new rooms copy, for a person. */
    brush: string;
    /** What putting the stamp down where it is would do. */
    said: string;
    /** The map's colours for a stamp's cells and links. */
    colours: { fresh: string; join: string; clash: string; replace: string; kept: string };
    onpick: (k: StampKind | null) => void;
    onsize: (i: number, by: number) => void;
    onturn: () => void;
    onflip: (axis: 'x' | 'y') => void;
    onroll: () => void;
    onseam: (on: boolean) => void;
    onkeep: (on: boolean) => void;
    onoverwrite: (on: boolean) => void;
    onclose: () => void;
  } = $props();

  /** A shape as a little drawing: its cells, and lines for its links (dashed up and down). */
  function preview(s: Shape) {
    const xs = s.cells.map(([x]) => x);
    const ys = s.cells.map(([, y]) => y);
    const [x0, y0] = [Math.min(...xs), Math.min(...ys)];
    const w = Math.max(...xs) - x0 + 1;
    const h = Math.max(...ys) - y0 + 1;
    const at = (i: number) => [(s.cells[i]![0] - x0) * 10 + 5, (s.cells[i]![1] - y0) * 10 + 5] as const;
    return {
      box: `0 0 ${w * 10} ${h * 10}`,
      lines: s.links.filter((l) => l.dir < 8).map((l) => [...at(l.a), ...at(l.b)] as const),
      stairs: s.links.filter((l) => l.dir > 7).map((l) => [...at(l.a), ...at(l.b)] as const),
      cells: s.cells.map((_, i) => at(i)),
    };
  }
  const PREVIEWS = STAMPS.map((k) => preview(k.make(k.sizes.map((z) => z.start), 1)));
</script>

<section class="drawer" aria-label="Stamps" style:--fresh={colours.fresh} style:--join={colours.join} style:--clash={colours.clash} style:--replace={colours.replace} style:--kept={colours.kept}>
  <header>
    <b>Stamps</b>
    <span class="faint">{kind ? (touch ? 'Tap to aim, tap again to put it down' : 'Click the map to put it down') : touch ? 'Pick a shape' : 'Pick a shape (1 to 9)'}</span>
    <button type="button" class="btn icon sm ghost" onclick={onclose} title="Put the stamps away (S)" aria-label="Put the stamps away"><X size={15} /></button>
  </header>
  <div class="shapes" role="listbox" aria-label="Shapes">
    {#each STAMPS as k, i (k.id)}
      {@const p = PREVIEWS[i]!}
      <button type="button" role="option" aria-selected={kind?.id === k.id} class="shape" class:on={kind?.id === k.id} title="{k.name}: {k.about}" onclick={() => onpick(kind?.id === k.id ? null : k)}>
        {#if i < 9 && !touch}<span class="key">{i + 1}</span>{/if}
        <svg viewBox={p.box} aria-hidden="true">
          {#each p.lines as [x1, y1, x2, y2], j (j)}<line {x1} {y1} {x2} {y2} />{/each}
          {#each p.stairs as [x1, y1, x2, y2], j (j)}<line class="ud" {x1} {y1} {x2} {y2} />{/each}
          {#each p.cells as [x, y], j (j)}<rect x={x - 3} y={y - 3} width="6" height="6" rx="1.6" class:first={j === 0} />{/each}
        </svg>
        <span class="name">{k.name}</span>
      </button>
    {/each}
  </div>
  {#if kind}
    <div class="held">
      <p class="about"><b>{kind.name}</b> {kind.about.toLowerCase()}.</p>
      <p class="said" class:faint={!said} aria-live="polite">{said || (touch ? 'Tap the map to see what it would do there.' : 'Point at the map to see what it would do there.')}</p>
      {#each kind.sizes as z, i (z.word)}
        <div class="size">
          {#if !touch}<span class="faint">{i ? '{ }' : '[ ]'}</span>{/if}
          <button type="button" class="btn icon sm" disabled={sizes[i]! <= z.min} onclick={() => onsize(i, -1)} aria-label="Smaller"><Minus size={14} /></button>
          <span class="n tnum">{sizes[i]}</span>
          <button type="button" class="btn icon sm" disabled={sizes[i]! >= z.max} onclick={() => onsize(i, 1)} aria-label="Bigger"><Plus size={14} /></button>
          <span>{z.word}</span>
        </div>
      {/each}
      <div class="acts">
        <button type="button" class="btn sm" onclick={onturn} title="Turn it a quarter clockwise (R)"><RotateCw size={14} />Turn</button>
        <button type="button" class="btn sm" onclick={() => onflip('x')} title="Mirror it left to right (X)"><ArrowLeftRight size={14} />X</button>
        <button type="button" class="btn sm" onclick={() => onflip('y')} title="Mirror it top to bottom (Y)"><ArrowUpDown size={14} />Y</button>
        {#if kind.random}<button type="button" class="btn sm" onclick={onroll} title="Another one (Space)"><Dices size={14} />Roll</button>{/if}
      </div>
      <div class="switches">
        <label>
          <input class="switch" type="checkbox" role="switch" checked={seam} onchange={(e) => onseam(e.currentTarget.checked)} />
          <span>Join the rooms it touches</span>{#if !touch}<span class="kbd">J</span>{/if}
        </label>
        <label>
          <input class="switch" type="checkbox" role="switch" checked={keep} onchange={(e) => onkeep(e.currentTarget.checked)} />
          <span>Leave rooms under it alone</span>{#if !touch}<span class="kbd">Shift</span>{/if}
        </label>
        <label>
          <input class="switch" type="checkbox" role="switch" checked={overwrite} onchange={(e) => onoverwrite(e.currentTarget.checked)} />
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
  header, .shapes, .held { flex: none; }
  .shapes { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 4px; }
  .shape {
    position: relative; display: flex; flex-direction: column; align-items: center; gap: 2px;
    min-height: 0; padding: 6px 2px 4px; border-radius: 11px; border: 1px solid transparent;
    background: transparent; box-shadow: none; color: var(--ink-2);
  }
  .shape:hover { background: var(--surface-3); }
  .shape.on { background: var(--mint-2); border-color: var(--outline); color: var(--ink); }
  .shape svg { width: 40px; height: 28px; overflow: visible; }
  .shape line { stroke: var(--ink-3); stroke-width: 1.6; stroke-linecap: round; }
  .shape line.ud { stroke-dasharray: 2 2; }
  .shape rect { fill: var(--surface); stroke: var(--ink-2); stroke-width: 1.1; }
  .shape rect.first { fill: var(--mint-ink); stroke: var(--mint-ink); }
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
    .shapes { grid-template-columns: none; grid-auto-flow: column; grid-auto-columns: 66px; overflow-x: auto; overscroll-behavior-x: contain; scrollbar-width: thin; }
    .about { display: none; }
  }
</style>
