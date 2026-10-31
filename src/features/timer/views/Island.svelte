<script lang="ts">
  // Island views for timer, stopwatch and Pomodoro (docs/modules/timer.md §4).
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import type { Countdown } from '$lib/ipc/bindings/Countdown';
  import type { Stopwatch } from '$lib/ipc/bindings/Stopwatch';
  import type { Pomodoro } from '$lib/ipc/bindings/Pomodoro';
  import { Timer, Coffee, Brain } from '@lucide/svelte';
  import ActionButtons from '$lib/components/ActionButtons.svelte';
  import ProgressRing from '$lib/components/ProgressRing.svelte';
  import { useTicker, now } from '$lib/stores/clock.svelte';
  import { ceilSeconds, formatDuration, formatTime } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  type Variant = 'compact' | 'peek' | 'alert' | 'bubble';
  let { activity, variant }: { activity: Activity; variant: Variant } = $props();

  const fast = $derived(activity.kind === 'stopwatch.running' && variant === 'peek');
  $effect(() => useTicker(fast ? 100 : 1000));

  function countdownRemaining(c: Countdown, at: number): number {
    if (c.state.kind === 'running') return c.state.endsAt - at;
    if (c.state.kind === 'paused') return c.state.remainingMs;
    return 0;
  }
  function pomodoroRemaining(p: Pomodoro, at: number): number {
    if (p.run.kind === 'running') return p.run.endsAt - at;
    if (p.run.kind === 'paused') return p.run.remainingMs;
    return 0;
  }
  function stopwatchElapsed(s: Stopwatch, at: number): number {
    return s.accumulatedMs + (s.startedAt ? at - s.startedAt : 0);
  }

  const view = $derived.by(() => {
    const at = now();
    const p = activity.payload as Record<string, unknown>;
    switch (activity.kind) {
      case 'timer.running': {
        const c = p.timer as Countdown;
        const rem = countdownRemaining(c, at);
        return {
          time: formatDuration(ceilSeconds(rem)),
          progress: 1 - rem / Math.max(1, c.durationMs),
          title: c.label ?? t('timer.title'),
          sub: (p.count as number) > 1 ? t('timer.more', { n: (p.count as number) - 1 }) : '',
          tone: 'accent',
          icon: Timer,
          paused: c.state.kind === 'paused',
        };
      }
      case 'timer.finished': {
        const c = p as unknown as Countdown;
        const ended = c.state.kind === 'ringing' ? c.state.endedAt : at;
        return { time: '0:00', progress: 1, title: t('timer.finished', { label: c.label ?? '' }), sub: t('timer.endedAt', { time: formatTime(ended) }), tone: 'accent', icon: Timer, paused: false };
      }
      case 'stopwatch.running': {
        const s = p as unknown as Stopwatch;
        const el = stopwatchElapsed(s, at);
        const lap = s.laps.at(-1);
        return { time: formatDuration(el, variant === 'peek'), progress: (el % 60000) / 60000, title: t('stopwatch.title'), sub: lap ? t('stopwatch.lastLap', { time: formatDuration(lap) }) : '', tone: 'accent', icon: Timer, paused: !s.startedAt };
      }
      case 'pomodoro.running':
      case 'pomodoro.phase_end': {
        const pm = p as unknown as Pomodoro & { next?: string; finished?: string };
        const phase = pm.phase ?? pm.next ?? 'work';
        const rem = pm.run ? pomodoroRemaining(pm, at) : 0;
        const waiting = activity.kind === 'pomodoro.phase_end';
        return {
          time: formatDuration(ceilSeconds(rem)),
          progress: pm.phaseMs ? 1 - rem / pm.phaseMs : 1,
          title: waiting ? t(`pomodoro.next.${phase}`) : t(`pomodoro.phase.${phase}`),
          sub: pm.completedWork !== undefined ? '●'.repeat(pm.completedWork % 4) + '○'.repeat(4 - (pm.completedWork % 4)) : '',
          tone: phase === 'work' ? 'accent' : 'break',
          icon: phase === 'work' ? Brain : Coffee,
          paused: pm.run?.kind === 'paused',
        };
      }
      default:
        return { time: '', progress: 0, title: activity.kind, sub: '', tone: 'accent', icon: Timer, paused: false };
    }
  });
  const color = $derived(view.tone === 'break' ? 'var(--c-accent-bright)' : 'var(--c-accent)');
</script>

{#if variant === 'bubble'}
  <div class="bubble"><ProgressRing value={view.progress} size={22} {color} /></div>
{:else if variant === 'compact'}
  <div class="compact" class:paused={view.paused}>
    <ProgressRing value={view.progress} size={20} {color} />
    <span class="spacer"></span>
    <span class="time tabular">{view.time}</span>
  </div>
{:else}
  <div class="peek" class:alert={variant === 'alert'}>
    <div class="head">
      <view.icon size={18} />
      <span class="title">{view.title}</span>
    </div>
    {#if activity.kind !== 'pomodoro.phase_end' && activity.kind !== 'timer.finished'}
      <div class="big tabular" class:paused={view.paused}>{view.time}</div>
    {/if}
    {#if view.sub}<div class="sub">{view.sub}</div>{/if}
    <ActionButtons {activity} labels={variant === 'alert'} />
  </div>
{/if}

<style>
  .compact {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 0 var(--sp-5);
  }
  .spacer {
    flex: 1;
  }
  .time {
    font-size: var(--fs-body-island);
    font-weight: var(--fw-semibold);
    color: var(--c-accent-bright);
  }
  .paused .time,
  .big.paused {
    color: var(--c-text-2);
  }
  .bubble {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
  }
  .peek {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-3);
    padding: var(--sp-5) var(--sp-7);
    height: 100%;
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    color: var(--c-text-2);
    font-size: var(--fs-small);
  }
  .alert .head {
    color: var(--c-text-1);
    font-size: var(--fs-title-s);
    font-weight: var(--fw-semibold);
  }
  .big {
    font-family: var(--font-display);
    font-size: var(--fs-timer-xl);
    font-weight: var(--fw-semibold);
    line-height: 1;
    color: var(--c-accent-bright);
  }
  .sub {
    color: var(--c-text-3);
    font-size: var(--fs-small);
    letter-spacing: 2px;
  }
</style>
