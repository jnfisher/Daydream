<!--
  silvermere's remote control, on the Publish page: the admin API and Daydream's token for it,
  whether a publish starts silvermere again, and a line to everyone in the game. Whatever is
  changed here is kept per game in Daydream's workspace (/api/control), never in silvermere.
-->
<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import Megaphone from '@lucide/svelte/icons/megaphone';
  import Play from '@lucide/svelte/icons/play';
  import Plug from '@lucide/svelte/icons/plug';
  import Send from '@lucide/svelte/icons/send';
  import { api, type GameControl } from '../api';
  import { loadPlan, pub } from './publishing.svelte';
  import { app } from './state.svelte';
  import { messageOf, toast } from './toasts.svelte';

  let { c }: { c: GameControl } = $props();

  const MAKE = 'silvermere admin token new daydream --scopes read,write,admin';

  const TOKEN: Record<NonNullable<GameControl['token_state']>, { cls: string; text: string }> = {
    ok: { cls: 'ok', text: 'Works' },
    unknown: { cls: 'warn', text: 'Not known to silvermere yet' },
    'no-read': { cls: 'warn', text: 'Needs more scopes' },
    unchecked: { cls: '', text: 'Not checked' },
    none: { cls: '', text: 'None yet' },
  };
  const tokenPill = $derived(TOKEN[c.token_state ?? 'none']);

  const FROM: Record<NonNullable<GameControl['admin_from']>, string> = {
    set: 'set here',
    'silvermere.toml': 'from silvermere.toml',
    default: 'silvermere’s default',
    unknown: 'not known',
  };
  const adminSays = $derived.by(() => {
    if (!c.admin) return 'Daydream does not know where it is: silvermere.toml turns it off, or there is none to read.';
    const from = FROM[c.admin_from ?? 'unknown'];
    if (!c.running) return `${from}; silvermere is not running`;
    return `${from}; ${c.answers ? 'it answers' : 'it does not answer'}`;
  });

  const canSay = $derived(c.running && c.answers && (c.token_state === 'ok' || c.token_state === 'no-read'));

  let editing: 'token' | 'admin' | null = $state(null);
  let tokenIn = $state('');
  let adminIn = $state('');
  let line = $state('');
  let busy = $state(false);

  /** Saves a change to the control settings, then reads the plan again: its steps follow them. */
  async function change(body: Parameters<typeof api.setControl>[0], done: string): Promise<boolean> {
    busy = true;
    try {
      const fresh = await api.setControl(body);
      if (pub.plan) pub.plan.control = fresh;
      toast(done);
      void loadPlan();
      return true;
    } catch (e) {
      toast(messageOf(e), { kind: 'error' });
      return false;
    } finally {
      busy = false;
    }
  }

  async function saveToken(e: SubmitEvent) {
    e.preventDefault();
    const token = tokenIn.trim();
    if (!token) return;
    if (await change({ token }, 'Token kept for this game.')) {
      tokenIn = '';
      editing = null;
    }
  }

  async function forgetToken() {
    if (!confirm('Forget Daydream’s admin token for this game? You will stop silvermere yourself when you publish.')) return;
    await change({ token: '' }, 'Token forgotten.');
  }

  async function saveAdmin(e: SubmitEvent) {
    e.preventDefault();
    if (await change({ admin: adminIn.trim() }, adminIn.trim() ? 'Admin API address kept.' : 'Back to the address in silvermere.toml.')) editing = null;
  }

  function editAdmin() {
    adminIn = c.admin_from === 'set' ? (c.admin ?? '') : '';
    editing = editing === 'admin' ? null : 'admin';
  }

  async function setAuto(on: boolean) {
    await change({ autostart: on }, on ? 'A publish starts silvermere again.' : 'You start silvermere again after a publish.');
  }

  async function send(e: SubmitEvent) {
    e.preventDefault();
    const text = line.trim();
    if (!text) return;
    busy = true;
    try {
      const r = await api.say(text);
      toast(r.delivered ? `${r.delivered} player${r.delivered === 1 ? '' : 's'} heard it.` : 'Nobody is in the game to hear it.');
      line = '';
    } catch (err) {
      toast(messageOf(err), { kind: 'error' });
    } finally {
      busy = false;
    }
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(MAKE);
      toast('Copied.');
    } catch {
      toast('Your browser would not copy it: select the line instead.', { kind: 'error' });
    }
  }

  // who is in the game, and whether silvermere answers, kept fresh while the page is open
  $effect(() => {
    const t = setInterval(async () => {
      if (document.visibilityState !== 'visible' || pub.job?.state === 'running' || app.route.place !== 'publish') return;
      try {
        const fresh = await api.control();
        const now = pub.plan?.control;
        if (!pub.plan || fresh.engine !== 'silvermere' || !now) return;
        // what changes the plan's steps reads the whole plan again
        const shape = (g: GameControl) => [g.running, g.answers, g.token_state, g.autostart, g.start?.kind].join();
        if (shape(fresh) !== shape(now)) void loadPlan();
        else pub.plan.control = fresh;
      } catch {
        // the next look asks again
      }
    }, 20_000);
    return () => clearInterval(t);
  });
</script>

