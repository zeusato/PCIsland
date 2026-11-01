import NotebookPen from '@lucide/svelte/icons/notebook-pen';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';

export function registerNotes(): void {
  registerQuickAction({ id: 'notes.create', labelKey: 'island.launcher.note', icon: NotebookPen, order: 20, panel: QuickPanel });
}
