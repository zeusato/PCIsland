<script lang="ts">
  // Settings (docs/modules/full-window.md §5). Every change goes through settings_update,
  // which validates, persists and broadcasts to both windows.
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import SettingRow from '$lib/components/form/SettingRow.svelte';
  import Toggle from '$lib/components/form/Toggle.svelte';
  import Select from '$lib/components/form/Select.svelte';
  import NumberInput from '$lib/components/form/NumberInput.svelte';
  import { appCommands, inTauri, playerCommands } from '$lib/ipc/commands';
  import type { ModuleState } from '$lib/ipc/bindings/ModuleState';
  import type { ShortcutError } from '$lib/ipc/bindings/ShortcutError';
  import type { SoundId } from '$lib/ipc/bindings/SoundId';
  import { onShortcutErrors } from '$lib/ipc/events';
  import type { Playlist } from '$lib/ipc/bindings/Playlist';
  import type { UpdateInfo } from '$lib/ipc/bindings/UpdateInfo';
  import type { AppInfo } from '$lib/ipc/bindings/AppInfo';
  import { live, updateSettings } from '$lib/stores/live.svelte';
  import { t } from '$lib/i18n/index.svelte';

  const s = $derived(live.settings);
  let info = $state<AppInfo | null>(null);
  let modules = $state<Record<string, ModuleState>>({});
  let playlists = $state<Playlist[]>([]);
  let update = $state<{ checking: boolean; result: UpdateInfo | null | undefined }>({ checking: false, result: undefined });
  $effect(() => {
    if (!inTauri) return;
    appCommands.info().then((i) => (info = i));
    appCommands.modules().then((m) => (modules = m));
    playerCommands.playlists().then((p) => (playlists = p));
    appCommands.shortcutErrors().then((e) => (shortcutErrors = e));
    const sub = onShortcutErrors((e) => (shortcutErrors = e));
    return () => void sub.then((u) => u());
  });
  let shortcutErrors = $state<ShortcutError[]>([]);
  let startupError = $state('');
  async function setAutostart(value: boolean) {
    startupError = '';
    try { await updateSettings('app', { autostart: value }); info = await appCommands.info(); }
    catch { startupError = t('settings.autostartFailed'); }
  }
  const soundOptions = $derived(
    ['chime', 'bell', 'softTick', 'pop', 'none'].map((v) => ({ value: v, label: t(`settings.sounds.${v}`) })),
  );
  function pickSound(key: 'soundTimer' | 'soundReminder' | 'soundPomodoro') {
    return (value: unknown) => {
      updateSettings('app', { [key]: value });
      if (value !== 'none') appCommands.playSound(value as SoundId);
    };
  }
  async function checkUpdate() {
    update = { checking: true, result: undefined };
    update = { checking: false, result: await appCommands.checkUpdateNow().catch(() => null) };
  }
  const set = (section: string, key: string) => (value: unknown) => updateSettings(section, { [key]: value });
</script>

<h1>{t('main.nav.settings')}</h1>

