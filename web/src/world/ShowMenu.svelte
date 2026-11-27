<!--
  The map's Show menu: what its rooms are coloured by, the layers drawn over them, the scripted
  links, its layout (laid out afresh, exported and imported) and what the marks mean. Its code
  loads after the map is first drawn (World.svelte), so the World's own stays small.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Download from '@lucide/svelte/icons/download';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Upload from '@lucide/svelte/icons/upload';
  import { api } from '../api';
  import { sfx } from '../shell/prefs.svelte';
  import { app } from '../shell/state.svelte';
  import { messageOf, toast } from '../shell/toasts.svelte';
  import { ICONS } from './icons';
  import { COLOUR_BY, EXIT_LEGEND, ROOM_LEGEND, type ColourBy, type ExitLook, type IconName, type Palette } from './look';
  import type { Layers } from './scene';
  import type { WireKind, WiresFor } from './wires';

  interface Props {
    map: number;
    colourBy: ColourBy;
    layers: Layers;
    wiresFor: WiresFor;
    /** How many of the map's rooms are drawn where they were moved to, and the room its layout starts from. */
    pinCount: number;
    startRoom: number;
    pal: Palette;
    oncolour: (by: ColourBy) => void;
    onlayer: (k: keyof Layers, on: boolean) => void;
    onwires: (v: WiresFor) => void;
    /** The map laid out by its exits again. */
    onlayout: () => void;
    onimport: (file: File) => void;
  }

  let { map, colourBy, layers, wiresFor, pinCount, startRoom, pal, oncolour, onlayer, onwires, onlayout, onimport }: Props = $props();

  let importer = $state<HTMLInputElement | undefined>();

  const LAYERS: [keyof Layers, string, string][] = [
    ['labels', 'Place names', 'One name for each run of rooms that share it'],
    ['symbols', 'Exit symbols and room marks', 'Doors, keys, traps, shops, NPCs and where far exits lead'],
    ['grid', 'Grid dots', 'The cells rooms sit on'],
  ];

  const WIRES_FOR: { id: WiresFor; label: string }[] = [
    { id: 'selected', label: 'Room selected' },
    { id: 'all', label: 'Every room' },
    { id: 'off', label: 'Off' },
  ];

  const WIRE_KEY: { kind: WireKind; icon: IconName; label: string }[] = [
    { kind: 'teleport', icon: 'wand', label: 'A script teleports players' },
    { kind: 'spell', icon: 'sparkles', label: 'A spell teleports players' },
    { kind: 'remote', icon: 'radio', label: 'A remote action works an exit' },
    { kind: 'death', icon: 'skull', label: 'Death room' },
    { kind: 'hangup', icon: 'log-out', label: 'Hang-up room' },
    { kind: 'control', icon: 'crosshair', label: 'Control room' },
  ];

  const exitColour = (l: ExitLook) => (l.sort ? pal.exit[l.sort] : pal.path);

  /** The export's name: the world's, as a file name. */
  const fileStem = () => (app.current?.label ?? 'world').split('(')[0]!.trim().replace(/[^A-Za-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '') || 'world';

  async function exportLayouts() {
    try {
      const f = await api.mapFile();
      const rooms = Object.values(f.maps).reduce((a, p) => a + Object.keys(p.pins).length, 0);
      if (!rooms) {
        toast('No map has been laid out by hand yet, so there is nothing to export.');
        return;
      }
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([JSON.stringify(f, null, 1)], { type: 'application/json' }));
      a.download = `${fileStem()}-map-layouts.json`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 2000);
      const maps = Object.keys(f.maps).length;
      toast(`Exported the layouts of ${maps === 1 ? 'one map' : `${maps} maps`} (${rooms.toLocaleString()} rooms).`);
    } catch (e) {
      sfx('nope');
      toast(messageOf(e), { kind: 'error' });
    }
  }
</script>

