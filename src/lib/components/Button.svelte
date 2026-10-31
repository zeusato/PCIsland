<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    onclick,
    variant = 'default',
    disabled = false,
    children,
    title = '',
  }: { onclick?: (e: MouseEvent) => void; variant?: 'default' | 'primary' | 'ghost' | 'danger'; disabled?: boolean; children: Snippet; title?: string } = $props();
</script>

<button class={variant} {disabled} {onclick} {title}>{@render children()}</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-3);
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    padding: var(--sp-3) var(--sp-5);
    cursor: pointer;
    white-space: nowrap;
  }
  button:hover:not(:disabled) {
    border-color: var(--c-accent);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .primary {
    background: var(--c-accent);
    border-color: var(--c-accent);
    color: #000;
    font-weight: var(--fw-semibold);
  }
  .ghost {
    background: transparent;
    border-color: transparent;
  }
  .ghost:hover:not(:disabled) {
    background: var(--c-surface-3);
    border-color: transparent;
  }
  .danger:hover:not(:disabled) {
    border-color: var(--c-danger);
    color: var(--c-danger);
  }
</style>
