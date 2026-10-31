<script lang="ts">
  import type { Snippet } from 'svelte';
  let { label, hint = '', children }: { label: string; hint?: string; children: Snippet } = $props();
</script>

<div class="row">
  <div class="text">
    <div class="label">{label}</div>
    {#if hint}<div class="hint">{hint}</div>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-6);
    padding: var(--sp-5) 0;
    border-bottom: 1px solid var(--c-border);
  }
  .label {
    font-weight: var(--fw-medium);
  }
  .hint {
    color: var(--c-text-3);
    font-size: var(--fs-small);
    margin-top: var(--sp-1);
  }
  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--sp-4);
  }
</style>
