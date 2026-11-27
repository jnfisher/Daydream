<!--
  The Select tool's panel, beside the map (a sheet along the bottom of a small screen): how rooms
  are picked (a box, a lasso, the brush or the wand: keys 1 to 4), whether a pick starts afresh,
  adds to the rooms picked or takes from them (Shift and Alt for a moment), how many are picked,
  and the fields to set on them all at once (name, light, monsters, control room and room type),
  each showing what the rooms share or that they differ.
-->
<svelte:options css="injected" />

<script lang="ts">
  import Brush from '@lucide/svelte/icons/brush';
  import CheckCheck from '@lucide/svelte/icons/check-check';
  import Flag from '@lucide/svelte/icons/flag';
  import Focus from '@lucide/svelte/icons/focus';
  import House from '@lucide/svelte/icons/house';
  import Lasso from '@lucide/svelte/icons/lasso';
  import Minus from '@lucide/svelte/icons/minus';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import Skull from '@lucide/svelte/icons/skull';
  import SquareDashed from '@lucide/svelte/icons/square-dashed';
  import SunDim from '@lucide/svelte/icons/sun-dim';
  import Type from '@lucide/svelte/icons/type';
  import WandSparkles from '@lucide/svelte/icons/wand-sparkles';
  import X from '@lucide/svelte/icons/x';
  import type { Component } from 'svelte';
  import { MONSTER_GROUP, ROOM_TYPE } from '../layouts';
  import type { MapRoom } from './model';
  import { BRUSH_SIZES, LIGHT_PICKS, lightWord, mapValue, PICK_FIELDS, PICK_MODES, PICK_TOOLS, sharedOf, type PickField, type PickMode, type PickTool } from './picks';

  let {
    tool,
    mode,
    rooms,
    total,
    map,
    touch,
    ranges,
    brush,
    nameMax,
    named,
    ontool,
    onmode,
    onbrush,
    onset,
    onall,
    onclear,
    onshow,
    onclose,
  }: {
    tool: PickTool;
    mode: PickMode;
    /** The rooms picked, in room order. */
    rooms: MapRoom[];
    /** How many rooms the map has. */
    total: number;
    map: number;
    /** A finger, not a mouse: no keys to show. */
    touch: boolean;
    /** The whole numbers each Rooms column holds, by column. */
    ranges: Record<string, [number, number] | null>;
    /** How many rooms across the brush takes. */
    brush: number;
    nameMax: number;
    /** A room of this map for a person ("Town square 1/245"), or null when the map has no such room. */
    named: (n: number) => string | null;
    ontool: (t: PickTool) => void;
    onmode: (m: PickMode) => void;
    /** The brush a size larger (1) or smaller (-1). */
    onbrush: (by: number) => void;
    onset: (f: PickField, values: [string, string | number][]) => void;
    onall: () => void;
    onclear: () => void;
    onshow: () => void;
    onclose: () => void;
  } = $props();

  const TOOL_ICONS: Record<PickTool, Component<{ size?: number }>> = { box: SquareDashed, lasso: Lasso, brush: Brush, wand: WandSparkles };
  const FIELD_ICONS: Record<PickField['id'], Component<{ size?: number }>> = { name: Type, light: SunDim, monsters: Skull, control: Flag, type: House };

  /** What each way of picking does, for a mouse and for a finger. */
  const HOW: Record<PickTool, [string, string]> = {
    box: ['Drag a box around rooms; a click picks one.', 'Drag a box around rooms; a tap picks one.'],
    lasso: ['Draw around rooms; a click picks one.', 'Draw around rooms; a tap picks one.'],
    brush: ['Paint over rooms to pick them.', 'Paint over rooms to pick them.'],
    wand: [
      'Click a room for the rooms joined to it in its colour. Colour the map by area, monsters or light to pick by those.',
      'Tap a room for the rooms joined to it in its colour. Colour the map by area, monsters or light to pick by those.',
    ],
  };

  const group = (g: number) => MONSTER_GROUP[g] ?? `Mon Type ${g}`;

  /** What the rooms picked share in a field, for a person; `mixed` when they differ. */
  function said(f: PickField): { text: string; mixed: boolean } {
    const s = sharedOf(rooms, f);
    if (s === null) return { text: '', mixed: false };
    if (s === 'mixed') return { text: 'Mixed', mixed: true };
    const [a, b, c, d] = s.one as [number, number, number, number];
    switch (f.id) {
      case 'name':
        return { text: s.one[0] ? `“${s.one[0]}”` : 'No name', mixed: false };
      case 'light':
        return { text: `${a}, ${lightWord(a).toLowerCase()}`, mixed: false };
      case 'monsters':
        return { text: c === 0 || d === 0 ? `${group(a)}, none come` : `${group(a)} ${b} to ${c}, up to ${d}`, mixed: false };
      case 'control':
        return { text: a ? (named(a) ?? `room ${map}/${a}, not on this map`) : 'None', mixed: false };
      case 'type':
        return { text: ROOM_TYPE[a] ?? `Type ${a}`, mixed: false };
    }
  }

  /** The field being set, and what it is set to. */
  let open = $state<PickField['id'] | null>(null);
  let text = $state('');
  let nums = $state<(number | null)[]>([]);
  const editing = $derived(PICK_FIELDS.find((f) => f.id === open) ?? null);

  function edit(f: PickField) {
    if (open === f.id) {
      open = null;
      return;
    }
    const s = sharedOf(rooms, f);
    const first = rooms[0];
    const from = s && s !== 'mixed' ? s.one : first ? f.columns.map((c) => mapValue(first, c) ?? 0) : [];
    if (f.id === 'name') text = s && s !== 'mixed' ? String(from[0] ?? '') : '';
    else nums = from.map(Number);
    open = f.id;
  }

  /** What is wrong with the values typed, if anything. */
  const problem = $derived.by(() => {
    const f = editing;
    if (!f || f.id === 'name') return '';
    for (const [i, c] of f.columns.entries()) {
      const v = nums[i];
      if (typeof v !== 'number' || !Number.isInteger(v)) return `${c} takes a whole number.`;
      const r = ranges[c];
      if (r && (v < r[0] || v > r[1])) return `${c} goes from ${r[0]} to ${r[1]}.`;
    }
    const [a, b, c] = nums as number[];
    if (f.id === 'control' && a && named(a) === null) return `Map ${map} has no room ${a}.`;
    if (f.id === 'monsters' && c && b! > c) return 'Min Index is over Max Index.';
    return '';
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const f = editing;
    if (!f || problem) return;
    onset(f, f.id === 'name' ? [['Name', text.trim()]] : f.columns.map((c, i) => [c, nums[i] as number]));
    open = null;
  }

  const many = $derived(rooms.length === 1 ? 'the room' : `${rooms.length.toLocaleString()} rooms`);
  const range = (c: string) => ranges[c] ?? [-32768, 32767];
</script>

<section class="drawer" aria-label="Select rooms">
  <header>
    <b>Select</b>
    <span class="faint">Pick rooms, then set their fields</span>
    <button type="button" class="btn icon sm ghost" onclick={onclose} title="Done selecting (V)" aria-label="Done selecting"><X size={15} /></button>
  </header>
  <div class="ways" role="radiogroup" aria-label="Pick rooms with">
    {#each PICK_TOOLS as t, i (t.id)}
      {@const Icon = TOOL_ICONS[t.id]}
      <button type="button" role="radio" aria-checked={tool === t.id} class="way" class:on={tool === t.id} title="{t.about}{touch ? '' : ` (${i + 1})`}" onclick={() => ontool(t.id)}>
        {#if !touch}<span class="key">{i + 1}</span>{/if}
        <Icon size={18} />
        <span class="name">{t.name}</span>
      </button>
    {/each}
  </div>
  <div class="modes" role="radiogroup" aria-label="Each pick">
    {#each PICK_MODES as m (m.id)}
      <button type="button" role="radio" aria-checked={mode === m.id} class:on={mode === m.id} title={m.about} onclick={() => onmode(m.id)}>{m.name}</button>
    {/each}
  </div>
  {#if tool === 'brush'}
    <div class="size">
      <span>Size</span>
      <button type="button" class="btn icon sm" disabled={brush <= BRUSH_SIZES[0]!} onclick={() => onbrush(-1)} aria-label="Smaller" title={touch ? 'Smaller' : 'Smaller ([)'}><Minus size={14} /></button>
      <span class="n tnum">{brush}</span>
      <button type="button" class="btn icon sm" disabled={brush >= BRUSH_SIZES.at(-1)!} onclick={() => onbrush(1)} aria-label="Bigger" title={touch ? 'Bigger' : 'Bigger (])'}><Plus size={14} /></button>
      <span>{brush === 1 ? 'room across' : 'rooms across'}</span>
    </div>
  {/if}
  <p class="how">{HOW[tool][touch ? 1 : 0]} {touch ? 'Two fingers move the map.' : 'Shift adds, Alt takes away, and Space with a drag moves the map.'}</p>
  <div class="count">
    <b class="tnum" aria-live="polite">{rooms.length ? (rooms.length === 1 ? `Room ${map}/${rooms[0]!.room}` : `${rooms.length.toLocaleString()} rooms`) : 'No rooms yet'}</b>
    <span class="grow"></span>
    {#if rooms.length}
      <button type="button" class="btn sm ghost" onclick={onshow} title="Show the rooms picked"><Focus size={14} />Show</button>
    {/if}
    <button type="button" class="btn sm ghost" onclick={onall} disabled={rooms.length >= total} title="Pick every room of map {map}{touch ? '' : ' (Ctrl+A)'}"><CheckCheck size={14} />All</button>
    {#if rooms.length}
      <button type="button" class="btn sm ghost" onclick={onclear} title="Let go of them{touch ? '' : ' (Esc)'}"><X size={14} />Clear</button>
    {/if}
  </div>
  {#if rooms.length}
    <div class="sets">
      <div class="eyebrow">Set on {many}</div>
      {#each PICK_FIELDS as f (f.id)}
        {@const s = said(f)}
        {@const Icon = FIELD_ICONS[f.id]}
        <div class="setter" class:open={open === f.id}>
          <button type="button" class="row" aria-expanded={open === f.id} onclick={() => edit(f)}>
            <span class="ic"><Icon size={14} /></span>
            <span class="what">{f.label}</span>
            <span class="value" class:mixed={s.mixed}>{s.text}</span>
            <Pencil size={13} />
          </button>
          {#if open === f.id}
            <form class="edit" onsubmit={submit}>
              {#if f.id === 'name'}
                <!-- svelte-ignore a11y_autofocus -->
                <input type="text" bind:value={text} maxlength={nameMax} spellcheck="false" autocomplete="off" aria-label="Name" placeholder={s.mixed ? 'A name for them all' : 'No name'} autofocus />
              {:else if f.id === 'light'}
                <label class="line">
                  <span>Light</span>
                  <!-- svelte-ignore a11y_autofocus -->
                  <input type="number" class="num" bind:value={nums[0]} min={range('Light')[0]} max={range('Light')[1]} step="1" autofocus />
                  <span class="faint">{typeof nums[0] === 'number' ? lightWord(nums[0]) : ''}</span>
                </label>
                <div class="picks">
                  {#each LIGHT_PICKS as v (v)}
                    <button type="button" class="chip" class:on={nums[0] === v} onclick={() => (nums[0] = v)}>{lightWord(v)} <span class="tnum">{v}</span></button>
                  {/each}
                </div>
              {:else if f.id === 'monsters'}
                <label class="line">
                  <span>Mon Type</span>
                  <select bind:value={nums[0]}>
                    {#each Object.entries(MONSTER_GROUP) as [v, label] (v)}<option value={Number(v)}>{label}</option>{/each}
                  </select>
                </label>
                <div class="line">
                  <label><span>Min Index</span><input type="number" class="num" bind:value={nums[1]} min={range('Min Index')[0]} max={range('Min Index')[1]} step="1" /></label>
                  <label><span>Max Index</span><input type="number" class="num" bind:value={nums[2]} min={range('Max Index')[0]} max={range('Max Index')[1]} step="1" /></label>
                </div>
                <label class="line">
                  <span>Max Regen</span>
                  <input type="number" class="num" bind:value={nums[3]} min={range('Max Regen')[0]} max={range('Max Regen')[1]} step="1" />
                  <span class="faint">at a time</span>
                </label>
                <p class="note">Monsters of the group with an index from Min to Max come, up to Max Regen in each room; a Max Index or Max Regen of 0 brings none.</p>
              {:else if f.id === 'control'}
                <label class="line">
                  <span>Control Room</span>
                  <!-- svelte-ignore a11y_autofocus -->
                  <input type="number" class="num" bind:value={nums[0]} min="0" max={range('Control Room')[1]} step="1" autofocus />
                  <button type="button" class="btn sm ghost" onclick={() => (nums[0] = 0)}>None</button>
                </label>
                <p class="note">
                  {#if typeof nums[0] === 'number' && nums[0] > 0}{named(nums[0]) ?? `Map ${map} has no room ${nums[0]}.`}{:else}No control room.{/if}
                  Its Max Area caps how many monsters the rooms it controls bring between them.
                </p>
              {:else}
                <label class="line">
                  <span>Type</span>
                  <select bind:value={nums[0]}>
                    {#each Object.entries(ROOM_TYPE) as [v, label] (v)}<option value={Number(v)}>{label}</option>{/each}
                  </select>
                </label>
              {/if}
              {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
              <div class="acts">
                <button type="submit" class="btn sm primary" disabled={!!problem}>Set on {many}</button>
                <button type="button" class="btn sm ghost" onclick={() => (open = null)}>Cancel</button>
              </div>
            </form>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .drawer {
    position: absolute; z-index: 6; left: var(--edge); top: calc(var(--edge) + 50px);
    width: 300px; max-height: calc(100% - 2 * var(--edge) - 120px); overflow: auto;
    border-radius: 18px; padding: 8px; display: flex; flex-direction: column; gap: 8px;
    background: color-mix(in srgb, var(--surface) 92%, transparent); backdrop-filter: blur(8px);
    border: 1px solid var(--line-2); box-shadow: 0 2px 0 color-mix(in srgb, var(--shadow) 12%, transparent), var(--float);
  }
  .drawer > * { flex: none; }
  header { display: flex; align-items: center; gap: 8px; padding: 0 0 0 6px; }
  header b { font-size: 14px; }
  header .faint { flex: 1; font-size: 11.5px; font-weight: 700; }
  .ways { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 4px; }
  .way {
    position: relative; display: flex; flex-direction: column; align-items: center; gap: 3px;
    min-height: 0; padding: 8px 2px 6px; border-radius: 11px; border: 1px solid transparent;
    background: transparent; box-shadow: none; color: var(--ink-2);
  }
  .way:hover { background: var(--surface-3); }
  .way.on { background: var(--mint-2); border-color: var(--outline); color: var(--ink); }
  .way .name { font-size: 11px; font-weight: 800; }
  .way .key { position: absolute; top: 2px; left: 4px; font: 700 9.5px var(--font-mono); color: var(--ink-3); }
  .modes { display: flex; gap: 2px; padding: 3px; border-radius: 12px; background: var(--surface-2); border: 1px solid var(--line); }
  .modes button { flex: 1 1 0; min-height: 26px; padding: 0 6px; border: 0; border-radius: 9px; background: transparent; font-size: 12px; color: var(--ink-2); box-shadow: none; }
  .modes button:hover:not(.on) { background: var(--surface-3); }
  .modes button.on { background: var(--lilac-2); color: var(--lilac-ink); box-shadow: 0 0 0 1px var(--line-2); }
  .how { margin: 0 4px; font-size: 12px; line-height: 1.4; color: var(--ink-2); }
  .size { display: flex; align-items: center; gap: 6px; padding: 0 4px; font-size: 12.5px; font-weight: 700; }
  .size .n { min-width: 22px; text-align: center; font-weight: 800; }
  .count { display: flex; align-items: center; gap: 2px; padding: 6px 2px 0 6px; border-top: 1px dashed var(--line); }
  .count b { font-size: 13px; }
  .count .grow { flex: 1; }
  .count .btn { gap: 4px; padding: 0 7px; }
  .sets { display: flex; flex-direction: column; gap: 2px; }
  .sets .eyebrow { padding: 4px 6px 4px; }
  .setter { border-radius: 12px; border: 1px solid transparent; }
  .setter.open { background: var(--surface-2); border-color: var(--line); }
  .row {
    display: flex; align-items: center; gap: 8px; width: 100%; min-height: 34px; padding: 4px 8px;
    border: 0; border-radius: 11px; background: transparent; box-shadow: none; text-align: left; white-space: normal;
    color: var(--ink-3);
  }
  .row:hover:not(:disabled) { background: var(--surface-3); border-color: transparent; }
  .row .ic { width: 24px; height: 24px; border-radius: 8px; display: grid; place-items: center; flex: none; background: var(--surface-3); color: var(--ink-2); }
  .row .what { flex: none; width: 86px; font-size: 12.5px; font-weight: 800; color: var(--ink); }
  .row .value { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 700; color: var(--ink-2); overflow-wrap: anywhere; }
  .row .value.mixed { font-style: italic; color: var(--ink-3); }
  .edit { display: flex; flex-direction: column; gap: 8px; padding: 4px 8px 8px; }
  .edit input[type='text'] { width: 100%; }
  .line { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-size: 12.5px; font-weight: 800; }
  .line label { display: flex; align-items: center; gap: 6px; }
  .line select { flex: 1; min-width: 0; }
  .num { width: 76px; font-family: var(--font-mono); font-size: 13px; }
  .picks { display: flex; flex-wrap: wrap; gap: 4px; }
  .chip {
    min-height: 24px; padding: 0 8px; border-radius: 999px; font-size: 11.5px; font-weight: 800;
    background: var(--surface); border: 1px solid var(--line-2); box-shadow: none; gap: 5px;
  }
  .chip .tnum { color: var(--ink-3); font-family: var(--font-mono); font-size: 10.5px; }
  .chip.on { background: var(--lilac-2); color: var(--lilac-ink); border-color: var(--outline); }
  .note { margin: 0; font-size: 11.5px; line-height: 1.4; color: var(--ink-3); font-weight: 700; }
  .problem { margin: 0; font-size: 12px; font-weight: 800; color: var(--bad); }
  .acts { display: flex; gap: 6px; }
  @media (max-width: 700px) {
    .drawer { top: auto; bottom: calc(var(--edge) + 56px); left: var(--edge); right: var(--edge); width: auto; max-height: 46%; }
    .how { display: none; }
  }
</style>
