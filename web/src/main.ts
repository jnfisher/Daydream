// Daydream's web UI: the frame (the top bar, the rail of places, the Ctrl+K palette, the
// inspector and the toasts) around the places. Home, the World map, the Library, Story, the
// Workshop, Worlds and Publish are the frame's own; Players' editor and the old map
// (#/map-classic) are today's pages, shown inside it until the redesign reaches each one.

import './styles/tokens.css';
import './styles/base.css';
import './styles/legacy.css';
import { mount } from 'svelte';
import { api } from './api';
import * as editor from './editor';
import { games } from './games';
import { mapPage } from './map';
import Home from './shell/Home.svelte';
import Inspector from './shell/Inspector.svelte';
import Palette from './shell/Palette.svelte';
import { JUMPS } from './shell/places';
import Publish from './shell/Publish.svelte';
import { resumePublish } from './shell/publishing.svelte';
import PublishSheet from './shell/PublishSheet.svelte';
import Rail from './shell/Rail.svelte';
import { parse, recordHref, type Place, type Route } from './shell/routes';
import { app, closeInspector, loadRecent, redo, refreshNow, refreshStatus, undo } from './shell/state.svelte';
import { messageOf, toast } from './shell/toasts.svelte';
import ToastStack from './shell/ToastStack.svelte';
import TopBar from './shell/TopBar.svelte';
import Worlds from './shell/Worlds.svelte';

const $ = (id: string) => document.getElementById(id)!;

/** The section of the page each place shows. */
const SECTION: Record<Place, string> = {
  home: 'home',
  library: 'library',
  players: 'players',
  world: 'world',
  story: 'story',
  workshop: 'workshop',
  worlds: 'games',
  publish: 'publish',
};

// ---- the places, made the first time they are shown -------------------------------------

let theMap: ReturnType<typeof mapPage> | null = null;
let world: { show: (rest: string[], fresh: boolean) => Promise<void> } | null = null;
let library: { show: (r: Route) => Promise<boolean>; here: () => string; step: (by: number) => void } | null = null;
let worlds: { load: (refresh?: boolean) => Promise<void> } | null = null;
let story: { show: (rest: string[]) => void } | null = null;
let workshop: { show: (rest: string[]) => void } | null = null;
const made = new Set<string>();

function once(id: string, make: () => void) {
  if (made.has(id)) return;
  made.add(id);
  make();
}

// ---- routing -------------------------------------------------------------------------

let last: Route | null = null;
/** Where Library and Players were left, so their rail links go back there. */
const left: Partial<Record<'library' | 'players', string>> = {};

/** The hash a route is linked by, so a link copied from the address bar is the stable one. */
function canonical(r: Route): string {
  switch (r.place) {
    case 'library':
    case 'players':
      return r.table ? recordHref(r.table, r.key) : `#/${r.place}`;
    case 'world':
      if (r.rest[0] === 'classic') return ['#/map-classic', ...r.rest.slice(1)].join('/');
      return r.rest.length ? `#/world/${r.rest.join('/')}` : '#/world';
    case 'workshop':
      return r.rest[0] ? `#/workshop/${encodeURIComponent(r.rest[0])}` : '#/workshop';
    case 'story':
      return ['#/story', ...r.rest.map(encodeURIComponent)].join('/');
    default:
      return `#/${r.place}`;
  }
}

