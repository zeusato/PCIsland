<script lang="ts">
  import { live } from '$lib/stores/live.svelte';
  import { inTauri, playerCommands, webPlayerCommands } from '$lib/ipc/commands';
  import { onWebPlayerState } from '$lib/ipc/events';
  import type { WebPlayerSnapshot } from '$lib/ipc/bindings/WebPlayerSnapshot';
  import { t } from '$lib/i18n/index.svelte';
  import { islandInteraction } from '$lib/components/islandInteraction';
  const { fit } = islandInteraction();
  let { source, close, expand = false }: { source: string; close: () => void; expand?: boolean } = $props();
  let web = $state<WebPlayerSnapshot | null>(null);
  let error = $state(false);
  $effect(() => {
    if (!inTauri || source !== 'web') return;
    void webPlayerCommands.requestQueue().catch(() => error = true);
    void webPlayerCommands.getState().then((value) => web = value).catch(() => error = true);
    const sub = onWebPlayerState((value) => { if (!web || value.rev >= web.rev) web = value; });
    return () => void sub.then((stop) => stop());
  });
  // Web sites expose no queue index; the last saved track of the site marks the current row.
  const webCurrent = $derived(web?.history.find((item) => item.siteId === web?.siteId)?.trackUrl ?? null);
  async function select(index: number) {
    error = false;
    try {
      if (source === 'local' && live.player) await playerCommands.playTracks(live.player.queue.map((item) => item.id), index);
      else if (web?.queue[index]) await webPlayerCommands.selectQueue(web.queue[index].url);
    } catch { error = true; }
  }
</script>

<div class="queue" use:fit={expand}>
  <button class="back" onclick={(event) => { event.stopPropagation(); close(); }}>{t('common.back')} · {t('music.queue')}</button>
  <div class="tracks">
    {#each (source === 'local' ? live.player?.queue ?? [] : web?.queue ?? []) as item, index (index)}
      <button class:current={source === 'local' ? live.player?.index === index : 'url' in item && item.url === webCurrent} onclick={(event) => { event.stopPropagation(); void select(index); }}>{index + 1}. {item.title}</button>
    {:else}<p>{t('music.queueUnavailable')}</p>{/each}
  </div>
  {#if error}<p role="alert">{t('errors.unknown')}</p>{/if}
</div>

<style>
  .queue { align-self: start; width: 100%; min-height: 0; display: flex; flex-direction: column; padding: var(--sp-4) var(--sp-6); gap: var(--sp-3); }
  .tracks { max-height: var(--island-list-max-h); overflow-y: auto; }
  button { display: block; width: 100%; text-align: left; border: 0; border-radius: var(--r-control); background: var(--c-surface-2); color: var(--c-text-1); padding: var(--sp-3) var(--sp-4); margin-bottom: var(--sp-2); font-size: var(--fs-small); cursor: pointer; }
  .current { color: var(--c-accent-bright); }
  p { font-size: var(--fs-small); color: var(--c-text-2); }
</style>
