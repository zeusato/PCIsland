import Timer from '@lucide/svelte/icons/timer';
import Watch from '@lucide/svelte/icons/watch';
import Brain from '@lucide/svelte/icons/brain';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';
import StopwatchPanel from './views/StopwatchPanel.svelte';
import PomodoroPanel from './views/PomodoroPanel.svelte';
import { pomodoroLabel, pomodoroState } from './pomodoro.svelte';

export function registerTimer(): void {
  registerQuickAction({ id: 'timer.start', labelKey: 'island.launcher.timer', icon: Timer, order: 50, panel: QuickPanel });
  registerQuickAction({ id: 'stopwatch.start', labelKey: 'island.launcher.stopwatch', icon: Watch, order: 60, panel: StopwatchPanel });
  registerQuickAction({ id: 'pomodoro.start', labelKey: 'island.launcher.pomodoro', icon: Brain, order: 40, panel: PomodoroPanel, state: pomodoroState, status: pomodoroLabel });
}
