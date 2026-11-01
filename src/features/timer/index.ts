import Timer from '@lucide/svelte/icons/timer';
  import Watch from '@lucide/svelte/icons/watch';
  import Brain from '@lucide/svelte/icons/brain';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';
import StopwatchPanel from './views/StopwatchPanel.svelte';
import { pomodoroLabel, pomodoroState, pomodoroUi, quickPomodoro } from './pomodoro.svelte';

export function registerTimer(): void {
  registerQuickAction({ id: 'timer.start', labelKey: 'island.launcher.timer', icon: Timer, order: 50, panel: QuickPanel });
  registerQuickAction({ id: 'stopwatch.start', labelKey: 'island.launcher.stopwatch', icon: Watch, order: 60, panel: StopwatchPanel });
  registerQuickAction({ id: 'pomodoro.start', labelKey: 'island.launcher.pomodoro', icon: Brain, order: 40, run: quickPomodoro, state: pomodoroState, status: pomodoroLabel, busy: () => pomodoroUi.busy });
}