{#if s}
  <Card title={t('settings.general')}>
    <SettingRow label={t('settings.language')}>
      <Select label={t('settings.language')} value={s.app.language} options={[{ value: 'vi', label: 'Tiếng Việt' }, { value: 'en', label: 'English' }]} onchange={set('app', 'language')} />
    </SettingRow>
    <SettingRow label={t('settings.autostart')} hint={t('settings.autostartHint')}>
      <Toggle label={t('settings.autostart')} checked={s.app.autostart} onchange={setAutostart} />
    </SettingRow>
    {#if startupError}<p role="alert">{startupError}</p>{/if}
    {#if s.app.autostart}
      <p class="muted">{info?.startupCommand ?? t('settings.autostartMissing')}</p>
      <Button onclick={() => setAutostart(true)}>{t('settings.autostartRepair')}</Button>
    {/if}
    <SettingRow label={t('settings.soundVolume')}>
      <input type="range" min="0" max="1" step="0.05" value={s.app.soundVolume} onchange={(e) => set('app', 'soundVolume')(Number((e.currentTarget as HTMLInputElement).value))} />
      <Button onclick={() => appCommands.playSound('chime')}>{t('settings.testSound')}</Button>
    </SettingRow>
    {#each [['soundTimer', 'settings.soundFor.timer'], ['soundReminder', 'settings.soundFor.reminder'], ['soundPomodoro', 'settings.soundFor.pomodoro']] as const as [key, label] (key)}
      <SettingRow label={t(label)}>
        <Select label={t(label)} value={s.app[key]} options={soundOptions} onchange={pickSound(key)} />
      </SettingRow>
    {/each}
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
    <SettingRow label={t('settings.monitor')}>
      <Select label={t('settings.monitor')} value={s.island.monitor} options={['primary', 'cursor'].map((v) => ({ value: v, label: t(`settings.monitors.${v}`) }))} onchange={set('island', 'monitor')} />
    </SettingRow>
    <SettingRow label={t('settings.eyesFollow')}>
      <Toggle label={t('settings.eyesFollow')} checked={s.indicators.eyesFollowCursor} onchange={set('indicators', 'eyesFollowCursor')} />
    </SettingRow>
    <SettingRow label={t('settings.eyesMoods')} hint={t('settings.eyesMoodsHint')}>
      <Toggle label={t('settings.eyesMoods')} checked={s.indicators.eyesMoods} onchange={set('indicators', 'eyesMoods')} />
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
    <SettingRow label={t('settings.hideWhileActive')} hint={t('settings.hideWhileActiveHint')}>
      <Toggle label={t('settings.hideWhileActive')} checked={s.island.hideWhileActive} onchange={set('island', 'hideWhileActive')} />
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
    <SettingRow label={t('settings.glow')}>
      <input type="range" min="0" max="1" step="0.1" value={s.ui.glowIntensity} onchange={(e) => set('ui', 'glowIntensity')(Number((e.currentTarget as HTMLInputElement).value))} />
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
      {#each shortcutErrors.filter((x) => x.action === key) as err (err.accelerator)}
        <p class="shortcut-error">{t('settings.shortcutFailed', { accel: err.accelerator })}</p>
      {/each}
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
    <SettingRow label={t('settings.pomodoro.pauseMedia')}>
      <Toggle label={t('settings.pomodoro.pauseMedia')} checked={s.pomodoro.pauseMediaOnBreak} onchange={set('pomodoro', 'pauseMediaOnBreak')} />
    </SettingRow>
    <SettingRow label={t('settings.pomodoro.playlist')} hint={t('settings.pomodoro.playlistHint')}>
      <Select
        label={t('settings.pomodoro.playlist')}
        value={s.pomodoro.focusPlaylistId}
        options={[{ value: '', label: t('settings.none') }, ...playlists.map((p) => ({ value: p.id, label: p.name }))]}
        onchange={set('pomodoro', 'focusPlaylistId')}
      />
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
    <SettingRow label={t('settings.visualizer')} hint={t('settings.visualizerHint')}>
      <Select label={t('settings.visualizer')} value={s.visualizer.mode} options={['real', 'fake', 'off'].map((v) => ({ value: v, label: t(`settings.visualizers.${v}`) }))} onchange={set('visualizer', 'mode')} />
    </SettingRow>
    <SettingRow label={t('settings.webUnload')} hint={t('settings.webUnloadHint')}>
      <NumberInput label={t('settings.webUnload')} value={s.webPlayer.unloadAfterPausedMin} min={0} max={480} suffix={t('timer.minutes')} onchange={set('webPlayer', 'unloadAfterPausedMin')} />
    </SettingRow>
    <SettingRow label={t('settings.volumeAlert')}>
      <Toggle label={t('settings.volumeAlert')} checked={s.indicators.volumeAlert} onchange={set('indicators', 'volumeAlert')} />
    </SettingRow>
    <SettingRow label={t('settings.powerAlert')}>
      <Toggle label={t('settings.powerAlert')} checked={s.indicators.powerAlert} onchange={set('indicators', 'powerAlert')} />
    </SettingRow>
  </Card>

  <Card title={t('settings.modules')}>
    {#each Object.entries(modules) as [name, st] (name)}
      <SettingRow label={name} hint={st.state === 'disabled' ? st.reason : ''}>
        <span class="state {st.state}">{t(`settings.moduleState.${st.state}`)}</span>
      </SettingRow>
    {/each}
  </Card>

  <Card title={t('settings.data')}>
    {#if info}
      <SettingRow label="PCisland {info.version}" hint={info.portable ? t('settings.portable') : t('settings.installed')}>
        <Button onclick={() => appCommands.openDataFolder()}>{t('settings.openData')}</Button>
      </SettingRow>
    {/if}
    <SettingRow label={t('settings.checkUpdates')} hint={update.result === null ? t('settings.upToDate') : ''}>
      <Toggle label={t('settings.checkUpdates')} checked={s.app.checkUpdates} onchange={set('app', 'checkUpdates')} />
      <Button onclick={checkUpdate} disabled={update.checking}>{t('settings.checkNow')}</Button>
      {#if update.result}
        <Button variant="primary" onclick={() => update.result && appCommands.openUrl(update.result.url)}>{t('app.update.open')} {update.result.version}</Button>
      {/if}
    </SettingRow>
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
  .shortcut-error {
    margin: 0 0 var(--sp-4);
    color: var(--c-danger);
    font-size: var(--fs-small);
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
  .state {
    font-size: var(--fs-small);
    color: var(--c-success);
  }
  .state.restarting {
    color: var(--c-warning);
  }
  .state.disabled,
  .state.stopped {
    color: var(--c-danger);
  }
  input[type='range'] {
    accent-color: var(--c-accent);
  }
</style>
