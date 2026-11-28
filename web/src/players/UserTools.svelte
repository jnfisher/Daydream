<!--
  NMR's user tools, on a user's card: Paste Character (a capture of the game's stat output),
  Paste Items/Keys/Worn/$$ (its inventory), Paste Spellbook, the character Export and Import,
  Revive Character and Copy User. The pastes and the import fill the card's fields, to be
  saved as any edit (one Save, one undo); Revive and Copy write at once, each one undo.
-->
<svelte:options css="injected" />

<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import { Dialog } from 'bits-ui';
  import { titleOf } from '../ansi';
  import { api, type TableInfo } from '../api';
  import type { Draft } from '../library/draft.svelte';
  import { sfx } from '../shell/prefs.svelte';
  import { recordHref } from '../shell/routes';
  import { realmChanged, rowsOf, undo } from '../shell/state.svelte';
  import { messageOf, toast } from '../shell/toasts.svelte';
  import {
    chooseItem,
    exportUser,
    IMPORT_DEFAULTS,
    importUser,
    parseInventory,
    parseSpellbook,
    parseStats,
    placeInventory,
    placeSpells,
    type Ambiguous,
    type ImportOptions,
    type Named,
    type UserTool as Tool,
  } from './users';

  let { tool = $bindable(), draft, table }: { tool: Tool | null; draft: Draft; table: TableInfo } = $props();

  const TITLE: Record<Tool, string> = {
    stats: 'Paste stats',
    items: 'Paste inventory',
    spells: 'Paste spellbook',
    import: 'Import a character',
    export: 'Export the character',
    revive: 'Revive',
    copy: 'Copy to another BBS name',
  };
  const SAYS: Record<Tool, string> = {
    stats: "Paste a capture of the character's stat output (or MMUD Explorer's Copy Only Stats). Class, race, level, exp, lives, CP, HP, mana and the six stats are read into the card.",
    items: "Paste a capture of the game's inventory (i). Items, keys, worn items and coins are read into the card. The worn list is replaced either way when the capture names places.",
    spells: "Paste a capture of the game's spellbook (spells). The spells are read into the card's spellbook.",
    import: "Paste a character exported from NMR's or Daydream's user editor. What you tick below is read into the card.",
    export: "NMR's character export of the saved record, to import into another user, here or in NMR.",
    revive: 'HP and mana back to full, and, as you choose, their lives, the spells on them and where they stand. It is written at once; Undo takes it back.',
    copy: 'This user, as saved, onto another BBS name: a new one, or one already here (which keeps its own first name). Limited items, gangs and the like follow unpredictably, so it is really for moving a character between BBS accounts.',
  };

  const bbs = $derived(String(draft.row?.key[0] ?? ''));
  const v = (c: string) => Number(draft.get(c)) || 0;

  let text = $state('');
  let busy = $state(false);
  let error = $state('');
  /** Names that matched more than one record: which each is, chosen in a second step. */
  let choosing = $state<{ list: Ambiguous[]; records: Named[]; what: string; picked: number[]; done: (picked: Map<number, Named>) => void } | null>(null);
  let opts = $state<ImportOptions>({ ...IMPORT_DEFAULTS });
  // Revive's choices
  let lives = $state(true);
  let spells = $state(true);
  let move = $state<'stay' | 'back' | 'halls' | 'room'>('stay');
  let back = $state(1);
  let toMap = $state(1);
  let toRoom = $state(2189);
  // Copy's
  let toBbs = $state('');
  let firstName = $state('');
  let exists = $state<boolean | null>(null);

  const open = $derived(tool !== null);
  const castOn = $derived(v('Poison') !== 0 || Array.from({ length: 10 }, (_, i) => v(`Spell Cast ${i}`)).some((x) => x > 0));
  const inHalls = $derived(v('Map Number') === 1 && v('Room Number') === 2189);
  /** The saved record as NMR exports it. */
  const exported = $derived(tool === 'export' && draft.row ? exportUser(draft.row.values) : '');

  // each time a tool opens: its box starts afresh (a paste's from the clipboard, where it may)
  $effect(() => {
    const t = tool;
    if (!t) return;
    error = '';
    choosing = null;
    busy = false;
    text = '';
    opts = { ...IMPORT_DEFAULTS };
    lives = v('Lives') < 9;
    spells = castOn;
    move = 'stay';
    toBbs = '';
    firstName = '';
    exists = null;
    if (t === 'items' || t === 'spells' || t === 'import') {
      navigator.clipboard
        ?.readText()
        .then((s) => {
          if (tool === t && !text) text = s;
        })
        .catch(() => {
          // pasted by hand
        });
    }
  });

  // Copy: whether the BBS name is a user already
  $effect(() => {
    const name = toBbs.trim();
    exists = null;
    if (tool !== 'copy' || !name) return;
    let gone = false;
    const t = setTimeout(() => {
      api
        .rowIfAny('Users', [name])
        .then((r) => {
          if (!gone) exists = r.values !== null;
        })
        .catch(() => {});
    }, 250);
    return () => {
      gone = true;
      clearTimeout(t);
    };
  });

  const close = () => (tool = null);

  /** The card's values as typed, unsaved edits included. */
  const current = () => Object.fromEntries(table.columns.map((c) => [c.name, draft.typed(c.name)]));

  /** Puts values into the card's fields, unsaved, and says so. */
  function fill(values: Record<string, string>, what: string) {
    const now = current();
    const set = Object.entries(values).filter(([c, val]) => draft.has(c) && now[c] !== val);
    for (const [c, val] of set) draft.set(c, val);
    if (set.length) {
      sfx('pop');
      toast(`${what}: ${set.length} field${set.length === 1 ? '' : 's'} filled in. Save to keep them.`);
      close();
    } else error = 'Nothing in the text was recognised, or it is all as the card has it.';
  }

  /** Items or spells as the pastes match them, in file order. */
  async function named(t: 'Items' | 'Spells'): Promise<Named[]> {
    const cols = t === 'Items' ? ['Number', 'Name', 'Type', 'Uses'] : ['Number', 'Name'];
    const { rows } = await api.rowsWith(t, cols);
    return rows
      .map(([, , [n, name, type, uses]]) => ({ number: Number(n), name: String(name ?? ''), type: Number(type ?? 0), uses: Number(uses ?? 0) }))
      .sort((a, b) => a.number - b.number);
  }

  /** NMR's question when one name matched several records: which one. */
  function choose(list: Ambiguous[], records: Named[], what: string): Promise<Map<number, Named>> {
    if (!list.length) return Promise.resolve(new Map());
    return new Promise((done) => (choosing = { list, records, what, picked: list.map((a) => a.numbers[0]!), done }));
  }
  function chosen() {
    const c = choosing;
    if (!c) return;
    const by = new Map(c.records.map((r) => [r.number, r]));
    const out = new Map<number, Named>();
    c.list.forEach((a, i) => out.set(a.numbers[0]!, by.get(c.picked[i]!)!));
    choosing = null;
    c.done(out);
  }

  async function run(how?: 'clear' | 'add') {
    error = '';
    busy = true;
    try {
      if (tool === 'stats') {
        if (text.trim().length < 10) return void (error = 'Paste the capture first.');
        const { values, race, cls } = parseStats(text);
        for (const [t, name, col] of [
          ['Races', race, 'Race'],
          ['Classes', cls, 'Class'],
        ] as const) {
          if (!name) continue;
          for (const [k, title] of await rowsOf(t)) if (title !== null && titleOf(String(title)) === name) values[col] = String(k[0]);
        }
        fill(values, 'Paste stats');
      } else if (tool === 'items') {
        if (text.length < 10) return void (error = 'Paste the capture first.');
        const items = await named('Items');
        const { values, ambiguous } = placeInventory(parseInventory(text), items, current(), how === 'clear');
        const picked = await choose(ambiguous, items, 'item');
        for (const [from, to] of picked) if (from !== to.number) chooseItem(values, from, to);
        fill(values, 'Paste inventory');
      } else if (tool === 'spells') {
        if (text.length < 10) return void (error = 'Paste the capture first.');
        const list = await named('Spells');
        const { values, ambiguous } = placeSpells(parseSpellbook(text), list, current(), how === 'clear');
        const picked = await choose(ambiguous, list, 'spell');
        for (const [from, to] of picked) {
          if (from === to.number) continue;
          for (const [c, val] of Object.entries(values)) if (val === String(from)) values[c] = String(to.number);
        }
        fill(values, 'Paste spellbook');
      } else if (tool === 'import') {
        if (!text.trim()) return void (error = 'Paste the exported character first.');
        fill(importUser(text, opts), 'Import');
      } else if (tool === 'revive') await revive();
      else if (tool === 'copy') await copy();
    } catch (e) {
      sfx('nope');
      error = messageOf(e);
    } finally {
      busy = false;
    }
  }

  async function revive() {
    if (draft.dirty) return void (error = 'Save or discard the card’s changes first.');
    const to = move === 'back' ? { back: Math.min(19, Math.max(1, back)) } : move === 'halls' ? 'halls' : move === 'room' ? { room: { map: toMap, room: toRoom } } : null;
    await api.preset({ preset: 'revive_character', user: bbs, lives, spells, to });
    sfx('pop');
    toast(`Revived ${bbs}`, { action: 'Undo', onAction: () => void undo() });
    close();
    realmChanged();
  }

  async function copy() {
    const name = toBbs.trim();
    if (!name) return void (error = 'Type the BBS name to copy to.');
    if (name === bbs) return void (error = 'That is this user.');
    if (draft.dirty) return void (error = 'Save or discard the card’s changes first.');
    const values = { ...draft.row!.values };
    const there = await api.rowIfAny('Users', [name]);
    if (there.values) {
      delete values['BBS Name'];
      values['First Name'] = String(there.values['First Name'] ?? '').replace(/ /g, '');
      await api.save('Users', [name], values);
    } else {
      const first = firstName.replace(/ /g, '');
      if (!first) return void (error = 'A new user needs a first name of its own (each must be unique).');
      values['BBS Name'] = name;
      values['First Name'] = first;
      await api.insert('Users', values);
    }
    sfx('pop');
    toast(`Copied ${bbs} to ${name}`, { action: 'Undo', onAction: () => void undo() });
    close();
    location.hash = recordHref('Users', [name]);
  }

  async function copyOut() {
    try {
      await navigator.clipboard.writeText(exported);
      toast('Copied the character to the clipboard.');
    } catch {
      error = 'This browser would not copy it: select the text and copy it by hand.';
    }
  }

  const IMPORTS: [keyof ImportOptions, string][] = [
    ['name', 'Character name'],
    ['gang', 'Gang name'],
    ['deets', 'Lives, suicide password, channel, cash'],
    ['rooms', 'Rooms'],
    ['clearItems', 'Clear items first'],
    ['clearKeys', 'Clear keys first'],
    ['clearSpells', 'Clear spells first'],
    ['clearAbils', 'Clear abilities first'],
  ];
</script>

<Dialog.Root
  {open}
  onOpenChange={(o) => {
    if (!o) close();
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="scrim" />
    <Dialog.Content class="ut-sheet">
      {#if tool}
        <div class="ut-h">
          <Dialog.Title class="ut-title">{TITLE[tool]}</Dialog.Title>
          <Dialog.Close class="btn icon sm ghost" aria-label="Close"><X size={16} /></Dialog.Close>
        </div>
        <Dialog.Description class="ut-says">{SAYS[tool]}</Dialog.Description>
        {#if choosing}
          <p class="ask">More than one {choosing.what} has these names. Which is it?</p>
          <div class="choose">
            {#each choosing.list as a, i (i)}
              <label>
                <span>{a.name}</span>
                <select bind:value={choosing.picked[i]}>
                  {#each a.numbers as n (n)}<option value={n}>{n} {choosing.records.find((r) => r.number === n)?.name ?? ''}</option>{/each}
                </select>
              </label>
            {/each}
          </div>
          <div class="go">
            <button type="button" class="btn sm ghost" onclick={() => (choosing = null)}>Cancel</button>
            <button type="button" class="btn sm primary" onclick={chosen}>Use these</button>
          </div>
        {:else if tool === 'export'}
          <textarea readonly rows="12" aria-label="The exported character" value={exported}></textarea>
          <div class="go">
            <button type="button" class="btn sm primary" onclick={() => void copyOut()}>Copy</button>
          </div>
        {:else if tool === 'revive'}
          <div class="opts">
            <label class="tick"><input type="checkbox" bind:checked={lives} /> Lives back to 9 <span class="faint">(they have {v('Lives')})</span></label>
            <label class="tick"><input type="checkbox" bind:checked={spells} disabled={!castOn} /> Clear the spells on them{castOn ? '' : ' (none are)'}</label>
            <fieldset>
              <legend>Where they stand{inHalls ? ' (the Halls of the Dead now)' : ''}</legend>
              <label class="tick"><input type="radio" bind:group={move} value="stay" /> Leave them where they are</label>
              <label class="tick"><input type="radio" bind:group={move} value="back" /> Back along their trail <input type="number" min="1" max="19" bind:value={back} aria-label="Rooms back" class="n" /> rooms</label>
              <label class="tick"><input type="radio" bind:group={move} value="halls" /> To the Halls of the Dead (1/2189)</label>
              <label class="tick"
                ><input type="radio" bind:group={move} value="room" /> To map <input type="number" min="1" bind:value={toMap} aria-label="Map" class="n" /> room
                <input type="number" min="1" bind:value={toRoom} aria-label="Room" class="n" /></label
              >
            </fieldset>
          </div>
          <div class="go">
            <button type="button" class="btn sm ghost" onclick={close}>Cancel</button>
            <button type="button" class="btn sm primary" disabled={busy} onclick={() => void run()}>{busy ? 'Reviving…' : 'Revive'}</button>
          </div>
        {:else if tool === 'copy'}
          <div class="opts">
            <label class="line">BBS name <input type="text" bind:value={toBbs} maxlength="30" spellcheck="false" placeholder="exactly, case and all" /></label>
            {#if exists === false}
              <label class="line">First name <input type="text" bind:value={firstName} maxlength="11" spellcheck="false" placeholder="unique in the world" /></label>
              <p class="faint small">{toBbs.trim()} is not a user here yet: a new one is made.</p>
            {:else if exists}
              <p class="warn small">{toBbs.trim()} is a user already: everything but their first name is replaced.</p>
            {/if}
          </div>
          <div class="go">
            <button type="button" class="btn sm ghost" onclick={close}>Cancel</button>
            <button type="button" class="btn sm primary" disabled={busy || !toBbs.trim()} onclick={() => void run()}>{busy ? 'Copying…' : 'Copy'}</button>
          </div>
        {:else}
          <textarea rows="12" spellcheck="false" aria-label="The capture" placeholder="Paste here" bind:value={text}></textarea>
          {#if tool === 'import'}
            <div class="ticks">
              {#each IMPORTS as [k, label] (k)}<label class="tick"><input type="checkbox" bind:checked={opts[k]} /> {label}</label>{/each}
            </div>
          {/if}
          <div class="go">
            <button type="button" class="btn sm ghost" onclick={close}>Cancel</button>
            {#if tool === 'items' || tool === 'spells'}
              <button type="button" class="btn sm" disabled={busy} onclick={() => void run('add')} title="Put what is found after what is there">Add to theirs</button>
              <button type="button" class="btn sm primary" disabled={busy} onclick={() => void run('clear')} title="Empty the {tool === 'items' ? 'items, keys and worn lists' : 'spellbook'} first">Replace theirs</button>
            {:else}
              <button type="button" class="btn sm primary" disabled={busy} onclick={() => void run()}>{tool === 'import' ? 'Import' : 'Read it in'}</button>
            {/if}
          </div>
        {/if}
        {#if error}<p class="bad small" role="alert">{error}</p>{/if}
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.ut-sheet) {
    position: fixed; z-index: 201; left: 50%; top: 9vh; transform: translateX(-50%);
    width: min(640px, calc(100vw - 24px)); max-height: 82vh; overflow: auto; display: flex; flex-direction: column; gap: 10px;
    background: var(--surface); color: var(--ink); border: var(--stroke) solid var(--outline); border-radius: 20px;
    box-shadow: 0 5px 0 var(--shadow), var(--float); padding: 14px 16px; animation: fade-in .15s;
  }
  .ut-h { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  :global(.ut-title) { margin: 0; font-size: 18px; }
  :global(.ut-says) { margin: 0; color: var(--ink-2); font-size: 13px; font-weight: 600; }
  textarea { width: 100%; font: 13px/1.35 var(--font-mono); resize: vertical; }
  .go { display: flex; justify-content: flex-end; gap: 8px; flex-wrap: wrap; }
  .ticks { display: grid; grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr)); gap: 4px 12px; }
  .opts { display: grid; gap: 8px; }
  .tick { display: flex; align-items: center; gap: 6px; font-size: 13.5px; font-weight: 700; flex-wrap: wrap; }
  .line { display: grid; grid-template-columns: 7rem minmax(0, 1fr); align-items: center; gap: 8px; font-size: 13.5px; font-weight: 800; color: var(--ink-2); }
  fieldset { border: 1px solid var(--line); border-radius: 12px; padding: 8px 12px; margin: 0; display: grid; gap: 6px; }
  legend { font-size: 12.5px; font-weight: 800; color: var(--ink-3); padding: 0 4px; }
  input.n { width: 5rem; }
  .ask { margin: 0; font-weight: 800; }
  .choose { display: grid; gap: 6px; }
  .choose label { display: grid; grid-template-columns: minmax(0, 12rem) minmax(0, 1fr); gap: 8px; align-items: center; font-weight: 700; font-size: 13.5px; }
  .small { font-size: 12.5px; margin: 0; }
  .bad { color: var(--bad); font-weight: 700; }
  .warn { color: var(--warn); font-weight: 700; }
  @media (max-width: 760px) {
    :global(.ut-sheet) { top: 8px; max-height: calc(100dvh - 16px); padding: 12px; }
    .line, .choose label { grid-template-columns: minmax(0, 1fr); gap: 4px; }
  }
</style>
