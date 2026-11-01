<script lang="ts">
  import { timerCommands } from '$lib/ipc/commands';
  import { live } from '$lib/stores/live.svelte';
  import { now, useTicker } from '$lib/stores/clock.svelte';
  import { formatDuration } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';
  let { close }: { close: () => void } = $props();
  const watch = $derived(live.timer?.stopwatch);
  const running = $derived(watch?.startedAt != null);
  $effect(() => running ? useTicker(100) : undefined);
  const elapsed = $derived((watch?.accumulatedMs ?? 0) + (watch?.startedAt != null ? now() - watch.startedAt : 0));
</script>

<div class="stopwatch">
  <div class="time tabular">{formatDuration(elapsed, true)}</div>
  <div class="controls">
    <button class:active={running} onclick={(e) => { e.stopPropagation(); void timerCommands.stopwatch(running ? 'pause' : 'start'); }}>{t(running ? 'timer.action.pause' : 'timer.start')}</button>
    <button disabled={!running} onclick={(e) => { e.stopPropagation(); void timerCommands.stopwatch('lap'); }}>{t('timer.action.lap')}</button>
    <button onclick={(e) => { e.stopPropagation(); void timerCommands.stopwatch('reset'); }}>{t('stopwatch.reset')}</button>
    <button onclick={(e) => { e.stopPropagation(); close(); }}>{t('common.back')}</button>
  </div>
  <div class="laps">{#each watch?.laps ?? [] as lap, i (i)}<div>#{i + 1} · {formatDuration(lap, true)}</div>{/each}</div>
</div>

<style>
  .stopwatch { display: flex; flex: 1; min-height: 0; flex-direction: column; gap: var(--sp-4); }
  .time { font-size: var(--fs-display); text-align: center; }
  .controls { display: flex; justify-content: center; gap: var(--sp-3); }
  button { background: var(--c-surface-2); color: var(--c-text-1); border: 1px solid var(--c-border-strong); border-radius: var(--r-control); padding: var(--sp-3) var(--sp-4); cursor: pointer; }
  .active { border-color: var(--c-accent); color: var(--c-accent-bright); }
  .laps { overflow: auto; min-height: 0; font-size: var(--fs-small); }
</style>
