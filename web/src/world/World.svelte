<!--
  The World: the open world's rooms as a map you look around in. Drag to pan, scroll or pinch
  to zoom; click a room to open its card beside the map, and the arrow keys (or the number pad)
  walk its exits. The rooms can be coloured by place, area, monsters, light or your own edits,
  and the legend picks one colour out. F shows the map on the whole screen.
-->
<!-- loaded when the World is first opened, so its styles travel with its code -->
<svelte:options css="injected" />

<script lang="ts">
  import { Popover } from 'bits-ui';
  import { onMount, untrack } from 'svelte';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import LayersIcon from '@lucide/svelte/icons/layers';
  import LocateFixed from '@lucide/svelte/icons/locate-fixed';
  import MapIcon from '@lucide/svelte/icons/map';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import Minus from '@lucide/svelte/icons/minus';
  import Plus from '@lucide/svelte/icons/plus';
  import Scan from '@lucide/svelte/icons/scan';
  import X from '@lucide/svelte/icons/x';
  import { api } from '../api';
  import { read, sfx, write } from '../shell/prefs.svelte';
  import { app, closeInspector, inspect } from '../shell/state.svelte';
  import { messageOf, toast, toasts } from '../shell/toasts.svelte';
  import { ICONS } from './icons';
  import { COLOUR_BY, EXIT_LEGEND, palette, ROOM_LEGEND, type ColourBy, type ExitLook, type IconName } from './look';
  import { DIR_WORDS, kindName, type MapRoom } from './model';
  import { Scene, type Camera, type Hit, type Layers, type LegendEntry, type Stub } from './scene';

  let canvas: HTMLCanvasElement;
  let mini: HTMLCanvasElement;
  let scene: Scene | null = null;

  let map = $state(0);
  let maps = $state<[number, number][]>([]);
  let count = $state(0);
  let loading = $state(false);
  let failed = $state('');
  let selected = $state<MapRoom | null>(null);
  let colourBy = $state<ColourBy>(colourFrom(read('dd-map-colour', 'place')));
  let layers = $state<Layers>(layersFrom(read('dd-map-layers', '')));
  let spot = $state<string[]>([]);
  let legend = $state<LegendEntry[]>([]);
  let allLegend = $state(false);
  let zoom = $state(1);
  let tip = $state<{ x: number; y: number; title: string; sub: string } | null>(null);
  let trail = $state<{ map: number; room: number | null }[]>([]);
  let focus = $state(false);
  let live = $state('');
  let note = $state('');
  let card = $state(0);
  let width = $state(0);
  let mapsOpen = $state(false);
  let mapFilter = $state('');
  let pal = $state(palette());
  /** The rooms changed since the realm was read: written meanwhile, so read again when shown. */
  let stale = false;
  /** Where the camera was on each map, so coming back to one finds it as it was left. */
  const cams = new Map<number, Camera>();


  function colourFrom(v: string): ColourBy {
    return COLOUR_BY.some((c) => c.id === v) ? (v as ColourBy) : 'place';
  }

  function layersFrom(v: string): Layers {
    const l: Layers = { labels: true, grid: true, symbols: true };
    try {
      const o: unknown = JSON.parse(v || '{}');
      if (o && typeof o === 'object') for (const k of Object.keys(l) as (keyof Layers)[]) if (typeof (o as Record<string, unknown>)[k] === 'boolean') l[k] = (o as Record<string, boolean>)[k]!;
    } catch {
      // the defaults
    }
    return l;
  }

  /** Whether the hint fits along the bottom, between the map's edge and the overview. */
  const roomy = $derived(width - card >= 900);
  /** How many of the legend's colours show before "more" (none on a phone: they would cover the map). */
  const legendShown = $derived(width && width < 700 ? 0 : 7);

  const lastKey = () => `dd-map:${app.current?.realm_dir ?? ''}`;
  const here = () => app.route.place === 'world' && app.route.rest[0] !== 'classic';

  /** Resolved once the canvas is set up (mount() returns before it is). */
  let ready = () => {};
  const mounted = new Promise<void>((r) => (ready = r));

  /** Shows the World at a route's map and room (`#/world/2/14`). */
  export async function show(rest: string[], fresh: boolean): Promise<void> {
    await mounted;
    const m = Number(rest[0]) || map || Number(read(lastKey(), '1')) || 1;
    const n = rest[1] ? Number(rest[1]) : NaN;
    let fitted = false;
    if (m !== map || fresh || stale || !scene?.layout) fitted = await load(m, m === map);
    // a map seen for the first time opens on the room asked for close enough to see it
    if (Number.isFinite(n) && n !== selected?.room) choose(n, fitted ? 'close' : 'centre');
    else if (Number.isFinite(n)) history.replaceState(null, '', `#/world/${map}/${n}`);
  }

  /** Reads a map's rooms. Returns whether the camera was moved to show them all. */
  async function load(m: number, keep: boolean): Promise<boolean> {
    if (!scene) return false;
    if (map && map !== m) cams.set(map, { ...scene.cam });
    loading = true;
    let fitted = false;
    try {
      const a = await api.map(m);
      const moved = a.map !== map;
      if (moved) {
        scene.select(null);
        selected = null;
      }
      map = a.map;
      maps = a.maps;
      count = a.rooms.length;
      write(lastKey(), String(map));
      scene.setEdited(edited);
      scene.setRooms(a.rooms);
      stale = false;
      failed = '';
      if (moved || !keep) {
        const c = cams.get(map);
        if (c) scene.setCamera(c);
        else {
          scene.fitMain(false);
          fitted = true;
        }
      }
      // the room may have been renamed or removed
      selected = selected ? (scene.by.get(selected.room) ?? null) : null;
      legend = scene.legend();
    } catch (e) {
      failed = messageOf(e);
    } finally {
      loading = false;
    }
    return fitted;
  }

  /**
   * Selects a room: its card opens beside the map and the address names it. `centre` puts it
   * in the middle, `close` in the middle close enough to see it, and `keep` only moves the map
   * when the room is near its edge, as walking does.
   */
  function choose(n: number | null, how: 'centre' | 'close' | 'keep' | 'none' = 'none') {
    if (!scene) return;
    const r = n === null ? null : (scene.by.get(n) ?? null);
    scene.select(r?.room ?? null);
    selected = r;
    history.replaceState(null, '', r ? `#/world/${map}/${r.room}` : `#/world/${map}`);
    if (!r) return;
    card = scene.setCover(true);
    inspect({ table: 'Rooms', key: [map, r.room], name: r.name }, true);
    if (how === 'centre') scene.centreOn(r.room);
    else if (how === 'close') scene.centreOn(r.room, { zoom: Math.max(scene.cam.z, 1.1), animate: false });
    else if (how === 'keep') scene.keepInView(r.room);
    live = `${r.name || 'Room'}, room ${map}/${r.room}. ${exitsSaid(r)}`;
  }

  function exitsSaid(r: MapRoom): string {
    if (!r.exits.length) return 'No exits.';
    return `Exits ${r.exits.map((e) => DIR_WORDS[e.dir] ?? '?').join(', ')}.`;
  }

  /** A note in the hint bar for a moment (a toast when the hint bar has no room). */
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  function say(text: string) {
    live = text;
    if (!roomy) {
      if (!toasts.some((t) => t.text === text)) toast(text, { ms: 2600 });
      return;
    }
    note = text;
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => (note = ''), 2600);
  }

  /** Goes to another map (and room), remembering where it came from. */
  function go(m: number, n: number | null) {
    if (m === map && n !== null) {
      choose(n, 'centre');
      return;
    }
    trail = [...trail, { map, room: selected?.room ?? null }].slice(-8);
    location.hash = n === null ? `#/world/${m}` : `#/world/${m}/${n}`;
  }

  function back() {
    const t = trail.at(-1);
    if (!t) return;
    trail = trail.slice(0, -1);
    location.hash = t.room === null ? `#/world/${t.map}` : `#/world/${t.map}/${t.room}`;
  }

  function walk(d: number) {
    if (!scene?.layout) return;
    const r = selected;
    if (!r) {
      choose(scene.start, 'centre');
      return;
    }
    const e = r.exits.find((x) => x.dir === d);
    if (!e) {
      sfx('nope');
      say(`No exit ${DIR_WORDS[d]} from ${r.name || `room ${r.room}`}.`);
      return;
    }
    if (e.kind === 8) {
      go(e.para1, e.to);
      return;
    }
    if (!scene.by.has(e.to)) {
      sfx('nope');
      say(`The exit ${DIR_WORDS[d]} leads to room ${e.to}, which map ${map} does not have.`);
      return;
    }
    choose(e.to, 'keep');
  }

  function follow(s: Stub) {
    const e = s.exit;
    if (s.sort === 'map') go(e.para1, e.to);
    else if (s.sort === 'missing') {
      sfx('nope');
      say(`Map ${map} has no room ${e.to}.`);
    } else choose(e.to, 'centre');
  }

  const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

  function tipFor(h: Hit): { title: string; sub: string } {
    if (h.stub) {
      const e = h.stub.exit;
      const way = cap(DIR_WORDS[e.dir] ?? 'An exit');
      const kind = e.kind ? `${kindName(e.kind)} exit` : 'Exit';
      if (h.stub.sort === 'map') return { title: `${way} to map ${e.para1}`, sub: `${kind} to room ${e.para1}/${e.to}. Click to go there.` };
      if (h.stub.sort === 'missing') return { title: `${way} to room ${e.to}`, sub: `Map ${map} has no room ${e.to}.` };
      const t = scene?.by.get(e.to);
      return { title: `${way} to ${t?.name || `room ${e.to}`}`, sub: `${kind} to room ${map}/${e.to}, drawn elsewhere. Click to go there.` };
    }
    const r = h.room!;
    return { title: r.name || `Room ${r.room}`, sub: `Room ${map}/${r.room}` };
  }

  // ---- what the map shows --------------------------------------------------------------

  function setColour(by: ColourBy) {
    if (by === colourBy) return;
    colourBy = by;
    write('dd-map-colour', by);
    spot = [];
    allLegend = false;
    scene?.setColour(by);
    legend = scene?.legend() ?? [];
    sfx('toggle');
  }

  function setLayer(k: keyof Layers, on: boolean) {
    layers = { ...layers, [k]: on };
    write('dd-map-layers', JSON.stringify(layers));
    scene?.setLayers(layers);
    sfx('toggle');
  }

  function toggleSpot(key: string) {
    spot = spot.includes(key) ? spot.filter((k) => k !== key) : [...spot, key];
    scene?.setSpot(spot);
    sfx('toggle');
  }

  function clearSpot() {
    spot = [];
    scene?.setSpot(spot);
  }

  const shownLegend = $derived(allLegend ? legend : legend.slice(0, legendShown));

  /** The rooms of this map changed since the realm was read or published. */
  const edited = $derived.by(() => {
    const s = new Set<number>();
    for (const c of app.changes?.changes ?? []) {
      if (c.nmr === 'Rooms' && c.key && Number(c.key[0]) === map && c.how !== 'deleted') s.add(Number(c.key[1]));
    }
    return s;
  });

  $effect(() => {
    const e = edited;
    untrack(() => {
      if (!scene) return;
      scene.setEdited(e);
      if (colourBy === 'edits') legend = scene.legend();
    });
  });

  // the card beside the map: the map's middle moves left of it, and the map follows the room it shows
  $effect(() => {
    const top = app.inspecting.at(-1);
    const open = app.inspecting.length > 0;
    untrack(() => {
      if (!scene) return;
      card = scene.setCover(open);
      scene.draw();
      if (!top || top.table !== 'Rooms' || !here()) return;
      const [m, n] = [Number(top.key[0]), Number(top.key[1])];
      if (m === map && n === selected?.room) return;
      if (m === map) choose(n, 'centre');
      else go(m, n);
    });
  });

  // full screen ends when the World is left
  $effect(() => {
    if (app.route.place !== 'world' && untrack(() => focus)) untrack(() => setFocus(false));
  });

  const filteredMaps = $derived(mapFilter.trim() ? maps.filter(([m]) => String(m).startsWith(mapFilter.trim())) : maps);

  function setFocus(on: boolean) {
    focus = on;
    document.body.classList.toggle('world-focus', on);
    try {
      if (on && !document.fullscreenElement) void document.documentElement.requestFullscreen?.().catch(() => {});
      else if (!on && document.fullscreenElement) void document.exitFullscreen().catch(() => {});
    } catch {
      // the page may not be allowed full screen: the map still fills the window
    }
    if (on) say('Full screen. F or Esc to come back.');
  }

  // ---- the keyboard ------------------------------------------------------------------

  const KEY_DIR: Record<string, number> = {
    ArrowUp: 0, ArrowDown: 1, ArrowRight: 2, ArrowLeft: 3, PageUp: 4, Home: 5, PageDown: 6, End: 7,
    Numpad8: 0, Numpad2: 1, Numpad6: 2, Numpad4: 3, Numpad9: 4, Numpad7: 5, Numpad3: 6, Numpad1: 7,
  };

  function onKey(e: KeyboardEvent) {
    if (!here() || e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return;
    if (app.paletteOpen || document.querySelector('dialog[open], [role=dialog]')) return;
    const t = e.target;
    if (t instanceof HTMLElement) {
      if (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)) return;
      // the card, the bar and the rail keep their own keys
      if (t.closest('#dd-inspector, #dd-top, #dd-rail, .pop')) return;
    }
    if (e.key === 'Escape') {
      if (focus) setFocus(false);
      return;
    }
    const d = e.shiftKey ? undefined : (KEY_DIR[e.code] ?? KEY_DIR[e.key]);
    if (d !== undefined) {
      e.preventDefault();
      walk(d);
      return;
    }
    const k = e.key.toLowerCase();
    let done = true;
    if (e.shiftKey && e.code === 'Digit1') scene?.fit();
    else if (e.shiftKey && e.code === 'Digit2') {
      if (selected) scene?.frameRoom(selected.room);
    } else if (e.shiftKey) done = false;
    else if (k === 'u') walk(8);
    else if (k === 'd') walk(9);
    else if (k === 'h' && scene) choose(scene.start, 'centre');
    else if (k === '+' || k === '=') scene?.zoomBy(1.4);
    else if (k === '-' || k === '_') scene?.zoomBy(1 / 1.4);
    else if (k === 'f') setFocus(!focus);
    else done = false;
    if (done) e.preventDefault();
  }

  // ---- start -------------------------------------------------------------------------

  let reloadTimer: ReturnType<typeof setTimeout> | undefined;
  function written() {
    if (!here() || !map) {
      stale = true;
      return;
    }
    clearTimeout(reloadTimer);
    reloadTimer = setTimeout(() => void load(map, true), 150);
  }

  onMount(() => {
    scene = new Scene(canvas, mini, {
      pick: (r) => {
        if (r) {
          choose(r.room);
          sfx('pop');
        } else if (selected) {
          // empty space: the room is put down, and its card closes
          const top = app.inspecting.at(-1);
          choose(null);
          if (top?.table === 'Rooms' && Number(top.key[0]) === map) closeInspector();
        }
      },
      follow,
      hover: (h, x, y) => (tip = h ? { x, y, ...tipFor(h) } : null),
      moved: (c) => (zoom = c.z),
    });
    scene.setColour(colourBy);
    scene.setLayers(layers);
    const retheme = () => {
      pal = palette();
      scene?.retheme();
      legend = scene?.legend() ?? [];
    };
    const mo = new MutationObserver(retheme);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-dd'] });
    const dark = matchMedia('(prefers-color-scheme: dark)');
    dark.addEventListener('change', retheme);
    const unfull = () => {
      if (!document.fullscreenElement && focus) setFocus(false);
    };
    document.addEventListener('fullscreenchange', unfull);
    document.addEventListener('keydown', onKey);
    window.addEventListener('dd:wrote', written);
    window.addEventListener('dd:realm', written);
    // the names are drawn in the display font once it has loaded
    void document.fonts?.load('600 14px Fredoka').then(() => scene?.draw());
    ready();
    return () => {
      mo.disconnect();
      dark.removeEventListener('change', retheme);
      document.removeEventListener('fullscreenchange', unfull);
      document.removeEventListener('keydown', onKey);
      window.removeEventListener('dd:wrote', written);
      window.removeEventListener('dd:realm', written);
      scene?.destroy();
      scene = null;
    };
  });

  const pct = $derived(`${Math.round(zoom * 100)}%`);
  const exitColour = (l: ExitLook) => (l.sort ? pal.exit[l.sort] : pal.path);
