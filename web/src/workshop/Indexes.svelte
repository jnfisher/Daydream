<!--
  Monster groups › Move a group or index: NMR's Monster Group/Index Changer. Monsters of a run
  of one group's indexes take another group and indexes, and the rooms whose Min to Max Index
  range spawns them move with them, as one step of the history. A room's range moves as one,
  so the runs the rooms spawn are listed: pick one to move it whole.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import { api, type SpawnRun } from '../api';
  import { MONSTER_GROUP } from '../layouts';
  import { app } from '../shell/state.svelte';
  import { messageOf } from '../shell/toasts.svelte';
  import { whole } from './fields';
  import { endOf, indexChangesOf, lineText, type IndexLine } from './moves';
  import Run from './Run.svelte';

  /** Rooms' runs shown before "and N more". */
  const RUNS = 30;

  const GROUPS = Object.entries(MONSTER_GROUP).map(([v, name]) => [Number(v), String(name)] as const);
  const groupName = (g: number) => String(MONSTER_GROUP[g] ?? `Group ${g}`);

  interface Row extends IndexLine {
    id: number;
  }
  let ids = 0;
  const first = GROUPS[0]?.[0] ?? 0;
  const row = (group = first, from = '', to = '', toGroup = group, start = ''): Row => ({ id: ++ids, group, from, to, toGroup, start });

  let lines = $state<Row[]>([row()]);
  let spawns = $state<SpawnRun[] | null>(null);
  let error = $state('');
  let only = $state<number | 'all'>('all');
  let all = $state(false);

  let asked = 0;
  $effect(() => {
    void app.changes;
    void app.current;
    const my = ++asked;
    api
      .spawnRuns()
      .then((r) => my === asked && ((spawns = r.runs), (error = '')))
      .catch((e) => my === asked && (error = messageOf(e)));
  });

  const listed = $derived((spawns ?? []).filter((r) => only === 'all' || r.group === only).sort((a, b) => a.group - b.group || a.from - b.from));
  const spawnGroups = $derived([...new Set((spawns ?? []).map((r) => r.group))].sort((a, b) => a - b));

  /** A run the rooms spawn: into the last line if it is empty, else a line of its own. */
  function take(r: SpawnRun) {
    const last = lines.at(-1)!;
    const to = r.to === r.from ? '' : String(r.to);
    if (!last.from.trim()) {
      Object.assign(last, { group: r.group, from: String(r.from), to, toGroup: r.group, start: last.start });
      return;
    }
    const end = whole(endOf(last));
    lines.push(row(r.group, String(r.from), to, last.toGroup === last.group ? r.group : last.toGroup, Number.isNaN(end) || last.toGroup !== r.group ? '' : String(end + 1)));
  }

  const changes = $derived(indexChangesOf(lines));
  const blocked = $derived('error' in changes ? changes.error : '');
  const body = $derived('changes' in changes ? changes.changes : []);

  const label = $derived.by(() => {
    const said = lines.filter((l) => lineText(l));
    if (said.length !== 1) return `Move ${body.length.toLocaleString()} monster groups and indexes`;
    const l = said[0]!;
    const span = (a: number, b: number) => (a === b ? `${a}` : `${a}–${b}`);
    const [a, b, s] = [whole(l.from), l.to.trim() ? whole(l.to) : whole(l.from), whole(l.start)];
    return `Move ${groupName(l.group)} ${span(a, b)} to ${groupName(l.toGroup)} ${span(s, s + b - a)}`;
  });

  const done = (n: number) => `Moved ${body.length.toLocaleString()} ${body.length === 1 ? 'index' : 'indexes'}: ${n.toLocaleString()} monsters and rooms changed.`;
  const roomsOf = (r: SpawnRun) => r.rooms.map(([m, n]) => `${m}/${n}`).join(', ');
</script>

