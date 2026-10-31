import type { Activity } from '$lib/ipc/bindings/Activity';
import { payloadOf } from '$lib/registry/payloads';
import { positionAt } from '../../features/media/store.svelte';

/** A fraction only for tasks with a known duration; live streams have no endpoint. */
export function activityProgress(activity: Activity | null, at: number): number | null {
  if (!activity) return null;
  const np = payloadOf(activity, 'media.now_playing');
  if (np) {
    const position = positionAt(np, at);
    return position !== null && np.durationMs && np.durationMs > 0 ? clamp(position / np.durationMs) : null;
  }
  const timer = payloadOf(activity, 'timer.running')?.timer;
  if (timer) {
    const remaining = timer.state.kind === 'running' ? timer.state.endsAt - at
      : timer.state.kind === 'paused' ? timer.state.remainingMs : 0;
    return clamp(1 - remaining / Math.max(1, timer.durationMs));
  }
  const pomo = payloadOf(activity, 'pomodoro.running');
  if (pomo) {
    const remaining = pomo.run.kind === 'running' ? pomo.run.endsAt - at
      : pomo.run.kind === 'paused' ? pomo.run.remainingMs : 0;
    return clamp(1 - remaining / Math.max(1, pomo.phaseMs));
  }
  return null;
}

function clamp(value: number): number {
  return Math.max(0, Math.min(1, value));
}

export function progressRunning(activity: Activity | null): boolean {
  if (!activity) return false;
  return payloadOf(activity, 'media.now_playing')?.status === 'playing'
    || payloadOf(activity, 'timer.running')?.timer.state.kind === 'running'
    || payloadOf(activity, 'pomodoro.running')?.run.kind === 'running';
}
