<!--
  Story: the conversations and commands quests are made of. Pick a monster that talks or a
  room players type in (its exits' words, its commands), and its story opens as a map
  (StoryGraph); a text block's opens from its card. The tables of text, and NMR's Quest
  Organizer as it was, are below.
-->
<svelte:options css="injected" />

<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Search from '@lucide/svelte/icons/search';
  import { api, type Cell } from '../api';
  import { REMOTE, TEXT } from '../library/exits';
  import { questOrganizer } from '../quests';
  import { kindOf } from '../shell/kinds';
  import Mascot from '../shell/Mascot.svelte';
  import { keyLabel, recordHref, storyHref, storyStart } from '../shell/routes';
  import { app } from '../shell/state.svelte';
  import StoryGraph from './StoryGraph.svelte';

  let rest = $state<string[]>([]);

  /** Shows what follows `#/story`: a story's start, or nothing for the list of them. */
  export function show(next: string[]): void {
    rest = next;
  }

  const start = $derived(storyStart(rest, app.tables));

  // ---- the stories there are -------------------------------------------------------------

  interface Entry {
    table: string;
    key: Cell[];
    name: string;
    /** What it starts with: its talk block, its exits' words, its commands. */
    sub: string;
  }

  let talkers = $state<Entry[] | null>(null);
  let rooms = $state<Entry[] | null>(null);
  let stale = true;
  const forget = () => (stale = true);
  $effect(() => {
    window.addEventListener('dd:wrote', forget);
    window.addEventListener('dd:realm', forget);
    return () => {
      window.removeEventListener('dd:wrote', forget);
      window.removeEventListener('dd:realm', forget);
    };
  });

  const has = (t: string) => app.tables.some((x) => x.name === t);
  const num = (v: Cell | undefined) => Number(v) || 0;
  const DIRS10 = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

  // read when the list is shown, and again when the world has changed since
  $effect(() => {
    if (start || !stale) return;
    stale = false;
    if (has('Monsters'))
      api
        .rowsWith('Monsters', ['Greet Txt', 'Talk Txt'], ['Greet Txt', 'Talk Txt'])
        .then((r) => {
          talkers = r.rows.map(([key, name, [greet, talk]]) => ({
            table: 'Monsters',
            key,
            name: String(name ?? ''),
            sub: [num(greet) && `talk block ${greet}`, num(talk) && `chatter ${talk}`].filter(Boolean).join(', '),
          }));
        })
        .catch(() => (talkers = []));
    if (has('Rooms')) {
      // the rooms with commands, or with an exit whose words take players through it or work
      // another (one that leads nowhere answers none)
      const types = DIRS10.map((d) => `Type ${d}`);
      const exits = DIRS10.map((d) => `Exit ${d}`);
      api
        .rowsWith('Rooms', ['Command Text', ...types, ...exits], ['Command Text'], { columns: types, values: [TEXT, REMOTE] })
        .then((r) => {
          rooms = r.rows.flatMap(([key, name, [cmd, ...more]]) => {
            const words = DIRS10.filter((d) => (num(more[d]) === TEXT || num(more[d]) === REMOTE) && num(more[10 + d])).length;
            const sub = [words && (words === 1 ? 'words of an exit' : `words of ${words} exits`), num(cmd) && `commands ${cmd}`].filter(Boolean).join(', ');
            return sub ? [{ table: 'Rooms', key, name: String(name ?? ''), sub }] : [];
          });
        })
        .catch(() => (rooms = []));
    }
  });

  let query = $state('');
  const SHOWN = 40;

  /** The entries whose name has every word of the query, or whose number starts with it. */
  function matching(list: Entry[] | null): Entry[] {
    const q = query.trim().toLowerCase();
    if (!list || !q) return list ?? [];
    const words = q.split(/\s+/);
    return list.filter((e) => {
      const name = e.name.toLowerCase();
      return words.every((w) => name.includes(w)) || keyLabel(e.key).startsWith(q) || String(e.key.at(-1)).startsWith(q);
    });
  }

  const LISTS = $derived([
    { title: 'Monsters that talk', table: 'Monsters', hint: 'What they say when greeted, the topics players can ask about, and what the answers run.', all: talkers, found: matching(talkers) },
    { title: 'Rooms players type in', table: 'Rooms', hint: 'What players can type there, the words of its exits and its commands, and what each does.', all: rooms, found: matching(rooms) },
  ]);

  const TABLES = [
    ['Textblocks', 'The words of quests, signs and conversations, and the scripts behind them'],
    ['Messages', 'What players read when things happen: attacks, spells, rooms'],
    ['Actions', 'The emotes players type: smile, wave, bow'],
  ] as const;

  // NMR's Quest Organizer, made the first time it is opened
  let classic = $state(false);
  function organizer(node: HTMLElement) {
    node.replaceChildren(questOrganizer());
  }
</script>

{#if start}
  {#key `${start.table}/${keyLabel(start.key)}`}
    <StoryGraph {start} />
  {/key}
{:else}
  <div class="story">
    <header class="top">
      <Mascot mood="dream" size={64} />
      <div class="say">
        <h1>Story</h1>
        <p class="muted">The conversations and commands quests are made of. Pick one to see it as a map.</p>
      </div>
      <label class="find">
        <Search size={16} />
        <input type="search" placeholder="Find a monster or a room" aria-label="Find a monster or a room" bind:value={query} />
      </label>
    </header>

    <div class="lists">
      {#each LISTS as l (l.table)}
        {#if has(l.table)}
          {@const k = kindOf(l.table)}
          <section class="card list {k.cls}" aria-label={l.title}>
            <header>
              <span class="tile"><k.icon size={16} /></span>
              <h2>{l.title}</h2>
              {#if l.all}<span class="count">{l.found.length === l.all.length ? l.all.length.toLocaleString() : `${l.found.length.toLocaleString()} of ${l.all.length.toLocaleString()}`}</span>{/if}
            </header>
            <p class="hint">{l.hint}</p>
            {#if !l.all}
              <p class="faint">Reading them…</p>
            {:else if !l.found.length}
              <p class="faint">{l.all.length ? 'None match.' : 'None in this world.'}</p>
            {:else}
              <ul>
                {#each l.found.slice(0, SHOWN) as e (keyLabel(e.key))}
                  <li>
                    <a href={storyHref(e.table, e.key)}>
                      <b>{e.name || `${k.one} ${keyLabel(e.key)}`}</b>
                      <span class="num">{keyLabel(e.key)}</span>
                      <span class="sub">{e.sub}</span>
                    </a>
                  </li>
                {/each}
              </ul>
              {#if l.found.length > SHOWN}<p class="faint more">And {(l.found.length - SHOWN).toLocaleString()} more: find one by its name or number.</p>{/if}
            {/if}
          </section>
        {/if}
      {/each}
    </div>

    <div class="ways">
      {#each TABLES as [t, sub] (t)}
        {#if has(t)}
          {@const k = kindOf(t)}
          <a class="card way {k.cls}" href={recordHref(t)}>
            <span class="tile"><k.icon size={18} /></span>
            <span><b>{k.many}</b><span class="sub">{sub}</span></span>
          </a>
        {/if}
      {/each}
    </div>

    <section class="card classic">
      <button type="button" class="fold" aria-expanded={classic} onclick={() => (classic = !classic)}>
        <h2>Quest Organizer</h2>
        <span class="faint">NMR's tree of quests, from a quest file</span>
        <ChevronDown size={16} class={classic ? 'turned' : ''} />
      </button>
      {#if classic}<div use:organizer></div>{/if}
    </section>
  </div>
{/if}

<style>
  .story { display: flex; flex-direction: column; gap: 16px; max-width: 1100px; margin: 0 auto; }
  .top { display: flex; align-items: center; gap: 16px; flex-wrap: wrap; }
  .say { flex: 1 1 280px; }
  h1 { margin: 0; font-size: 30px; }
  .say p { margin: 2px 0 0; font-weight: 700; }
  .find { display: flex; align-items: center; gap: 8px; flex: 0 1 320px; min-width: 220px; padding: 0 12px; border: var(--stroke) solid var(--line-2); border-radius: 999px; background: var(--surface); color: var(--ink-3); }
  .find:focus-within { border-color: var(--outline); }
  .find input { border: 0; background: none; box-shadow: none; padding: 9px 0; min-width: 0; flex: 1; font: inherit; color: var(--ink); outline: none; }
  .lists { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 12px; align-items: start; }
  .list { display: grid; gap: 8px; }
  .list header { display: flex; align-items: center; gap: 10px; }
  .list .tile { width: 30px; height: 30px; border-radius: 10px; }
  h2 { margin: 0; font-size: 17px; }
  .count { margin-left: auto; font: 700 12px var(--font-mono); color: var(--ink-3); }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; max-height: 460px; overflow: auto; }
  li a { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 0 10px; padding: 6px 8px; border-radius: 10px; text-decoration: none; color: var(--ink); }
  li a:hover, li a:focus-visible { background: var(--surface-2); }
  li b { font-weight: 800; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .num { font: 600 12px var(--font-mono); color: var(--ink-3); align-self: center; }
  li .sub { grid-column: 1 / -1; font-size: 12px; font-weight: 700; color: var(--ink-3); }
  .more { margin: 0; font-size: 12.5px; }
  .ways { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 12px; }
  .way { display: flex; align-items: center; gap: 12px; text-decoration: none; color: var(--ink); transition: transform .15s var(--pop); }
  .way:hover { transform: translateY(-2px); border-color: var(--outline); }
  .way .tile { width: 38px; height: 38px; border-radius: 12px; }
  .way b { display: block; font: 600 17px var(--font-display); }
  .way .sub { display: block; font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  .classic { display: grid; gap: 10px; }
  .fold { display: flex; align-items: baseline; justify-content: flex-start; gap: 10px; width: 100%; border: 0; background: none; padding: 0; min-height: 0; box-shadow: none; text-align: left; flex-wrap: wrap; }
  .fold:hover:not(:disabled) { background: none; }
  .fold .faint { font-size: 12.5px; font-weight: 700; }
  .fold :global(svg) { align-self: center; }
  .fold :global(.turned) { transform: rotate(180deg); }
</style>
