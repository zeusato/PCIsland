<script lang="ts">
  import Play from '@lucide/svelte/icons/play';
  import Pause from '@lucide/svelte/icons/pause';
  import X from '@lucide/svelte/icons/x';
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Flag from '@lucide/svelte/icons/flag';
  import SkipForward from '@lucide/svelte/icons/skip-forward';
  import Square from '@lucide/svelte/icons/square';
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import Toggle from '$lib/components/form/Toggle.svelte';
  import FeedbackToast from '$lib/components/FeedbackToast.svelte';
  import { controlPomodoro, pomodoroLabel, pomodoroUi } from '../../../features/timer/pomodoro.svelte';
  import { inTauri, timerCommands } from '$lib/ipc/commands';
  import type { Countdown } from '$lib/ipc/bindings/Countdown';
  import type { PomodoroStats } from '$lib/ipc/bindings/PomodoroStats';
  import { live } from '$lib/stores/live.svelte';
  import { useTicker, now } from '$lib/stores/clock.svelte';
  import { ceilSeconds, formatDuration } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  $effect(() => useTicker(live.timer?.stopwatch.startedAt ? 100 : 1000));

  let minutes = $state(10);
  let label = $state('');
  let stats = $state<PomodoroStats | null>(null);
  const timer = $derived(live.timer);

  $effect(() => {
    void timer?.rev;
    const d = new Date(Date.now());
    const startOfDay = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
    if (inTauri) timerCommands.stats(startOfDay).then((s) => (stats = s));
  });

  const remaining = (c: Countdown) =>
    c.state.kind === 'running' ? c.state.endsAt - now() : c.state.kind === 'paused' ? c.state.remainingMs : 0;
  const elapsed = $derived(timer ? timer.stopwatch.accumulatedMs + (timer.stopwatch.startedAt ? now() - timer.stopwatch.startedAt : 0) : 0);
  const pomodoroLeft = $derived.by(() => {
    const p = timer?.pomodoro;
    if (!p) return 0;
    return p.run.kind === 'running' ? p.run.endsAt - now() : p.run.kind === 'paused' ? p.run.remainingMs : 0;
  });
</script>

<h1>{t('main.nav.timer')}</h1>
<FeedbackToast />

