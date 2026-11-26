<!--
  Worlds: every game Daydream found on this computer, which one is open and whether each
  runs; opening another, or reading one again into its working copy; and where Daydream
  looked (/api/games). Opening a world starts the page again on it.
-->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import HardDrive from '@lucide/svelte/icons/hard-drive';
  import House from '@lucide/svelte/icons/house';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import Minus from '@lucide/svelte/icons/minus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';
  import Server from '@lucide/svelte/icons/server';
  import Terminal from '@lucide/svelte/icons/terminal';
  import { games, how, KINDS, type Game, type Listing, type Looked, type Status } from '../games';
  import Mascot from './Mascot.svelte';
  import Sky from './Sky.svelte';
  import { ago } from './time';

  let listing = $state<Listing | null>(null);
  let failure = $state('');
  let looking = $state(false);
  /** What is being opened: a game's id, or `folder` / `silvermere` for a typed one. */
  let busy = $state<string | null>(null);
  let errors = $state<Record<string, string>>({});
  /** The game whose working copy is about to be read again, once the person says so. */
  let confirming = $state<string | null>(null);
  let status = $state<Record<string, Status | 'checking'>>({});
  let folder = $state('');
  let database = $state('');

  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

  /** The games and where Daydream looked, afresh; `refresh` searches the disk again. */
  export async function load(refresh = false): Promise<void> {
    looking = refresh;
    try {
      const l = await games.list(refresh);
      listing = l;
      failure = '';
      for (const g of l.games) {
        if (realmFolder(g)) continue;
        status[g.id] = 'checking';
        games
          .status(g.id)
          .then((s) => (status[g.id] = s))
          .catch((e) => (status[g.id] = { running: null, error: message(e), elsewhere: [] }));
      }
    } catch (e) {
      failure = message(e);
    } finally {
      looking = false;
    }
  }
  void load();

  async function open(body: Record<string, unknown>, key: string) {
    busy = key;
    errors[key] = '';
    try {
      await games.open(body);
      // another world: the page starts again on it
      location.hash = '#/home';
      location.reload();
    } catch (e) {
      errors[key] = message(e);
      busy = null;
    }
  }

  /** A realm folder Daydream made, edited where it is: no game runs on it. */
  const realmFolder = (g: Game) => g.kind === 'db' && !g.live && !g.copy;

  /** `emu (MBBSEmu, .db files)`: its name, and what it is. */
  function parts(g: Game): { name: string; what: string } {
    const m = /^(.*?) \((.*)\)$/.exec(g.label);
    if (m) return { name: m[1]!, what: m[2]! };
    return { name: g.label, what: KINDS[g.kind] };
  }

  /** The colour and picture of a game's card, by what it is. */
  function look(g: Game) {
    if (g.kind === 'silvermere') return { tone: 'mint', Icon: Server };
    if (realmFolder(g)) return { tone: 'lilac', Icon: FolderOpen };
    if (g.kind === 'db') return { tone: 'sky', Icon: Terminal };
    return { tone: 'peach', Icon: HardDrive };
  }

  function stateOf(g: Game): { text: string; cls: string; title?: string; pulse?: boolean } {
    if (realmFolder(g)) return { text: 'realm folder', cls: 'lilac' };
    const s = status[g.id];
    if (!s || s === 'checking') return { text: 'checking…', cls: '' };
    if (s.running === null) return { text: 'can’t tell', cls: 'warn', title: s.error };
    if (s.running) return { text: 'running', cls: 'ok', title: s.by ?? undefined, pulse: true };
    if (s.elsewhere.length) {
      return { text: 'stopped here', cls: '', title: `Running elsewhere on this computer: ${s.elsewhere.join('; ')}` };
    }
    return { text: 'stopped', cls: '' };
  }

  const pulled = (g: Game) => {
    const t = g.copy ? Date.parse(g.copy.pulled) : NaN;
    return Number.isNaN(t) ? null : ago(t / 1000);
  };

  // the open one first
  const found = $derived([...(listing?.games ?? [])].sort((a, b) => Number(b.current) - Number(a.current)));
  const lede = $derived.by(() => {
    if (!listing) return 'Looking for the games on this computer…';
    const n = found.length;
    if (!n) return 'Daydream found no game yet. Open a folder below, or start Daydream with --search FOLDER to look somewhere else.';
    return `Daydream found ${n === 1 ? 'one game' : `${n} games`} on this computer. Open one to edit it: every game keeps its own working copy, so switching never loses an edit.`;
  });

  function lookedLine(l: Looked): string {
    const what = l.found.length ? `found ${l.found.join(', ')}` : 'nothing found';
    if (l.kind === 'copies') return `Working copies of games opened before: ${l.found.length ? l.found.join(', ') : 'none yet'}`;
    if (l.kind === 'setting') return `silvermere’s setting: ${what}`;
    const deep = l.depth === 1 ? '1 level down' : `${l.depth} levels down`;
    return `${l.given ? 'Given with --search' : 'A usual place'}, ${deep}: ${what}`;
  }
