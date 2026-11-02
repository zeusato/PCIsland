<script lang="ts">
  // Pomodoro page: session, settings and focus music in one place (docs/modules/full-window.md §5).
  import Play from '@lucide/svelte/icons/play';
  import SkipForward from '@lucide/svelte/icons/skip-forward';
  import Square from '@lucide/svelte/icons/square';
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import SettingRow from '$lib/components/form/SettingRow.svelte';
  import Toggle from '$lib/components/form/Toggle.svelte';
  import Select from '$lib/components/form/Select.svelte';
  import NumberInput from '$lib/components/form/NumberInput.svelte';
  import { controlPomodoro, pomodoroLabel, pomodoroUi } from '../../../features/timer/pomodoro.svelte';
  import SoundsPanel from '../../../features/focus_sounds/views/SoundsPanel.svelte';
  import { inTauri, playerCommands, timerCommands } from '$lib/ipc/commands';
  import type { PomodoroStats } from '$lib/ipc/bindings/PomodoroStats';
  import type { Playlist } from '$lib/ipc/bindings/Playlist';
  import { live, updateSettings } from '$lib/stores/live.svelte';
  import { useTicker, now } from '$lib/stores/clock.svelte';
  import { ceilSeconds, formatDuration } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  $effect(() => useTicker(1000));
  const s = $derived(live.settings);
  const p = $derived(live.timer?.pomodoro ?? null);
  let stats = $state<PomodoroStats | null>(null);
  let playlists = $state<Playlist[]>([]);

  $effect(() => {
    void live.timer?.rev;
    const d = new Date(Date.now());
    if (inTauri) timerCommands.stats(new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()).then((x) => (stats = x));
  });
  $effect(() => { if (inTauri) playerCommands.playlists().then((x) => (playlists = x)); });

  const left = $derived(!p ? 0 : p.run.kind === 'running' ? p.run.endsAt - now() : p.run.kind === 'paused' ? p.run.remainingMs : 0);
  const set = (key: string) => (value: unknown) => updateSettings('pomodoro', { [key]: value });
</script>

<h1>{t('main.nav.pomodoro')}</h1>

<Card title={t('pomodoro.session')}>
  <div class="item">
    {#if p}
      <span class="big tabular">{formatDuration(ceilSeconds(left))}</span>
      <span class="grow">{pomodoroLabel()} · {'●'.repeat(p.completedWork % 4)}{'○'.repeat(4 - (p.completedWork % 4))}</span>
      {#if p.run.kind === 'waiting'}
        <Button variant="primary" onclick={() => timerCommands.pomodoro('next')}><Play size={14} />{t('timer.action.startNext')}</Button>
      {/if}
      <Button variant="ghost" title={t('timer.action.skip')} onclick={() => timerCommands.pomodoro('skip')}><SkipForward size={14} /></Button>
      <Button variant={p.phase === 'work' ? 'primary' : 'success'} disabled={pomodoroUi.busy} onclick={() => controlPomodoro('stop')}><Square size={14} />{t('timer.action.stop')}</Button>
    {:else}
      <span class="big tabular">{formatDuration((s?.pomodoro.workMin ?? 25) * 60_000)}</span>
      <span class="grow">{t('pomodoro.ready')}</span>
      <Button variant="primary" disabled={pomodoroUi.busy} onclick={() => controlPomodoro('start')}><Play size={14} />{t('pomodoro.start')}</Button>
    {/if}
  </div>
  {#if stats}<p class="muted">{t('pomodoro.today', { n: stats.completedWork, minutes: Math.round(stats.focusedMs / 60000) })}</p>{/if}
  <div class="item">
    <span class="grow">{t('focus.manual')}</span>
    <Toggle label={t('focus.manual')} checked={live.timer?.focusManual ?? false} onchange={(v) => timerCommands.focus(v)} />
  </div>
</Card>

{#if s}
  <Card title={t('pomodoro.settings')}>
    <SettingRow label={t('settings.pomodoro.work')}>
      <NumberInput label={t('settings.pomodoro.work')} value={s.pomodoro.workMin} min={1} max={180} suffix={t('timer.minutes')} onchange={set('workMin')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.short')}>
      <NumberInput label={t('settings.pomodoro.short')} value={s.pomodoro.shortBreakMin} min={1} max={60} suffix={t('timer.minutes')} onchange={set('shortBreakMin')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.long')}>
      <NumberInput label={t('settings.pomodoro.long')} value={s.pomodoro.longBreakMin} min={1} max={120} suffix={t('timer.minutes')} onchange={set('longBreakMin')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.autoBreaks')}>
      <Toggle label={t('settings.pomodoro.autoBreaks')} checked={s.pomodoro.autoStartBreaks} onchange={set('autoStartBreaks')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.focus')} hint={t('settings.pomodoro.focusHint')}>
      <Toggle label={t('settings.pomodoro.focus')} checked={s.pomodoro.focusWithPomodoro} onchange={set('focusWithPomodoro')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.pauseMedia')}>
      <Toggle label={t('settings.pomodoro.pauseMedia')} checked={s.pomodoro.pauseMediaOnBreak} onchange={set('pauseMediaOnBreak')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.playlist')} hint={t('settings.pomodoro.playlistHint')}>
      <Select
        label={t('settings.pomodoro.playlist')}
        value={s.pomodoro.focusPlaylistId}
        options={[{ value: '', label: t('settings.none') }, ...playlists.map((x) => ({ value: x.id, label: x.name }))]}
        onchange={set('focusPlaylistId')}
      />
    </SettingRow>
  </Card>
{/if}

<Card title={t('focusSounds.title')}>
  <SoundsPanel />
</Card>

<style>
  h1 { margin: 0 0 var(--sp-6); font-size: var(--fs-title); }
  .item { display: flex; align-items: center; gap: var(--sp-4); padding: var(--sp-3) 0; }
  .big { font-size: var(--fs-display); font-weight: var(--fw-semibold); min-width: 120px; }
  .grow { flex: 1; }
  .muted { margin: 0; color: var(--c-text-3); font-size: var(--fs-small); }
</style>
