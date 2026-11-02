<script lang="ts">
  // Launcher panel: Pomodoro controls plus focus music packs (docs/modules/timer.md §3,
  // focus-sounds.md §3) so packs can be fetched without opening the full window.
  import Play from '@lucide/svelte/icons/play';
  import SkipForward from '@lucide/svelte/icons/skip-forward';
  import Square from '@lucide/svelte/icons/square';
  import Button from '$lib/components/Button.svelte';
  import { live } from '$lib/stores/live.svelte';
  import { useTicker, now } from '$lib/stores/clock.svelte';
  import { ceilSeconds, formatDuration } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';
  import { timerCommands } from '$lib/ipc/commands';
  import { controlPomodoro, pomodoroLabel, pomodoroUi } from '../pomodoro.svelte';
  import SoundsPanel from '../../focus_sounds/views/SoundsPanel.svelte';

  let { close }: { close: () => void } = $props();
  $effect(() => useTicker(1000));
  const p = $derived(live.timer?.pomodoro ?? null);
  const left = $derived(!p ? 0 : p.run.kind === 'running' ? p.run.endsAt - now() : p.run.kind === 'paused' ? p.run.remainingMs : 0);
</script>

<div class="panel" role="dialog" aria-label="Pomodoro" tabindex="-1" onkeydown={(e) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } }}>
  <div class="row">
    {#if p}
      <span class="time tabular">{formatDuration(ceilSeconds(left))}</span>
      <span class="phase">{pomodoroLabel()} · {'●'.repeat(p.completedWork % 4)}{'○'.repeat(4 - (p.completedWork % 4))}</span>
    {:else}
      <span class="phase">{t('pomodoro.ready')} · {live.settings?.pomodoro.workMin ?? 25} {t('timer.minutes')}</span>
    {/if}
  </div>
  <div class="row actions">
    {#if p}
      {#if p.run.kind === 'waiting'}
        <Button variant="primary" onclick={() => timerCommands.pomodoro('next')}><Play size={14} />{t('timer.action.startNext')}</Button>
      {/if}
      <Button variant="ghost" title={t('timer.action.skip')} onclick={() => timerCommands.pomodoro('skip')}><SkipForward size={14} /></Button>
      <Button disabled={pomodoroUi.busy} onclick={() => controlPomodoro('stop')}><Square size={14} />{t('timer.action.stop')}</Button>
    {:else}
      <Button variant="primary" disabled={pomodoroUi.busy} onclick={() => controlPomodoro('start')}><Play size={14} />{t('pomodoro.start')}</Button>
    {/if}
  </div>
  <h4>{t('focusSounds.title')}</h4>
  <SoundsPanel compact />
</div>

<style>
  .panel { display: flex; flex-direction: column; gap: var(--sp-4); }
  .row { display: flex; align-items: baseline; gap: var(--sp-4); }
  .actions { align-items: center; flex-wrap: wrap; }
  .time { font-size: var(--fs-title-s); font-weight: var(--fw-semibold); }
  .phase { color: var(--c-text-2); font-size: var(--fs-small); }
  h4 { margin: var(--sp-2) 0 0; font-size: var(--fs-small); color: var(--c-text-2); }
</style>
