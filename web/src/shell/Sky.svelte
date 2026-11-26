<!-- Clouds by day and stars by night behind a friendly header; decoration only. -->
<script lang="ts">
  let { clouds = 3, stars = 18, seed = 5 }: { clouds?: number; stars?: number; seed?: number } = $props();

  // the same sky every time for the same seed
  const at = (i: number) => {
    const x = Math.sin(i * 99.13 + seed) * 10000;
    return x - Math.floor(x);
  };
</script>

<div class="sky-art" aria-hidden="true">
  {#each { length: clouds }, i (i)}
    <svg class="cloud" viewBox="0 0 100 50" style:left="{8 + at(i) * 80}%" style:top="{10 + at(i + 7) * 55}%" style:width="{70 + at(i + 3) * 90}px" style:opacity={0.55 + at(i + 5) * 0.4}>
      <circle cx="30" cy="32" r="15" /><circle cx="51" cy="23" r="20" /><circle cx="73" cy="31" r="14" /><rect x="15" y="30" width="72" height="18" rx="9" />
    </svg>
  {/each}
  {#each { length: stars }, i (i)}
    <i class="star" style:left="{at(i + 20) * 100}%" style:top="{at(i + 40) * 100}%" style:animation-delay="{(at(i + 60) * 3).toFixed(2)}s"></i>
  {/each}
</div>

<style>
  .sky-art { position: absolute; inset: 0; pointer-events: none; }
</style>
