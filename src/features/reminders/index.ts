import { Bell } from '@lucide/svelte';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';

export function registerReminders(): void {
  registerQuickAction({ id: 'reminders.create', labelKey: 'island.launcher.reminder', icon: Bell, order: 40, panel: QuickPanel });
}
