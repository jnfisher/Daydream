<!--
  Ctrl+K: one box for everything. Records by name or number (`item sword`, `r 1/224`,
  `868`), the commands (Publish, Undo, a theme), and the places and tables to go to.
  Enter opens a record beside the page; Shift+Enter opens its full page.
-->
<script lang="ts">
  import { Command, Dialog } from 'bits-ui';
  import type { Component } from 'svelte';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import CloudUpload from '@lucide/svelte/icons/cloud-upload';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import PaletteIcon from '@lucide/svelte/icons/palette';
  import Redo2 from '@lucide/svelte/icons/redo-2';
  import Search from '@lucide/svelte/icons/search';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import Volume2 from '@lucide/svelte/icons/volume-2';
  import type { Cell } from '../api';
  import { kindOf, TABLE_ORDER } from './kinds';
  import Mascot from './Mascot.svelte';
  import { PLACES } from './places';
  import { THEMES, prefs, setSound, setTheme } from './prefs.svelte';
  import { keyLabel, PLAYER_TABLES, recordHref } from './routes';
  import { app, inspect, redo, rowsOf, undo, type RecordRef } from './state.svelte';

  const mod = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent) ? '⌘' : 'Ctrl';

  interface Hit { table: string; key: Cell[]; name: string; score: number }
  interface Cmd { id: string; label: string; hint?: string; icon: Component<{ size?: number }>; words: string; kbd?: string; run: () => void }

  /** Words that narrow a search to one table: `item sword`, `r 1/224`. */
  const NARROW: Record<string, string> = {
    i: 'Items', item: 'Items', items: 'Items',
    m: 'Monsters', mon: 'Monsters', monster: 'Monsters', monsters: 'Monsters',
    r: 'Rooms', room: 'Rooms', rooms: 'Rooms',
    sp: 'Spells', spell: 'Spells', spells: 'Spells',
    sh: 'Shops', shop: 'Shops', shops: 'Shops',
    tb: 'Textblocks', text: 'Textblocks', textblock: 'Textblocks', textblocks: 'Textblocks',
    msg: 'Messages', message: 'Messages', messages: 'Messages',
    class: 'Classes', classes: 'Classes', race: 'Races', races: 'Races',
    action: 'Actions', actions: 'Actions', user: 'Users', users: 'Users', gang: 'Gangs', gangs: 'Gangs',
  };

  let query = $state('');
  let selected = $state('');
  let hits: Hit[] = $state([]);
  let searching = $state(false);
  let shift = false;
  let generation = 0;

  const words = $derived(query.trim().toLowerCase().split(/\s+/).filter(Boolean));

  async function search(ws: string[]): Promise<Hit[]> {
    let tables = TABLE_ORDER.filter((t) => app.tables.some((x) => x.name === t));
    let rest = ws;
    const only = NARROW[ws[0] ?? ''];
    if (only && ws.length > 1) {
      tables = tables.filter((t) => t === only);
      rest = ws.slice(1);
    }
    const text = rest.join(' ');
    const byKey = /^\d+(\/\d+)*$/.test(text);
    const each = await Promise.all(
      tables.map(async (table) => {
        let rows: [Cell[], Cell][] = [];
        try {
          rows = await rowsOf(table);
        } catch {
          // a table the realm cannot read: left out
        }
        const found: Hit[] = [];
        for (const [key, title] of rows) {
          const name = title === null ? '' : String(title);
          const n = name.toLowerCase();
          let score = 0;
          if (byKey && keyLabel(key) === text) score = 100;
          else if (!n) continue;
          else if (n === text) score = 90;
          else if (n.startsWith(text)) score = 70;
          else if (rest.every((w) => n.includes(w))) score = rest.every((w) => n.startsWith(w) || n.includes(` ${w}`)) ? 50 : 30;
          // shorter names first among equals
          if (score) found.push({ table, key, name, score: score - Math.min(n.length, 50) / 100 });
        }
        return found.sort((a, b) => b.score - a.score).slice(0, only ? 20 : 6);
      }),
    );
    return each.flat().sort((a, b) => b.score - a.score).slice(0, 24);
  }

  $effect(() => {
    const ws = words;
    const mine = ++generation;
    if (!ws.length) {
      hits = [];
      searching = false;
      return;
    }
    searching = true;
    void search(ws).then((found) => {
      if (mine !== generation) return;
      hits = found;
      searching = false;
    });
  });

  const go = (href: string) => () => {
    location.hash = href;
  };

  const commands: Cmd[] = $derived.by(() => {
    const list: Cmd[] = [
      { id: 'publish', label: 'Publish…', hint: 'Review the changes and write them into the game', icon: CloudUpload, words: 'publish write send game', run: go('#/publish') },
    ];
    const u = app.history?.undo;
    if (u) list.push({ id: 'undo', label: `Undo: ${u.label}`, icon: Undo2, words: 'undo take back', kbd: `${mod} Z`, run: () => void undo() });
    const r = app.history?.redo;
    if (r) list.push({ id: 'redo', label: `Redo: ${r.label}`, icon: Redo2, words: 'redo again', kbd: `${mod} ⇧ Z`, run: () => void redo() });
    list.push({ id: 'worlds', label: 'Open another world…', hint: 'Every game Daydream found on this computer', icon: FolderOpen, words: 'open world game switch picker', run: go('#/worlds') });
    for (const t of THEMES) {
      if (t.id !== prefs.theme) list.push({ id: `theme-${t.id}`, label: `Theme: ${t.name}`, hint: t.sub, icon: PaletteIcon, words: 'theme dark light night day colors colours', run: () => setTheme(t.id) });
    }
    list.push({ id: 'sound', label: prefs.sound ? 'Turn the little sounds off' : 'Turn the little sounds on', icon: Volume2, words: 'sounds audio mute quiet', run: () => setSound(!prefs.sound) });
    return list;
  });

  const destinations: Cmd[] = $derived.by(() => {
    const list: Cmd[] = PLACES.map((p) => ({ id: p.id, label: p.label, hint: p.hint, icon: ArrowRight, words: `go ${p.id}`, kbd: `g ${p.key}`, run: go(p.href) }));
    for (const t of TABLE_ORDER) {
      if (!app.tables.some((x) => x.name === t)) continue;
      const k = kindOf(t);
      list.push({ id: `table-${t}`, label: k.many, hint: PLAYER_TABLES.includes(t) ? 'in Players' : 'in the Library', icon: k.icon, words: `go table ${t}`, run: go(recordHref(t)) });
    }
    return list;
  });

  const matches = (c: Cmd) => words.every((w) => `${c.label} ${c.words}`.toLowerCase().includes(w));
  const shownCommands = $derived(words.length ? commands.filter(matches) : commands.slice(0, 4));
  const shownPlaces = $derived(words.length ? destinations.filter(matches).slice(0, 8) : destinations.slice(0, PLACES.length));
  const recent = $derived(words.length ? [] : app.recent.slice(0, 5));
  const nothing = $derived(words.length > 0 && !searching && !hits.length && !shownCommands.length && !shownPlaces.length);

  // the first answer is the one Enter takes, as the answers change
  $effect(() => {
    const first = recent[0] ? `rec:${refId(recent[0])}` : hits[0] ? `rec:${refId(hits[0])}` : shownCommands[0] ? `cmd:${shownCommands[0].id}` : shownPlaces[0] ? `go:${shownPlaces[0].id}` : '';
    selected = first;
  });

  // Shift held as an answer is taken opens the full page; seen before the list handles the key
  $effect(() => {
    const note = (e: KeyboardEvent | PointerEvent) => (shift = e.shiftKey);
    window.addEventListener('keydown', note, true);
    window.addEventListener('keyup', note, true);
    window.addEventListener('pointerdown', note, true);
    return () => {
      window.removeEventListener('keydown', note, true);
      window.removeEventListener('keyup', note, true);
      window.removeEventListener('pointerdown', note, true);
    };
  });

  function refId(r: { table: string; key: Cell[] }) {
    return `${r.table}:${keyLabel(r.key)}`;
  }

  function close() {
    app.paletteOpen = false;
  }

  function openRecord(r: RecordRef) {
    close();
    if (shift) location.hash = recordHref(r.table, r.key);
    else inspect({ table: r.table, key: r.key, name: r.name });
  }

  function run(c: Cmd) {
    close();
    c.run();
  }
