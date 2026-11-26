<!-- This browser's choices: the theme and the little sounds, and the licenses. -->
<script lang="ts">
  import { Popover } from 'bits-ui';
  import Check from '@lucide/svelte/icons/check';
  import FileText from '@lucide/svelte/icons/file-text';
  import PaletteIcon from '@lucide/svelte/icons/palette';
  import Volume2 from '@lucide/svelte/icons/volume-2';
  import { THEMES, prefs, setSound, setTheme } from './prefs.svelte';
</script>

<Popover.Root>
  <Popover.Trigger class="btn icon ghost" title="Theme and sounds" aria-label="Theme and sounds"><PaletteIcon size={18} /></Popover.Trigger>
  <Popover.Portal>
    <Popover.Content class="pop theme-pop" sideOffset={8} align="end" collisionPadding={8}>
      <div class="eyebrow head">Theme</div>
      <div role="radiogroup" aria-label="Theme">
        {#each THEMES as t (t.id)}
          {@const on = prefs.theme === t.id}
          <button type="button" class="menu-item" class:on role="radio" aria-checked={on} onclick={() => setTheme(t.id)}>
            <span class="swatches">{#each t.swatches as s (s)}<i style:background={s}></i>{/each}</span>
            <span>{t.name}<span class="sub">{t.sub}</span></span>
            {#if on}<span class="end"><Check size={16} /></span>{/if}
          </button>
        {/each}
      </div>
      <div class="menu-sep"></div>
      <label class="menu-item">
        <Volume2 size={16} />
        <span>Little sounds<span class="sub">Soft pops when you save, undo and publish</span></span>
        <span class="end"><input class="switch" type="checkbox" role="switch" checked={prefs.sound} onchange={(e) => setSound(e.currentTarget.checked)} /></span>
      </label>
      <div class="menu-sep"></div>
      <a class="menu-item" href="/notices.txt" target="_blank" rel="noopener">
        <FileText size={16} />
        <span>Licenses<span class="sub">The open-source parts Daydream is built from</span></span>
      </a>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.theme-pop) { width: min(320px, calc(100vw - 16px)); }
  .head { padding: 8px 10px 6px; }
  .switch {
    appearance: none; width: 36px; height: 22px; border-radius: 999px; padding: 0; margin: 0; cursor: pointer;
    background: var(--surface-3); border: 1.5px solid var(--line-2); position: relative; transition: background .15s;
  }
  .switch::after {
    content: ''; position: absolute; top: 2px; left: 2px; width: 15px; height: 15px; border-radius: 50%;
    background: var(--surface); box-shadow: 0 1px 2px var(--shadow); transition: transform .2s var(--pop);
  }
  .switch:checked { background: var(--mint); border-color: var(--outline); }
  .switch:checked::after { transform: translateX(14px); }
</style>