<Card title={t('timer.title')}>
  <div class="new">
    <input type="number" min="1" max="600" bind:value={minutes} aria-label={t('timer.minutes')} />
    <span class="muted">{t('timer.minutes')}</span>
    <input class="label" placeholder={t('timer.labelPlaceholder')} bind:value={label} />
    <Button variant="primary" onclick={() => timerCommands.start(minutes * 60_000, label || undefined).then(() => (label = ''))}>{t('timer.start')}</Button>
  </div>
  {#each timer?.timers ?? [] as c (c.id)}
    <div class="item">
      <span class="big tabular" class:ringing={c.state.kind === 'ringing'}>{formatDuration(ceilSeconds(remaining(c)))}</span>
      <span class="grow">{c.label ?? ''}</span>
      {#if c.state.kind === 'running'}
        <Button variant="ghost" onclick={() => timerCommands.control(c.id, 'pause')}><Pause size={14} /></Button>
      {:else if c.state.kind === 'paused'}
        <Button variant="ghost" onclick={() => timerCommands.control(c.id, 'resume')}><Play size={14} /></Button>
      {:else}
        <Button variant="ghost" onclick={() => timerCommands.control(c.id, 'repeat')}><RotateCcw size={14} /></Button>
      {/if}
      <Button variant="ghost" onclick={() => timerCommands.control(c.id, 'add_minute')}><Plus size={14} />1′</Button>
      <Button variant="ghost" onclick={() => timerCommands.control(c.id, 'cancel')}><X size={14} /></Button>
    </div>
  {/each}
</Card>

<Card title={t('stopwatch.title')}>
  <div class="item">
    <span class="big tabular">{formatDuration(elapsed, true)}</span>
    <span class="grow"></span>
    {#if timer?.stopwatch.startedAt}
      <Button onclick={() => timerCommands.stopwatch('lap')}><Flag size={14} />{t('timer.action.lap')}</Button>
      <Button onclick={() => timerCommands.stopwatch('pause')}><Pause size={14} /></Button>
    {:else}
      <Button variant="primary" onclick={() => timerCommands.stopwatch('start')}><Play size={14} /></Button>
    {/if}
    <Button variant="ghost" onclick={() => timerCommands.stopwatch('reset')}><RotateCcw size={14} /></Button>
  </div>
  {#if timer?.stopwatch.laps.length}
    <ol class="laps">
      {#each timer.stopwatch.laps as lap, i (i)}
        <li class="tabular">#{i + 1} · {formatDuration(lap, true)} <span class="muted">(+{formatDuration(lap - (timer.stopwatch.laps[i - 1] ?? 0), true)})</span></li>
      {/each}
    </ol>
  {/if}
</Card>

<Card title="Pomodoro">
  <div class="item">
    {#if timer?.pomodoro}
      <span class="big tabular">{formatDuration(ceilSeconds(pomodoroLeft))}</span>
      <span class="grow">{t(`pomodoro.phase.${timer.pomodoro.phase}`)} · {'●'.repeat(timer.pomodoro.completedWork % 4)}{'○'.repeat(4 - (timer.pomodoro.completedWork % 4))}</span>
      {#if timer.pomodoro.run.kind === 'running'}
        <Button variant={timer.pomodoro.phase === 'work' ? 'primary' : 'success'} disabled={pomodoroUi.busy} title={t('timer.action.pause')} onclick={() => controlPomodoro('pause')}><Pause size={14} />{pomodoroLabel()}</Button>
      {:else if timer.pomodoro.run.kind === 'paused'}
        <Button variant="warning" disabled={pomodoroUi.busy} title={t('timer.action.resume')} onclick={() => controlPomodoro('resume')}><Play size={14} />{pomodoroLabel()}</Button>
      {:else}
        <Button variant="primary" onclick={() => timerCommands.pomodoro('next')}><Play size={14} />{t('timer.action.startNext')}</Button>
      {/if}
      <Button variant="ghost" onclick={() => timerCommands.pomodoro('skip')}><SkipForward size={14} /></Button>
      <Button variant="ghost" onclick={() => timerCommands.pomodoro('stop')}><Square size={14} /></Button>
    {:else}
      <Button variant="primary" disabled={pomodoroUi.busy} onclick={() => controlPomodoro('start')}><Play size={14} />{t('pomodoro.start')}</Button>
      <span class="grow"></span>
    {/if}
  </div>
  {#if stats}
    <p class="muted">{t('pomodoro.today', { n: stats.completedWork, minutes: Math.round(stats.focusedMs / 60000) })}</p>
  {/if}
  <div class="item">
    <span class="grow">{t('focus.manual')}</span>
    <Toggle label={t('focus.manual')} checked={timer?.focusManual ?? false} onchange={(v) => timerCommands.focus(v)} />
  </div>
</Card>

<style>
  h1 {
    margin: 0 0 var(--sp-6);
    font-size: var(--fs-title);
  }
  .new,
  .item {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) 0;
  }
  input {
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-3) var(--sp-4);
    font: inherit;
    width: 80px;
    user-select: text;
  }
  input.label {
    flex: 1;
    width: auto;
  }
  .big {
    font-size: var(--fs-display);
    font-weight: var(--fw-semibold);
    min-width: 120px;
  }
  .ringing {
    color: var(--c-accent-bright);
  }
  .grow {
    flex: 1;
  }
  .muted {
    color: var(--c-text-3);
    font-size: var(--fs-small);
  }
  .laps {
    margin: 0;
    padding-left: var(--sp-7);
    font-size: var(--fs-small);
    max-height: 160px;
    overflow: auto;
  }
</style>
