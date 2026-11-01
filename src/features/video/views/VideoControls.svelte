<script lang="ts">
  // Island controls of the floating video, shown in the transport row (docs/modules/
  // video-player.md §3 step 7). Never focuses the video window unless asked to show it.
  import { onMount } from 'svelte';
  import Volume2 from '@lucide/svelte/icons/volume-2';
  import VolumeX from '@lucide/svelte/icons/volume-x';
  import AppWindow from '@lucide/svelte/icons/app-window';
  import X from '@lucide/svelte/icons/x';
  import { inTauri } from '$lib/ipc/commands';
  import { onVideoState, videoCommands } from '$lib/ipc/video';
  import { t } from '$lib/i18n/index.svelte';

  let muted = $state(false);
  onMount(() => {
    if (!inTauri) return;
    void videoCommands.state().then((s) => (muted = s.muted)).catch(() => {});
    const sub = onVideoState((s) => (muted = s.muted));
    return () => void sub.then((u) => u());
  });
  const run = (e: MouseEvent, f: () => Promise<void>) => {
    e.stopPropagation();
    void f().catch(() => {});
  };
</script>

<div class="extras">
  <button aria-label={muted ? t('video.unmute') : t('video.mute')} title={muted ? t('video.unmute') : t('video.mute')}
    onclick={(e) => run(e, () => videoCommands.control('mute'))}>
    {#if muted}<VolumeX size={14} />{:else}<Volume2 size={14} />{/if}
  </button>
  <button aria-label={t('video.showWindow')} title={t('video.showWindow')} onclick={(e) => run(e, () => videoCommands.window('show'))}>
    <AppWindow size={14} />
  </button>
  <button aria-label={t('video.close')} title={t('video.close')} onclick={(e) => run(e, () => videoCommands.window('close'))}>
    <X size={14} />
  </button>
</div>

<style>
  /* Right end of the media transport row (Peek `trailing`). */
  .extras {
    display: flex;
    gap: var(--sp-2);
  }
  button {
    display: grid;
    place-items: center;
    padding: var(--sp-2) var(--sp-3);
    border: 0;
    border-radius: var(--r-control);
    background: var(--c-surface-2);
    color: var(--c-text-2);
    cursor: pointer;
  }
  button:hover {
    color: var(--c-text-1);
  }
</style>
