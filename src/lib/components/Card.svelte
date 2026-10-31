<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title = '', children, actions }: { title?: string; children: Snippet; actions?: Snippet } = $props();
</script>

<section class="card">
  {#if title || actions}
    <header>
      {#if title}<h2>{title}</h2>{/if}
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </header>
  {/if}
  {@render children()}
</section>

<style>
  .card {
    background: var(--c-surface-2);
    border-radius: var(--r-card);
    padding: var(--sp-7);
    margin-bottom: var(--sp-6);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--sp-5);
    gap: var(--sp-5);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-title-s);
    font-weight: var(--fw-semibold);
  }
  .actions {
    display: flex;
    gap: var(--sp-4);
  }
</style>
