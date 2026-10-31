import Timer from '@lucide/svelte/icons/timer';
  import Watch from '@lucide/svelte/icons/watch';
  import Brain from '@lucide/svelte/icons/brain';
import { timerCommands } from '$lib/ipc/commands';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';
import { pomodoroLabel, pomodoroState, pomodoroUi, quickPomodoro } from './pomodoro.svelte';

export function registerTimer(): void {
  registerQuickAction({ id: 'timer.start', labelKey: 'island.launcher.timer', icon: Timer, order: 10, panel: QuickPanel });
  registerQuickAction({ id: 'stopwatch.start', labelKey: 'island.launcher.stopwatch', icon: Watch, order: 20, run: () => timerCommands.stopwatch('start') });
  registerQuickAction({ id: 'pomodoro.start', labelKey: 'island.launcher.pomodoro', icon: Brain, order: 30, run: quickPomodoro, state: pomodoroState, status: pomodoroLabel, busy: () => pomodoroUi.busy });
}
