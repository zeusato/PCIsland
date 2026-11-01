<script lang="ts">
  // Paste a link → play it in the floating video window (docs/modules/video-player.md §3).
  // Used by the island launcher (with `island`) and the full window's Video page.
  import { onMount, tick } from 'svelte';
  import ClipboardPaste from '@lucide/svelte/icons/clipboard-paste';
  import Play from '@lucide/svelte/icons/play';
  import { inTauri } from '$lib/ipc/commands';
  import { holdFocus } from '$lib/ipc/focus';
  import { videoCommands } from '$lib/ipc/video';
  import type { SniffedStream } from '$lib/ipc/bindings/SniffedStream';
  import { t } from '$lib/i18n/index.svelte';

  let { island = false, onplaying }: { island?: boolean; onplaying?: () => void } = $props();
  let url = $state('');
  let input: HTMLInputElement | undefined = $state();
  let busy = $state(false);
  let invalid = $state(false);
  let error = $state<string | null>(null);
  /** The failed link can still play through the site's embed (fallback A). */
  let canEmbed = $state(false);
  let needsTools = $state(false);
  let streams = $state<SniffedStream[]>([]);

  $effect(() => {
    if (!island) return;
    const focus = holdFocus();
    void focus.ready.then(() => input?.focus());
    return focus.release;
  });
  onMount(async () => {
    if (!inTauri) return;
    const copied = await videoCommands.clipboard().catch(() => null);
    if (copied && !url) url = copied;
    await tick();
    input?.select();
  });
  // Validate while typing (debounced) so a bad link never opens a window.
  $effect(() => {
    const value = url.trim();
    if (!value || !inTauri) {
      invalid = false;
      return;
    }
    const id = setTimeout(async () => {
      invalid = (await videoCommands.classify(value).catch(() => null)) === null;
    }, 200);
    return () => clearTimeout(id);
  });

  const messages: Record<string, string> = {
    'video.unsupported_link': 'video.error.unsupported',
    'video.resolve_failed': 'video.error.resolve',
    'video.no_stream': 'video.error.noStream',
    'video.sniff_none': 'video.error.sniffNone',
    'video.sniff_blob_only': 'video.error.sniffBlob',
    'video.sniff_busy': 'video.error.busy',
    'video.tools_install': 'video.error.install',
  };

  async function play(embed = false) {
    const value = url.trim();
    if (busy || !value || invalid) return;
    busy = true;
    error = null;
    canEmbed = false;
    needsTools = false;
    streams = [];
    try {
      const result = await videoCommands.open(value, embed);
      if (result.result === 'needsTools') needsTools = true;
      else if (result.result === 'choose') streams = result.streams;
      else onplaying?.();
    } catch (e) {
      const code = (e as { code?: string })?.code ?? '';
      error = t(messages[code] ?? 'errors.unknown');
      canEmbed = code === 'video.resolve_failed' || code === 'video.no_stream';
    } finally {
      busy = false;
    }
  }
  async function installTools() {
    busy = true;
    error = null;
    try {
      await videoCommands.installTools();
      needsTools = false;
      busy = false;
      await play();
    } catch {
      error = t('video.error.install');
    } finally {
      busy = false;
    }
  }
  async function pick(id: string) {
    busy = true;
    try {
      await videoCommands.openStream(id);
      streams = [];
      onplaying?.();
    } catch {
      error = t('errors.unknown');
    } finally {
      busy = false;
    }
  }
  async function paste() {
    const copied = await videoCommands.clipboard().catch(() => null);
    if (copied) url = copied;
    else error = t('video.clipboardEmpty');
    input?.focus();
  }
</script>

<div class="panel">
  <div class="row">
    <input
      bind:this={input}
      bind:value={url}
      placeholder={t('video.placeholder')}
      aria-label={t('video.placeholder')}
      aria-invalid={invalid}
      disabled={busy}
      onkeydown={(e) => {
        e.stopPropagation();
        if (e.key === 'Enter') void play();
      }}
    />
    <button class="icon" title={t('video.paste')} aria-label={t('video.paste')} disabled={busy} onclick={paste}><ClipboardPaste size={16} /></button>
    <button class="primary" disabled={busy || !url.trim() || invalid} onclick={() => void play()}>
      <Play size={14} />{busy ? t('video.resolving') : t('video.play')}
    </button>
  </div>
  {#if invalid}<p class="hint warn">{t('video.error.unsupported')}</p>{/if}
  {#if needsTools}
    <div class="box" role="dialog" aria-label={t('video.tools.title')}>
      <p>{t('video.tools.ask')}</p>
      <div class="row">
        <button class="primary" disabled={busy} onclick={installTools}>{busy ? t('video.tools.installing') : t('video.tools.install')}</button>
        <button disabled={busy} onclick={() => void play(true)}>{t('video.useEmbed')}</button>
      </div>
    </div>
  {/if}
  {#if streams.length}
    <p class="hint">{t('video.chooseStream')}</p>
    <div class="streams">
      {#each streams as s (s.id)}
        <button disabled={busy} onclick={() => void pick(s.id)} title={s.label}>{s.hls ? 'HLS' : t('video.file')} · {s.label || s.id}</button>
      {/each}
    </div>
  {/if}
  {#if error}
    <p class="hint warn" role="alert">{error}</p>
    {#if canEmbed}<button disabled={busy} onclick={() => void play(true)}>{t('video.useEmbed')}</button>{/if}
  {/if}
  {#if !island}<p class="hint">{t('video.hint')}</p>{/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-width: 0;
  }
  .row {
    display: flex;
    gap: var(--sp-2);
    align-items: center;
  }
  input {
    flex: 1;
    min-width: 0;
    font: inherit;
    font-size: var(--fs-small);
    color: var(--c-text-1);
    background: var(--c-surface-2);
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    padding: var(--sp-3);
  }
  input[aria-invalid='true'] {
    border-color: var(--c-warning);
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    padding: var(--sp-3) var(--sp-4);
    background: var(--c-surface-2);
    color: var(--c-text-1);
    font-size: var(--fs-small);
    cursor: pointer;
    white-space: nowrap;
  }
  button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .icon {
    padding: var(--sp-3);
  }
  .primary {
    background: var(--c-accent-soft);
    border-color: var(--c-accent);
    color: var(--c-accent-bright);
  }
  .hint {
    margin: 0;
    color: var(--c-text-2);
    font-size: var(--fs-caption);
  }
  .warn {
    color: var(--c-warning);
  }
  .box {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-1);
    font-size: var(--fs-small);
  }
  .box p {
    margin: 0;
  }
  .streams {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    max-height: var(--island-list-max-h);
    overflow-y: auto;
  }
  .streams button {
    justify-content: flex-start;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