<section class="card server" aria-labelledby="server-h">
  <div class="card-h">
    <h3 id="server-h">Server</h3>
    <span class="end"><span class="pill {tokenPill.cls}"><KeyRound size={13} />{tokenPill.text}</span></span>
  </div>

  <div class="part">
    <div class="k"><KeyRound size={15} />Admin token</div>
    <p>{c.token_text}</p>
    {#if c.token && editing !== 'token'}
      <div class="acts">
        <button type="button" class="btn sm" disabled={busy} onclick={() => (editing = 'token')}>Replace it</button>
        <button type="button" class="btn sm ghost danger" disabled={busy} onclick={forgetToken}>Forget it</button>
      </div>
    {:else}
      <form class="inline" onsubmit={saveToken}>
        <input type="password" autocomplete="off" spellcheck="false" maxlength="512" aria-label="Admin token" placeholder="Paste the token silvermere printed" bind:value={tokenIn} disabled={busy} />
        <button type="submit" class="btn sm sticker" disabled={busy || !tokenIn.trim()}>Keep it</button>
        {#if c.token}<button type="button" class="btn sm ghost" onclick={() => ((editing = null), (tokenIn = ''))}>Not now</button>{/if}
      </form>
      <p class="hint">
        Make one where silvermere runs, add the <code>[[admin.tokens]]</code> block it prints to silvermere.toml, and start silvermere again once:
      </p>
      <div class="cmd">
        <code>{MAKE}</code>
        <button type="button" class="btn icon sm ghost" title="Copy" aria-label="Copy the command" onclick={copy}><Copy size={14} /></button>
      </div>
    {/if}
  </div>

  <div class="part">
    <div class="k"><Plug size={15} />Admin API</div>
    <div class="api">
      <span class="dot" class:pulse={c.answers} style:color={c.answers ? 'var(--mint)' : 'var(--ink-3)'}></span>
      <code>{c.admin ?? 'not known'}</code>
      {#if c.custom}<button type="button" class="btn sm ghost end" onclick={editAdmin}>{editing === 'admin' ? 'Not now' : 'Change'}</button>{/if}
    </div>
    <p class="faint">{adminSays}</p>
    {#if editing === 'admin'}
      <form class="inline" onsubmit={saveAdmin}>
        <input type="url" spellcheck="false" aria-label="Admin API address" placeholder="http://127.0.0.1:8080" bind:value={adminIn} disabled={busy} />
        <button type="submit" class="btn sm sticker" disabled={busy}>Keep it</button>
      </form>
      <p class="hint">Leave it empty to use silvermere.toml’s <code>admin_bind</code>. A new address forgets the token: it goes only where it was given.</p>
    {/if}
  </div>

  <div class="part">
    <div class="k"><Play size={15} />After a publish</div>
    {#if c.start}
      <label class="auto">
        <span>
          Start silvermere again
          <code class="sub">{c.start.command}</code>
        </span>
        <input class="switch" type="checkbox" role="switch" checked={!!c.autostart} disabled={busy || !c.start.can} onchange={(e) => void setAuto(e.currentTarget.checked)} />
      </label>
      {#if !c.start.can}
        <p class="hint">silvermere runs as a system service, and Daydream is not allowed to start one: you run that command after a publish.</p>
      {/if}
    {:else}
      <p>Daydream did not find what runs silvermere, so you start it again after a publish.</p>
    {/if}
  </div>

  {#if canSay}
    <form class="part say" onsubmit={send}>
      <label class="k" for="say-line"><Megaphone size={15} />Say something to everyone in the game</label>
      <div class="inline">
        <input id="say-line" type="text" maxlength="1000" placeholder="The dragon has moved to the old mill…" bind:value={line} disabled={busy} />
        <button type="submit" class="btn sm sticker" disabled={busy || !line.trim()}><Send size={14} />Send</button>
      </div>
    </form>
  {/if}
</section>

<style>
  h3 { margin: 0; font-size: 17px; }
  .end { margin-left: auto; }
  .part { display: flex; flex-direction: column; gap: 6px; padding: 12px 0; border-top: 1px dashed var(--line); }
  .card-h + .part { border-top: 0; padding-top: 2px; }
  .part:last-child { padding-bottom: 0; }
  .k { display: flex; align-items: center; gap: 7px; font: 800 12.5px var(--font-ui); color: var(--ink); }
  .k :global(svg) { color: var(--ink-3); }
  .part p { margin: 0; font-size: 12.5px; font-weight: 600; color: var(--ink-2); line-height: 1.45; }
  .part p.hint { color: var(--ink-3); }
  code { font: 600 12px var(--font-mono); overflow-wrap: anywhere; }
  .acts { display: flex; gap: 6px; flex-wrap: wrap; }
  .inline { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .inline input { flex: 1 1 200px; min-height: 30px; font: 600 12.5px var(--font-ui); }
  .cmd { display: flex; align-items: center; gap: 6px; padding: 4px 4px 4px 10px; border-radius: 10px; background: var(--surface-2); border: 1px solid var(--line); }
  .cmd code { flex: 1; min-width: 0; color: var(--ink); user-select: all; }
  .api { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .api code { color: var(--ink); }
  .auto { display: flex; align-items: center; gap: 12px; cursor: pointer; font-size: 13px; font-weight: 700; color: var(--ink); }
  .auto > span { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .auto .sub { color: var(--ink-3); font-size: 11.5px; }
</style>
