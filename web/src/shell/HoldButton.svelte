<!--
  A button that acts only when held down until it fills (about a second), so a stray click
  never publishes. Enter or Space acts at once: a keyboard press is deliberate already, and a
  screen reader's press is a keyboard press.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    onfire,
    onshort,
    disabled = false,
    ms = 1100,
    describedby,
    children,
  }: {
    onfire: () => void;
    /** Let go before it filled. */
    onshort?: () => void;
    disabled?: boolean;
    ms?: number;
    describedby?: string;
    children: Snippet;
  } = $props();

  let holding = $state(false);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function down(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    holding = true;
    timer = setTimeout(() => {
      timer = null;
      holding = false;
      onfire();
    }, ms);
  }

  function up() {
    if (!timer) return;
    clearTimeout(timer);
    timer = null;
    holding = false;
    onshort?.();
  }

  function leave() {
    if (!timer) return;
    clearTimeout(timer);
    timer = null;
    holding = false;
  }

  function click(e: MouseEvent) {
    // no pointer behind it: Enter, Space or a screen reader
    if (e.detail === 0 && !disabled) onfire();
  }
</script>

<button
  type="button"
  class="btn rainbow hold"
  class:holding
  {disabled}
  style:--hold="{ms}ms"
  aria-describedby={describedby}
  onpointerdown={down}
  onpointerup={up}
  onpointerleave={leave}
  onpointercancel={leave}
  onclick={click}
  oncontextmenu={(e) => e.preventDefault()}
>
  <span class="fill" aria-hidden="true"></span>
  {@render children()}
</button>

<style>
  .hold {
    position: relative; overflow: hidden; min-height: 52px; padding: 0 22px; border-radius: 16px;
    font-size: 15.5px; touch-action: none; user-select: none; -webkit-user-select: none; -webkit-touch-callout: none;
  }
  .fill {
    position: absolute; inset: 0; transform-origin: left; transform: scaleX(0); pointer-events: none;
    background: color-mix(in srgb, var(--outline) 18%, transparent);
  }
  .holding .fill { transform: scaleX(1); transition: transform var(--hold) linear; }
  .hold > :global(:not(.fill)) { position: relative; }
</style>
