<!--
  Players, at a glance: who plays this world. How many users, gangs and bank books; who is in
  the game now (when silvermere says); the highest levels; the classes and races played; the
  gangs by size; who carries the most coin and who has no lives left. Each name opens its
  card. Shown where the Users list has no record open.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Coins from '@lucide/svelte/icons/coins';
  import Crown from '@lucide/svelte/icons/crown';
  import HeartCrack from '@lucide/svelte/icons/heart-crack';
  import Radio from '@lucide/svelte/icons/radio';
  import Trophy from '@lucide/svelte/icons/trophy';
  import UsersRound from '@lucide/svelte/icons/users-round';
  import { api, type Player } from '../api';
  import { coinsOf } from '../library/cards';
  import type { ListRow } from '../library/list';
  import { nameOf } from '../library/names.svelte';
  import Mascot from '../shell/Mascot.svelte';
  import { recordHref } from '../shell/routes';
  import Sky from '../shell/Sky.svelte';
  import { app } from '../shell/state.svelte';

  let { rows, reading }: { rows: ListRow[]; reading: boolean } = $props();

  const num = (v: unknown) => Number(v) || 0;
  const fmt = (n: number) => n.toLocaleString();
  const plural = (n: number, one: string, many = `${one}s`) => `${fmt(n)} ${n === 1 ? one : many}`;
  const level = (r: ListRow) => num(r.get('Level'));
  const named = (table: string, n: number) => (n ? nameOf(table, [n]) || `${table === 'Races' ? 'Race' : 'Class'} ${n}` : 'None');
  const what = (r: ListRow) => [named('Races', num(r.get('Race'))), named('Classes', num(r.get('Class')))].join(' ');
  const who = (r: ListRow) => r.name || r.label;

  const top = $derived(rows.slice().sort((a, b) => level(b) - level(a) || who(a).localeCompare(who(b))).slice(0, 8));
  const richest = $derived(
    rows
      .map((r) => ({ r, coins: coinsOf(r.get) }))
      .filter((x) => x.coins > 0)
      .sort((a, b) => b.coins - a.coins)
      .slice(0, 5),
  );
  const dead = $derived(rows.filter((r) => num(r.get('Lives')) === 0 && level(r) > 0));
  const average = $derived(rows.length ? Math.round(rows.reduce((n, r) => n + level(r), 0) / rows.length) : 0);

  /** How many of each value of `col` (a class, a race), most first. */
  function counts(col: string, table: string): { label: string; n: number }[] {
    const by = new Map<number, number>();
    for (const r of rows) by.set(num(r.get(col)), (by.get(num(r.get(col))) ?? 0) + 1);
    return [...by.entries()].sort((a, b) => b[1] - a[1]).map(([v, n]) => ({ label: named(table, v), n }));
  }
  const classes = $derived(counts('Class', 'Classes'));
  const races = $derived(counts('Race', 'Races'));
  const widest = $derived(Math.max(1, classes[0]?.n ?? 1, races[0]?.n ?? 1));

  // the gangs, by size (as the users name them), and the bank books
  let gangs = $state<{ key: string; name: string; leader: string; members: number }[] | null>(null);
  let books = $state<{ n: number; banked: number } | null>(null);
  // who is in the game now, when the game can say (silvermere)
  let online = $state<Player[] | null>(null);
  $effect(() => {
    void rows;
    let gone = false;
    api
      .rowsWith('Gangs', ['Leader'])
      .then((r) => {
        if (gone) return;
        const size = new Map<string, number>();
        for (const u of rows) {
          const g = String(u.get('Gang') ?? '').trim();
          if (g) size.set(g, (size.get(g) ?? 0) + 1);
        }
        gangs = r.rows
          .map(([k, title, [leader]]) => ({ key: String(k[0] ?? ''), name: String(title ?? '') || String(k[0] ?? ''), leader: String(leader ?? '').trim(), members: size.get(String(k[0] ?? '')) ?? 0 }))
          .sort((a, b) => b.members - a.members || a.name.localeCompare(b.name));
      })
      .catch(() => (gangs = []));
    api
      .rowsWith('BankBooks', ['Amount'])
      .then((r) => {
        if (!gone) books = { n: r.rows.length, banked: r.rows.reduce((n, [, , [a]]) => n + num(a), 0) };
      })
      .catch(() => (books = null));
    if (app.current?.kind === 'silvermere')
      api
        .control()
        .then((c) => {
          if (!gone) online = c.running && c.players ? c.players : null;
        })
        .catch(() => {});
    return () => (gone = true);
  });

  /** A player in the game, by their character's name, to the user it is. */
  const byName = $derived(new Map(rows.map((r) => [who(r).toLowerCase(), r])));
