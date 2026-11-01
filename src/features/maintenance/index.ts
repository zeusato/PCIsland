import BrushCleaning from '@lucide/svelte/icons/brush-cleaning';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';
export function registerMaintenance() {
  registerQuickAction({ id: 'maintenance.open', labelKey: 'maintenance.title', icon: BrushCleaning, order: 80, panel: QuickPanel });
}
