<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import { t } from '$lib/i18n/index.svelte';
  let { message, action, busy, onconfirm, oncancel }: { message: string; action: string; busy: boolean; onconfirm: () => void; oncancel: () => void } = $props();
</script>

<div class="confirm" role="alertdialog" aria-label={message}>
  <p>{message}</p>
  <div class="actions">
    <Button variant="danger" disabled={busy} onclick={onconfirm}>{action}</Button>
    <Button variant="ghost" disabled={busy} onclick={oncancel}>{t('common.cancel')}</Button>
  </div>
</div>

<style>
  .confirm { display: grid; gap: var(--sp-4); padding: var(--sp-4); border: 1px solid var(--c-danger); border-radius: var(--r-control); }
  p { margin: 0; color: var(--c-text-1); line-height: 1.5; }
  .actions { display: flex; flex-wrap: wrap; gap: var(--sp-4); }
</style>
