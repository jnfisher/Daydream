<!-- A file to bring in: dropped on it, or picked. It shows the one picked, and its size. -->
<svelte:options css="injected" />

<script lang="ts">
  import FileUp from '@lucide/svelte/icons/file-up';
  import X from '@lucide/svelte/icons/x';

  interface Props {
    file: File | null;
    /** The endings it takes: `.mdb,.json`. */
    accept: string;
    /** What the file is: "NMR export". */
    label: string;
    /** Where it comes from. */
    hint?: string;
  }

  let { file = $bindable(), accept, label, hint = '' }: Props = $props();

  let input = $state<HTMLInputElement | undefined>();
  let over = $state(false);

  const endings = $derived(accept.split(',').map((a) => a.trim().toLowerCase()));
  const fits = (f: File) => endings.some((e) => f.name.toLowerCase().endsWith(e));
  const size = (n: number) =>
    n < 1024 ? `${n} bytes` : n < 1024 * 1024 ? `${Math.round(n / 1024).toLocaleString()} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;

  function drop(e: DragEvent) {
    e.preventDefault();
    over = false;
    const f = e.dataTransfer?.files[0];
    if (f) file = f;
  }
</script>

<div
  class="drop"
  class:over
  class:has={!!file}
  role="group"
  aria-label={label}
  ondragenter={(e) => {
    e.preventDefault();
    over = true;
  }}
  ondragover={(e) => e.preventDefault()}
  ondragleave={(e) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node | null)) over = false;
  }}
  ondrop={drop}
>
  <span class="ic"><FileUp size={20} /></span>
  {#if file}
    <span class="what">
      <b>{file.name}</b>
      <small class:bad={!fits(file)}>{size(file.size)}{fits(file) ? '' : `: not ${endings.join(' or ')}, so it may not read`}</small>
    </span>
    <button type="button" class="btn sm" onclick={() => input?.click()}>Pick another</button>
    <button type="button" class="btn sm icon" aria-label="Forget {file.name}" title="Forget it" onclick={() => (file = null)}><X size={14} /></button>
  {:else}
    <span class="what">
      <b>{label}</b>
      <small>{hint ? `${hint}. ` : ''}Drop it here, or pick it.</small>
    </span>
    <button type="button" class="btn sm sticker" onclick={() => input?.click()}>Pick a file</button>
  {/if}
  <input
    bind:this={input}
    type="file"
    {accept}
    hidden
    onchange={(e) => {
      const f = e.currentTarget.files?.[0];
      e.currentTarget.value = '';
      if (f) file = f;
    }}
  />
</div>

<style>
  .drop {
    display: flex; align-items: center; gap: 12px; flex-wrap: wrap; padding: 12px 14px; border-radius: 16px;
    border: 2px dashed var(--line-2); background: var(--surface-2); transition: border-color .15s, background .15s;
  }
  .drop.has { border-style: solid; border-color: var(--line); background: var(--surface); }
  .drop.over { border-color: var(--outline); background: var(--sky-2); }
  .ic { width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center; flex: none; background: var(--sky-2); color: var(--sky-ink); }
  .what { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 200px; }
  .what b { font: 600 15px/1.2 var(--font-display); overflow-wrap: anywhere; }
  .what small { font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  .what small.bad { color: var(--bad); }
</style>
