<!-- A table of counts, a row for each table or file: how many records it gains, changes or loses. -->
<svelte:options css="injected" />

<script lang="ts">
  let { heads, rows }: { heads: string[]; rows: (string | number)[][] } = $props();

  /** A column of counts sits to the right, head and all. */
  const counts = $derived(heads.map((_, j) => rows.length > 0 && rows.every((r) => typeof r[j] === 'number')));
</script>

<div class="wrap">
  <table>
    <thead>
      <tr>{#each heads as h, i (i)}<th class:num={counts[i]}>{h}</th>{/each}</tr>
    </thead>
    <tbody>
      {#each rows as r, i (i)}
        <tr>
          {#each r as c, j (j)}
            <td class:num={counts[j]} class:zero={c === 0}>{typeof c === 'number' ? c.toLocaleString() : c}</td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  /* as wide as its columns need, so a count sits near its table's name */
  .wrap { width: fit-content; min-width: min(100%, 360px); max-width: 100%; overflow: auto; max-height: min(60vh, 520px); border: 1px solid var(--line); border-radius: 14px; }
  table { width: 100%; border-collapse: collapse; font-size: 13px; }
  th { position: sticky; top: 0; background: var(--surface); text-align: left; font: 800 11px/1.2 var(--font-ui); letter-spacing: .05em; text-transform: uppercase; color: var(--ink-3); padding: 8px 14px; border-bottom: 1px solid var(--line); white-space: nowrap; }
  td { padding: 5px 14px; border-top: 1px dashed var(--line); font-weight: 700; }
  tbody tr:first-child td { border-top: 0; }
  .num { text-align: right; font-family: var(--font-mono); white-space: nowrap; }
  td.zero { color: var(--ink-3); font-weight: 600; }
</style>
