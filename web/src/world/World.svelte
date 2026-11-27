<!--
  The World: the open world's rooms as a map you look around in. Drag to pan, scroll or pinch
  to zoom; click a room to open its card beside the map, and the arrow keys (or the number pad)
  walk its exits. The rooms can be coloured by place, area, monsters, light or your own edits,
  and the legend picks one colour out. F shows the map on the whole screen.

  Build mode (B) digs with the same keys: a way with an exit is walked, a room drawn next door
  is joined, and free cells get new rooms; digits first make a line ("3" then an arrow), "."
  does the last move again, and Enter names the room. The map shows each edit at once and the
  server writes them in turn; each one's toast can undo it.

  In Build mode a room can be carried to a free cell (or moved a cell with Shift and an arrow).
  The first move on a map keeps every room where it is drawn, so the map holds still from then
  on; the places are kept beside the working copy, never in the game, and can be exported.

  Stamps (S) put a shape of rooms down at once: the shape follows the pointer see-through, each
  cell tinted by what it would do, and a click puts it down as one save, which one undo takes
  back with the places its rooms were drawn in.

  Select (V) picks rooms with a box, a lasso, a brush or the wand (the rooms joined to one in its
  colour), and its panel sets their name, light, monsters, control room or room type together,
  as one save. Its Paint puts light, monsters or a control room onto the rooms a drag goes over,
  a save for each stroke, with the map coloured by what it paints.

  Walk (P) playtests from the selected room: the arrow keys take a pretend player through the
  exits as the game would, its words for each room and each refusal in a panel, and the map
  follows them, onto other maps too. A room clicked puts the player there; Esc goes back to
  editing at the room reached.

  All maps (W, or zooming out past a map) shows every map as a disc and the map change exits
  between them as lines (WorldView.svelte); a disc opens its map.
-->
<!-- loaded when the World is first opened, so its styles travel with its code -->
<svelte:options css="injected" />

