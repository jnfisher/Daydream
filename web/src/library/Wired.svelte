<!--
  How a monster is wired to the rest of the world: the weapon its attack lines name, the line
  a room reads when it appears and when it dies, the spells it casts then, the names it is
  given, what it says when greeted or asked, and what it says now and then. Each is edited
  here, with what players will read.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Footprints from '@lucide/svelte/icons/footprints';
  import MessageSquare from '@lucide/svelte/icons/message-square';
  import Skull from '@lucide/svelte/icons/skull';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import Sword from '@lucide/svelte/icons/sword';
  import Tag from '@lucide/svelte/icons/tag';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import Users from '@lucide/svelte/icons/users';
  import { renderAnsi } from '../ansi';
  import { noteOf, targetOf } from '../layouts';
  import { arrivalLine, attacksOf, COLOUR, nameOdds, talkOdds, topicsOf } from './combat';
  import type { Draft } from './draft.svelte';
  import FieldInput from './FieldInput.svelte';
  import RefChip from './RefChip.svelte';
  import { blockOf, linesOf, recordOf } from './records.svelte';

  let { draft }: { draft: Draft } = $props();

  const num = (v: unknown) => Number(v) || 0;
  const text = (v: unknown) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);
  const get = (col: string) => num(draft.get(col));
  const name = $derived(text(draft.get('Name')) || 'it');

  const weapon = $derived(get('Weapon Number') ? recordOf('Items', [get('Weapon Number')]) : null);
  const move = $derived(linesOf(get('Move Msg')));
  const death = $derived(linesOf(get('Death Msg')));
  const names = $derived(blockOf(get('Desc Txt')));
  const greet = $derived(blockOf(get('Greet Txt')));
  const talk = $derived(blockOf(get('Talk Txt')));
  const odds = $derived(names ? nameOdds(names.text, name) : null);
  /** The name a room sees: the list's likeliest, else its own. */
  const shownName = $derived(odds?.length ? odds.reduce((a, b) => (b.pct > a.pct ? b : a)).name : name);

  const arrives = $derived(move === undefined ? undefined : arrivalLine(get('Move Msg'), move, shownName, get('Desc Txt') !== 0, 'north'));
  /** A Move Msg written name first, on a monster without a name list, reads backwards. */
  const backwards = $derived(!!move && get('Desc Txt') === 0 && (move[0].match(/%l?s/g) ?? []).length >= 2);
  /** The attacks whose messages share the Death Msg's record. */
  const shared = $derived.by(() => {
    const d = get('Death Msg');
    if (!d) return [];
    return attacksOf(draft.get)
      .map((a, i) => ({ a, i }))
      .filter(({ a }) => a.type !== 0 && (a.hit === d || a.dodge === d || a.miss === d))
      .map(({ i }) => i + 1);
  });

  function ansi(node: HTMLElement, line: string) {
    const draw = (t: string) => node.replaceChildren(renderAnsi(t));
    draw(line);
    return { update: draw };
  }

  const pct = (p: number) => (p >= 10 ? `${Math.round(p)}%` : `${p.toFixed(1).replace(/\.0$/, '')}%`);
  const field = (col: string) => ({ col, target: targetOf('Monsters', col), hint: noteOf('Monsters', col) });
</script>

