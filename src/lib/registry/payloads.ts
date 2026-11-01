// Typed activity payloads (ADR-010). The backend keeps payloads as JSON so the core
// does not depend on modules; each module exports its payload types through ts-rs and
// this map ties them to their `kind`. Views read payloads only through `payloadOf`.
import type { Activity } from '$lib/ipc/bindings/Activity';
import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
import type { CountdownsPayload } from '$lib/ipc/bindings/CountdownsPayload';
import type { Countdown } from '$lib/ipc/bindings/Countdown';
import type { Stopwatch } from '$lib/ipc/bindings/Stopwatch';
import type { Pomodoro } from '$lib/ipc/bindings/Pomodoro';
import type { Reminder } from '$lib/ipc/bindings/Reminder';
import type { CountPayload } from '$lib/ipc/bindings/CountPayload';
import type { IdlePayload } from '$lib/ipc/bindings/IdlePayload';
import type { VolumePayload } from '$lib/ipc/bindings/VolumePayload';
import type { PowerPayload } from '$lib/ipc/bindings/PowerPayload';
import type { BatteryLowPayload } from '$lib/ipc/bindings/BatteryLowPayload';
import type { Note } from '$lib/ipc/bindings/Note';
import type { NoteTask } from '$lib/ipc/bindings/NoteTask';
import type { ShelfPayload } from '$lib/ipc/bindings/ShelfPayload';
import type { UpdateInfo } from '$lib/ipc/bindings/UpdateInfo';

export interface ActivityPayloads {
  'media.now_playing': NowPlayingDto;
  'media.track_changed': NowPlayingDto;
  'timer.running': CountdownsPayload;
  'timer.finished': Countdown;
  'stopwatch.running': Stopwatch;
  'pomodoro.running': Pomodoro;
  /** Pomodoro waiting for the next phase; notifications use a native toast. */
  'pomodoro.phase_end': Pomodoro;
  'reminders.due': Reminder;
  'reminders.upcoming': Reminder;
  'reminders.missed_summary': CountPayload;
  'indicators.idle': IdlePayload;
  'indicators.volume': VolumePayload;
  'indicators.power': PowerPayload;
  'indicators.battery_low': BatteryLowPayload;
  'notes.pinned': Note;
  'notes.task_due': NoteTask;
  'notes.saved': null;
  'shelf.items': ShelfPayload;
  'shelf.dropped': CountPayload;
  'settings.recovered': null;
  'app.update': UpdateInfo;
}

export type ActivityKind = keyof ActivityPayloads;

/** The payload of `activity` if its kind is `kind`, otherwise null. */
export function payloadOf<K extends ActivityKind>(activity: Activity, kind: K): ActivityPayloads[K] | null {
  return activity.kind === kind ? (activity.payload as ActivityPayloads[K]) : null;
}

/** For views that handle one kind only: the payload, typed by the kind they expect. */
export function payload<K extends ActivityKind>(activity: Activity, _kind: K): ActivityPayloads[K] {
  return activity.payload as ActivityPayloads[K];
}
