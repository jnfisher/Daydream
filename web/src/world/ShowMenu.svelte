<!--
  The map's Show menu: what its rooms are coloured by, the map's style, the layers drawn over
  them, the scripted links, its layout (flat or in levels, laid out afresh, exported and
  imported) and what the marks mean. Its code loads after the map is first drawn (World.svelte), so the World's own
  stays small.
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
  import { COLOUR_BY, contrast, EXIT_LEGEND, MAP_STYLES, palette, ROOM_LEGEND, type ColourBy, type ExitLook, type IconName, type MapStyle, type Palette } from './look';
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
    /** Drawn in levels: Up and Down lead a level up and down. */
    levels: boolean;
    /** The map's palette, in its style; the style; and the theme's palette (the style `theme`). */
    pal: Palette;
    mapStyle: MapStyle;
    themePal: Palette;
    oncolour: (by: ColourBy) => void;
    onstyle: (s: MapStyle) => void;
    onlayer: (k: keyof Layers, on: boolean) => void;
    onwires: (v: WiresFor) => void;
    /** The map laid out by its exits again. */
    onlayout: () => void;
    /** The map drawn in levels, or flat. */
    onlevels: (on: boolean) => void;
    onimport: (file: File) => void;
  }

  let { map, colourBy, layers, wiresFor, pinCount, startRoom, levels, pal, mapStyle, themePal, oncolour, onstyle, onlayer, onwires, onlayout, onlevels, onimport }: Props = $props();

  let importer = $state<HTMLInputElement | undefined>();

  const LAYERS: [keyof Layers, string, string][] = [
    ['labels', 'Place names', 'One name for each run of rooms that share it'],
    ['symbols', 'Exit symbols and room marks', 'Doors, keys, traps, shops, NPCs and where far exits lead'],
    ['grid', 'The grid', 'The cells rooms sit on'],
    ['notes', 'Your notes', 'Notes, labels and regions, never published'],
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
  /** A room's marks on its plain colour: its outline's, or else what shows on it. */
  const roomInk = $derived(contrast(pal.line, pal.plain) >= 3 ? pal.line : pal.dark ? '#ffffff' : '#1a1636');
  const paletteOf = (s: MapStyle) => (s === 'theme' ? themePal : palette(s));

  /** The export's name: the world's, as a file name. */
  const fileStem = () => (app.current?.label ?? 'world').split('(')[0]!.trim().replace(/[^A-Za-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '') || 'world';

  async function exportLayouts() {
    try {
      const f = await api.mapFile();
      const rooms = Object.values(f.maps).reduce((a, p) => a + Object.keys(p.pins).length + Object.keys(p.level_pins ?? {}).length, 0);
      const notes = Object.values(f.maps).reduce((a, p) => a + (p.notes?.length ?? 0), 0);
      if (!rooms && !notes) {
        toast('No map has been laid out by hand or has notes yet, so there is nothing to export.');
        return;
      }
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([JSON.stringify(f, null, 1)], { type: 'application/json' }));
      a.download = `${fileStem()}-map-layouts.json`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 2000);
      const maps = Object.keys(f.maps).length;
      toast(`Exported the layouts and notes of ${maps === 1 ? 'one map' : `${maps} maps`} (${rooms.toLocaleString()} rooms${notes ? `, ${notes.toLocaleString()} notes` : ''}).`);
    } catch (e) {
      sfx('nope');
      toast(messageOf(e), { kind: 'error' });
    }
  }
</script>

