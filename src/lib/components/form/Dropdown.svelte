<script lang="ts">
  import { tick } from 'svelte';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Check from '@lucide/svelte/icons/check';

  let { label, options, value = $bindable(''), open = $bindable(false), disabled = false, onchange }: {
    label: string;
    options: Array<{ value: string; label: string; description?: string }>;
    value?: string;
    open?: boolean;
    disabled?: boolean;
    onchange?: (value: string) => void;
  } = $props();
  const id = $props.id();
  let root: HTMLDivElement | undefined = $state();
  let trigger: HTMLButtonElement | undefined = $state();
  let cursor = $state(0);
  let above = $state(false);
  const selected = $derived(options.find(option => option.value === value));

  async function reveal() {
    if (disabled) return;
    open = true;
    cursor = Math.max(0, options.findIndex(option => option.value === value));
    await tick();
    if (root) {
      const boundary = root.closest('.launcher')?.getBoundingClientRect().bottom ?? window.innerHeight;
      const height = root.querySelector<HTMLElement>('.options')?.offsetHeight ?? 0;
      above = boundary - root.getBoundingClientRect().bottom < height + 8;
    }
    root?.querySelector<HTMLElement>(`[data-index="${cursor}"]`)?.scrollIntoView({ block: 'nearest' });
  }
  function choose(index: number) {
    const option = options[index];
    if (!option) return;
    value = option.value;
    open = false;
    onchange?.(value);
    trigger?.focus();
  }
  async function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && open) { event.stopPropagation(); event.preventDefault(); open = false; return; }
    if (event.key === 'Tab') { open = false; return; }
    if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault(); event.stopPropagation();
      if (!open) { await reveal(); return; }
      cursor = event.key === 'Home' ? 0 : event.key === 'End' ? options.length - 1 :
        Math.max(0, Math.min(options.length - 1, cursor + (event.key === 'ArrowDown' ? 1 : -1)));
      await tick();
      root?.querySelector<HTMLElement>(`[data-index="${cursor}"]`)?.scrollIntoView({ block: 'nearest' });
    } else if (open && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault(); event.stopPropagation(); choose(cursor);
    }
  }
</script>

<svelte:window onpointerdown={event => { if (open && event.target instanceof Node && !root?.contains(event.target)) open = false; }} />
<div class="dropdown" bind:this={root} class:open class:disabled>
  <button bind:this={trigger} class="trigger" role="combobox" aria-label={label} aria-expanded={open}
    aria-controls={`${id}-list`} aria-activedescendant={open ? `${id}-${cursor}` : undefined}
    {disabled} title={selected?.label ?? label} onclick={() => open ? open = false : void reveal()} onkeydown={keydown}>
    <span>{selected?.label ?? label}</span><ChevronDown size={16} />
  </button>
  {#if open && !disabled}
    <div class="options" class:above id={`${id}-list`} role="listbox" aria-label={label}>
      {#each options as option, index (option.value)}
        <button id={`${id}-${index}`} data-index={index} class="option" class:highlight={cursor === index}
          role="option" aria-selected={option.value === value} tabindex="-1" title={option.label}
          onpointermove={() => cursor = index} onclick={() => choose(index)}>
          <span class="copy"><span class="option-label">{option.label}</span>{#if option.description}<small>{option.description}</small>{/if}</span>
          <span class="check">{#if option.value === value}<Check size={15} />{/if}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dropdown { position:relative; width:100%; min-width:0; max-width:100%; }
  button { color:var(--c-text-1); font:inherit; cursor:pointer; }
  .trigger { display:flex; align-items:center; justify-content:space-between; gap:var(--sp-4); width:100%; min-width:0; min-height:var(--sp-10); padding:var(--sp-4) var(--sp-5); border:1px solid var(--c-border-strong); border-radius:var(--r-control); background:var(--c-surface-3); text-align:left; }
  .trigger>span { min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .trigger :global(svg) { flex:none; color:var(--c-text-2); transition:transform 140ms ease; }
  .open .trigger { border-color:var(--c-accent); }
  .open .trigger :global(svg) { transform:rotate(180deg); color:var(--c-accent-bright); }
  .trigger:focus-visible { outline:2px solid var(--c-accent); outline-offset:var(--sp-1); }
  .disabled { opacity:.4; } .trigger:disabled { cursor:default; }
  .options { position:absolute; z-index:30; top:calc(100% + var(--sp-2)); left:0; width:100%; display:flex; flex-direction:column; gap:var(--sp-1); max-height:var(--dropdown-max-h); overflow-y:auto; overscroll-behavior:contain; padding:var(--sp-2); background:var(--c-surface-2); border:1px solid var(--c-border-strong); border-radius:var(--r-control); box-shadow:var(--shadow-peek); }
  .options.above { top:auto; bottom:calc(100% + var(--sp-2)); }
  .option { display:flex; align-items:center; gap:var(--sp-4); width:100%; min-width:0; flex:none; text-align:left; padding:var(--sp-4); border:0; border-radius:var(--sp-3); background:transparent; }
  .option.highlight { background:var(--c-surface-3); }
  .option[aria-selected=true] { color:var(--c-accent-bright); background:var(--c-accent-soft); }
  .copy { flex:1; min-width:0; } .option-label { display:block; overflow-wrap:anywhere; line-height:1.45; }
  small { display:block; font-size:var(--fs-caption); color:var(--c-text-3); margin-top:var(--sp-1); }
  .check { display:flex; flex:none; width:var(--sp-6); }
  @media (prefers-reduced-motion:reduce) { .trigger :global(svg) { transition:none; } }
</style>