<script lang="ts">
  import { Popover } from 'bits-ui';
  import { onMount, tick, untrack } from 'svelte';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Footprints from '@lucide/svelte/icons/footprints';
  import LayersIcon from '@lucide/svelte/icons/layers';
  import LocateFixed from '@lucide/svelte/icons/locate-fixed';
  import MapIcon from '@lucide/svelte/icons/map';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import Minus from '@lucide/svelte/icons/minus';
  import Pickaxe from '@lucide/svelte/icons/pickaxe';
  import Plus from '@lucide/svelte/icons/plus';
  import Scan from '@lucide/svelte/icons/scan';
  import SquareDashedMousePointer from '@lucide/svelte/icons/square-dashed-mouse-pointer';
  import StampIcon from '@lucide/svelte/icons/stamp';
  import Waypoints from '@lucide/svelte/icons/waypoints';
  import X from '@lucide/svelte/icons/x';
  import { api, type Cell } from '../api';
  import type PlayPanel from '../story/PlayPanel.svelte';
  import { read, sfx, write } from '../shell/prefs.svelte';
  import { app, closeInspector, dropLocalStep, inspect, localStep, realmChanged, undo, type LocalStep } from '../shell/state.svelte';
  import { dismiss, messageOf, toast, toasts } from '../shell/toasts.svelte';
  import { dug, joined, moveFrom, reachCount, reached, reachLinks, renamed, type Reach } from './build';
  import { COLOUR_BY, palette, type ColourBy } from './look';
  import { SPRAY, wiresOf, type End, type Wire, type WireKind, type WiresFor } from './wires';
  import type { Host } from './host';
  import { cellKey, cellsOf, OPPOSITE, STEP, type Spot } from './layout';
  import { DIR_WORDS, KEY_DIR, kindName, mapName, type MapFile, type MapLink, type MapName, type MapRoom, type PinCells, type Scripts, type When } from './model';
  import { Scene, type Camera, type Ghost, type Hit, type Layers, type LegendEntry, type Stub } from './scene';
  import type { Selector } from './selector.svelte';
  import type { Stamper } from './stamper.svelte';
  import type ShowMenuKind from './ShowMenu.svelte';
  import type WorldViewKind from './WorldView.svelte';

  let canvas: HTMLCanvasElement;
  let mini: HTMLCanvasElement;
  let scene: Scene | null = null;

  let map = $state(0);
  let maps = $state<[number, number][]>([]);
  /** Each map's commonest room name, and the map change exits between maps (for All maps). */
  let names = $state.raw<MapName[]>([]);
  let links = $state.raw<MapLink[]>([]);
  /** The links no exit draws (scripts, spells, remote actions), and for which rooms they show as wires. */
  let scripts = $state.raw<Scripts | undefined>(undefined);
  let wiresFor = $state<WiresFor>(wiresFrom(read('dd-map-wires', 'selected')));
  /** Bumped when the map's rooms change (read, or an edit shown): the wires follow them. */
  let roomsAt = $state(0);
  const namesOf = $derived(new Map(names.map((n) => [n.map, n])));
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
  /** What a corridor dragged out of the selected room (or a Shift line) would make, for the hint bar (`Dig`, `3 rooms east…`), and whether it is being dragged. */
  let reachNote = $state<[string, string] | null>(null);
  let reachDrag = $state(false);
  let card = $state(0);
  let width = $state(0);
  /** The width of the view's buttons at the top right: the tools at the top left wrap short of them. */
  let viewWidth = $state(0);
  let mapsOpen = $state(false);
  let mapFilter = $state('');
  let pal = $state(palette());
  // Build mode
  let building = $state(false);
  /** Digits typed before a direction key: how many times to go. */
  let countText = $state('');
  /** The last move, for "." to make again. */
  let lastMove: { dir: number; count: number } | null = null;
  /** The room being named, and where its field is. */
  let naming = $state<{ room: number; value: string } | null>(null);
  let namingAt = $state<{ x: number; y: number } | null>(null);
  let nameField = $state<HTMLInputElement | undefined>();
  /** Edits sent and not yet answered. */
  let pending = $state(0);
  let chain: Promise<void> = Promise.resolve();
  /** An edit failed: those queued after it are not sent, and the map is read again. */
  let broken = false;
  /** Counts the edits shown before the server had them: a read begun before one is out of date. */
  let edits = 0;
  /**
   * The card catches up with the room selected once the edits under way are in: `follow` when
   * it showed a room of this map as Build mode moved on, `open` when a room was clicked meanwhile
   * (it may not be written yet).
   */
  let cardFollows: false | 'follow' | 'open' = false;
  let buildToast = 0;
  /** The rooms Build mode went to, latest last: where it goes back to when an undo takes its room away. */
  let visited: number[] = [];
  /** The rooms changed since the realm was read: written meanwhile, so read again when shown. */
  let stale = false;
  /** Where the camera was on each map, so coming back to one finds it as it was left. */
  const cams = new Map<number, Camera>();
  // the map's tools, whose code loads the first time each is used
  /** The stamps' code and the stamp held, once the drawer has been opened. */
  let kit = $state<typeof import('./stamping') | null>(null);
  let stamper = $state.raw<Stamper | null>(null);
  /** The stamp drawer is open. */
  let stamping = $state(false);
  /** The Select tool's code and the rooms it picks, once it has been taken up. */
  let picker = $state<typeof import('./picking') | null>(null);
  let selector = $state.raw<Selector | null>(null);
  /** The Select tool is on. */
  let picking = $state(false);
  /** Walk mode: a playtest from a room, in a panel whose code loads the first time. */
  let walking = $state(false);
  let WalkPanel = $state.raw<typeof PlayPanel | null>(null);
  /** All maps: loaded the first time it is opened; shown while `worldShown`. */
  let WorldView = $state.raw<typeof WorldViewKind | null>(null);
  let worldView = $state<ReturnType<typeof WorldViewKind>>();
  let worldShown = $state(false);
  /** The Show menu (what the map shows, and what its marks mean): its code loads once the map is first drawn, or when the menu opens. */
  let ShowMenu = $state.raw<typeof ShowMenuKind | null>(null);
  let showFailed = $state('');
  let walker = $state<ReturnType<typeof PlayPanel>>();
  /** The room the walk started from. */
  let walkStart = $state.raw<{ table: string; key: Cell[] } | null>(null);
  /** How high the walk's panel is, and the tools along the top: the player's room is kept in view between them. */
  let walkHigh = $state(0);
  let toolsHigh = $state(0);
  /** A finger, not a mouse: a stamp is aimed with one tap and put down with another. */
  const coarse = typeof matchMedia === 'function' && matchMedia('(pointer: coarse)').matches;

  function colourFrom(v: string): ColourBy {
    return COLOUR_BY.some((c) => c.id === v) ? (v as ColourBy) : 'place';
  }

  function wiresFrom(v: string): WiresFor {
    return v === 'all' || v === 'off' ? v : 'selected';
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
  /** Whether the colourings fit along the top beside the tools (else they are under Show alone, as on a small screen). */
  const wideTop = $derived(width - card >= 1000);
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
    if (Number.isFinite(n) && n !== selected?.room) choose(n, fitted ? 'close' : 'centre', !walking);
    else if (Number.isFinite(n)) history.replaceState(null, '', `#/world/${map}/${n}`);
  }

  /** Reads a map's rooms. Returns whether the camera was moved to show them all. */
  async function load(m: number, keep: boolean): Promise<boolean> {
    if (!scene) return false;
    if (map && map !== m) cams.set(map, { ...scene.cam });
    loading = true;
    let fitted = false;
    const gen = edits;
    try {
      const a = await api.map(m);
      // an edit was shown meanwhile: this read may not have it, and the one after it will
      if (a.map === map && gen !== edits) return false;
      const moved = a.map !== map;
      if (moved) {
        scene.select(null);
        selected = null;
      }
      map = a.map;
      maps = a.maps;
      names = a.names ?? [];
      links = a.links ?? [];
      scripts = a.scripts;
      count = a.rooms.length;
      write(lastKey(), String(map));
      scene.setEdited(edited);
      // moves being saved are newer than this read's
      if (moved || !pinsPending) {
        pins = pinsFrom(a.pins);
        pinCount = pins.size;
      }
      scene.setRooms(a.rooms, pins);
      roomsAt++;
      startRoom = scene.start;
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
      const had = selected?.room;
      selected = selected ? (scene.by.get(selected.room) ?? null) : null;
      if (had !== undefined && !selected && !moved) gone(had);
      legend = scene.legend();
      toolsChanged(moved);
      // the Show menu's code, once the map is drawn
      if (!ShowMenu) setTimeout(loadShow, 800);
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
  function choose(n: number | null, how: 'centre' | 'close' | 'keep' | 'none' = 'none', withCard = true) {
    if (!scene) return;
    const r = n === null ? null : (scene.by.get(n) ?? null);
    scene.select(r?.room ?? null);
    selected = r;
    history.replaceState(null, '', r ? `#/world/${map}/${r.room}` : `#/world/${map}`);
    if (!r) return;
    if (building && visited.at(-1) !== r.room) visited = [...visited, r.room].slice(-50);
    if (withCard && pending) cardFollows = 'open';
    else if (withCard || (cardOnMap() && !pending)) {
      card = scene.setCover(true);
      inspect({ table: 'Rooms', key: [map, r.room], name: r.name }, true);
    } else if (cardOnMap()) cardFollows = 'follow';
    if (how === 'centre') scene.centreOn(r.room);
    else if (how === 'close') scene.centreOn(r.room, { zoom: Math.max(scene.cam.z, 1.1), animate: false });
    else if (how === 'keep') scene.keepInView(r.room);
    live = `${r.name || 'Room'}, room ${map}/${r.room}. ${exitsSaid(r)}`;
  }

  /** The selected room went (an undo took it away): its card closes, and Build mode goes back to the last room it was in that is still here. */
  function gone(n: number) {
    const top = app.inspecting.at(-1);
    if (top?.table === 'Rooms' && Number(top.key[0]) === map && Number(top.key[1]) === n) closeInspector();
    for (let i = visited.length - 1; i >= 0; i--) {
      const v = visited[i]!;
      if (v !== n && scene?.by.has(v)) {
        choose(v, 'keep', false);
        return;
      }
    }
  }

  /** Whether the card shows a room of this map. */
  function cardOnMap(): boolean {
    const top = app.inspecting.at(-1);
    return top?.table === 'Rooms' && Number(top.key[0]) === map;
  }

  function exitsSaid(r: MapRoom): string {
    if (!r.exits.length) return 'No exits.';
    return `Exits ${r.exits.map((e) => DIR_WORDS[e.dir] ?? '?').join(', ')}.`;
  }

  /** A note in the hint bar for a moment (a toast when the hint bar has no room). */
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  /** A line in the bar along the bottom (a toast on a small screen, unless `quiet`: then only for a screen reader). */
  function say(text: string, quiet = false) {
    live = text;
    // the walk's panel has the bar's place
    if (!roomy || walking) {
      if (!quiet && !toasts.some((t) => t.text === text)) toast(text, { ms: 2600 });
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
    if (s.sort === 'missing') {
      sfx('nope');
      say(`Map ${map} has no room ${e.to}.`);
    } else if (walking) walker?.put(s.sort === 'map' ? e.para1 : map, e.to);
    else if (s.sort === 'map') go(e.para1, e.to);
    else choose(e.to, 'centre');
  }

  /** A wire clicked: to its far end from the rooms whose wires these are. */
  function followWire(w: Wire) {
    const mine = (e: End) => e.map === map && wired.has(e.room);
    const to = mine(w.to) && !mine(w.from) ? w.from : w.to;
    if (to.map === map && !scene?.by.has(to.room)) {
      sfx('nope');
      say(`Map ${map} has no room ${to.room}.`);
    } else if (walking) walker?.put(to.map, to.room);
    else go(to.map, to.room);
  }

  const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

  /** A command's words as a player types them: the first of its `|` alternatives, and how many more. */
  function typed(words: string[]): string {
    const all = words.flatMap((w) => w.split('|')).map((w) => w.trim()).filter(Boolean);
    if (!all.length) return 'its words';
    return `“${all[0]}”${all.length > 1 ? ` (or ${all.length - 1} other way${all.length > 2 ? 's' : ''} of saying it)` : ''}`;
  }

  /** How a wire's room sets it off, in words. */
  function setOff(w: When | undefined, from: string): string {
    switch (w?.by) {
      case 'typed':
        return `Typing ${typed([w.words])} in ${from}`;
      case 'words':
        return `Typing ${typed(w.words)} in ${from}`;
      case 'walking':
        return `Walking ${DIR_WORDS[w.dir]} from ${from}`;
      default:
        return `The spell of ${from}, on whoever is there,`;
    }
  }

  const WIRE_NAMES: Record<WireKind, string> = { teleport: 'Teleport', spell: 'Spell teleport', remote: 'Remote action', death: 'Death room', hangup: 'Hang-up room', control: 'Control room' };

  /** What a wire says when pointed at; `n` of them share a chip, to rooms of another map (or rooms this map lacks), or from one. */
  function wireTip(w: Wire, n = 1): { title: string; sub: string } {
    if (n > 1) {
      const out = w.to.map !== map || !scene?.by.has(w.to.room);
      const e = out ? w.to : w.from;
      const rooms = e.map === map ? `${n} rooms map ${map} lacks` : `${n} rooms of map ${e.map}`;
      return { title: `${WIRE_NAMES[w.kind]} ${out ? 'to' : 'from'} ${rooms}`, sub: `The first: ${wireTip(w).sub}` };
    }
    const at = (e: End) => (e.map === map ? called(e.room) : `room ${e.map}/${e.room}`);
    const [a, b] = [at(w.from), at(w.to)];
    const go = w.to.map === map && !scene?.by.has(w.to.room) ? ` Map ${map} has no room ${w.to.room}.` : ' Click to go there.';
    if (w.kind === 'death') return { title: `Death room: ${b}`, sub: `Whoever dies in ${a} wakes in ${b} (its Death Room).${go}` };
    if (w.kind === 'hangup') return { title: `Hang-up room: ${b}`, sub: `Whoever leaves the game in ${a}, or hangs up, comes back in ${b} (its Exit Room).${go}` };
    if (w.kind === 'control') return { title: `Control room: ${b}`, sub: `Its Max Area caps the monsters ${a} and the other rooms it controls spawn between them.${go}` };
    const t = w.target;
    const by = w.source?.table === 'Spells' ? `spell ${w.source.key[0]}, ${w.source.name || 'unnamed'}` : t?.table === 'Textblocks' ? `text block ${t.record}: ${t.what}` : 'its Remote Action exit';
    const how = setOff(w.when, a);
    // one of more than are drawn
    const of = w.fan ? ` One of ${w.fan.toLocaleString()} ${WIRE_NAMES[w.kind].toLowerCase()}s from ${a}: the map draws ${SPRAY}.` : '';
    if (w.kind === 'remote') return { title: `${WIRE_NAMES.remote} on the ${DIR_WORDS[w.exit ?? 0]} exit of ${b}`, sub: `${how} works that exit (${by}).${of}${go}` };
    const where = w.among && t ? `a room rolled from ${t.to_room} to ${t.to_last} of map ${t.to_map}, ${b} among them` : b;
    return { title: `${WIRE_NAMES[w.kind]} to ${w.among ? `one of ${w.among} rooms` : b}`, sub: `${how} ${w.kind === 'spell' ? 'casts a spell that teleports them' : 'teleports them'} to ${where} (${by}).${of}${go}` };
  }

  function tipFor(h: Hit): { title: string; sub: string } {
    if (h.wire) return wireTip(h.wire, h.wires);
    if (h.ghost) {
      const from = named(selected?.room ?? 0);
      const way = DIR_WORDS[h.ghost.dir] ?? '';
      if (h.ghost.to === null) return { title: `Dig a room ${way}`, sub: `A copy of ${from}, with an exit each way. Drag it out for a corridor, or onto a room to join that one.` };
      return { title: `Join ${way} to ${named(h.ghost.to)}`, sub: `An exit each way between ${from} and room ${map}/${h.ghost.to}. Drag it onto another room to join that one instead.` };
    }
    if (h.stub && !picking) {
      const e = h.stub.exit;
      const way = cap(DIR_WORDS[e.dir] ?? 'An exit');
      const kind = e.kind ? `${kindName(e.kind)} exit` : 'Exit';
      if (h.stub.sort === 'map') {
        const rooms = maps.find(([m]) => m === e.para1)?.[1];
        const name = rooms ? mapName(namesOf, e.para1, rooms) : null;
        const sub = rooms ? `${kind} to room ${e.para1}/${e.to}. Click to go there.` : `${kind} to room ${e.para1}/${e.to}. This world has no rooms on map ${e.para1}: an add-on’s map, perhaps.`;
        return { title: `${way} to map ${e.para1}${name ? ` · ${name}` : ''}`, sub };
      }
      if (h.stub.sort === 'missing') return { title: `${way} to room ${e.to}`, sub: `Map ${map} has no room ${e.to}.` };
      const t = scene?.by.get(e.to);
      return { title: `${way} to ${t?.name || `room ${e.to}`}`, sub: `${kind} to room ${map}/${e.to}, drawn elsewhere. Click to go there.` };
    }
    const r = h.room!;
    if (picking && selector) return { title: r.name || `Room ${r.room}`, sub: selector.about(r) };
    return { title: r.name || `Room ${r.room}`, sub: building ? `Room ${map}/${r.room}. Drag to move it.` : `Room ${map}/${r.room}` };
  }

  // ---- Build mode ----------------------------------------------------------------------

  /** The most rooms a move makes or walks. */
  const MOST = 99;

  const named = (n: number) => scene?.by.get(n)?.name.trim() || `room ${map}/${n}`;
  /** A room's name and number: rooms often share a name (a street, a forest). */
  const called = (n: number) => {
    const name = scene?.by.get(n)?.name.trim();
    return name ? `${name} ${map}/${n}` : `room ${map}/${n}`;
  };

  function setBuilding(on: boolean) {
    if (on === building) return;
    if (on && walking) stopWalking(false);
    building = on;
    countText = '';
    if (!on && naming) finishNaming(true);
    if (!on && stamping) {
      stamping = false;
      stamper?.pick(null);
    }
    scene?.setBuild(on);
    if (on && !selected && scene?.layout) choose(scene.start, 'centre', false);
    live = on ? 'Build mode: the arrow keys dig, join and walk.' : 'Build mode off.';
    sfx('toggle');
  }

  /** The digits typed before a direction key, as a count (one when none were). */
  function takeCount(): number {
    const c = Math.min(MOST, Number(countText) || 1);
    countText = '';
    return c;
  }

  /** A direction key in Build mode, `count` times: walks an exit, joins the room drawn next door, or digs. */
  function build(dir: number, count = 1) {
    if (!scene?.layout) return;
    if (!selected) {
      choose(scene.start, 'centre', false);
      return;
    }
    lastMove = { dir, count };
    let at = selected.room;
    let left = Math.max(1, Math.min(MOST, count));
    let made = false;
    while (left > 0 && scene.layout) {
      const m = moveFrom(scene.by, scene.layout, at, dir, left);
      if (m.kind === 'stop') {
        sfx('nope');
        say(m.why);
        break;
      }
      if (m.kind === 'leave') {
        if (at !== selected?.room) choose(at, 'keep', false);
        go(m.exit.para1, m.exit.to);
        return;
      }
      if (m.kind === 'walk') {
        at = m.to;
        left--;
      } else if (m.kind === 'join') {
        joinRooms(at, dir, m.to);
        at = m.to;
        left--;
        made = true;
      } else {
        digRooms(at, dir, m.rooms);
        at = m.rooms.at(-1)!;
        left -= m.rooms.length;
        made = true;
      }
    }
    if (made) sfx('pop');
    if (at !== selected?.room) choose(at, 'keep', false);
  }

  function digRooms(from: number, dir: number, rooms: number[]) {
    if (!scene) return;
    const way = DIR_WORDS[dir];
    const what =
      rooms.length > 1
        ? `Dug ${rooms.length} rooms ${way} of ${named(from)}, ${map}/${rooms[0]} to ${map}/${rooms.at(-1)}`
        : `Dug room ${map}/${rooms[0]} ${way} of ${named(from)}`;
    patch(dug(scene.rooms, from, dir, rooms));
    const m = map;
    send(() => api.dig({ map: m, room: from, dir, to_map: m, to_room: rooms[0]!, count: rooms.length }), what);
    unpinNew(m, rooms);
  }

  /** New rooms are drawn beside the rooms they are dug from, not where old rooms of their numbers were. */
  function unpinNew(m: number, rooms: number[]) {
    const old = rooms.filter((n) => pins.has(n));
    if (!old.length) return;
    const next = new Map(pins);
    for (const n of old) next.delete(n);
    showPins(m, next);
    void savePins(m, Object.fromEntries(old.map((n) => [String(n), null])), false).catch(() => {});
  }

  /** What a corridor or join reached out of a room makes, for the hint bar: `Dig`, `3 rooms east, 1/3464–3466 · joins Field 1/12`. */
  function reachSaid(r: Reach): [string, string] {
    const way = DIR_WORDS[r.dir] ?? '';
    if (r.kind === 'join') return ['Join', `${way} to ${called(r.to)}${r.oneWay ? `, one-way: it has an exit ${DIR_WORDS[OPPOSITE[r.dir]!]} already` : ''}`];
    const c = reachCount(r);
    const list = c.joined.length === 1 ? called(c.joined[0]!) : `${c.joined.length} rooms`;
    const parts: string[] = [];
    if (c.made.length === 1) parts.push(`a room ${way}, ${map}/${c.made[0]}`);
    else if (c.made.length) parts.push(`${c.made.length} rooms ${way}, ${map}/${c.made[0]}–${c.made.at(-1)}`);
    if (c.joined.length) parts.push(c.made.length ? `joins ${list}` : `${way} to ${list}`);
    const oneWay = r.links.filter((l) => l.out && l.back === 'taken').map((l) => l.to);
    if (oneWay.length) parts.push(`one-way into ${oneWay.length === 1 ? called(oneWay[0]!) : `${oneWay.length} rooms`}: ${oneWay.length === 1 ? 'it has' : 'they have'} an exit ${DIR_WORDS[OPPOSITE[r.dir]!]} already`);
    if (r.stop) parts.push(`stops short: ${r.stop.why}`);
    else if (!parts.length) parts.push(`the way ${way} is there already`);
    return [c.made.length || !c.joined.length ? 'Dig' : 'Join', parts.join(' · ')];
  }

  /** A corridor dragged out of the selected room (or a Shift line) made as one step, or a join onto the room it was let go over; the room it ends at is picked. */
  function reachOut(r: Reach) {
    if (!scene) return;
    const from = scene.by.get(r.from);
    if (!from) return;
    const way = DIR_WORDS[r.dir] ?? '';
    if (r.kind === 'join') {
      joinAcross(r.from, r.dir, r.to);
      sfx('pop');
      choose(r.to, 'keep', false);
      return;
    }
    const c = reachCount(r);
    const end = r.cells.at(-1)?.room;
    if (!c.exits) {
      // nothing to make: the ways are there already, or it stops at once
      if (r.stop) {
        sfx('nope');
        say(r.stop.why);
      }
      if (end !== undefined) choose(end, 'keep', false);
      return;
    }
    const m = map;
    const list = c.joined.length === 1 ? called(c.joined[0]!) : `${c.joined.length} rooms`;
    const what = c.made.length
      ? `Dug ${c.made.length === 1 ? `room ${m}/${c.made[0]}` : `${c.made.length} rooms`} ${way} of ${named(r.from)}${c.made.length > 1 ? `, ${m}/${c.made[0]} to ${m}/${c.made.at(-1)}` : ''}${c.joined.length ? `, joining ${list}` : ''}`
      : `Joined ${called(r.from)} ${way} to ${list}`;
    patch(reached(scene.rooms, r, from));
    const req = { map: m, brush: r.from, rooms: c.made, links: reachLinks(r), overwrite: false, what: `a corridor ${way} of ${m}/${r.from}`, dug: true };
    send(() => api.stamp(req), what);
    unpinNew(m, c.made);
    sfx('pop');
    if (r.stop) say(r.stop.why, true);
    if (end !== undefined) choose(end, 'keep', false);
  }

  function joinRooms(from: number, dir: number, to: number) {
    if (!scene) return;
    const what = `Joined ${called(from)} ${DIR_WORDS[dir]} to ${called(to)}`;
    patch(joined(scene.rooms, from, dir, to).rooms);
    const m = map;
    send(() => api.dig({ map: m, room: from, dir, to_map: m, to_room: to, existing: true }), what);
  }

  /**
   * A join dragged onto a room across the map. Its exit is not the way the rooms are drawn from
   * each other, so a map drawn by its exits alone would be laid out again around it: first every
   * room keeps the place it is drawn in, as the first stamp or move on a map does. Ctrl+Z (or its
   * toast) takes back both.
   */
  function joinAcross(from: number, dir: number, to: number) {
    const l = scene?.layout;
    if (!scene || !l) return;
    if (pins.size) return joinRooms(from, dir, to);
    const m = map;
    const before = pins;
    const next = new Map([...l.at].map(([n, s]) => [n, { ...s }]));
    const label = `Join ${called(from)} ${DIR_WORDS[dir]} to ${called(to)}`;
    patch(joined(scene.rooms, from, dir, to).rooms);
    showPins(m, next);
    send(
      async () => {
        const out = await api.dig({ map: m, room: from, dir, to_map: m, to_room: to, existing: true });
        await savePins(m, cellsOf(next), true);
        const step = out.step;
        if (step !== null) {
          const back = async (places: ReadonlyMap<number, Spot>, redo: boolean) => {
            dismiss(buildToast);
            await (redo ? api.redo(step) : api.undo(step));
            showPins(m, places);
            await savePins(m, cellsOf(places), true);
            realmChanged();
          };
          localStep({ label, after: step, undo: () => back(before, false), redo: () => back(next, true) });
        }
        return out;
      },
      `Joined ${called(from)} ${DIR_WORDS[dir]} to ${called(to)}. The map keeps this layout now: rooms stay where they are drawn`,
      () => showPins(m, before),
    );
  }

  /** Shows the map's rooms as an edit leaves them, before the server has them. */
  function patch(rooms: MapRoom[]) {
    if (!scene) return;
    edits++;
    scene.setRooms(rooms);
    roomsAt++;
    count = rooms.length;
    maps = maps.map(([m, c]) => [m, m === map ? rooms.length : c]);
    selected = selected ? (scene.by.get(selected.room) ?? null) : null;
    legend = scene.legend();
    toolsChanged(false);
  }

  /** The map's rooms or places changed (`moved`: it is another map): the tools follow. */
  function toolsChanged(moved: boolean) {
    stamper?.changed(moved);
    selector?.changed(moved);
  }

  /**
   * Sends an edit once those before it are in; `done` says what it did (from its answer), in a
   * toast that can undo it. If it fails, `failed` puts back what was shown.
   */
  function send<T>(work: () => Promise<T>, done: string | ((out: T) => string), failed?: () => void) {
    pending++;
    chain = chain.then(async () => {
      if (broken) return;
      try {
        const out = await work();
        const said = typeof done === 'function' ? done(out) : done;
        const n = (out as { notes?: string[] } | undefined)?.notes;
        const notes = n?.length ? ` ${n.join(' ')}` : '';
        dismiss(buildToast);
        buildToast = toast(`${said}.${notes}`, { action: 'Undo', onAction: () => void undo() });
      } catch (e) {
        // what came after it rested on it: the map is read again as the server has it
        broken = true;
        failed?.();
        sfx('nope');
        dismiss(buildToast);
        toast(messageOf(e), { kind: 'error' });
      } finally {
        pending--;
        if (!pending) {
          broken = false;
          void settle();
        }
      }
    });
  }

  /** Every edit is in: the map is read again as the server has it, and the card catches up. */
  async function settle() {
    if (!map || !here()) {
      stale = true;
      return;
    }
    await load(map, true);
    if (pending) return;
    if (selected && (cardFollows === 'open' || (cardFollows === 'follow' && cardOnMap()))) {
      card = scene?.setCover(true) ?? 0;
      inspect({ table: 'Rooms', key: [map, selected.room], name: selected.name }, true);
    }
    cardFollows = false;
  }

  async function startNaming() {
    if (!selected || !scene) return;
    naming = { room: selected.room, value: selected.name.trim() };
    placeNaming();
    scene.setNaming(selected.room);
    await tick();
    nameField?.focus();
    nameField?.select();
  }

  function placeNaming() {
    const s = naming ? scene?.roomOnScreen(naming.room) : null;
    namingAt = s ? { x: s.x, y: s.y - s.half - 22 } : null;
  }

  function finishNaming(save: boolean) {
    const n = naming;
    if (!n) return;
    naming = null;
    namingAt = null;
    scene?.setNaming(null);
    canvas.focus({ preventScroll: true });
    if (!save || !scene) return;
    const name = n.value.trim();
    const r = scene.by.get(n.room);
    if (!r || name === r.name.trim()) return;
    patch(renamed(scene.rooms, n.room, name));
    const m = map;
    send(() => api.save('Rooms', [m, n.room], { Name: name }), name ? `Named room ${m}/${n.room} “${name}”` : `Cleared room ${m}/${n.room}'s name`);
  }

  /** The longest name a room takes. */
  const nameMax = $derived(app.tables.find((t) => t.name === 'Rooms')?.columns.find((c) => c.name === 'Name')?.size ?? 53);

  // ---- rooms moved by hand ---------------------------------------------------------------

  /** Where this map's rooms moved by hand are drawn (kept beside the working copy). */
  let pins: ReadonlyMap<number, Spot> = new Map();
  let pinCount = $state(0);
  /** The room the layout starts from. */
  let startRoom = $state(1);
  /** Saves of them sent and not yet answered: a read of the map meanwhile keeps these. */
  let pinsPending = 0;
  let pinChain: Promise<unknown> = Promise.resolve();

  const pinsFrom = (c: PinCells | undefined) => new Map(Object.entries(c ?? {}).map(([k, [x, y]]) => [Number(k), { x, y }]));

  /** Draws map `m`'s rooms with these places, if it is the map shown. */
  function showPins(m: number, next: ReadonlyMap<number, Spot>) {
    if (m !== map || !scene) return;
    pins = next;
    pinCount = next.size;
    scene.setPins(next);
    toolsChanged(false);
  }

  /** Saves places of map `m`'s rooms, after those sent before; answers the realm history's newest step. */
  function savePins(m: number, body: Record<string, [number, number] | null>, replace: boolean): Promise<number> {
    pinsPending++;
    const done = pinChain.then(async () => {
      try {
        return (await api.pins(m, body, replace)).step;
      } finally {
        pinsPending--;
      }
    });
    pinChain = done.catch(() => {});
    return done;
  }

  /** A change of places, shown at once: Ctrl+Z or its toast takes it back. */
  function placesChanged(m: number, before: ReadonlyMap<number, Spot>, next: ReadonlyMap<number, Spot>, label: string, said: string, body: Record<string, [number, number] | null>, replace: boolean) {
    showPins(m, next);
    const step: LocalStep = {
      label,
      after: Infinity,
      // the move's own toast would undo something else now
      undo: async () => {
        dismiss(buildToast);
        showPins(m, before);
        await savePins(m, cellsOf(before), true);
      },
      redo: async () => {
        dismiss(buildToast);
        showPins(m, next);
        await savePins(m, cellsOf(next), true);
      },
    };
    localStep(step);
    dismiss(buildToast);
    buildToast = toast(said, { action: 'Undo', onAction: () => void undo() });
    savePins(m, body, replace).then(
      (newest) => (step.after = newest),
      (e) => {
        dropLocalStep(step);
        showPins(m, before);
        sfx('nope');
        dismiss(buildToast);
        toast(messageOf(e), { kind: 'error' });
      },
    );
  }

  /** Build mode: a room carried or nudged to a free cell. The first move on a map keeps every room where it is drawn. */
  function moveRoom(n: number, x: number, y: number) {
    const l = scene?.layout;
    if (!scene || !l || !l.at.has(n) || l.cell.has(cellKey(x, y))) return;
    const first = pins.size === 0;
    const next = new Map(first ? [...l.at].map(([k, s]) => [k, { ...s }]) : pins);
    next.set(n, { x, y });
    const said = first ? `Moved ${called(n)}. The map keeps this layout now: rooms stay where they are drawn.` : `Moved ${called(n)}.`;
    placesChanged(map, pins, next, `Move ${called(n)}`, said, first ? cellsOf(next) : { [String(n)]: [x, y] }, first);
    sfx('pop');
    if (selected?.room !== n) choose(n, 'keep', false);
    else scene.keepInView(n);
  }

  /** Build mode, Shift and a direction: the selected room a cell that way. */
  function nudge(dir: number) {
    const l = scene?.layout;
    const r = selected;
    const step = STEP[dir];
    const p = r ? l?.at.get(r.room) : undefined;
    if (!l || !r || !p || !step) return;
    const [x, y] = [p.x + step[0], p.y + step[1]];
    const there = l.cell.get(cellKey(x, y));
    if (there !== undefined) {
      sfx('nope');
      say(`${cap(called(there))} is in the way.`);
      return;
    }
    moveRoom(r.room, x, y);
  }

  /** Back to the layout the exits make. */
  function layOutAfresh() {
    if (!pins.size) return;
    placesChanged(map, pins, new Map(), `Lay out map ${map} afresh`, `Map ${map} is laid out by its exits again.`, {}, true);
    sfx('toggle');
  }

  // ---- the map's tools -------------------------------------------------------------------

  /** What the map's tools borrow: the scene, the map shown, and the page's ways of showing and saving an edit. */
  const host: Host = {
    get scene() {
      return scene!;
    },
    get canvas() {
      return canvas;
    },
    get map() {
      return map;
    },
    get count() {
      return count;
    },
    get selected() {
      return selected;
    },
    get card() {
      return card;
    },
    get pins() {
      return pins;
    },
    get nameMax() {
      return nameMax;
    },
    coarse,
    say,
    tell: (text) => (live = text),
    called,
    patch,
    send,
    showPins,
    savePins,
    quiet: () => dismiss(buildToast),
    colour: (by) => setColour(by, true),
    stamp: (id) => void holdSaved(id),
  };

  /** The drawer of stamps open (Build mode with it) or put away. */
  async function setStamping(on: boolean) {
    if (on === stamping) return;
    if (on && !kit) {
      try {
        kit = await import('./stamping');
        stamper = new kit.Stamper(host, () => void setStamping(false));
      } catch (e) {
        toast(messageOf(e), { kind: 'error' });
        return;
      }
    }
    stamping = on;
    if (on && walking) stopWalking(false);
    if (on && picking) await setPicking(false);
    if (on && !building) setBuilding(true);
    stamper?.opened(on);
    sfx('toggle');
  }

  /** The stamps opened holding one saved from the map. */
  async function holdSaved(id: string) {
    await setStamping(true);
    if (stamping) stamper?.pickSaved(id);
  }

  /** The Select tool taken up (its panel with it) or put away; the rooms picked wait for it. */
  async function setPicking(on: boolean) {
    if (on === picking) return;
    if (on && !picker) {
      try {
        picker = await import('./picking');
        selector = new picker.Selector(host, () => void setPicking(false));
      } catch (e) {
        toast(messageOf(e), { kind: 'error' });
        return;
      }
    }
    if (on && stamping) await setStamping(false);
    if (on && walking) stopWalking(false);
    picking = on;
    tip = null;
    selector?.turn(on);
    sfx('toggle');
  }

  /** Walk mode: a playtest from the selected room (or the start room), the map following the player. */
  async function startWalking() {
    if (walking || !scene?.layout) return;
    const from = selected?.room ?? scene.start;
    if (!WalkPanel) {
      try {
        WalkPanel = (await import('../story/PlayPanel.svelte')).default;
      } catch (e) {
        toast(messageOf(e), { kind: 'error' });
        return;
      }
    }
    if (building) setBuilding(false);
    if (stamping) await setStamping(false);
    if (picking) await setPicking(false);
    // the room's card makes way for the map
    if (cardOnMap()) closeInspector();
    choose(from, 'keep', false);
    walkStart = { table: 'Rooms', key: [map, from] };
    walking = true;
    tip = null;
    sfx('toggle');
  }

  /** Walk mode ends at the room the player reached, its card open to edit it (unless another tool takes over). */
  function stopWalking(withCard = true) {
    if (!walking) return;
    walking = false;
    walkStart = null;
    if (withCard && selected) choose(selected.room, 'keep', true);
    canvas.focus({ preventScroll: true });
    sfx('toggle');
  }

  /** Where the walk took the player: the map follows, to another map when they crossed to one. */
  function walkedTo(m: number, n: number) {
    if (m !== map) go(m, n);
    else if (selected?.room !== n) choose(n, 'keep', false);
  }

  /** Loads the Show menu's code, if it has not been. */
  function loadShow() {
    if (ShowMenu) return;
    showFailed = '';
    import('./ShowMenu.svelte').then(
      (m) => (ShowMenu = m.default),
      (e) => (showFailed = messageOf(e)),
    );
  }

  // ---- all maps ------------------------------------------------------------------------

  /** All maps: every map as a disc, joined by their map changes, over the map. */
  async function openWorld() {
    if (worldShown || !map) return;
    if (!WorldView) {
      try {
        WorldView = (await import('./WorldView.svelte')).default;
      } catch (e) {
        toast(messageOf(e), { kind: 'error' });
        return;
      }
    }
    stopWalking(false);
    if (stamping) await setStamping(false);
    if (picking) await setPicking(false);
    if (cardOnMap()) closeInspector();
    mapsOpen = false;
    tip = null;
    worldShown = true;
    sfx('toggle');
  }

  /** The picture zooms into the disc picked, and then makes way. */
  let worldTimer: ReturnType<typeof setTimeout> | undefined;
  function leaveWorld() {
    clearTimeout(worldTimer);
    worldTimer = setTimeout(() => (worldShown = false), matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 200);
  }

  /** Back into the map, all of it in view if it was zoomed out past. */
  function closeWorld() {
    leaveWorld();
    scene?.fitIfPast();
    canvas.focus({ preventScroll: true });
    sfx('toggle');
  }

  /** A disc or a line picked: its map, at the room. */
  function fromWorld(m: number, room: number | null) {
    leaveWorld();
    // the map left keeps a view of all its rooms for next time
    scene?.fitIfPast(false);
    if (m !== map) go(m, room);
    else if (room !== null) choose(room, 'centre');
    canvas.focus({ preventScroll: true });
  }

  // the walk's panel along the bottom: the player's room is kept in view above it, under the tools
  $effect(() => {
    const [h, t] = walking ? [walkHigh, toolsHigh] : [0, 0];
    untrack(() => {
      scene?.setBand(t && t + 20, h && h + 24);
      if (h && selected) scene?.keepInView(selected.room);
    });
  });

  async function importLayouts(file: File) {
    try {
      let f: MapFile | null = null;
      try {
        f = JSON.parse(await file.text()) as MapFile;
      } catch {
        // said below
      }
      if (!f || f.daydream !== 'map' || !f.maps || typeof f.maps !== 'object') throw new Error(`${file.name} is not a map layout Daydream exported.`);
      const given = f;
      const before = await api.mapFile();
      const back: MapFile = { daydream: 'map', version: 1, maps: Object.fromEntries(Object.keys(given.maps).map((k) => [k, before.maps[k] ?? { pins: {} }])) };
      const r = await api.importMapFile(given);
      const reread = () => (map ? load(map, true) : Promise.resolve(false));
      localStep({
        label: `Import ${file.name}`,
        after: r.step,
        undo: async () => {
          dismiss(buildToast);
          await api.importMapFile(back);
          await reread();
        },
        redo: async () => {
          dismiss(buildToast);
          await api.importMapFile(given);
          await reread();
        },
      });
      await reread();
      sfx('pop');
      dismiss(buildToast);
      buildToast = toast(`Imported the layouts of ${r.maps === 1 ? 'one map' : `${r.maps} maps`} (${r.pins.toLocaleString()} rooms).`, { action: 'Undo', onAction: () => void undo() });
    } catch (e) {
      sfx('nope');
      toast(messageOf(e), { kind: 'error' });
    }
  }

  // ---- what the map shows --------------------------------------------------------------

  function setColour(by: ColourBy, quiet = false) {
    if (by === colourBy) return;
    colourBy = by;
    write('dd-map-colour', by);
    spot = [];
    allLegend = false;
    scene?.setColour(by);
    legend = scene?.legend() ?? [];
    if (!quiet) sfx('toggle');
  }

  function setLayer(k: keyof Layers, on: boolean) {
    layers = { ...layers, [k]: on };
    write('dd-map-layers', JSON.stringify(layers));
    scene?.setLayers(layers);
    sfx('toggle');
  }

  function setWiresFor(v: WiresFor) {
    if (v === wiresFor) return;
    wiresFor = v;
    write('dd-map-wires', v);
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

  /** The rooms whose wires show: those picked, or the room selected. */
  const wired = $derived<ReadonlySet<number>>(picking && selector?.picked.size ? selector.picked : selected ? new Set([selected.room]) : new Set());

  // the wires follow the rooms they are for, the map's rooms, and the layer's choice
  $effect(() => {
    const [mode, s, rooms] = [wiresFor, scripts, wired];
    void roomsAt;
    untrack(() => {
      if (!scene) return;
      scene.setWires(mode === 'off' ? [] : wiresOf(map, scene.by, s, rooms, mode === 'all').wires, map, mode === 'all');
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

  // full screen and naming end when the World is left
  $effect(() => {
    if (app.route.place === 'world') return;
    untrack(() => {
      if (focus) setFocus(false);
      if (naming) finishNaming(true);
    });
  });

  const filteredMaps = $derived.by(() => {
    const q = mapFilter.trim().toLowerCase();
    return q ? maps.filter(([m, c]) => String(m).startsWith(q) || !!mapName(namesOf, m, c)?.toLowerCase().includes(q)) : maps;
  });

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

  /** A direction key: the walk's player goes that way, Build mode digs, or the selection walks. */
  function step(d: number) {
    if (walking) walker?.walk(d);
    else if (building) build(d, takeCount());
    else walk(d);
  }

  function onKey(e: KeyboardEvent) {
    if (!here() || e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return;
    if (app.paletteOpen || document.querySelector('dialog[open], [role=dialog]')) return;
    const t = e.target;
    if (t instanceof HTMLElement) {
      if (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)) return;
      // the card, the bar and the rail keep their own keys
      if (t.closest('#dd-inspector, #dd-top, #dd-rail, .pop')) return;
    }
    // All maps keeps its own keys: W or Esc go back into the map
    if (worldShown) {
      if (e.key === 'Escape' || (e.key.toLowerCase() === 'w' && !e.shiftKey)) {
        e.preventDefault();
        worldView?.back();
      }
      return;
    }
    if (stamping && stamper?.keys(e)) return;
    if (picking && selector?.keys(e)) return;
    if (e.key === 'Escape') {
      if (scene?.carrying) scene.dropCarry();
      else if (scene?.reachingOut) scene.dropReach();
      else if (countText) countText = '';
      else if (walking) stopWalking();
      else if (building) setBuilding(false);
      else if (focus) setFocus(false);
      return;
    }
    // Build mode: digits first make a line (the number pad's are directions)
    if (building && !e.shiftKey && e.code.startsWith('Digit') && (countText || e.key !== '0')) {
      e.preventDefault();
      countText = (countText + e.key).slice(-2);
      return;
    }
    const d = KEY_DIR[e.code] ?? KEY_DIR[e.key];
    if (d !== undefined && e.shiftKey) {
      // Build mode: Shift moves the room a cell instead
      if (!building) return;
      e.preventDefault();
      countText = '';
      nudge(d);
      return;
    }
    if (d !== undefined) {
      e.preventDefault();
      step(d);
      return;
    }
    const k = e.key.toLowerCase();
    let done = true;
    if (e.shiftKey && e.code === 'Digit1') scene?.fit();
    else if (e.shiftKey && e.code === 'Digit2') {
      if (selected) scene?.frameRoom(selected.room);
    } else if (e.shiftKey) done = false;
    else if (k === 'u') step(8);
    else if (k === 'd') step(9);
    else if (k === 'p') walking ? stopWalking() : void startWalking();
    else if (k === 'w') void openWorld();
    else if (k === 'b') setBuilding(!building);
    else if (k === 's') void setStamping(!stamping);
    else if (k === 'v') void setPicking(!picking);
    else if (k === '.' && building) {
      if (lastMove) build(lastMove.dir, lastMove.count);
    } else if ((e.key === 'Enter' || e.key === 'F2') && building) {
      if (selected) void startNaming();
    } else if (k === 'h' && scene) walking ? walker?.put(map, scene.start) : choose(scene.start, 'centre', !building);
    else if (k === '+' || k === '=') scene?.zoomBy(1.4);
    else if (k === '-' || k === '_') scene?.zoomBy(1 / 1.4);
    else if (k === 'f') setFocus(!focus);
    else done = false;
    if (done) e.preventDefault();
  }

  // ---- start -------------------------------------------------------------------------

  let reloadTimer: ReturnType<typeof setTimeout> | undefined;
  function written() {
    // a newer save, or an undo: Build mode's toast would undo something else now
    if (!pending) dismiss(buildToast);
    // edits under way: the map is read again once they are in
    if (pending) return;
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
        // walking: a room clicked puts the player there
        if (walking) {
          if (r) walker?.put(map, r.room);
          return;
        }
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
      wire: followWire,
      out: () => void openWorld(),
      hover: (h, x, y) => (tip = h ? { x, y, ...tipFor(h) } : null),
      moved: (c) => {
        zoom = c.z;
        if (naming) placeNaming();
      },
      build: (g: Ghost) => build(g.dir),
      move: (n, x, y) => moveRoom(n, x, y),
      reach: (r) => reachOut(r),
      reaching: (r) => {
        reachNote = r ? reachSaid(r) : null;
        reachDrag = !!scene?.reachingOut;
      },
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
    // Shift and Alt change what a stamp held would do; Space and Ctrl+A are the Select tool's
    const onMods = (e: KeyboardEvent) => {
      if (!here()) return;
      stamper?.held(e);
      // Build mode: Shift and a click digs a line to the cell
      if (e.key === 'Shift') scene?.setShift(e.type === 'keydown');
    };
    const onPickKeys = (e: KeyboardEvent) => selector?.onKey(e, here());
    document.addEventListener('fullscreenchange', unfull);
    document.addEventListener('keydown', onKey);
    document.addEventListener('keydown', onMods);
    document.addEventListener('keyup', onMods);
    document.addEventListener('keydown', onPickKeys);
    document.addEventListener('keyup', onPickKeys);
    const letGo = () => {
      selector?.letGo();
      stamper?.letGo();
      scene?.setShift(false);
    };
    window.addEventListener('blur', letGo);
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
      document.removeEventListener('keydown', onMods);
      document.removeEventListener('keyup', onMods);
      document.removeEventListener('keydown', onPickKeys);
      document.removeEventListener('keyup', onPickKeys);
      window.removeEventListener('blur', letGo);
      window.removeEventListener('dd:wrote', written);
      window.removeEventListener('dd:realm', written);
      scene?.destroy();
      scene = null;
    };
  });

  const pct = $derived(`${Math.round(zoom * 100)}%`);
</script>

<div class="world" class:focus class:walking class:narrow={!wideTop} style:--card="{card}px" style:--view="{viewWidth}px" bind:clientWidth={width}>
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

  <div class="float tl" bind:offsetHeight={toolsHigh}>
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
          <button type="button" class="menu-item all" onclick={() => void openWorld()}>
            <span class="num"><Waypoints size={14} /></span>
            <span>All maps<span class="sub">Every map, and the map changes between them</span></span>
            <span class="end"><span class="kbd">W</span></span>
          </button>
          <div class="menu-sep"></div>
          <div class="eyebrow head">Maps in this world</div>
          {#if maps.length > 12}
            <input class="find" type="search" placeholder="Map number or name" aria-label="Find a map" bind:value={mapFilter} />
          {/if}
          <div class="list">
            {#each filteredMaps as [m, c] (m)}
              {@const name = mapName(namesOf, m, c)}
              <button
                type="button"
                class="menu-item"
                class:on={m === map}
                title="Map {m}: {c.toLocaleString()} rooms"
                onclick={() => {
                  mapsOpen = false;
                  mapFilter = '';
                  if (m !== map) go(m, null);
                }}
              >
                <span class="num">{m}</span><span class="nm">{name ?? `Map ${m}`}</span>
                <span class="end faint tnum">{c.toLocaleString()}{#if m === map}<Check size={15} />{/if}</span>
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
    <button
      type="button"
      class="btn glass build-btn"
      class:on={building}
      aria-pressed={building}
      onclick={() => setBuilding(!building)}
      title={building ? 'Done building (B or Esc)' : 'Build: dig, join and name rooms with the keys (B)'}
    >
      <Pickaxe size={15} /><span>Build</span>{#if countText}<b class="count tnum">{countText}×</b>{/if}
    </button>
    <button
      type="button"
      class="btn glass build-btn"
      class:on={stamping}
      aria-pressed={stamping}
      onclick={() => void setStamping(!stamping)}
      title={stamping ? 'Put the stamps away (S)' : 'Stamps: a shape of rooms put down at once (S)'}
    >
      <StampIcon size={15} /><span>Stamps</span>
    </button>
    <button
      type="button"
      class="btn glass build-btn"
      class:on={picking}
      aria-pressed={picking}
      onclick={() => void setPicking(!picking)}
      title={picking ? 'Done selecting (V)' : 'Select: pick rooms and set their fields together (V)'}
    >
      <SquareDashedMousePointer size={15} /><span>Select</span>{#if picking && selector?.picked.size}<b class="count tnum">{selector.picked.size.toLocaleString()}</b>{/if}
    </button>
    <button
      type="button"
      class="btn glass build-btn"
      class:on={walking}
      aria-pressed={walking}
      onclick={() => (walking ? stopWalking() : void startWalking())}
      title={walking ? 'Back to editing (P or Esc)' : 'Walk: playtest from the room with the arrow keys (P)'}
    >
      <Footprints size={15} /><span>Walk</span>
    </button>
    {#if pending}<span class="pill lilac">Saving…</span>{:else if loading}<span class="pill lilac">Reading map…</span>{/if}
  </div>

  <div class="float tr" bind:clientWidth={viewWidth}>
    <div class="seg glass wide-only" role="radiogroup" aria-label="Colour rooms by">
      {#each COLOUR_BY as c (c.id)}
        <button type="button" role="radio" aria-checked={colourBy === c.id} class:on={colourBy === c.id} onclick={() => setColour(c.id)}>{c.label}</button>
      {/each}
    </div>
    <Popover.Root onOpenChange={(open) => open && loadShow()}>
      <Popover.Trigger class="btn glass" aria-label="What the map shows"><LayersIcon size={15} /><span class="wide-only">Show</span></Popover.Trigger>
      <Popover.Portal>
        <Popover.Content class="pop layers-pop" sideOffset={8} align="end" collisionPadding={8}>
          {#if ShowMenu}
            <ShowMenu
              {map}
              {colourBy}
              {layers}
              {wiresFor}
              {pinCount}
              {startRoom}
              {pal}
              oncolour={setColour}
              onlayer={setLayer}
              onwires={setWiresFor}
              onlayout={layOutAfresh}
              onimport={(f) => void importLayouts(f)}
            />
          {:else}
            <p class="lay">{showFailed || 'Reading the menu…'}</p>
          {/if}
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

  {#if stamping && kit && stamper}
    <kit.StampDrawer {stamper} colours={pal.stamp} onclose={() => void setStamping(false)} />
  {/if}

  {#if picking && picker && selector}
    <picker.PickBar {selector} onclose={() => void setPicking(false)} />
  {/if}

  {#if walking && WalkPanel && walkStart}
    <WalkPanel bind:this={walker} bind:high={walkHigh} start={walkStart} walking onclose={() => stopWalking()} onmove={walkedTo} />
  {/if}

  {#if worldShown && WorldView}
    <WorldView bind:this={worldView} {maps} {names} {links} current={map} {pal} onopen={fromWorld} onclose={closeWorld} />
  {/if}

  {#if roomy && !walking}
    <div class="float hint map-hint glass" aria-hidden="true">
      {#if note}
        {note}
      {:else if stamper?.kind}
        <b class="mode">Stamp</b> click to put it down <span class="kbd">R</span> turn <span class="kbd">X</span><span class="kbd">Y</span> mirror {#if stamper.kind.sizes.length}<span class="kbd">[</span><span class="kbd">]</span> size {/if}{#if stamper.kind.random}<span class="kbd">Space</span> roll {/if}<span class="kbd">J</span> join <span class="kbd">Esc</span> put away
      {:else if stamping}
        <b class="mode">Stamps</b> pick a shape <span class="kbd">1</span>…<span class="kbd">9</span> <span class="kbd">S</span> put them away
      {:else if picking && selector}
        {#if selector.tool === 'paint'}
          <b class="mode">Paint</b> drag over rooms <span class="kbd">[</span><span class="kbd">]</span> size <span class="kbd">Alt</span> take a room's {selector.paintDef.label.toLowerCase()} <span class="kbd">1</span>…<span class="kbd">4</span> pick rooms <span class="kbd">Space</span> pan <span class="kbd">Esc</span> {selector.picked.size ? 'let go' : 'done'}
        {:else}
          <b class="mode">Select</b> <span class="kbd">1</span>…<span class="kbd">5</span> how {#if selector.tool === 'brush'}<span class="kbd">[</span><span class="kbd">]</span> size {/if}<span class="kbd">Shift</span> add <span class="kbd">Alt</span> take away <span class="kbd">Space</span> pan <span class="kbd">Esc</span> {selector.picked.size ? 'let go' : 'done'}
        {/if}
      {:else if building && reachNote}
        <b class="mode">{reachNote[0]}</b> {reachNote[1]} {#if reachDrag}<span class="kbd">Esc</span> cancel{:else}· click to make it{/if}
      {:else if building}
        <b class="mode">Build</b><span class="kbd">←↑→↓</span> dig or join <span class="kbd">3</span><span class="kbd">→</span> a line <span class="kbd">Shift</span> click a line <span class="kbd">.</span> again <span class="kbd">Enter</span> name <span class="kbd">B</span> done
      {:else if selected}
        <span class="kbd">←↑→↓</span> walk its exits <span class="kbd">U</span><span class="kbd">D</span> up and down <span class="kbd">P</span> playtest <span class="kbd">Shift 2</span> close up
      {:else}
        Click a room for its card <span class="kbd">H</span> start room <span class="kbd">P</span> playtest <span class="kbd">W</span> all maps <span class="kbd">F</span> full screen
      {/if}
    </div>
  {/if}

  {#if naming && namingAt}
    <input
      bind:this={nameField}
      class="name-field"
      style:left="{namingAt.x}px"
      style:top="{namingAt.y}px"
      maxlength={nameMax}
      spellcheck="false"
      autocomplete="off"
      aria-label="Room {map}/{naming.room}'s name"
      bind:value={naming.value}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          finishNaming(true);
        } else if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          finishNaming(false);
        }
      }}
      onblur={() => finishNaming(true)}
    />
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
  /* the tools' cursors: the canvas names the tool on */
  .map:global([data-tool='stamp']), .map:global([data-tool='box']), .map:global([data-tool='lasso']), .map:global([data-tool='brush']), .map:global([data-tool='paint']) { cursor: crosshair; }
  .map:global([data-tool='wand']) { cursor: pointer; }
  /* Paint's pipette: Lucide's, drawn dark over light so it shows on either theme's map */
  .map:global([data-tool='dropper']) {
    cursor: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' fill='none' stroke-linecap='round' stroke-linejoin='round'%3E%3Cg id='p'%3E%3Cpath d='m12 9-8.414 8.414A2 2 0 0 0 3 18.828v1.344a2 2 0 0 1-.586 1.414A2 2 0 0 1 3.828 21h1.344a2 2 0 0 0 1.414-.586L15 12M18 9l.4.4a1 1 0 1 1-3 3l-3.8-3.8a1 1 0 1 1 3-3l.4.4 3.4-3.4a1 1 0 1 1 3 3zM2 22l.414-.414'/%3E%3C/g%3E%3Cuse href='%23p' stroke='%23fff' stroke-width='4'/%3E%3Cuse href='%23p' stroke='%23111' stroke-width='2'/%3E%3C/svg%3E") 2 22, copy;
  }
  .map:global(.pan-key) { cursor: grab; }
  .map:global(.panning), .map:global(.carrying) { cursor: grabbing; }
  .map:global(.reaching) { cursor: crosshair; }
  .map:focus-visible { box-shadow: inset 0 0 0 3px var(--focus); }

  .float { position: absolute; z-index: 5; display: flex; align-items: center; gap: 6px; }
  .glass {
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid var(--line-2);
    box-shadow: 0 2px 0 color-mix(in srgb, var(--shadow) 12%, transparent), var(--float);
  }
  .float :global(.btn.glass) { border: 1px solid var(--line-2); }
  .tl { top: var(--edge); left: var(--edge); flex-wrap: wrap; max-width: calc(100% - var(--card) - 2 * var(--edge) - var(--view) - 4px); }
  .tr { top: var(--edge); right: calc(var(--edge) + var(--card)); }
  :global(.map-chip) { gap: 8px !important; padding: 0 12px 0 6px !important; border-radius: 999px !important; }
  :global(.map-chip) .ic { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; background: var(--mint-2); color: var(--mint-ink); }
  :global(.map-chip) small { font-weight: 700; color: var(--ink-3); font-size: 12px; }

  .build-btn { gap: 7px; }
  .build-btn.on { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); box-shadow: 0 2px 0 var(--outline); }
  .build-btn .count { font-size: 12px; padding: 0 6px; border-radius: 6px; background: var(--surface); color: var(--ink); }
  .hint .mode { color: var(--lilac-ink); margin-right: 2px; }
  .name-field {
    position: absolute; z-index: 7; transform: translate(-50%, -50%);
    width: min(320px, 70%); height: 30px; padding: 0 12px; border-radius: 999px;
    border: 2px solid var(--outline); background: var(--surface); color: var(--ink);
    font: 800 13px var(--font-body, Nunito, sans-serif); text-align: center; box-shadow: 0 3px 0 var(--shadow);
  }
  .name-field:focus { outline: 3px solid var(--focus); outline-offset: 1px; }
  .lay { margin: 10px; font-size: 12.5px; color: var(--ink-3); }

  .seg { display: inline-flex; gap: 2px; padding: 3px; border-radius: 12px; }
  .seg button { min-height: 28px; padding: 0 10px; border: 0; border-radius: 9px; background: transparent; font-size: 12.5px; color: var(--ink-2); box-shadow: none; }
  .seg button:hover:not(.on) { background: var(--surface-3); }
  .seg button.on { background: var(--lilac-2); color: var(--lilac-ink); box-shadow: 0 0 0 1px var(--line-2); }

  :global(.maps-pop) { width: min(300px, calc(100vw - 16px)); }
  :global(.maps-pop) .head { padding: 8px 10px 6px; }
  :global(.maps-pop) .list { max-height: min(360px, 50vh); overflow: auto; }
  :global(.maps-pop) .find { margin: 0 6px 6px; width: calc(100% - 12px); }
  :global(.maps-pop) .num { min-width: 30px; height: 22px; padding: 0 6px; border-radius: 8px; display: inline-grid; place-items: center; background: var(--mint-2); color: var(--mint-ink); font: 700 12px var(--font-mono); }
  :global(.maps-pop) .none { margin: 6px 10px; font-size: 13px; }
  :global(.maps-pop) .nm { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  :global(.maps-pop) .all .num { background: var(--lilac-2); color: var(--lilac-ink); }
  :global(.layers-pop) { width: min(360px, calc(100vw - 16px)); max-height: min(78vh, 640px); overflow: auto; }

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

  .narrow .wide-only { display: none !important; }
  @media (max-width: 1100px) {
    .wide-only { display: none !important; }
  }
  @media (max-width: 700px) {
    .overview { width: auto; }
    .mini { display: none; }
  }
  /* walking on a phone: the tools make way for the map (the walk's panel goes back to editing) */
  @media (max-width: 760px) {
    .walking .build-btn { display: none; }
  }
</style>
