<script lang="ts">
  import { onDestroy } from 'svelte';
  import Repeat from '@lucide/svelte/icons/repeat';
  import Repeat1 from '@lucide/svelte/icons/repeat-1';
  import { inTauri, playerCommands, webPlayerCommands } from '$lib/ipc/commands';
  import { onWebPlayerState } from '$lib/ipc/events';
  import type { WebPlayerSnapshot } from '$lib/ipc/bindings/WebPlayerSnapshot';
  import { live } from '$lib/stores/live.svelte';
  import { t } from '$lib/i18n/index.svelte';

  let { source }: { source: string } = $props();
  type Mode = 'off' | 'one' | 'all';
  let web = $state<WebPlayerSnapshot | null>(null);
  let pending = $state<Mode | null>(null);
  let menu = $state(false);
  let error = $state(false);
  let pendingRev = $state(0);
  let timeout: ReturnType<typeof setTimeout> | undefined;
  const mode = $derived(source === 'local' ? live.player?.repeat ?? 'off' : web?.repeat ?? 'off');
  $effect(() => {
    if (source !== 'web' || !inTauri) return;
    let active = true;
    const update = (value: WebPlayerSnapshot) => {
      if (active && (!web || value.rev >= web.rev)) web = value;
    };
    const sub = onWebPlayerState(update);
    void sub.then(() => webPlayerCommands.getState()).then(update).catch(() => { if (active) error = true; });
    return () => { active = false; void sub.then((stop) => stop()); };
  });
  $effect(() => {
    if (pending && (mode === pending || (source === 'web' && web?.repeatError && web.rev > pendingRev))) {
      error = source === 'web' && !!web?.repeatError;
      pending = null;
      clearTimeout(timeout);
    }
  });
  onDestroy(() => clearTimeout(timeout));
  async function choose(event: MouseEvent, next: Mode) {
    event.stopPropagation();
    if (pending) return;
    menu = false;
    pendingRev = web?.rev ?? 0;
    pending = next; error = false;
    // The label changes only when the player acknowledges the new mode.
    timeout = setTimeout(() => { pending = null; error = true; }, 3000);
    try {
      if (source === 'local') await playerCommands.control(`repeat_${next}`);
      else await webPlayerCommands.setRepeat(next);
    } catch { pending = null; error = true; clearTimeout(timeout); }
  }
</script>

{#if source === 'local' || source === 'web'}
  <div class="repeat-control">
    <button class="repeat-toggle" class:active={mode !== 'off'} class:failed={error} disabled={pending !== null} aria-expanded={menu}
      aria-label={t('music.repeat.change', { mode: t(`music.repeat.${mode}`) })}
      title={t(error || web?.repeatError ? 'music.repeat.failed' : 'music.repeat.change', { mode: t(`music.repeat.${mode}`) })}
      onclick={(event) => { event.stopPropagation(); menu = !menu; }} ondblclick={(event) => event.stopPropagation()}>
      {#if mode === 'one'}<Repeat1 size={16} />{:else}<Repeat size={16} />{/if}
      <span>{t(`music.repeat.${mode}`)}</span>
    </button>
    {#if menu}
      <div class="options">
        {#each ['one', 'all', 'off'] as choice (choice)}
          <button data-mode={choice} class:active={mode === choice} aria-pressed={mode === choice} onclick={(event) => choose(event, choice as Mode)}>{t(`music.repeat.${choice}`)}</button>
        {/each}
      </div>
    {:else if error || web?.repeatError}<span class="error" role="status">{t('music.repeat.failed')}</span>{/if}
  </div>
{/if}

<style>
  .repeat-control { position: relative; }
  button { display: flex; align-items: center; gap: var(--sp-2); padding: var(--sp-2) var(--sp-3); border: 0; border-radius: var(--r-control); background: var(--c-surface-2); color: var(--c-text-2); font: inherit; font-size: var(--fs-caption); cursor: pointer; }
  button.active { color: var(--c-accent-bright); }
  button.failed { color: var(--c-warning); }
  button:disabled { opacity: 0.6; cursor: wait; }
  .options { position: absolute; bottom: calc(100% + var(--sp-2)); left: 0; padding: var(--sp-2); background: var(--c-surface-3); border: 1px solid var(--c-border-strong); border-radius: var(--r-control); z-index: 2; }
  .options button { width: 100%; white-space: nowrap; background: transparent; padding: var(--sp-3) var(--sp-4); }
  .options button:hover { background: var(--c-surface-2); }
  .error { position: absolute; bottom: 100%; left: 0; width: 220px; padding: var(--sp-3); background: var(--c-surface-3); color: var(--c-warning); border-radius: var(--r-control); font-size: var(--fs-caption); z-index: 1; }
</style>