async function route() {
  let r = parse(location.hash, app.tables);
  if (!app.current && r.place !== 'worlds') {
    // nothing open yet: the picker is the page
    history.replaceState(null, '', '#/worlds');
    r = { place: 'worlds', rest: [] };
  }
  if ((r.place === 'library' || r.place === 'players') && !r.table && left[r.place]) {
    // the rail's link: back to where it was left
    history.replaceState(null, '', left[r.place]);
    r = parse(left[r.place]!, app.tables);
  }
  const was = last;
  if (r.place === 'library' && !library) {
    // the Library's code loads the first time it is opened
    const { default: Library } = await import('./library/Library.svelte');
    library ??= mount(Library, { target: $('library') });
  }
  const records = r.place === 'library' ? library : r.place === 'players' ? { show: editor.showRoute, here: editor.here } : null;
  if (records) {
    if (!(await records.show(r))) {
      // unsaved edits kept: the address goes back to them
      history.replaceState(null, '', records.here());
      return;
    }
    left[r.place as 'library' | 'players'] = records.here();
  }
  const want = records ? records.here() : canonical(r);
  if (location.hash !== want) history.replaceState(null, '', want);
  last = r;
  app.route = r;

  const classic = r.place === 'world' && r.rest[0] === 'classic';
  const section = classic ? 'map' : SECTION[r.place];
  for (const el of document.querySelectorAll<HTMLElement>('.dd-main > .place')) el.hidden = el.id !== section;
  switch (r.place) {
    case 'world':
      if (classic) {
        if (!theMap) {
          theMap = mapPage();
          $('map').append(theMap.el);
        }
        // coming back from elsewhere, the rooms may have been edited: drawn afresh
        void theMap.show(was?.place !== 'world' || was.rest[0] !== 'classic');
        break;
      }
      if (!world) {
        // the map's code loads the first time it is opened
        const { default: World } = await import('./world/World.svelte');
        world ??= mount(World, { target: $('world') });
      }
      // coming back from elsewhere, the rooms may have been edited: read afresh
      void world.show(r.rest, was?.place !== 'world' || was.rest[0] === 'classic');
      break;
    case 'workshop':
      if (!workshop) {
        // the Workshop's code loads the first time it is opened
        const { default: Workshop } = await import('./workshop/Workshop.svelte');
        workshop ??= mount(Workshop, { target: $('workshop') });
      }
      workshop.show(r.rest);
      break;
    case 'worlds':
      // what is found, open and running changes: asked afresh every time
      if (worlds) void worlds.load();
      else worlds = mount(Worlds, { target: $('games') });
      break;
    case 'story':
      if (!story) {
        // Story's code loads the first time it is opened
        const { default: Story } = await import('./story/Story.svelte');
        story ??= mount(Story, { target: $('story') });
      }
      story.show(r.rest);
      break;
    case 'publish':
      once('publish', () => mount(Publish, { target: $('publish') }));
      void refreshStatus();
      break;
  }
  // a new place starts at its top
  if (was?.place !== r.place) $(section).scrollTop = 0;
}

// ---- the keyboard ----------------------------------------------------------------------

const typing = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));

let chord = 0;

function keys(e: KeyboardEvent) {
  const mod = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  if (mod && key === 'k') {
    e.preventDefault();
    app.paletteOpen = !app.paletteOpen;
    return;
  }
  // a dialog has the keys while it is open
  if (app.paletteOpen || document.querySelector('dialog[open], [role=dialog]')) return;
  if (typing(e.target) || e.altKey) return;
  if (mod && key === 'z') {
    e.preventDefault();
    void (e.shiftKey ? redo() : undo());
    return;
  }
  if (mod && key === 'y') {
    e.preventDefault();
    void redo();
    return;
  }
  if (mod) return;
  if (e.key === 'Escape' && app.inspecting.length) {
    // the page's own Escape waits for the next press
    e.preventDefault();
    closeInspector();
    return;
  }
  if (e.key === '/') {
    e.preventDefault();
    app.paletteOpen = true;
    return;
  }
  if (chord && Date.now() - chord < 1500) {
    chord = 0;
    const href = JUMPS[key];
    if (href) {
      e.preventDefault();
      location.hash = href;
    }
    return;
  }
  if (key === 'g' && !e.shiftKey) {
    chord = Date.now();
    return;
  }
  if ((e.key === '[' || e.key === ']') && (app.route.place === 'library' || app.route.place === 'players')) {
    e.preventDefault();
    (app.route.place === 'library' ? library : editor)?.step(e.key === '[' ? -1 : 1);
  }
}

// ---- start -----------------------------------------------------------------------------

async function start() {
  mount(TopBar, { target: $('dd-top') });
  mount(Rail, { target: $('dd-rail') });
  mount(Inspector, { target: $('dd-inspector') });
  mount(Home, { target: $('home') });
  const overlays = document.body.appendChild(document.createElement('div'));
  mount(Palette, { target: overlays });
  mount(ToastStack, { target: overlays });
  mount(PublishSheet, { target: overlays });
  document.addEventListener('keydown', keys);

  try {
    const listing = await games.list();
    app.current = listing.current;
    if (listing.current) {
      app.tables = await api.tables();
      editor.start(app.tables);
      loadRecent();
      void refreshNow();
      void refreshStatus();
      void resumePublish();
    }
  } catch (e) {
    toast(messageOf(e), { kind: 'error' });
  }
  window.addEventListener('hashchange', () => void route().catch((e) => toast(messageOf(e), { kind: 'error' })));
  await route();

  // the changes and whether the game runs, kept current while the page is looked at
  const poll = () => {
    if (document.visibilityState !== 'visible') return;
    void refreshNow();
  };
  setInterval(poll, 20_000);
  setInterval(() => document.visibilityState === 'visible' && void refreshStatus(), 30_000);
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      void refreshNow();
      void refreshStatus();
    }
  });
}

void start();