</script>

<div class="roster">
  <section class="hero sky">
    <Sky clouds={3} stars={18} seed={11} />
    <div class="in">
      <Mascot mood={rows.length ? 'joy' : 'dream'} size={96} />
      <div class="say">
        <h1>Players</h1>
        {#if reading && !rows.length}
          <p>Reading who plays…</p>
        {:else if !rows.length}
          <p>Nobody plays this world yet. Users appear here once the game has made them, or add one with New.</p>
        {:else}
          <p>
            {plural(rows.length, 'user')}, level {fmt(average)} on average{gangs?.length ? `, in ${plural(gangs.length, 'gang')}` : ''}{books ? `, with ${plural(books.n, 'bank book')} holding ${fmt(books.banked)} copper` : ''}.
          </p>
          <div class="acts">
            <a class="btn" href="#/gangs"><UsersRound size={16} /> Gangs</a>
            <a class="btn" href="#/bankbooks"><Coins size={16} /> Bank books</a>
            <a class="btn" href="#/users?grid">Every user in a grid</a>
          </div>
        {/if}
      </div>
    </div>
  </section>

  {#if rows.length}
    <div class="board">
      {#if online}
        <section class="card wide">
          <h3><Radio size={16} /> In the game now</h3>
          {#if online.length}
            <div class="chips">
              {#each online as p (p.name)}
                {@const u = byName.get(p.name.toLowerCase())}
                {#if u}
                  <a class="who" href={recordHref('Users', u.key)}><b>{p.name}</b><span>level {p.level} {p.race} {p.class}</span></a>
                {:else}
                  <span class="who"><b>{p.name}</b><span>level {p.level} {p.race} {p.class}</span></span>
                {/if}
              {/each}
            </div>
          {:else}
            <p class="quiet">Nobody is on right now.</p>
          {/if}
        </section>
      {/if}

      <section class="card">
        <h3><Trophy size={16} /> Highest levels</h3>
        <ol class="ranks">
          {#each top as r, i (r.label)}
            <li>
              <span class="rank p{i + 1}">{i + 1}</span>
              <a href={recordHref('Users', r.key)}><b>{who(r)}</b><span class="faint">{what(r)}</span></a>
              <span class="lv">{level(r)}</span>
            </li>
          {/each}
        </ol>
      </section>

      <section class="card">
        <h3>Classes played</h3>
        <div class="bars">
          {#each classes as c (c.label)}
            <div class="bar"><span class="nm">{c.label}</span><span class="track"><span class="fill" style:width="{(100 * c.n) / widest}%"></span></span><span class="v">{c.n}</span></div>
          {/each}
        </div>
        <h3 class="next">Races</h3>
        <div class="bars">
          {#each races as c (c.label)}
            <div class="bar"><span class="nm">{c.label}</span><span class="track"><span class="fill race" style:width="{(100 * c.n) / widest}%"></span></span><span class="v">{c.n}</span></div>
          {/each}
        </div>
      </section>

      <section class="card">
        <h3><UsersRound size={16} /> Gangs</h3>
        {#if !gangs}
          <p class="quiet">Reading the gangs…</p>
        {:else if !gangs.length}
          <p class="quiet">No gangs yet.</p>
        {:else}
          <ul class="list">
            {#each gangs.slice(0, 8) as g (g.key)}
              <li>
                <a href={recordHref('Gangs', [g.key])}><b>{g.name}</b><span class="faint"><Crown size={11} /> {g.leader || 'no leader'}</span></a>
                <span class="lv">{plural(g.members, 'member')}</span>
              </li>
            {/each}
          </ul>
          {#if gangs.length > 8}<a class="btn sm ghost" href="#/gangs">All {fmt(gangs.length)} gangs</a>{/if}
        {/if}
      </section>

      <section class="card">
        <h3><Coins size={16} /> Carrying the most coin</h3>
        {#if richest.length}
          <ul class="list">
            {#each richest as x (x.r.label)}
              <li>
                <a href={recordHref('Users', x.r.key)}><b>{who(x.r)}</b><span class="faint">{what(x.r)}</span></a>
                <span class="lv" title="In copper: a gold is 100, a platinum 10,000, a runic 1,000,000">{fmt(x.coins)}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="quiet">Nobody carries any coin.</p>
        {/if}
        {#if dead.length}
          <h3 class="next"><HeartCrack size={16} /> No lives left</h3>
          <div class="chips">
            {#each dead.slice(0, 12) as r (r.label)}<a class="pill" href={recordHref('Users', r.key)}>{who(r)}</a>{/each}
            {#if dead.length > 12}<span class="faint">and {fmt(dead.length - 12)} more</span>{/if}
          </div>
          <p class="quiet small">Revive, in a user's ⋯ menu, gives them back.</p>
        {/if}
      </section>
    </div>
  {/if}
</div>

<style>
  .roster { padding: 18px clamp(12px, 2.5vw, 26px) 60px; display: flex; flex-direction: column; gap: 16px; max-width: 1100px; margin: 0 auto; }
  .hero { border-radius: 26px; border: var(--stroke) solid var(--outline); box-shadow: 0 5px 0 var(--shadow); padding: 22px clamp(16px, 3vw, 30px); }
  .in { position: relative; display: flex; align-items: center; gap: clamp(14px, 3vw, 26px); flex-wrap: wrap; }
  .say { flex: 1 1 320px; min-width: 0; }
  h1 { margin: 0 0 4px; font-size: clamp(26px, 4vw, 34px); line-height: 1.1; }
  .say p { margin: 0 0 12px; font-size: 15px; font-weight: 700; color: var(--ink-2); max-width: 62ch; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; }
  .board { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 16px; }
  .wide { grid-column: 1 / -1; }
  h3 { margin: 0 0 10px; font-size: 16.5px; display: flex; align-items: center; gap: 7px; }
  h3.next { margin-top: 16px; }
  .quiet { margin: 0; color: var(--ink-2); font-weight: 600; }
  .small { font-size: 12.5px; margin-top: 6px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .who { display: inline-flex; flex-direction: column; padding: 6px 12px; border-radius: 14px; border: 1px solid var(--line); background: color-mix(in srgb, var(--mint) 22%, var(--surface)); color: var(--ink); text-decoration: none; }
  .who span { font-size: 12px; font-weight: 700; color: var(--ink-2); }
  a.who:hover { border-color: var(--outline); }
  .ranks, .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .ranks li, .list li { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 10px; padding: 5px 6px; border-radius: 12px; }
  .list li { grid-template-columns: minmax(0, 1fr) auto; }
  .ranks li:hover, .list li:hover { background: var(--surface-2); }
  .ranks a, .list a { display: flex; flex-direction: column; min-width: 0; color: var(--ink); text-decoration: none; }
  .ranks a b, .list a b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ranks a span, .list a span { font-size: 12px; font-weight: 700; display: inline-flex; align-items: center; gap: 3px; }
  .rank { width: 26px; height: 26px; border-radius: 50%; display: grid; place-items: center; font: 600 13px var(--font-display); background: var(--surface-3); color: var(--ink-2); }
  .rank.p1 { background: var(--butter); color: var(--on-pastel); }
  .rank.p2 { background: var(--sky); color: var(--on-pastel); }
  .rank.p3 { background: var(--peach); color: var(--on-pastel); }
  .lv { font: 600 15px var(--font-display); color: var(--ink-2); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .bars { display: grid; gap: 5px; }
  .bar { display: grid; grid-template-columns: minmax(0, 8rem) minmax(0, 1fr) 2.5rem; align-items: center; gap: 8px; font-size: 13px; font-weight: 700; }
  .bar .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track { height: 10px; border-radius: 999px; background: var(--surface-2); overflow: hidden; }
  .fill { display: block; height: 100%; border-radius: 999px; background: var(--lilac); }
  .fill.race { background: var(--mint); }
  .bar .v { text-align: right; color: var(--ink-3); font-variant-numeric: tabular-nums; }
</style>