<div class="wire">
  <div class="w">
    <span class="gi t-item"><Sword size={15} /></span>
    <div class="body">
      <div class="lead">Wields <FieldInput {draft} {...field('Weapon Number')} label="Weapon Number" compact /></div>
      <div class="sub">
        {#if !get('Weapon Number')}Nothing: its attack lines leave the weapon's words out.
        {:else if weapon}Its name, "{text(weapon['Name'])}", and the words of its hit and miss messages fill its attack lines.
        {/if}
      </div>
    </div>
  </div>

  <div class="w">
    <span class="gi t-msg"><Footprints size={15} /></span>
    <div class="body">
      <div class="lead">Arrives <FieldInput {draft} {...field('Move Msg')} label="Move Msg" compact /></div>
      {#if arrives}<div class="mini" use:ansi={arrives}></div>{:else if arrives === ''}<div class="sub">Line 1 of its Move Msg is empty: the room reads nothing.</div>{/if}
      {#if backwards}
        <p class="callout warn"><TriangleAlert size={15} /><span>Without a name list (Desc Txt) the game fills this line with where it came from first, then its name, so it reads as above. Give it a name list, or a line written that way round.</span></p>
      {/if}
      <div class="sub">Line 1 of the Move Msg, when it appears in a room. Its moves after that use the game's own lines.</div>
    </div>
  </div>

  <div class="w">
    <span class="gi t-msg"><Skull size={15} /></span>
    <div class="body">
      <div class="lead">Dies <FieldInput {draft} {...field('Death Msg')} label="Death Msg" compact /></div>
      {#if death}<div class="mini" use:ansi={COLOUR.grey + death[2]}></div>{/if}
      {#if shared.length}
        <div class="sub">Line 3 of message {get('Death Msg')} goes to the room. The same message is attack {shared.join(' and ')}'s, so its other lines are attack lines: a change to it changes both.</div>
      {:else if get('Death Msg')}<div class="sub">Line 3 of the message goes to the room.</div>
      {/if}
    </div>
  </div>

  <div class="w">
    <span class="gi t-spell"><Sparkles size={15} /></span>
    <div class="body">
      <div class="lead">When it appears, casts <FieldInput {draft} {...field('Create Spell')} label="Create Spell" compact /></div>
      <div class="lead">When it dies, casts <FieldInput {draft} {...field('Death Spell')} label="Death Spell" compact /></div>
    </div>
  </div>

  <div class="w">
    <span class="gi t-text"><Tag size={15} /></span>
    <div class="body">
      <div class="lead">Named from <FieldInput {draft} {...field('Desc Txt')} label="Desc Txt" compact /></div>
      {#if odds}
        <div class="chips" aria-label="The names it may be given">
          {#each odds.slice(0, 10) as o (o.name)}<span class="tag" title="{pct(o.pct)} of the time">{o.name} <small>{pct(o.pct)}</small></span>{/each}
          {#if odds.length > 10}<span class="faint">+{odds.length - 10} more</span>{/if}
        </div>
        <div class="sub">The list is read line by line with a 1 in 10 chance to stop at each, so its first lines are the likeliest and the last takes what is left.</div>
      {:else if names === null && get('Desc Txt')}<div class="sub bad">There is no text block {get('Desc Txt')}: it keeps its own name.</div>
      {:else}<div class="sub">Its own name: "{name}".</div>
      {/if}
    </div>
  </div>

  <div class="w">
    <span class="gi t-text"><MessageSquare size={15} /></span>
    <div class="body">
      <div class="lead">Greeted or asked <FieldInput {draft} {...field('Greet Txt')} label="Greet Txt" compact /></div>
      {#if greet}
        {@const topics = topicsOf(greet.text)}
        <div class="chips">
          {#if greet.link}<span class="faint">greeted, it shows</span><RefChip table="Textblocks" key={[greet.link, 0]} compact />{:else}<span class="faint">greeted, it does not understand</span>{/if}
        </div>
        {#if topics.length}
          <div class="chips">
            <span class="faint">asked about</span>
            {#each topics.slice(0, 8) as t (t.words + t.block)}<span class="topic"><span class="tag">{t.words}</span><RefChip table="Textblocks" key={[t.block, 0]} compact /></span>{/each}
            {#if topics.length > 8}<span class="faint">+{topics.length - 8} more</span>{/if}
          </div>
        {/if}
      {:else if get('Greet Txt') === 0}<div class="sub">Nothing: "{name} has nothing to tell you."</div>
      {/if}
    </div>
  </div>

  <div class="w">
    <span class="gi t-text"><Users size={15} /></span>
    <div class="body">
      <div class="lead">Now and then says <FieldInput {draft} {...field('Talk Txt')} label="Talk Txt" compact /></div>
      {#if talk}
        {@const lines = talkOdds(talk.text)}
        {#if lines.length}
          <div class="mini lines">
            {#each lines.slice(0, 6) as l, k (k)}<div><span class="p">{pct(l.pct)}</span><span use:ansi={COLOUR.grey + l.text}></span></div>{/each}
          </div>
          <div class="sub">Each time the game picks one of its monsters to talk, it says the first line whose chance beats a roll of 0 to 99, so the chances add up as they go.</div>
        {:else}<div class="sub">The block has no "chance text" lines, so it never says anything.</div>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .wire { display: grid; gap: 12px; }
  .w { display: grid; grid-template-columns: 30px minmax(0, 1fr); gap: 10px; align-items: start; }
  .w + .w { padding-top: 12px; border-top: 1px dashed var(--line); }
  .gi { width: 30px; height: 30px; border-radius: 10px; display: grid; place-items: center; background: var(--t-bg); color: var(--t-ink); }
  .body { display: grid; gap: 6px; min-width: 0; }
  .lead { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-weight: 800; font-size: 13.5px; }
  .sub { color: var(--ink-3); font-size: 12px; font-weight: 700; line-height: 1.45; }
  .sub.bad { color: var(--bad); }
  .mini { font: 13px/1.3 var(--font-mono); background: var(--term-bg); color: var(--term-fg); border-radius: 10px; padding: 7px 10px; white-space: pre-wrap; overflow-wrap: anywhere; border: var(--stroke) solid var(--term-frame); }
  .mini.lines { display: grid; gap: 2px; }
  .mini .p { display: inline-block; min-width: 3.2rem; color: #9b97c4; font: 800 11px var(--font-ui); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 12.5px; font-weight: 700; }
  .tag { display: inline-flex; align-items: center; gap: 4px; height: 24px; padding: 0 9px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); font-size: 12px; font-weight: 800; color: var(--ink-2); }
  .tag small { color: var(--ink-3); font-weight: 700; }
  .topic { display: inline-flex; align-items: center; gap: 3px; }
  .callout { margin: 0; }
</style>