</script>

<div class="worlds">
  <section class="hero sky">
    <Sky clouds={3} stars={16} seed={4} />
    <div class="in">
      <Mascot mood="happy" size={84} />
      <div class="say">
        <div class="eyebrow">Worlds</div>
        <h1>Your worlds</h1>
        <p>{lede}</p>
      </div>
    </div>
  </section>

  {#if failure}<p class="callout bad">{failure}</p>{/if}

  {#if found.length}
    <section aria-labelledby="found-h">
      <h2 id="found-h">Found here</h2>
      <div class="grid">
        {#each found as g (g.id)}
          {@const p = parts(g)}
          {@const lk = look(g)}
          {@const st = stateOf(g)}
          <article class="wc" class:here={g.current}>
            <div class="art" style:--a="var(--{lk.tone}-2)" style:--b="var(--{lk.tone})">
              <span class="pill {st.cls}" title={st.title}>
                {#if st.text === 'checking…'}<LoaderCircle size={12} class="spin" />{:else if st.text === 'running'}<span class="dot" class:pulse={st.pulse}></span>{/if}
                {st.text}
              </span>
              <span class="eng" style:color="var(--{lk.tone}-ink)"><lk.Icon size={26} /></span>
            </div>
            <div class="bd">
              <div class="top">
                <h3 title={g.label}>{p.name}</h3>
                {#if g.current}<span class="pill lilac">open now</span>{/if}
              </div>
              <div class="meta">
                <span class="what">{p.what}</span>
                <!-- a game's own .db files are copied when it is first opened -->
                <span>{how({ kind: g.kind, live: g.live, copy: !!g.copy || g.live })}</span>
                {#if g.copy}<span>Working copy read {pulled(g) ?? 'before'}.</span>{/if}
                <code title={g.location}>{g.location}</code>
              </div>
              {#if errors[g.id]}<p class="callout bad err">{errors[g.id]}</p>{/if}
              {#if confirming === g.id}
                <div class="confirm" role="group" aria-label="Read the game again">
                  <p>Replace the working copy with {p.name} as it is now? Edits not yet published are set aside beside it, as <code>.previous</code> (replacing an older one).</p>
                  <div class="acts">
                    <button type="button" class="btn sm primary sticker" disabled={!!busy} onclick={() => { confirming = null; void open({ id: g.id, refresh: true }, g.id); }}>{g.current ? 'Read it again' : 'Read it again and open it'}</button>
                    <button type="button" class="btn sm ghost" onclick={() => (confirming = null)}>Keep my copy</button>
                  </div>
                </div>
              {:else}
                <div class="acts">
                  {#if g.current}
                    <a class="btn sm primary sticker" href="#/home"><House size={14} />Go to its Home</a>
                  {:else}
                    <button type="button" class="btn sm primary sticker" disabled={!!busy} onclick={() => open({ id: g.id }, g.id)}>
                      {#if busy === g.id}<LoaderCircle size={14} class="spin" />Opening…{:else}Open{/if}
                    </button>
                  {/if}
                  {#if g.copy}
                    <button type="button" class="btn sm" disabled={!!busy} title="Read the game again into its working copy" onclick={() => (confirming = g.id)}>
                      {#if busy === g.id && g.current}<LoaderCircle size={14} class="spin" />Reading…{:else}<RefreshCw size={14} />Read again{/if}
                    </button>
                  {/if}
                </div>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    </section>
  {/if}

  <div class="cols">
    <section class="card" aria-labelledby="other-h">
      <h2 id="other-h">Open another</h2>
      {#if listing?.custom}
        <p class="quiet">A game folder or a silvermere database Daydream did not find: a folder on the computer Daydream runs on, or a PostgreSQL URL (Neon and other hosts).</p>
        <form class="ask" onsubmit={(e) => { e.preventDefault(); void open({ kind: 'folder', location: folder }, 'folder'); }}>
          <label for="w-folder">Game folder</label>
          <div class="row">
            <input id="w-folder" type="text" bind:value={folder} placeholder="C:\WGSERV, or a folder of .db files" autocomplete="off" spellcheck="false" />
            <button type="submit" class="btn sticker" disabled={!!busy || !folder.trim()}>
              {#if busy === 'folder'}<LoaderCircle size={14} class="spin" />Opening…{:else}<FolderOpen size={15} />Open{/if}
            </button>
          </div>
          {#if errors.folder}<p class="callout bad err">{errors.folder}</p>{/if}
        </form>
        <form class="ask" onsubmit={(e) => { e.preventDefault(); void open({ kind: 'silvermere', location: database }, 'silvermere'); }}>
          <label for="w-db">silvermere’s database</label>
          <div class="row">
            <input id="w-db" type="text" bind:value={database} placeholder="silvermere.db, or postgres://user:password@host/db" autocomplete="off" spellcheck="false" />
            <button type="submit" class="btn sticker" disabled={!!busy || !database.trim()}>
              {#if busy === 'silvermere'}<LoaderCircle size={14} class="spin" />Opening…{:else}<Server size={15} />Open{/if}
            </button>
          </div>
          {#if errors.silvermere}<p class="callout bad err">{errors.silvermere}</p>{/if}
        </form>
        {#if listing.workspace}<p class="help">Working copies are kept in <code>{listing.workspace}</code>.</p>{/if}
      {:else if listing}
        <p class="quiet">This Daydream is reached with an access token, so it opens only the games it found itself. To add one, start it with <code>--search FOLDER</code>, or with <code>--allow-any-source</code> to allow typing any folder or database here.</p>
      {/if}
    </section>

    <section class="card" aria-labelledby="looked-h">
      <div class="card-h">
        <h2 id="looked-h">Where Daydream looked</h2>
        <button type="button" class="btn sm end" disabled={looking} onclick={() => load(true)}>
          {#if looking}<LoaderCircle size={14} class="spin" />Looking…{:else}<Search size={14} />Look again{/if}
        </button>
      </div>
      <ul class="looked">
        {#each listing?.looked ?? [] as l (l.kind + l.where)}
          <li>
            <span class="mark" class:yes={l.found.length > 0}>
              {#if l.found.length}<Check size={15} />{:else}<Minus size={15} />{/if}
            </span>
            <span class="lk">
              <code title={l.where}>{l.where}</code>
              <span class="s">{lookedLine(l)}</span>
            </span>
          </li>
        {/each}
      </ul>
      <p class="help">Daydream looks a few folders down from each, never in system or hidden folders. A game in a container shows here only if its folder is on this computer.</p>
    </section>
  </div>
</div>

<style>
  .worlds { padding: 18px clamp(12px, 2.5vw, 26px) 60px; display: flex; flex-direction: column; gap: 16px; max-width: 1240px; margin: 0 auto; }
  .hero { border-radius: 26px; border: var(--stroke) solid var(--outline); box-shadow: 0 5px 0 var(--shadow); padding: 22px clamp(16px, 3vw, 34px); }
  .in { position: relative; display: flex; align-items: center; gap: clamp(14px, 3vw, 24px); }
  .say { flex: 1 1 auto; min-width: 0; }
  h1 { margin: 6px 0 0; font-size: clamp(24px, 3.4vw, 34px); line-height: 1.08; }
  .say p { margin: 6px 0 0; color: var(--ink-2); font-weight: 700; max-width: 62ch; }
  h2 { margin: 0 0 10px; font-size: 18px; }
  .card-h h2 { margin: 0; }
  .end { margin-left: auto; }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(270px, 1fr)); gap: 14px; }
  .wc { display: flex; flex-direction: column; min-width: 0; background: var(--surface); border: var(--stroke) solid var(--outline); border-radius: 20px; box-shadow: 0 4px 0 var(--shadow); overflow: hidden; }
  .wc.here { box-shadow: 0 4px 0 var(--shadow), 0 0 0 4px var(--lilac-2); }
  .art { position: relative; height: 74px; background: linear-gradient(135deg, var(--a), var(--b)); border-bottom: var(--stroke) solid var(--outline); }
  .art .pill { position: absolute; top: 10px; left: 10px; background: var(--surface); }
  .art .pill.ok { background: var(--ok-bg); }
  .art .pill.warn { background: var(--warn-bg); }
  .art .pill.lilac { background: var(--lilac-2); }
  .eng { position: absolute; right: 12px; bottom: -22px; width: 48px; height: 48px; border-radius: 15px; display: grid; place-items: center; background: var(--surface); border: var(--stroke) solid var(--outline); box-shadow: 0 3px 0 var(--shadow); }
  .bd { display: flex; flex-direction: column; gap: 8px; padding: 12px 14px 14px; flex: 1; }
  .top { display: flex; align-items: center; gap: 8px; padding-right: 56px; min-width: 0; }
  h3 { margin: 0; font-size: 18px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .meta { display: flex; flex-direction: column; gap: 3px; font-size: 12.5px; font-weight: 600; color: var(--ink-2); line-height: 1.4; }
  .meta .what { font-weight: 800; color: var(--ink); }
  .meta code { font: 600 11.5px var(--font-mono); color: var(--ink-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; margin-top: auto; padding-top: 4px; }
  .acts :global(svg) { margin-right: 4px; }
  .confirm { margin-top: auto; display: flex; flex-direction: column; gap: 6px; padding: 10px; border-radius: 14px; background: var(--butter-2); color: var(--butter-ink); font-size: 12.5px; font-weight: 700; }
  .confirm p { margin: 0; }
  .err { margin: 0; font-size: 12.5px; overflow-wrap: anywhere; }

  .cols { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  @media (max-width: 900px) { .cols { grid-template-columns: minmax(0, 1fr); } }
  .quiet { margin: 0 0 10px; color: var(--ink-2); font-weight: 600; font-size: 13px; line-height: 1.5; }
  .ask { display: flex; flex-direction: column; gap: 5px; margin-top: 10px; }
  .ask label { font-size: 12px; font-weight: 800; color: var(--ink-2); }
  .row { display: flex; gap: 8px; }
  .row input { flex: 1; min-width: 0; min-height: 36px; font: 600 13px var(--font-mono); background: var(--surface); }
  .row :global(svg) { margin-right: 4px; }
  .help { margin: 10px 0 0; font-size: 12px; font-weight: 700; color: var(--ink-3); overflow-wrap: anywhere; }

  .looked { list-style: none; margin: 8px 0 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .looked li { display: flex; gap: 10px; align-items: flex-start; min-width: 0; }
  .mark { flex: none; width: 24px; height: 24px; border-radius: 8px; display: grid; place-items: center; background: var(--surface-2); color: var(--ink-3); }
  .mark.yes { background: var(--ok-bg); color: var(--ok); }
  .lk { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .lk code { font: 600 12px var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lk .s { font-size: 12px; font-weight: 600; color: var(--ink-2); }

  :global(.worlds .spin) { animation: spin 1.1s linear infinite; }
  @keyframes spin { to { rotate: 360deg; } }
  @media (prefers-reduced-motion: reduce) { :global(.worlds .spin) { animation: none; } }

  @media (max-width: 600px) {
    .hero { padding: 18px 16px; }
    .in { gap: 12px; }
    .in > :global(.mascot) { width: 56px; height: 56px; }
    .row { flex-wrap: wrap; }
    .row .btn { flex: 1 1 auto; }
  }
</style>
