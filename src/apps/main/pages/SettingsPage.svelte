<script lang="ts">
  // Settings (docs/modules/full-window.md §5). Every change goes through settings_update,
  // which validates, persists and broadcasts to both windows.
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import SettingRow from '$lib/components/form/SettingRow.svelte';
  import Toggle from '$lib/components/form/Toggle.svelte';
  import Select from '$lib/components/form/Select.svelte';
  import NumberInput from '$lib/components/form/NumberInput.svelte';
  import { appCommands, inTauri } from '$lib/ipc/commands';
  import type { AppInfo } from '$lib/ipc/bindings/AppInfo';
  import { live, updateSettings } from '$lib/stores/live.svelte';
  import { t } from '$lib/i18n/index.svelte';

  const s = $derived(live.settings);
  let info = $state<AppInfo | null>(null);
  $effect(() => {
    if (inTauri) appCommands.info().then((i) => (info = i));
  });
  const set = (section: string, key: string) => (value: unknown) => updateSettings(section, { [key]: value });
</script>

<h1>{t('main.nav.settings')}</h1>

{#if s}
  <Card title={t('settings.general')}>
    <SettingRow label={t('settings.language')}>
      <Select label={t('settings.language')} value={s.app.language} options={[{ value: 'vi', label: 'Tiếng Việt' }, { value: 'en', label: 'English' }]} onchange={set('app', 'language')} />
    </SettingRow>
    <SettingRow label={t('settings.autostart')} hint={t('settings.autostartHint')}>
      <Toggle label={t('settings.autostart')} checked={s.app.autostart} onchange={set('app', 'autostart')} />
    </SettingRow>
    <SettingRow label={t('settings.soundVolume')}>
      <input type="range" min="0" max="1" step="0.05" value={s.app.soundVolume} onchange={(e) => set('app', 'soundVolume')(Number((e.currentTarget as HTMLInputElement).value))} />
      <Button onclick={() => appCommands.playSound('chime')}>{t('settings.testSound')}</Button>
    </SettingRow>
    <SettingRow label={t('settings.timeFormat')}>
      <Select label={t('settings.timeFormat')} value={s.ui.timeFormat} options={[{ value: '24h', label: '24h' }, { value: '12h', label: '12h' }]} onchange={set('ui', 'timeFormat')} />
    </SettingRow>
  </Card>

  <Card title={t('settings.island')}>
    <SettingRow label={t('settings.idleDisplay')}>
      <Select
        label={t('settings.idleDisplay')}
        value={s.island.idleDisplay}
        options={['eyes', 'clock', 'pill', 'hidden'].map((v) => ({ value: v, label: t(`settings.idle.${v}`) }))}
        onchange={set('island', 'idleDisplay')}
      />
    </SettingRow>
    <SettingRow label={t('settings.hoverDelay')}>
      <NumberInput label={t('settings.hoverDelay')} value={s.island.hoverDelayMs} min={100} max={800} step={50} suffix="ms" onchange={set('island', 'hoverDelayMs')} />
    </SettingRow>
    <SettingRow label={t('settings.collapseDelay')}>
      <NumberInput label={t('settings.collapseDelay')} value={s.island.collapseDelayMs} min={150} max={1500} step={50} suffix="ms" onchange={set('island', 'collapseDelayMs')} />
    </SettingRow>
    <SettingRow label={t('settings.hideFullscreen')}>
      <Toggle label={t('settings.hideFullscreen')} checked={s.island.hideInFullscreen} onchange={set('island', 'hideInFullscreen')} />
    </SettingRow>
    <SettingRow label={t('settings.alertOverFullscreen')}>
      <Select
        label={t('settings.alertOverFullscreen')}
        value={s.island.alertOverFullscreen}
        options={['never', 'critical', 'all'].map((v) => ({ value: v, label: t(`settings.alertOver.${v}`) }))}
        onchange={set('island', 'alertOverFullscreen')}
      />
    </SettingRow>
    <SettingRow label={t('settings.minimalMaximized')}>
      <Toggle label={t('settings.minimalMaximized')} checked={s.island.minimalWhenMaximized} onchange={set('island', 'minimalWhenMaximized')} />
    </SettingRow>
  </Card>

  <Card title={t('settings.appearance')}>
    <SettingRow label={t('settings.accent')}>
      <Select
        label={t('settings.accent')}
        value={s.ui.accent}
        options={['orange', 'amber', 'pink', 'blue'].map((v) => ({ value: v, label: t(`settings.accents.${v}`) }))}
        onchange={set('ui', 'accent')}
      />
    </SettingRow>
  </Card>

  <Card title={t('settings.shortcuts')}>
    {#each ['togglePeek', 'openMain', 'toggleHidden', 'quickNote'] as const as key (key)}
      <SettingRow label={t(`settings.shortcut.${key}`)}>
        <input
          class="shortcut"
          value={s.app.shortcuts[key]}
          onchange={(e) => updateSettings('app', { shortcuts: { [key]: (e.currentTarget as HTMLInputElement).value } })}
        />
      </SettingRow>
    {/each}
  </Card>

  <Card title="Pomodoro">
    <SettingRow label={t('settings.pomodoro.work')}>
      <NumberInput label={t('settings.pomodoro.work')} value={s.pomodoro.workMin} min={1} max={180} suffix={t('timer.minutes')} onchange={set('pomodoro', 'workMin')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.short')}>
      <NumberInput label={t('settings.pomodoro.short')} value={s.pomodoro.shortBreakMin} min={1} max={60} suffix={t('timer.minutes')} onchange={set('pomodoro', 'shortBreakMin')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.long')}>
      <NumberInput label={t('settings.pomodoro.long')} value={s.pomodoro.longBreakMin} min={1} max={120} suffix={t('timer.minutes')} onchange={set('pomodoro', 'longBreakMin')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.autoBreaks')}>
      <Toggle label={t('settings.pomodoro.autoBreaks')} checked={s.pomodoro.autoStartBreaks} onchange={set('pomodoro', 'autoStartBreaks')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.focus')} hint={t('settings.pomodoro.focusHint')}>
      <Toggle label={t('settings.pomodoro.focus')} checked={s.pomodoro.focusWithPomodoro} onchange={set('pomodoro', 'focusWithPomodoro')} />
    </SettingRow>
  </Card>

  <Card title={t('settings.media')}>
    <SettingRow label={t('settings.externalMedia')}>
      <Toggle label={t('settings.externalMedia')} checked={s.media.externalEnabled} onchange={set('media', 'externalEnabled')} />
    </SettingRow>
    <SettingRow label={t('settings.trackAlert')}>
      <Toggle label={t('settings.trackAlert')} checked={s.media.alertOnTrackChange} onchange={set('media', 'alertOnTrackChange')} />
    </SettingRow>
    <SettingRow label={t('settings.lyrics')} hint={t('settings.lyricsHint')}>
      <Toggle label={t('settings.lyrics')} checked={s.lyrics.enabled} onchange={set('lyrics', 'enabled')} />
    </SettingRow>
    <SettingRow label={t('settings.volumeAlert')}>
      <Toggle label={t('settings.volumeAlert')} checked={s.indicators.volumeAlert} onchange={set('indicators', 'volumeAlert')} />
    </SettingRow>
    <SettingRow label={t('settings.powerAlert')}>
      <Toggle label={t('settings.powerAlert')} checked={s.indicators.powerAlert} onchange={set('indicators', 'powerAlert')} />
    </SettingRow>
  </Card>

  <Card title={t('settings.data')}>
    {#if info}
      <SettingRow label="PCisland {info.version}" hint={info.portable ? t('settings.portable') : t('settings.installed')}>
        <Button onclick={() => appCommands.openDataFolder()}>{t('settings.openData')}</Button>
      </SettingRow>
    {/if}
    <SettingRow label={t('settings.pauseAlerts')}>
      <Button onclick={() => appCommands.pauseAlerts(60)}>{t('settings.pause1h')}</Button>
      <Button onclick={() => appCommands.pauseAlerts(null)}>{t('settings.resume')}</Button>
    </SettingRow>
    <SettingRow label={t('settings.quit')}>
      <Button variant="danger" onclick={() => appCommands.quit()}>{t('settings.quit')}</Button>
    </SettingRow>
  </Card>
{/if}

<style>
  h1 {
    margin: 0 0 var(--sp-6);
    font-size: var(--fs-title);
  }
  .shortcut {
    width: 180px;
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-3) var(--sp-4);
    font: inherit;
    user-select: text;
  }
  input[type='range'] {
    accent-color: var(--c-accent);
  }
</style>
