<script lang="ts">
  // Peek content when nothing is going on (docs/modules/island-window.md §4).
  import type { Component } from 'svelte';
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import { t, getLocale } from '$lib/i18n/index.svelte';
  import { quickActions, type QuickAction } from '$lib/registry/quickActions.svelte';
  import { islandInteraction } from '$lib/components/islandInteraction';
  import { live } from '$lib/stores/live.svelte';
  const { fit } = islandInteraction();

  let { initial = null, onclose, onopen }: { initial?: string | null; onclose?: () => void; onopen?: () => void } = $props();
  let panel = $state<{ action: QuickAction; component: Component<{ close: () => void }> } | null>(null);

  // Open a panel directly (hotkey "quickNote").
  $effect(() => {
    const a = quickActions().find((x) => x.id === initial);
    if (a?.panel) panel = { action: a, component: a.panel };
  });

  function closePanel() {
    panel = null;
    onclose?.();
  }
  let now = $state(new Date());

  $effect(() => {
    const id = setTimeout(() => (now = new Date()), 60_000 - (Date.now() % 60_000));
    return () => clearTimeout(id);
  });

  const date = $derived(
    now.toLocaleDateString(getLocale() === 'vi' ? 'vi-VN' : 'en-GB', {
      weekday: 'short',
      day: '2-digit',
      month: '2-digit',
    }),
  );
  const time = $derived(now.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' }));

  // Hidden tiles only leave the grid; hotkeys can still open their panels.
  const visible = $derived(quickActions().filter((a) => !live.settings?.island.hiddenActions?.includes(a.id)));

  function choose(action: QuickAction) {
    if (action.panel) { panel = { action, component: action.panel }; onopen?.(); }
    else action.run?.();
  }
</script>

<div class="launcher" use:fit>
  <div class="header">
    {#if panel}
      <button class="back" aria-label={t('common.back')} onclick={(e) => { e.stopPropagation(); closePanel(); }}><ChevronLeft size={16} /></button>
      <span class="title">{t(panel.action.labelKey)}</span>
    {:else}
      <span class="title tabular">{time}</span>
      <span class="date">{date}</span>
    {/if}
  </div>

  {#if panel}
    <panel.component close={closePanel} />
  {:else}
    <div class="grid">
      {#each visible as action (action.id)}
        <!-- Status stays available on hover; tiles keep one label line and one size. -->
        <button class="action" title={action.status ? `${t(action.labelKey)} · ${action.status()}` : t(action.labelKey)} data-state={action.state?.() ?? undefined} aria-pressed={action.state ? !!action.state() : undefined} disabled={action.busy?.() ?? false} onclick={(e) => { e.stopPropagation(); choose(action); }} ondblclick={(event) => event.stopPropagation()}>
          <action.icon size={18} />
          <span class="label">{t(action.labelKey)}</span>
        </button>
      {/each}
    </div>
    <span class="signature" class:still={live.settings?.island.reduceMotion === 'on'}>{t('island.launcher.signature')}</span>
  {/if}
</div>

<style>
  .launcher {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    padding: var(--sp-6) var(--sp-7);
    align-self: start;
    width: 100%;
    max-height: var(--island-panel-max-h);
    overflow-y: auto;
    min-height: 0;
  }
  .header {
    display: flex;
    align-items: baseline;
    gap: var(--sp-4);
  }
  .signature {
    align-self: flex-end;
    padding-right: var(--sp-2);
    color: var(--c-signature);
    font-family: var(--font-display);
    font-size: var(--fs-small);
    font-weight: var(--fw-regular);
    font-style: italic;
    letter-spacing: var(--signature-tracking);
    line-height: var(--sp-6);
    text-shadow: var(--glow-signature);
    opacity: 1;
    pointer-events: none;
    animation: signature-glow 3s ease-in-out 2;
  }
  @keyframes signature-glow {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.8; }
  }
  .signature.still { animation: none; }
  @media (prefers-reduced-motion: reduce) {
    .signature { animation: none; }
  }
  .title {
    font-size: var(--fs-title-s);
    font-weight: var(--fw-semibold);
  }
  .date {
    color: var(--c-text-2);
    font-size: var(--fs-small);
  }
  .back {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 50%;
    background: var(--c-surface-3);
    cursor: pointer;
    align-self: center;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    grid-auto-rows: 1fr;
    gap: var(--sp-4);
  }
  .action {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-2);
    min-width: 0;
    padding: var(--sp-4) var(--sp-2);
    border: 0;
    border-radius: var(--r-control);
    background: var(--c-surface-2);
    font-size: var(--fs-caption);
    color: var(--c-text-2);
    cursor: pointer;
  }
  .action:hover {
    background: var(--c-surface-3);
    color: var(--c-accent-bright);
  }
  .action[data-state='active'] { color: var(--c-accent-bright); background: var(--c-accent-soft); box-shadow: inset 0 0 0 1px var(--c-accent); }
  .action[data-state='break'] { color: var(--c-success); box-shadow: inset 0 0 0 1px var(--c-success); }
  .action[data-state='paused'] { color: var(--c-warning); box-shadow: inset 0 0 0 1px var(--c-warning); }
  .label { max-width: 100%; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .action:disabled { opacity: 0.5; cursor: default; }
</style>
