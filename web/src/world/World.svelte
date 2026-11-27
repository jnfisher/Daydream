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
-->
<!-- loaded when the World is first opened, so its styles travel with its code -->
<svelte:options css="injected" />

<script lang="ts">
  import { Popover } from 'bits-ui';
  import { onMount, tick, untrack } from 'svelte';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Download from '@lucide/svelte/icons/download';
  import LayersIcon from '@lucide/svelte/icons/layers';
  import LocateFixed from '@lucide/svelte/icons/locate-fixed';
  import MapIcon from '@lucide/svelte/icons/map';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import Minus from '@lucide/svelte/icons/minus';
  import Pickaxe from '@lucide/svelte/icons/pickaxe';
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Scan from '@lucide/svelte/icons/scan';
  import StampIcon from '@lucide/svelte/icons/stamp';
  import Upload from '@lucide/svelte/icons/upload';
  import X from '@lucide/svelte/icons/x';
  import { api } from '../api';
  import { read, sfx, write } from '../shell/prefs.svelte';
  import { app, closeInspector, dropLocalStep, inspect, localStep, realmChanged, undo, type LocalStep } from '../shell/state.svelte';
  import { dismiss, messageOf, toast, toasts } from '../shell/toasts.svelte';
  import { dug, joined, moveFrom, renamed } from './build';
  import { ICONS } from './icons';
  import { COLOUR_BY, EXIT_LEGEND, palette, ROOM_LEGEND, type ColourBy, type ExitLook, type IconName } from './look';
  import { cellKey, STEP, type Spot } from './layout';
  import { DIR_WORDS, kindName, type MapFile, type MapRoom, type PinCells } from './model';
  import { Scene, type Camera, type Ghost, type Hit, type Layers, type LegendEntry, type Stub } from './scene';
  import type { Placed, StampKind, StampOptions, StampRequest, Turn } from './stamps';

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
  // Stamps
  /** The stamps' code, once the drawer has been opened. */
  let kit = $state<typeof import('./stamping') | null>(null);
  const UPRIGHT: Turn = { turns: 0, flipX: false, flipY: false };
  /** The stamp drawer is open. */
  let stamping = $state(false);
  /** The stamp held, its sizes, its roll, how it is turned, and whether it joins the rooms it touches. */
  let stampKind = $state<StampKind | null>(null);
  let stampSizes = $state<number[]>([]);
  let stampSeed = $state(1);
  let stampTurn = $state<Turn>(UPRIGHT);
  let stampSeam = $state(false);
  /** Leave the rooms already there alone, and replace the exits in the way: switched on, or held down (Shift, Alt). */
  let stampKeep = $state(false);
  let stampOverwrite = $state(false);
  /** The cell the held stamp's first cell is over. */
  let aim = $state<[number, number] | null>(null);
  /** Shift (leave the rooms already there alone) and Alt (replace the exits in the way), held down. */
  let mods = $state({ shift: false, alt: false });
  /** A finger, not a mouse: a stamp is aimed with one tap and put down with another. */
  const coarse = typeof matchMedia === 'function' && matchMedia('(pointer: coarse)').matches;
  /** The held stamp as it would go down where it is aimed, and the room its new rooms copy. */
  let ghost = $state<Placed | null>(null);
  let ghostBrush = $state<number | null>(null);


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
    const gen = edits;
    try {
      const a = await api.map(m);
      // an edit was shown meanwhile: this read may not have it, and the one after it will
      if (a.map === map && gen !== edits) return false;
      const moved = a.map !== map;
      if (moved) {
        scene.select(null);
        selected = null;
        aim = null;
      }
      map = a.map;
      maps = a.maps;
      count = a.rooms.length;
      write(lastKey(), String(map));
      scene.setEdited(edited);
      // moves being saved are newer than this read's
      if (moved || !pinsPending) {
        pins = pinsFrom(a.pins);
        pinCount = pins.size;
      }
      scene.setRooms(a.rooms, pins);
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
      reghost();
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
    if (!roomy) {
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
    if (s.sort === 'map') go(e.para1, e.to);
    else if (s.sort === 'missing') {
      sfx('nope');
      say(`Map ${map} has no room ${e.to}.`);
    } else choose(e.to, 'centre');
  }

  const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

  function tipFor(h: Hit): { title: string; sub: string } {
    if (h.ghost) {
      const from = named(selected?.room ?? 0);
      const way = DIR_WORDS[h.ghost.dir] ?? '';
      if (h.ghost.to === null) return { title: `Dig a room ${way}`, sub: `A copy of ${from}, with an exit each way.` };
      return { title: `Join ${way} to ${named(h.ghost.to)}`, sub: `An exit each way between ${from} and room ${map}/${h.ghost.to}.` };
    }
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
    building = on;
    countText = '';
    if (!on && naming) finishNaming(true);
    if (!on && stamping) {
      stamping = false;
      pickStamp(null);
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
    // a new room is drawn beside the room it is dug from, not where an old room of its number was
    const old = rooms.filter((n) => pins.has(n));
    if (old.length) {
      const next = new Map(pins);
      for (const n of old) next.delete(n);
      showPins(m, next);
      void savePins(m, Object.fromEntries(old.map((n) => [String(n), null])), false).catch(() => {});
    }
  }

  function joinRooms(from: number, dir: number, to: number) {
    if (!scene) return;
    const what = `Joined ${called(from)} ${DIR_WORDS[dir]} to ${called(to)}`;
    patch(joined(scene.rooms, from, dir, to).rooms);
    const m = map;
    send(() => api.dig({ map: m, room: from, dir, to_map: m, to_room: to, existing: true }), what);
  }

  /** Shows the map's rooms as an edit leaves them, before the server has them. */
  function patch(rooms: MapRoom[]) {
    if (!scene) return;
    edits++;
    scene.setRooms(rooms);
    count = rooms.length;
    maps = maps.map(([m, c]) => [m, m === map ? rooms.length : c]);
    selected = selected ? (scene.by.get(selected.room) ?? null) : null;
    legend = scene.legend();
    reghost();
  }

  /** Sends an edit once those before it are in; `done` says what it did, in a toast that can undo it. */
  function send(work: () => Promise<{ notes?: string[] } | unknown>, done: string) {
    pending++;
    chain = chain.then(async () => {
      if (broken) return;
      try {
        const out = (await work()) as { notes?: string[] } | undefined;
        const notes = out?.notes?.length ? ` ${out.notes.join(' ')}` : '';
        dismiss(buildToast);
        buildToast = toast(`${done}.${notes}`, { action: 'Undo', onAction: () => void undo() });
      } catch (e) {
        // what came after it rested on it: the map is read again as the server has it
        broken = true;
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
  let importer = $state<HTMLInputElement | undefined>();

  const pinsFrom = (c: PinCells | undefined) => new Map(Object.entries(c ?? {}).map(([k, [x, y]]) => [Number(k), { x, y }]));
  const cellsOf = (p: ReadonlyMap<number, Spot>): Record<string, [number, number]> => Object.fromEntries([...p].map(([n, s]) => [String(n), [s.x, s.y]]));

  /** Draws map `m`'s rooms with these places, if it is the map shown. */
  function showPins(m: number, next: ReadonlyMap<number, Spot>) {
    if (m !== map || !scene) return;
    pins = next;
    pinCount = next.size;
    scene.setPins(next);
    reghost();
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

  // ---- stamps ---------------------------------------------------------------------------

  /** The drawer of stamps open (Build mode with it) or put away. */
  async function setStamping(on: boolean) {
    if (on === stamping) return;
    if (on && !kit) {
      try {
        kit = await import('./stamping');
      } catch (e) {
        toast(messageOf(e), { kind: 'error' });
        return;
      }
    }
    stamping = on;
    if (on && !building) setBuilding(true);
    if (!on) {
      pickStamp(null);
      stampKeep = stampOverwrite = false;
    }
    say(on ? (coarse ? 'Stamps: pick a shape, then tap the map where it goes.' : 'Stamps: pick a shape (1 to 9), then click the map to put it down.') : 'Stamps put away.', true);
    sfx('toggle');
  }

  /** Picks a stamp up (or puts the one held away). */
  function pickStamp(k: StampKind | null) {
    stampKind = k;
    tip = null;
    if (k) {
      stampSizes = k.sizes.map((z) => z.start);
      stampTurn = UPRIGHT;
      stampSeed = Math.floor(Math.random() * 2 ** 31);
      aim ??= startAim();
      // too far out to aim at a cell: come in close where it aims
      if (aim && scene && scene.cam.z < 0.35) scene.lookAt(aim[0], aim[1], 0.75, openMiddle());
      say(
        coarse
          ? `${k.name}: tap the map to aim it, and tap it again to put it down.`
          : `${k.name}: click the map to put it down. R turns it, X and Y mirror it${k.sizes.length ? ', [ and ] size it' : ''}.`,
        true,
      );
    } else {
      aim = null;
      mods = { shift: false, alt: false };
    }
    scene?.setHolding(!!k);
    reghost();
  }

  /** Where a stamp first aims: the selected room, else the middle of what the map shows. */
  function startAim(): [number, number] | null {
    const p = selected ? scene?.layout?.at.get(selected.room) : undefined;
    if (p) return [p.x, p.y];
    return scene && canvas ? scene.cellAt(...openMiddle()) : null;
  }

  /** The middle of the map that the card and the stamp drawer leave showing, on the canvas. */
  function openMiddle(): [number, number] {
    const c = canvas.getBoundingClientRect();
    let [x0, x1, y0, y1] = [0, canvas.clientWidth - card, 0, canvas.clientHeight];
    const d = canvas.parentElement?.querySelector('section.drawer')?.getBoundingClientRect();
    if (d?.width) {
      // a sheet along the bottom (a small screen), else a panel down the left
      if (d.top - c.top > c.height * 0.3) y1 = Math.min(y1, d.top - c.top);
      else x0 = Math.max(x0, d.right - c.left);
    }
    return [(x0 + x1) / 2, (y0 + y1) / 2];
  }

  const stampOptions = (m = mods): StampOptions => ({ keep: m.shift || stampKeep, overwrite: m.alt || stampOverwrite, seam: stampSeam });

  /** The held stamp, drawn where it would go down. */
  function reghost() {
    if (!scene?.layout || !stampKind || !aim || !kit) {
      ghost = null;
      ghostBrush = null;
      scene?.setStamp(null);
      return;
    }
    const p = kit.placed(scene.by, scene.layout, kit.turned(stampKind.make(stampSizes, stampSeed), stampTurn), aim[0], aim[1], stampOptions());
    ghost = p;
    ghostBrush = kit.brushOf(p, selected?.room ?? scene.start);
    scene.setStamp(p);
  }

  function sizeStamp(i: number, by: number) {
    const z = stampKind?.sizes[i];
    if (!z) return;
    stampSizes = stampSizes.map((v, j) => (j === i ? Math.max(z.min, Math.min(z.max, v + by)) : v));
    reghost();
  }

  /** A quarter turn clockwise, as the map shows it. */
  function turnStamp() {
    stampTurn = { ...stampTurn, turns: (stampTurn.turns + 1) % 4 };
    reghost();
  }

  /** Mirrors the stamp as the map shows it: once turned a quarter, across is up and down before the turn. */
  function flipStamp(axis: 'x' | 'y') {
    const across = (axis === 'x') === (stampTurn.turns % 2 === 0);
    stampTurn = across ? { ...stampTurn, flipX: !stampTurn.flipX } : { ...stampTurn, flipY: !stampTurn.flipY };
    reghost();
  }

  function rollStamp() {
    stampSeed = Math.floor(Math.random() * 2 ** 31);
    reghost();
  }

  function seamStamp(on: boolean) {
    stampSeam = on;
    reghost();
  }

  function keepStamp(on: boolean) {
    stampKeep = on;
    reghost();
  }

  function overwriteStamp(on: boolean) {
    stampOverwrite = on;
    reghost();
  }

  /** What the held stamp would do where it is aimed, for the drawer. */
  const ghostSaid = $derived.by(() => {
    if (!ghost || !kit) return '';
    const makes = ghost.rooms.length || ghost.links.some((l) => l.sort === 'make' || l.sort === 'replace');
    if (!makes) return ghost.count.left ? 'Here: nothing, the exits in its way are left (Alt replaces them).' : 'Here: nothing, it is all there already.';
    return `Here: ${kit.summary(map, ghost)}.`;
  });

  /** Puts the held stamp down with its first cell at (x, y). */
  function putStamp(x: number, y: number, m: { shift: boolean; alt: boolean }) {
    if (!scene?.layout || !stampKind || !kit) return;
    aim = [x, y];
    const o = stampOptions(m);
    const p = kit.placed(scene.by, scene.layout, kit.turned(stampKind.make(stampSizes, stampSeed), stampTurn), x, y, o);
    if (!p.rooms.length && !p.links.some((l) => l.sort === 'make' || l.sort === 'replace')) {
      sfx('nope');
      say(p.count.left ? 'Nothing to put down here: the exits in its way are left (hold Alt to replace them).' : 'Nothing to put down here: it is all there already.');
      reghost();
      return;
    }
    const brushNo = kit.brushOf(p, selected?.room ?? scene.start);
    const brush = brushNo === null ? undefined : scene.by.get(brushNo);
    if (!brush) {
      sfx('nope');
      say('Pick a room first: new rooms copy it.');
      return;
    }
    const m0 = map;
    const what = kit.named(stampKind);
    // where its new rooms are drawn: the first stamp on a map keeps every room where it is drawn
    const before = pins;
    const first = pins.size === 0;
    const next = new Map(first ? [...scene.layout.at].map(([k, s]) => [k, { ...s }]) : pins);
    for (const c of p.cells) if (c.sort === 'new' && c.room !== null) next.set(c.room, { x: c.x, y: c.y });
    const said = `Stamped ${what}: ${kit.summary(m0, p)}${first ? '. The map keeps this layout now: rooms stay where they are drawn' : ''}`;
    patch(kit.stamped(scene.rooms, p, brush));
    showPins(m0, next);
    sfx('pop');
    sendStamp(m0, kit.request(m0, brush.room, p, o, what), before, next, said);
  }

  /**
   * Writes a stamp once the edits before it are in, then the places its rooms are drawn in;
   * Ctrl+Z (or its toast) takes back both at once.
   */
  function sendStamp(m: number, req: StampRequest, before: ReadonlyMap<number, Spot>, next: ReadonlyMap<number, Spot>, said: string) {
    pending++;
    chain = chain.then(async () => {
      if (broken) return;
      try {
        const out = await api.stamp(req);
        const step = out.step;
        await savePins(m, cellsOf(next), true);
        if (step !== null) {
          localStep({
            label: `Stamp ${req.what}`,
            after: step,
            undo: async () => {
              dismiss(buildToast);
              await api.undo(step);
              showPins(m, before);
              await savePins(m, cellsOf(before), true);
              realmChanged();
            },
            redo: async () => {
              dismiss(buildToast);
              await api.redo(step);
              showPins(m, next);
              await savePins(m, cellsOf(next), true);
              realmChanged();
            },
          });
        }
        const notes = out.notes?.length ? ` ${out.notes.join(' ')}` : '';
        dismiss(buildToast);
        buildToast = toast(`${said}.${notes}`, { action: 'Undo', onAction: () => void undo() });
      } catch (e) {
        broken = true;
        sfx('nope');
        showPins(m, before);
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

  /** Shift and Alt held down change what the held stamp would do: shown as they change. */
  function onMods(e: KeyboardEvent) {
    if (!stampKind || !here()) return;
    const m = { shift: e.shiftKey, alt: e.altKey };
    if (e.key === 'Alt') e.preventDefault();
    if (m.shift === mods.shift && m.alt === mods.alt) return;
    mods = m;
    reghost();
  }

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

  // full screen and naming end when the World is left
  $effect(() => {
    if (app.route.place === 'world') return;
    untrack(() => {
      if (focus) setFocus(false);
      if (naming) finishNaming(true);
    });
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

  /** The stamp drawer's keys: 1 to 9 pick a stamp, and the one held is moved, turned, sized and put down. Answers whether the key was one. */
  function stampKeys(e: KeyboardEvent): boolean {
    const done = () => {
      e.preventDefault();
      return true;
    };
    if (e.key === 'Escape') {
      if (stampKind) pickStamp(null);
      else void setStamping(false);
      return done();
    }
    if (!e.shiftKey && /^Digit[1-9]$/.test(e.code)) {
      const k = kit?.STAMPS[Number(e.code.slice(5)) - 1] ?? null;
      pickStamp(k && k.id !== stampKind?.id ? k : null);
      return done();
    }
    if (!stampKind) return false;
    const d = KEY_DIR[e.code] ?? KEY_DIR[e.key];
    if (d !== undefined && !e.shiftKey) {
      const [dx, dy] = STEP[d]!;
      aim = aim ? [aim[0] + dx, aim[1] + dy] : startAim();
      if (aim) scene?.showCell(aim[0], aim[1]);
      reghost();
      return done();
    }
    if (e.key === 'Enter') {
      if (aim) putStamp(aim[0], aim[1], { shift: e.shiftKey, alt: false });
      return done();
    }
    const k = e.key.toLowerCase();
    if (k === 'r') turnStamp();
    else if (k === 'x') flipStamp('x');
    else if (k === 'y') flipStamp('y');
    else if (e.key === '[') sizeStamp(0, -1);
    else if (e.key === ']') sizeStamp(0, 1);
    else if (e.key === '{') sizeStamp(1, -1);
    else if (e.key === '}') sizeStamp(1, 1);
    else if (e.key === ' ' && stampKind.random) rollStamp();
    else if (k === 'j') seamStamp(!stampSeam);
    else return false;
    return done();
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
    if (stamping && stampKeys(e)) return;
    if (e.key === 'Escape') {
      if (scene?.carrying) scene.dropCarry();
      else if (countText) countText = '';
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
      if (building) build(d, takeCount());
      else walk(d);
      return;
    }
    const k = e.key.toLowerCase();
    let done = true;
    if (e.shiftKey && e.code === 'Digit1') scene?.fit();
    else if (e.shiftKey && e.code === 'Digit2') {
      if (selected) scene?.frameRoom(selected.room);
    } else if (e.shiftKey) done = false;
    else if (k === 'u') building ? build(8, takeCount()) : walk(8);
    else if (k === 'd') building ? build(9, takeCount()) : walk(9);
    else if (k === 'b') setBuilding(!building);
    else if (k === 's') void setStamping(!stamping);
    else if (k === '.' && building) {
      if (lastMove) build(lastMove.dir, lastMove.count);
    } else if ((e.key === 'Enter' || e.key === 'F2') && building) {
      if (selected) void startNaming();
    } else if (k === 'h' && scene) choose(scene.start, 'centre', !building);
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
      moved: (c) => {
        zoom = c.z;
        if (naming) placeNaming();
      },
      build: (g: Ghost) => build(g.dir),
      move: (n, x, y) => moveRoom(n, x, y),
      aim: (x, y) => {
        aim = [x, y];
        reghost();
      },
      stamp: (x, y, m) => putStamp(x, y, m),
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
    document.addEventListener('keydown', onMods);
    document.addEventListener('keyup', onMods);
    const letGo = () => {
      if (mods.shift || mods.alt) {
        mods = { shift: false, alt: false };
        reghost();
      }
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
      window.removeEventListener('blur', letGo);
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
    {#if pending}<span class="pill lilac">Saving…</span>{:else if loading}<span class="pill lilac">Reading map…</span>{/if}
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
          <div class="eyebrow head">Layout</div>
          <p class="lay">
            {#if pinCount}
              Map {map} is laid out by hand: rooms stay where they are drawn, and new rooms join them by their exits.
            {:else}
              Laid out by the exits, from room {startRoom}. In Build mode, drag a room (or press Shift and an arrow) to move it; the map then keeps its layout.
            {/if}
          </p>
          <div class="lay-acts">
            {#if pinCount}
              <button type="button" class="btn sm" onclick={layOutAfresh}><RotateCcw size={13} />Lay out by exits</button>
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
                if (f) void importLayouts(f);
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

  {#if stamping && kit}
    <kit.StampDrawer
      kind={stampKind}
      sizes={stampSizes}
      seam={stampSeam}
      keep={stampKeep || mods.shift}
      overwrite={stampOverwrite || mods.alt}
      touch={coarse}
      brush={ghostBrush === null ? 'the room picked' : called(ghostBrush)}
      said={ghostSaid}
      colours={pal.stamp}
      onpick={pickStamp}
      onsize={sizeStamp}
      onturn={turnStamp}
      onflip={flipStamp}
      onroll={rollStamp}
      onseam={seamStamp}
      onkeep={keepStamp}
      onoverwrite={overwriteStamp}
      onclose={() => void setStamping(false)}
    />
  {/if}

  {#if roomy}
    <div class="float hint map-hint glass" aria-hidden="true">
      {#if note}
        {note}
      {:else if stampKind}
        <b class="mode">Stamp</b> click to put it down <span class="kbd">R</span> turn <span class="kbd">X</span><span class="kbd">Y</span> mirror {#if stampKind.sizes.length}<span class="kbd">[</span><span class="kbd">]</span> size {/if}{#if stampKind.random}<span class="kbd">Space</span> roll {/if}<span class="kbd">J</span> join <span class="kbd">Esc</span> put away
      {:else if stamping}
        <b class="mode">Stamps</b> pick a shape <span class="kbd">1</span>…<span class="kbd">9</span> <span class="kbd">S</span> put them away
      {:else if building}
        <b class="mode">Build</b><span class="kbd">←↑→↓</span> dig or join <span class="kbd">3</span><span class="kbd">→</span> a line <span class="kbd">.</span> again <span class="kbd">Enter</span> name <span class="kbd">B</span> done
      {:else if selected}
        <span class="kbd">←↑→↓</span> walk its exits <span class="kbd">U</span><span class="kbd">D</span> up and down <span class="kbd">Shift 2</span> close up
      {:else}
        Click a room for its card <span class="kbd">H</span> start room <span class="kbd">F</span> full screen
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
  .map:global(.panning), .map:global(.carrying) { cursor: grabbing; }
  .map:global(.stamping) { cursor: crosshair; }
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
  .lay { margin: 0 10px 8px; font-size: 12.5px; line-height: 1.4; color: var(--ink-2); max-width: 290px; }
  .lay-acts { display: flex; flex-wrap: wrap; gap: 6px; margin: 0 10px 8px; }
  .lay-acts .btn { gap: 5px; }

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