</script>

<Dialog.Root bind:open={app.paletteOpen} onOpenChange={(o) => { if (!o) query = ''; }}>
  <Dialog.Portal>
    <Dialog.Overlay class="scrim" />
    <Dialog.Content class="palette" aria-label="Search everything">
      <Dialog.Title class="sr">Search everything</Dialog.Title>
      <Command.Root shouldFilter={false} bind:value={selected} loop label="Search everything">
        <div class="palette-in">
          <Search size={20} />
          <Command.Input bind:value={query} placeholder="Search records, places, commands…" aria-label="Search" />
          <span class="kbd">Esc</span>
        </div>
        <Command.List class="palette-list">
          <Command.Viewport>
            {#if recent.length}
              <Command.Group value="recent">
                <Command.GroupHeading class="grp eyebrow">Recently opened</Command.GroupHeading>
                <Command.GroupItems>
                  {#each recent as r (refId(r))}
                    {@const k = kindOf(r.table)}
                    <Command.Item value={`rec:${refId(r)}`} class="opt {k.cls}" onSelect={() => openRecord(r)}>
                      <span class="tile"><k.icon size={16} /></span>
                      <span class="txt"><span class="nm">{r.name || `${k.one} ${keyLabel(r.key)}`}</span><span class="sub">{k.one} {keyLabel(r.key)}</span></span>
                      <span class="end"><span class="kbd">↵</span></span>
                    </Command.Item>
                  {/each}
                </Command.GroupItems>
              </Command.Group>
            {/if}
            {#if hits.length}
              <Command.Group value="records">
                <Command.GroupHeading class="grp eyebrow">Records</Command.GroupHeading>
                <Command.GroupItems>
                  {#each hits as r (refId(r))}
                    {@const k = kindOf(r.table)}
                    <Command.Item value={`rec:${refId(r)}`} class="opt {k.cls}" onSelect={() => openRecord(r)}>
                      <span class="tile"><k.icon size={16} /></span>
                      <span class="txt"><span class="nm">{r.name || `${k.one} ${keyLabel(r.key)}`}</span><span class="sub">{k.one} {keyLabel(r.key)}</span></span>
                      <span class="end"><span class="kbd">↵ beside</span><span class="kbd">⇧↵ full page</span></span>
                    </Command.Item>
                  {/each}
                </Command.GroupItems>
              </Command.Group>
            {:else if searching}
              <div class="grp eyebrow">Searching the records…</div>
            {/if}
            {#if shownCommands.length}
              <Command.Group value="commands">
                <Command.GroupHeading class="grp eyebrow">Commands</Command.GroupHeading>
                <Command.GroupItems>
                  {#each shownCommands as c (c.id)}
                    <Command.Item value={`cmd:${c.id}`} class="opt" onSelect={() => run(c)}>
                      <span class="tile"><c.icon size={16} /></span>
                      <span class="txt"><span class="nm">{c.label}</span>{#if c.hint}<span class="sub">{c.hint}</span>{/if}</span>
                      {#if c.kbd}<span class="end"><span class="kbd">{c.kbd}</span></span>{/if}
                    </Command.Item>
                  {/each}
                </Command.GroupItems>
              </Command.Group>
            {/if}
            {#if shownPlaces.length}
              <Command.Group value="places">
                <Command.GroupHeading class="grp eyebrow">Go to</Command.GroupHeading>
                <Command.GroupItems>
                  {#each shownPlaces as c (c.id)}
                    {@const k = kindOf(c.id.startsWith('table-') ? c.id.slice(6) : null)}
                    <Command.Item value={`go:${c.id}`} class="opt {c.id.startsWith('table-') ? k.cls : ''}" onSelect={() => run(c)}>
                      <span class="tile"><c.icon size={16} /></span>
                      <span class="txt"><span class="nm">{c.label}</span>{#if c.hint}<span class="sub">{c.hint}</span>{/if}</span>
                      {#if c.kbd}<span class="end"><span class="kbd">{c.kbd}</span></span>{/if}
                    </Command.Item>
                  {/each}
                </Command.GroupItems>
              </Command.Group>
            {/if}
            {#if nothing}
              <div class="empty">
                <Mascot mood="worry" size={64} />
                <span>Nothing matches “{query.trim()}”.</span>
                <span class="faint">Try a number (868, 1/224) or start with a kind: item, monster, room, spell, shop, tb.</span>
              </div>
            {/if}
          </Command.Viewport>
        </Command.List>
      </Command.Root>
      <div class="palette-foot">
        <span><span class="kbd">↑</span> <span class="kbd">↓</span> move</span>
        <span><span class="kbd">↵</span> open beside the page</span>
        <span><span class="kbd">⇧↵</span> open the full page</span>
        <span class="hide-narrow">Start with <b>item</b>, <b>room</b>, <b>m</b>… to search one kind</span>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  .txt { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
</style>