<div class="ws-form">
  <div class="lines" role="group" aria-label="Groups and indexes to move">
    <div class="line head" aria-hidden="true"><span>Group</span><span>Indexes</span><span></span><span>New group</span><span>New indexes</span><span></span></div>
    {#each lines as l (l.id)}
      {@const end = endOf(l)}
      <div class="line">
        <select
          aria-label="Group"
          value={l.group}
          onchange={(e) => {
            // the new group follows, unless another was picked
            const g = Number(e.currentTarget.value);
            if (l.toGroup === l.group) l.toGroup = g;
            l.group = g;
          }}
        >
          {#each GROUPS as [v, name] (v)}<option value={v}>{name}</option>{/each}
        </select>
        <span class="span">
          <input class="num" inputmode="numeric" aria-label="First index to move" bind:value={l.from} />
          <span class="to">to</span>
          <input class="num" inputmode="numeric" aria-label="Last index to move (blank for one)" placeholder={l.from} bind:value={l.to} />
        </span>
        <span class="arrow" aria-hidden="true">→</span>
        <span class="span">
          <span class="to becomes" aria-hidden="true">becomes</span>
          <select aria-label="New group" bind:value={l.toGroup}>
            {#each GROUPS as [v, name] (v)}<option value={v}>{name}</option>{/each}
          </select>
        </span>
        <span class="span">
          <input class="num" inputmode="numeric" aria-label="New first index" bind:value={l.start} />
          {#if end && l.to.trim() && l.to.trim() !== l.from.trim()}<span class="to">to <b>{end}</b></span>{/if}
        </span>
        <span>
          {#if lines.length > 1}
            <button type="button" class="btn sm icon ghost" aria-label="Take this line out" title="Take this line out" onclick={() => (lines = lines.filter((x) => x.id !== l.id))}><X size={14} /></button>
          {/if}
        </span>
      </div>
    {/each}
    <div class="add"><button type="button" class="btn sm" onclick={() => lines.push(row(lines.at(-1)?.group, '', '', lines.at(-1)?.toGroup))}><Plus size={14} />Another run of indexes</button></div>
  </div>

  <div class="spawns">
    <div class="row">
      <span class="lbl">Rooms spawn</span>
      <select aria-label="Which group's runs" bind:value={only}>
        <option value="all">Every group</option>
        {#each spawnGroups as g (g)}<option value={g}>{groupName(g)}</option>{/each}
      </select>
    </div>
    {#if error}
      <p class="hint bad">{error}</p>
    {:else if spawns}
      <p class="hint">A room's Min to Max Index range moves as one, so move its indexes together. Pick a run to move it whole.</p>
      <div class="runs">
        {#each all ? listed : listed.slice(0, RUNS) as r (`${r.group}:${r.from}:${r.to}`)}
          <button type="button" class="chip" title="Rooms {roomsOf(r)}" onclick={() => take(r)}>
            {groupName(r.group)} <b>{r.from === r.to ? r.from : `${r.from}–${r.to}`}</b>
            <span class="n">{r.rooms.length} {r.rooms.length === 1 ? 'room' : 'rooms'}</span>
          </button>
        {/each}
        {#if listed.length > RUNS}
          <button type="button" class="btn sm ghost" onclick={() => (all = !all)}>{all ? 'Fewer' : `and ${(listed.length - RUNS).toLocaleString()} more`}</button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<Run run={(dry) => api.moveIndexes(body, dry, label)} key={JSON.stringify(body)} {blocked} {done} onapplied={() => (lines = [row()])} plain />

<style>
  .ws-form { display: flex; flex-direction: column; gap: 12px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font: 800 12.5px var(--font-ui); color: var(--ink-2); min-width: 7.5em; }
  .num { width: 5.5em; font-family: var(--font-mono); }
  /* one grid for every line, so the columns line up under their heads */
  .lines { display: grid; grid-template-columns: minmax(8em, max-content) max-content 1.5em minmax(8em, max-content) max-content minmax(0, auto); align-items: center; gap: 6px 8px; justify-content: start; }
  .line { display: contents; }
  .add { grid-column: 1 / -1; }
  .line.head span { font: 800 10.5px/1.2 var(--font-ui); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink-3); }
  .span { display: inline-flex; align-items: center; gap: 6px; }
  .to { font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  .to b { font-family: var(--font-mono); color: var(--ink-2); }
  .arrow { text-align: center; font-weight: 800; color: var(--lilac-ink); }
  /* on a phone the heads go, and this says which group is new */
  .becomes { display: none; }
  .span select { min-width: 0; flex: 1 1 auto; }
  .spawns { display: flex; flex-direction: column; gap: 6px; }
  .runs { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; max-height: 180px; overflow: auto; }
  .chip { display: inline-flex; align-items: center; gap: 6px; min-height: 26px; padding: 0 10px; border-radius: 999px; font-size: 12px; font-weight: 700; background: var(--surface-2); border: 1px solid var(--line); color: var(--ink-2); box-shadow: none; }
  .chip b { font-family: var(--font-mono); }
  .chip .n { color: var(--ink-3); font-weight: 700; }
  .chip:hover { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); }
  .hint { margin: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-3); max-width: 75ch; }
  .hint.bad { color: var(--bad); }
  @media (max-width: 720px) {
    /* a line in two rows: the group and indexes it moves, then where they go */
    .lines { grid-template-columns: minmax(0, 1fr) max-content; }
    .line .arrow, .line.head { display: none; }
    .line > span:last-child { grid-column: 1 / -1; }
    .becomes { display: inline; }
    .lbl { flex-basis: 100%; }
  }
</style>
