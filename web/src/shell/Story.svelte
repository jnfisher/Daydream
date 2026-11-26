<!--
  Story: quests and the text they are made of. For now NMR's Quest Organizer and the ways into
  the text blocks, messages and actions; the quest map comes later.
-->
<script lang="ts">
  import { questOrganizer } from '../quests';
  import { kindOf } from './kinds';
  import Mascot from './Mascot.svelte';
  import { recordHref } from './routes';
  import { app } from './state.svelte';

  const TABLES = [
    ['Textblocks', 'The words of quests, signs and conversations, and the scripts behind them'],
    ['Messages', 'What players read when things happen: attacks, spells, rooms'],
    ['Actions', 'The emotes players type: smile, wave, bow'],
  ] as const;

  function organizer(node: HTMLElement) {
    node.replaceChildren(questOrganizer());
  }
</script>

<div class="story">
  <header>
    <Mascot mood="dream" size={64} />
    <div>
      <h1>Story</h1>
      <p class="muted">Quests and the text they are made of.</p>
    </div>
  </header>
  <div class="ways">
    {#each TABLES as [t, sub] (t)}
      {#if app.tables.some((x) => x.name === t)}
        {@const k = kindOf(t)}
        <a class="card way {k.cls}" href={recordHref(t)}>
          <span class="tile"><k.icon size={18} /></span>
          <span><b>{k.many}</b><span class="sub">{sub}</span></span>
        </a>
      {/if}
    {/each}
  </div>
  <div use:organizer></div>
</div>

<style>
  .story { display: flex; flex-direction: column; gap: 16px; max-width: 1100px; margin: 0 auto; }
  header { display: flex; align-items: center; gap: 16px; }
  h1 { margin: 0; font-size: 30px; }
  header p { margin: 2px 0 0; font-weight: 700; }
  .ways { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 12px; }
  .way { display: flex; align-items: center; gap: 12px; text-decoration: none; color: var(--ink); transition: transform .15s var(--pop); }
  .way:hover { transform: translateY(-2px); border-color: var(--outline); }
  .way .tile { width: 38px; height: 38px; border-radius: 12px; }
  .way b { display: block; font: 600 17px var(--font-display); }
  .sub { display: block; font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
</style>