{#snippet glyph(name: IconName)}
  <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {#each ICONS[name] as [tag, attrs], i (i)}<svelte:element this={tag} {...attrs} />{/each}
  </svg>
{/snippet}

<div class="eyebrow head">Colour rooms by</div>
<div class="seg" role="radiogroup" aria-label="Colour rooms by">
  {#each COLOUR_BY as c (c.id)}
    <button type="button" role="radio" aria-checked={colourBy === c.id} class:on={colourBy === c.id} onclick={() => oncolour(c.id)}>{c.label}</button>
  {/each}
</div>
<div class="menu-sep"></div>
<div class="eyebrow head">Show on the map</div>
{#each LAYERS as [k, label, sub] (k)}
  <label class="menu-item">
    <span>{label}<span class="sub">{sub}</span></span>
    <span class="end"><input class="switch" type="checkbox" role="switch" checked={layers[k]} onchange={(e) => onlayer(k, e.currentTarget.checked)} /></span>
  </label>
{/each}
<div class="menu-sep"></div>
<div class="eyebrow head">Scripted links</div>
<div class="seg" role="radiogroup" aria-label="Scripted links">
  {#each WIRES_FOR as c (c.id)}
    <button type="button" role="radio" aria-checked={wiresFor === c.id} class:on={wiresFor === c.id} onclick={() => onwires(c.id)}>{c.label}</button>
  {/each}
</div>
<p class="lay">
  {#if wiresFor === 'off'}
    No dotted links.
  {:else}
    Dotted links from {wiresFor === 'all' ? 'every room' : 'the room selected'}: where its scripts and spells teleport players, and the exits its remote actions work{#if wiresFor === 'all'}, unless they scatter players over many rooms{/if}. The room selected shows its death, hang-up and control rooms too, and what leads to it.
  {/if}
</p>
<div class="menu-sep"></div>
<div class="eyebrow head">Layout</div>
<p class="lay">
  {#if pinCount}
    Map {map} is laid out by hand: rooms stay where they are drawn, and new rooms join them by their exits.
  {:else}
    Laid out by the exits, from room {startRoom}. In Build mode, drag a room (or press Shift and an arrow) to move it; the map then keeps its layout.
  {/if}
</p>
<div class="acts">
  {#if pinCount}
    <button type="button" class="btn sm" onclick={onlayout}><RotateCcw size={13} />Lay out by exits</button>
  {/if}
  <button type="button" class="btn sm" onclick={exportLayouts} title="Every map's layout, as a file another builder can import"><Download size={13} />Export</button>
  <button type="button" class="btn sm" onclick={() => importer?.click()} title="Layouts exported from Daydream: each map in the file takes its layout"><Upload size={13} />Import</button>
  <input
    bind:this={importer}
    type="file"
    accept=".json,application/json"
    hidden
    onchange={(e) => {
      const f = e.currentTarget.files?.[0];
      e.currentTarget.value = '';
      if (f) onimport(f);
    }}
  />
</div>
<div class="menu-sep"></div>
<div class="eyebrow head">What the marks mean</div>
<ul class="key">
  {#each ROOM_LEGEND as m (m.label)}
    <li><span class="mark room">{@render glyph(m.icon)}</span>{m.label}</li>
  {/each}
  <li>
    <span class="mark"><svg viewBox="0 0 28 14" width="28" height="14" aria-hidden="true"><path d="M2 7h24" stroke={pal.oneway} stroke-width="2" /><path d="M17 3l5 4-5 4z" fill={pal.oneway} /></svg></span>One-way exit
  </li>
  <li>
    <span class="mark"><svg viewBox="0 0 28 14" width="28" height="14" aria-hidden="true"><path d="M2 7h10" stroke={pal.path} stroke-width="2" /><rect x="12" y="2.5" width="14" height="9" rx="4.5" fill={pal.badge} stroke={pal.path} stroke-width="1.4" /></svg></span>Exit to a room drawn elsewhere
  </li>
  <li>
    <span class="mark"><svg viewBox="0 0 28 14" width="28" height="14" aria-hidden="true"><path d="M2 7h10" stroke={pal.portal} stroke-width="2" /><rect x="12" y="2.5" width="14" height="9" rx="4.5" fill={pal.portal} /></svg></span>Exit to another map
  </li>
  <li>
    <span class="mark"><svg viewBox="0 0 28 14" width="28" height="14" aria-hidden="true"><path d="M9 2l5 8H4z" fill={pal.ink} /><path d="M19 12l5-8H14z" fill={pal.ink} /></svg></span>Up and down
  </li>
  {#each WIRE_KEY as k (k.kind)}
    <li>
      <span class="mark wire" style:color={pal.wire[k.kind]}
        ><svg viewBox="0 0 28 14" width="28" height="14" aria-hidden="true"><path d="M3 11Q14 0 25 11" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-dasharray="0.1 4.6" /></svg
        >{@render glyph(k.icon)}</span
      >{k.label}
    </li>
  {/each}
  {#each EXIT_LEGEND as x (x.label)}
    <li>
      <span class="mark" style:color={exitColour(x.look)}>
        {#if x.look.icon}{@render glyph(x.look.icon)}{:else}<svg viewBox="0 0 28 14" width="28" height="14" aria-hidden="true"
            ><path d="M2 7h24" stroke="currentColor" stroke-width="2" />{#each Array(x.look.bars ?? 0) as _, i (i)}<path d="M{12 + i * 4} 2v10" stroke="currentColor" stroke-width="2.4" />{/each}</svg
          >{/if}
      </span>{x.label}
    </li>
  {/each}
</ul>

<style>
  .head { padding: 8px 10px 6px; }
  .lay { margin: 0 10px 8px; font-size: 12.5px; line-height: 1.4; color: var(--ink-2); max-width: 290px; }
  .acts { display: flex; flex-wrap: wrap; gap: 6px; margin: 0 10px 8px; }
  .acts .btn { gap: 5px; }

  .seg { display: flex; flex-wrap: wrap; gap: 2px; padding: 3px; margin: 0 6px 6px; border-radius: 12px; background: var(--surface-2); border: 1px solid var(--line); }
  .seg button { flex: 1 1 auto; min-height: 28px; padding: 0 10px; border: 0; border-radius: 9px; background: transparent; font-size: 12.5px; color: var(--ink-2); box-shadow: none; }
  .seg button:hover:not(.on) { background: var(--surface-3); }
  .seg button.on { background: var(--lilac-2); color: var(--lilac-ink); box-shadow: 0 0 0 1px var(--line-2); }

  .key { list-style: none; margin: 0; padding: 0 8px 6px; display: grid; grid-template-columns: 1fr 1fr; gap: 4px 10px; font-size: 12px; font-weight: 700; color: var(--ink-2); }
  .key li { display: flex; align-items: center; gap: 7px; min-width: 0; line-height: 1.25; }
  .mark { flex: none; width: 28px; height: 18px; display: grid; place-items: center; }
  .mark.room { width: 22px; height: 18px; border-radius: 6px; background: var(--surface-3); color: var(--ink); margin-inline: 3px; }
  /* a dotted arc, its symbol on a disc midway */
  .mark.wire > :global(svg) { grid-area: 1 / 1; }
  .mark.wire > :global(svg + svg) { background: var(--surface); border-radius: 50%; padding: 1px; box-sizing: content-box; }
</style>