{#snippet preview(p: Palette)}
  <!-- a corner of a map in the style: three rooms, a plain exit and a locked one, the second room selected -->
  <svg viewBox="0 0 84 40" aria-hidden="true">
    <rect width="84" height="40" fill={p.bg} />
    {#if p.grid === 'dots'}
      {#each [0, 1, 2, 3, 4, 5, 6] as i (i)}{#each [0, 1, 2] as j (j)}<rect x={i * 13 - 1.5} y={j * 13 + 6} width="1.6" height="1.6" fill={p.dot} />{/each}{/each}
    {:else if p.grid === 'lines'}
      <path d="M6 0v40M19 0v40M32 0v40M45 0v40M58 0v40M71 0v40M0 7h84M0 20h84M0 33h84" stroke={p.dot} stroke-opacity="0.1" />
    {/if}
    <path d="M19 13H45" stroke={p.path} stroke-width="2" />
    <path d="M45 13L65 27" stroke={p.exit.lock} stroke-width="2.2" />
    <rect x="12" y="6" width="14" height="14" rx={4 * p.round} fill={p.upDown ? p.plain : p.biome.street} stroke={p.line} stroke-width="1.2" />
    <rect x="58" y="20" width="14" height="14" rx={4 * p.round} fill={p.upDown ? '#4ee04e' : p.biome.forest} stroke={p.line} stroke-width="1.2" />
    <rect x="38" y="6" width="14" height="14" rx={4 * p.round} fill={p.upDown ? '#e8d020' : p.biome.shop} stroke={p.line} stroke-width="1.2" />
    <rect x="35.5" y="3.5" width="19" height="19" rx={6 * p.round} fill="none" stroke={p.sel} stroke-width="1.8" />
  </svg>
{/snippet}

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
<div class="eyebrow head">Map style</div>
<div class="styles" role="radiogroup" aria-label="Map style">
  {#each MAP_STYLES as m (m.id)}
    <button type="button" role="radio" aria-checked={mapStyle === m.id} class:on={mapStyle === m.id} title={m.sub} onclick={() => onstyle(m.id)}>
      {@render preview(paletteOf(m.id))}
      <span>{m.name}</span>
    </button>
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
<div class="seg" role="radiogroup" aria-label="Up and down">
  <button type="button" role="radio" aria-checked={!levels} class:on={!levels} onclick={() => onlevels(false)}>Flat</button>
  <button type="button" role="radio" aria-checked={levels} class:on={levels} onclick={() => onlevels(true)}>In levels</button>
</div>
<p class="lay">
  {#if levels}
    Up and down lead a level up and down, to rooms drawn in the same place: one level shows at a time, with the one below it faint. Press <span class="kbd">&lt;</span> and <span class="kbd">&gt;</span> for the next level.
  {:else}
    One plan: up and down are marks on their rooms, and the rooms only they reach are islands below.
  {/if}
  {#if pinCount}
    Map {map} is laid out by hand{levels ? ' in levels' : ''}: rooms stay where they are drawn, and new rooms join them by their exits.
  {:else}
    Laid out by the exits, from room {startRoom}. In Build mode, drag a room (or press Shift and an arrow) to move it; the map then keeps its layout{levels ? ' in levels' : ''}.
  {/if}
</p>
<div class="acts">
  {#if pinCount}
    <button type="button" class="btn sm" onclick={onlayout}><RotateCcw size={13} />Lay out by exits</button>
  {/if}
  <button type="button" class="btn sm" onclick={exportLayouts} title="Every map's layout and notes, as a file another builder can import"><Download size={13} />Export</button>
  <button type="button" class="btn sm" onclick={() => importer?.click()} title="Layouts exported from Daydream: each map in the file takes its layout and notes"><Upload size={13} />Import</button>
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
<ul class="key" style:--ground={pal.bg} style:--disc={pal.badge} style:--room-line={pal.line}>
  {#each ROOM_LEGEND as m (m.label)}
    <li><span class="mark room" style:background={pal.plain} style:color={roomInk}>{@render glyph(m.icon)}</span>{m.label}</li>
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
  .lay .kbd { height: 17px; font-size: 10.5px; }
  .acts { display: flex; flex-wrap: wrap; gap: 6px; margin: 0 10px 8px; }
  .acts .btn { gap: 5px; }

  .seg { display: flex; flex-wrap: wrap; gap: 2px; padding: 3px; margin: 0 6px 6px; border-radius: 12px; background: var(--surface-2); border: 1px solid var(--line); }
  .seg button { flex: 1 1 auto; min-height: 28px; padding: 0 10px; border: 0; border-radius: 9px; background: transparent; font-size: 12.5px; color: var(--ink-2); box-shadow: none; }
  .seg button:hover:not(.on) { background: var(--surface-3); }
  .seg button.on { background: var(--lilac-2); color: var(--lilac-ink); box-shadow: 0 0 0 1px var(--line-2); }

  .key { list-style: none; margin: 0; padding: 0 8px 6px; display: grid; grid-template-columns: 1fr 1fr; gap: 4px 10px; font-size: 12px; font-weight: 700; color: var(--ink-2); }
  .key li { display: flex; align-items: center; gap: 7px; min-width: 0; line-height: 1.25; }
  /* each mark on a scrap of the map's own ground, so its colours read as they do there */
  .mark { flex: none; width: 28px; height: 18px; display: grid; place-items: center; border-radius: 5px; background: var(--ground); }
  .mark.room { width: 22px; height: 18px; box-sizing: border-box; border: 1.5px solid var(--room-line); border-radius: 6px; margin-inline: 3px; box-shadow: 0 0 0 3px var(--ground); }
  /* a dotted arc, its symbol on a disc midway */
  .mark.wire > :global(svg) { grid-area: 1 / 1; }
  .mark.wire > :global(svg + svg) { background: var(--disc); border-radius: 50%; padding: 1px; box-sizing: content-box; }

  .styles { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; margin: 0 8px 8px; }
  .styles button {
    display: flex; flex-direction: column; align-items: stretch; gap: 4px; min-height: 0; padding: 4px 4px 5px; border-radius: 12px;
    border: 1.5px solid var(--line); background: var(--surface); box-shadow: none; font-size: 11.5px; font-weight: 800; color: var(--ink-2); white-space: normal;
  }
  .styles button:hover:not(.on) { background: var(--surface-2); }
  .styles button.on { border-color: var(--focus); color: var(--ink); box-shadow: 0 0 0 2px color-mix(in srgb, var(--focus) 30%, transparent); }
  .styles :global(svg) { display: block; width: 100%; height: auto; border-radius: 8px; border: 1px solid var(--line); }
  .styles span { line-height: 1.15; text-align: center; }
</style>
