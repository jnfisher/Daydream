<!--
  Reverie, Daydream's unicorn: white, with a rainbow mane. Dreaming by default; happy to see
  you, delighted when something lands, worried when something is wrong. A placeholder until
  the finished art exists.
-->
<script lang="ts">
  let { mood = 'dream', size = 40 }: { mood?: 'dream' | 'happy' | 'joy' | 'worry'; size?: number } = $props();

  const ink = '#2E2650';
  const sparkle = (x: number, y: number, s: number) =>
    `M${x} ${y - s}Q${x} ${y} ${x + s} ${y}Q${x} ${y} ${x} ${y + s}Q${x} ${y} ${x - s} ${y}Q${x} ${y} ${x} ${y - s}Z`;
</script>

<svg class="mascot" width={size} height={size} viewBox="0 0 120 120" role="img" aria-label="Reverie, Daydream's unicorn">
  <g stroke-linejoin="round">
    <path d="M42 31 C37 22 37 13 41 7 C48 11 54 18 56 27 Z" fill="#fff" stroke={ink} stroke-width="3" />
    <path d="M44 27 C41 21 41 16 42.5 12 C47 15 50.5 19 52 25 Z" fill="#FFB8D9" />
    <path d="M56 30 L89 1.5 L72.5 27.5 Z" fill="#FFE08A" stroke={ink} stroke-width="3" />
    <path d="M61.6 25.4 L68.4 27.4 M66.6 20.2 L72.8 22.6 M71.6 15 L76.8 17.6 M76.6 9.8 L80.6 12" stroke={ink} stroke-width="1.9" stroke-linecap="round" />
    <path d="M52 24 C66 22 78 32 84 44 C88 52 96 58 100 66 C104 74 100 84 92 86 C84 88 76 86 70 82 C66 88 64 96 66 108 L34 108 C32 92 30 76 30 62 C30 40 38 26 52 24 Z" fill="#fff" stroke={ink} stroke-width="3" />
    <path d="M36 104 C35 92 34 80 35 68 C40 80 46 92 58 100 L60 104 Z" fill="#ECE7FF" />
    <circle cx="29" cy="97" r="9.5" fill="#FFE08A" stroke={ink} stroke-width="2.6" />
    <circle cx="26" cy="81" r="11" fill="#8FE6C1" stroke={ink} stroke-width="2.6" />
    <circle cx="27.5" cy="64" r="11" fill="#8FD3FF" stroke={ink} stroke-width="2.6" />
    <circle cx="31.5" cy="47.5" r="11" fill="#C4AEFF" stroke={ink} stroke-width="2.6" />
    <circle cx="39.5" cy="33" r="10.5" fill="#FF9BCB" stroke={ink} stroke-width="2.6" />
    <circle cx="51" cy="30.5" r="7" fill="#FF9BCB" stroke={ink} stroke-width="2.6" />
    {#if mood === 'happy'}
      <circle cx="76" cy="50" r="4.6" fill={ink} /><circle cx="77.6" cy="48.4" r="1.6" fill="#fff" />
    {:else if mood === 'joy'}
      <path d="M70 52 Q76 44 82 52" fill="none" stroke={ink} stroke-width="3" stroke-linecap="round" />
    {:else if mood === 'worry'}
      <circle cx="76" cy="51" r="4.2" fill={ink} /><circle cx="77.3" cy="49.6" r="1.4" fill="#fff" />
      <path d="M69 42 Q75 39.5 81 43" fill="none" stroke={ink} stroke-width="2.4" stroke-linecap="round" />
    {:else}
      <path d="M70 49 Q76 55.5 82 49" fill="none" stroke={ink} stroke-width="3" stroke-linecap="round" />
      <path d="M71.5 52.5 L68.5 55.5 M75.5 54.2 L74.6 58.2" stroke={ink} stroke-width="2.2" stroke-linecap="round" />
    {/if}
    <ellipse cx="81" cy="63.5" rx="6.2" ry="3.6" fill="#FF8FC4" opacity=".55" />
    <ellipse cx="94" cy="72.5" rx="1.7" ry="2.3" fill={ink} />
    {#if mood === 'worry'}
      <path d="M86 82 Q90 80 94.5 81.5" fill="none" stroke={ink} stroke-width="2.2" stroke-linecap="round" />
    {:else}
      <path d="M85.5 80 Q90 82.5 94.5 79" fill="none" stroke={ink} stroke-width="2.2" stroke-linecap="round" />
      <path d={sparkle(99, 14, 6)} fill="#fff" stroke={ink} stroke-width="1.6" />
      <path d={sparkle(108, 29, 4)} fill="#FFE08A" stroke={ink} stroke-width="1.6" />
      {#if mood === 'joy'}
        <path d={sparkle(14, 18, 5)} fill="#FF9BCB" stroke={ink} stroke-width="1.6" />
        <path d={sparkle(108, 52, 4.5)} fill="#8FD3FF" stroke={ink} stroke-width="1.6" />
      {/if}
    {/if}
  </g>
</svg>