</script>

{#snippet glyph(name: IconName)}
  <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {#each ICONS[name] as [tag, attrs], i (i)}<svelte:element this={tag} {...attrs} />{/each}
  </svg>
{/snippet}

<div class="world" class:focus style:--card="{card}px" bind:clientWidth={width}>
  <canvas
    bind:this={canvas}
    class="map"
    tabindex="0"
    aria-label="Map {map} of the world's rooms. Click a room to open its card; the arrow keys walk its exits."
  ></canvas>
  <div class="sr" aria-live="polite">{live}</div>

  {#if tip}
    <div class="tip" style:left="{tip.x}px" style:top="{tip.y}px" role="tooltip">
      <b>{tip.title}</b><span>{tip.sub}</span>
    </div>
  {/if}

  <div class="float tl">
    {#if trail.length}
      {@const t = trail.at(-1)!}
      <button type="button" class="btn sm glass" onclick={back} title="Back to where you were">
        <ArrowLeft size={14} />Map {t.map}
      </button>
    {/if}
    <Popover.Root bind:open={mapsOpen}>
      <Popover.Trigger class="btn glass map-chip" aria-label="Pick a map">
        <span class="ic"><MapIcon size={15} /></span>
        <span>Map {map || '…'}</span>
        <small class="tnum">{count.toLocaleString()} rooms</small>
        <ChevronDown size={14} />
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content class="pop maps-pop" sideOffset={8} align="start" collisionPadding={8}>
          <div class="eyebrow head">Maps in this world</div>
          {#if maps.length > 12}
            <input class="find" type="search" placeholder="Map number" aria-label="Find a map" bind:value={mapFilter} />
          {/if}
          <div class="list">
            {#each filteredMaps as [m, c] (m)}
              <button
                type="button"
                class="menu-item"
                class:on={m === map}
                onclick={() => {
                  mapsOpen = false;
                  mapFilter = '';
                  if (m !== map) go(m, null);
                }}
              >
                <span class="num">{m}</span>Map {m}
                <span class="end faint tnum">{c.toLocaleString()} rooms{#if m === map}<Check size={15} />{/if}</span>
              </button>
            {:else}
              <p class="faint none">No map {mapFilter}.</p>
            {/each}
          </div>
          <div class="menu-sep"></div>
          <a class="menu-item" href="#/map-classic/{map || 1}{selected ? `/${selected.room}` : ''}">
            <span>The old map<span class="sub">NMR's 41 by 41 grid, with its room digger</span></span>
          </a>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
    {#if loading}<span class="pill lilac">Reading map…</span>{/if}
  </div>

  <div class="float tr">
    <div class="seg glass wide-only" role="radiogroup" aria-label="Colour rooms by">
      {#each COLOUR_BY as c (c.id)}
        <button type="button" role="radio" aria-checked={colourBy === c.id} class:on={colourBy === c.id} onclick={() => setColour(c.id)}>{c.label}</button>
      {/each}
    </div>
    <Popover.Root>
      <Popover.Trigger class="btn glass" aria-label="What the map shows"><LayersIcon size={15} /><span class="wide-only">Show</span></Popover.Trigger>
      <Popover.Portal>
        <Popover.Content class="pop layers-pop" sideOffset={8} align="end" collisionPadding={8}>
          <div class="eyebrow head">Colour rooms by</div>
          <div class="seg in-pop" role="radiogroup" aria-label="Colour rooms by">
            {#each COLOUR_BY as c (c.id)}
              <button type="button" role="radio" aria-checked={colourBy === c.id} class:on={colourBy === c.id} onclick={() => setColour(c.id)}>{c.label}</button>
            {/each}
          </div>
          <div class="menu-sep"></div>
          <div class="eyebrow head">Show on the map</div>
          {#each [['labels', 'Place names', 'One name for each run of rooms that share it'], ['symbols', 'Exit symbols and room marks', 'Doors, keys, traps, shops, NPCs and where far exits lead'], ['grid', 'Grid dots', 'The cells rooms sit on']] as [k, label, sub] (k)}
            <label class="menu-item">
              <span>{label}<span class="sub">{sub}</span></span>
              <span class="end"><input class="switch" type="checkbox" role="switch" checked={layers[k as keyof Layers]} onchange={(e) => setLayer(k as keyof Layers, e.currentTarget.checked)} /></span>
            </label>
          {/each}
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
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
    <button type="button" class="btn icon glass" onclick={() => setFocus(!focus)} title={focus ? 'Back to the app (F)' : 'Full screen (F)'} aria-label={focus ? 'Leave full screen' : 'Full screen'}>
      {#if focus}<Minimize2 size={15} />{:else}<Maximize2 size={15} />{/if}
    </button>
  </div>

  {#if legend.length > 1}
    <div class="float legend" aria-label="Legend: click a colour to pick its rooms out">
      {#each shownLegend as t (t.key)}
        <button type="button" class="chip glass" class:on={spot.includes(t.key)} class:off={spot.length > 0 && !spot.includes(t.key)} aria-pressed={spot.includes(t.key)} onclick={() => toggleSpot(t.key)}>
          <i style:background={t.colour}></i>{t.label}<span class="n tnum">{t.count.toLocaleString()}</span>
        </button>
      {/each}
      {#if legend.length > legendShown}
        <button type="button" class="chip glass more" aria-expanded={allLegend} onclick={() => (allLegend = !allLegend)}>
          {allLegend ? (legendShown ? 'Fewer' : 'Hide the colours') : legendShown ? `${legend.length - legendShown} more` : `${legend.length} colours`}
        </button>
      {/if}
      {#if spot.length}
        <button type="button" class="chip glass clear" onclick={clearSpot}><X size={12} />Show all</button>
      {/if}
    </div>
  {/if}

  <div class="float br glass overview">
    <div class="zoom">
      <button type="button" class="btn icon sm ghost" onclick={() => scene?.zoomBy(1 / 1.4)} title="Zoom out (-)" aria-label="Zoom out"><Minus size={15} /></button>
      <span class="pct tnum">{pct}</span>
      <button type="button" class="btn icon sm ghost" onclick={() => scene?.zoomBy(1.4)} title="Zoom in (+)" aria-label="Zoom in"><Plus size={15} /></button>
      <span class="grow"></span>
      {#if selected}
        <button type="button" class="btn icon sm ghost" onclick={() => selected && scene?.frameRoom(selected.room)} title="Close up on the room (Shift+2)" aria-label="Close up on the room"><LocateFixed size={15} /></button>
      {/if}
      <button type="button" class="btn sm ghost" onclick={() => scene?.fit()} title="Show every room (Shift+1)"><Scan size={14} />Fit</button>
    </div>
    <canvas bind:this={mini} class="mini" aria-label="Overview of the whole map. Click or drag to look there."></canvas>
  </div>

  {#if roomy}
    <div class="float hint glass" aria-hidden="true">
      {#if note}
        {note}
      {:else if selected}
        <span class="kbd">←↑→↓</span> walk its exits <span class="kbd">U</span><span class="kbd">D</span> up and down <span class="kbd">Shift 2</span> close up
      {:else}
        Click a room for its card <span class="kbd">H</span> start room <span class="kbd">F</span> full screen
      {/if}
    </div>
  {/if}

  {#if failed}
    <div class="float problem callout bad" role="alert">
      <span>Could not read map {map || ''}: {failed}</span>
      <button type="button" class="btn sm" onclick={() => void load(map || 1, true)}>Try again</button>
    </div>
  {:else if !loading && map && !count}
    <div class="float problem callout info">This world has no rooms on map {map} yet.</div>
  {/if}
</div>

<style>
  .world {
    position: absolute; inset: 0; overflow: hidden; background: var(--bg);
    --edge: 12px;
  }
  .map { position: absolute; inset: 0; width: 100%; height: 100%; display: block; touch-action: none; outline: none; cursor: grab; }
  .map:global(.panning) { cursor: grabbing; }
  .map:focus-visible { box-shadow: inset 0 0 0 3px var(--focus); }

  .float { position: absolute; z-index: 5; display: flex; align-items: center; gap: 6px; }
  .glass {
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid var(--line-2);
    box-shadow: 0 2px 0 color-mix(in srgb, var(--shadow) 12%, transparent), var(--float);
  }
  .float :global(.btn.glass) { border: 1px solid var(--line-2); }
  .tl { top: var(--edge); left: var(--edge); flex-wrap: wrap; max-width: calc(100% - var(--card) - 2 * var(--edge)); }
  .tr { top: var(--edge); right: calc(var(--edge) + var(--card)); }
  :global(.map-chip) { gap: 8px !important; padding: 0 12px 0 6px !important; border-radius: 999px !important; }
  :global(.map-chip) .ic { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; background: var(--mint-2); color: var(--mint-ink); }
  :global(.map-chip) small { font-weight: 700; color: var(--ink-3); font-size: 12px; }

  .seg { display: inline-flex; gap: 2px; padding: 3px; border-radius: 12px; }
  .seg button { min-height: 28px; padding: 0 10px; border: 0; border-radius: 9px; background: transparent; font-size: 12.5px; color: var(--ink-2); box-shadow: none; }
  .seg button:hover:not(.on) { background: var(--surface-3); }
  .seg button.on { background: var(--lilac-2); color: var(--lilac-ink); box-shadow: 0 0 0 1px var(--line-2); }
  .seg.in-pop { display: flex; flex-wrap: wrap; margin: 0 6px 6px; background: var(--surface-2); border: 1px solid var(--line); }
  .seg.in-pop button { flex: 1 1 auto; }

  :global(.maps-pop) { width: min(300px, calc(100vw - 16px)); }
  :global(.maps-pop) .head, :global(.layers-pop) .head { padding: 8px 10px 6px; }
  :global(.maps-pop) .list { max-height: min(360px, 50vh); overflow: auto; }
  :global(.maps-pop) .find { margin: 0 6px 6px; width: calc(100% - 12px); }
  :global(.maps-pop) .num { min-width: 30px; height: 22px; padding: 0 6px; border-radius: 8px; display: inline-grid; place-items: center; background: var(--mint-2); color: var(--mint-ink); font: 700 12px var(--font-mono); }
  :global(.maps-pop) .none { margin: 6px 10px; font-size: 13px; }
  :global(.layers-pop) { width: min(360px, calc(100vw - 16px)); max-height: min(78vh, 640px); overflow: auto; }
  .key { list-style: none; margin: 0; padding: 0 8px 6px; display: grid; grid-template-columns: 1fr 1fr; gap: 4px 10px; font-size: 12px; font-weight: 700; color: var(--ink-2); }
  .key li { display: flex; align-items: center; gap: 7px; min-width: 0; line-height: 1.25; }
  .mark { flex: none; width: 28px; height: 18px; display: grid; place-items: center; }
  .mark.room { width: 22px; height: 18px; border-radius: 6px; background: var(--surface-3); color: var(--ink); margin-inline: 3px; }

  /* the legend sits under the colours it explains */
  .legend {
    top: calc(var(--edge) + 44px); right: calc(var(--edge) + var(--card));
    flex-wrap: wrap; justify-content: flex-end; max-width: min(760px, calc(100% - var(--card) - 2 * var(--edge)));
  }
  .chip { display: inline-flex; align-items: center; gap: 6px; min-height: 28px; padding: 0 10px 0 7px; border-radius: 999px; font-size: 12px; color: var(--ink); }
  .chip i { width: 13px; height: 13px; border-radius: 4px; border: 1.5px solid var(--outline); display: block; flex: none; }
  .chip .n { color: var(--ink-3); font-weight: 700; }
  .chip.on { background: var(--lilac-2); border-color: var(--outline); }
  .chip.off { opacity: .55; }
  .chip.more, .chip.clear { padding: 0 10px; color: var(--ink-2); }

  .overview { right: calc(var(--edge) + var(--card)); bottom: var(--edge); flex-direction: column; align-items: stretch; gap: 4px; padding: 5px; border-radius: 16px; width: 214px; }
  .zoom { display: flex; align-items: center; gap: 2px; }
  .zoom .grow { flex: 1; }
  .pct { min-width: 44px; text-align: center; font-size: 12px; font-weight: 800; color: var(--ink-2); }
  .mini { width: 100%; height: 128px; border-radius: 11px; border: 1px solid var(--line); display: block; cursor: crosshair; touch-action: none; }

  .hint {
    left: calc((100% - var(--card)) / 2); bottom: var(--edge); transform: translateX(-50%);
    padding: 6px 12px; border-radius: 999px; font-size: 12px; font-weight: 700; color: var(--ink-2); white-space: nowrap; pointer-events: none;
  }
  .hint .kbd { margin-left: 6px; height: 18px; font-size: 10.5px; }
  .problem {
    top: 50%; left: calc((100% - var(--card)) / 2); transform: translate(-50%, -50%);
    max-width: min(520px, calc(100% - var(--card) - 2 * var(--edge))); align-items: center; gap: 10px;
  }

  .tip {
    position: absolute; z-index: 6; pointer-events: none; transform: translate(14px, 14px);
    max-width: 280px; padding: 7px 10px; border-radius: 10px; display: flex; flex-direction: column; gap: 2px;
    background: var(--surface); border: 1.5px solid var(--outline); box-shadow: 0 3px 0 var(--shadow);
    font-size: 12.5px; line-height: 1.3;
  }
  .tip span { color: var(--ink-3); font-weight: 700; font-size: 11.5px; }

  /* the editors' status line stays with them */
  :global(body:has(#world:not([hidden])) #status) { display: none; }

  /* full screen: the top bar and the rail make way */
  :global(body.world-focus .dd-app) { grid-template-columns: 0 minmax(0, 1fr); grid-template-rows: 0 minmax(0, 1fr); }
  :global(body.world-focus #dd-top), :global(body.world-focus #dd-rail) { display: none; }

  @media (max-width: 1100px) {
    .wide-only { display: none !important; }
  }
  @media (max-width: 700px) {
    .overview { width: auto; }
    .mini { display: none; }
  }
</style>
