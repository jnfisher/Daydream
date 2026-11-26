<!-- The six places, down the left (along the bottom on a phone). -->
<script lang="ts">
  import House from '@lucide/svelte/icons/house';
  import Library from '@lucide/svelte/icons/library';
  import MapIcon from '@lucide/svelte/icons/map';
  import Users from '@lucide/svelte/icons/users';
  import Workflow from '@lucide/svelte/icons/workflow';
  import Wrench from '@lucide/svelte/icons/wrench';
  import { PLACES } from './places';
  import { app } from './state.svelte';

  const ICONS = { home: House, world: MapIcon, library: Library, story: Workflow, players: Users, workshop: Wrench };
</script>

{#each PLACES as p (p.id)}
  {@const Icon = ICONS[p.id]}
  {@const on = app.route.place === p.id}
  <a class="rail-item" class:on href={p.href} title="{p.label}: {p.hint} (g then {p.key})" aria-current={on ? 'page' : undefined}>
    <span class="tile"><Icon size={19} /></span>{p.label}
  </a>
{/each}

<style>
  .rail-item {
    display: flex; flex-direction: column; align-items: center; gap: 3px;
    padding: 7px 2px 6px; border-radius: 14px; color: var(--ink-2); text-decoration: none;
    font-weight: 800; font-size: 11px; letter-spacing: .01em;
  }
  .tile {
    width: 40px; height: 32px; border-radius: 11px; display: grid; place-items: center;
    border: 1.5px solid transparent; transition: transform .2s var(--pop), background .15s;
  }
  .rail-item:hover { color: var(--ink); }
  .rail-item:hover .tile { background: var(--surface-2); transform: translateY(-1px); }
  .rail-item.on { color: var(--ink); }
  .rail-item.on .tile { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); box-shadow: 0 2px 0 var(--shadow); }
  @media (max-width: 760px) {
    .rail-item { flex: 1; font-size: 10.5px; padding: 4px 0; }
    .tile { height: 28px; }
  }
</style>
