// View registry: activity source → island view (docs/02-system-design.md §7).
import type { Component } from 'svelte';
import type { Activity } from '$lib/ipc/bindings/Activity';
import MediaIsland from '../../features/media/views/Island.svelte';
import TimerIsland from '../../features/timer/views/Island.svelte';
import RemindersIsland from '../../features/reminders/views/Island.svelte';
import IndicatorsIsland from '../../features/indicators/views/Island.svelte';
import NotesIsland from '../../features/notes/views/Island.svelte';
import ShelfIsland from '../../features/shelf/views/Island.svelte';
import GenericActivity from '../../apps/island/GenericActivity.svelte';

export type Variant = 'compact' | 'peek' | 'alert' | 'bubble';
type View = Component<{ activity: Activity; variant: Variant }>;

const bySource: Record<string, View> = {
  media: MediaIsland as View,
  timer: TimerIsland as View,
  reminders: RemindersIsland as View,
  indicators: IndicatorsIsland as View,
  notes: NotesIsland as View,
  shelf: ShelfIsland as View,
};

export function viewFor(activity: Activity): View {
  return bySource[activity.source] ?? (GenericActivity as View);
}

/** Alerts that need room for buttons: critical activities and alerts with actions. */
export function isLargeAlert(a: Activity): boolean {
  return a.priority >= 100 || a.actions.length > 0;
}
