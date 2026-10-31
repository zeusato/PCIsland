import { timerCommands } from '$lib/ipc/commands';
import { live } from '$lib/stores/live.svelte';
import { showFeedback } from '$lib/stores/feedback.svelte';
import { t } from '$lib/i18n/index.svelte';

export const pomodoroUi = $state({ busy: false });

export function pomodoroState(): 'active' | 'break' | 'paused' | null {
  const p = live.timer?.pomodoro;
  if (!p) return null;
  if (p.run.kind !== 'running') return 'paused';
  return p.phase === 'work' ? 'active' : 'break';
}

export function pomodoroLabel(): string {
  const p = live.timer?.pomodoro;
  if (!p) return t('pomodoro.ready');
  return p.run.kind === 'running' ? t(`pomodoro.phase.${p.phase}`) : t(`pomodoro.${p.run.kind}`);
}

export function pomodoroStartMessage(): string {
  const settings = live.settings?.pomodoro;
  const focus = !!live.timer?.focusManual || (settings?.focusWithPomodoro ?? true);
  return [
    t('pomodoro.started', { minutes: settings?.workMin ?? 25 }),
    t(focus ? 'pomodoro.focusOn' : 'pomodoro.focusOff'),
    t('pomodoro.windowsUnchanged'),
    settings?.pauseMediaOnBreak ? t('pomodoro.pauseMusic') : '',
    settings?.focusPlaylistId ? t('pomodoro.playlist') : '',
  ].filter(Boolean).join('\n');
}

export async function controlPomodoro(action: Parameters<typeof timerCommands.pomodoro>[0]): Promise<void> {
  if (pomodoroUi.busy) return;
  pomodoroUi.busy = true;
  try {
    await timerCommands.pomodoro(action);
    if (action === 'start') showFeedback(pomodoroStartMessage());
  } catch { showFeedback(t('errors.unknown')); }
  finally { pomodoroUi.busy = false; }
}

export async function quickPomodoro(): Promise<void> {
  const p = live.timer?.pomodoro;
  await controlPomodoro(!p ? 'start' : p.run.kind === 'running' ? 'pause' : p.run.kind === 'paused' ? 'resume' : 'next');
}
