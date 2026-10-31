import { expect, it } from 'vitest';
import type { Activity } from '$lib/ipc/bindings/Activity';
import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
import { activityProgress, progressRunning } from './progress';

function activity(kind: string, payload: unknown): Activity {
  return { id: 'test:1', source: 'test', kind, payload, priority: 50, transientMs: null, placement: 'auto', startedAt: 0, actions: [] };
}
const np: NowPlayingDto = { sourceId: 'web', appName: 'Online', title: 'Track', artist: null, album: null, artworkUrl: null, status: 'playing', positionMs: 1000, positionAt: 1000, durationMs: 10000, rate: 1, caps: { playPause: true, next: false, previous: false, seek: false }, isAd: false };

it('interpolates track progress, freezes paused tracks, and excludes streams', () => {
  expect(activityProgress(activity('media.now_playing', np), 5000)).toBe(0.5);
  expect(activityProgress(activity('media.now_playing', { ...np, status: 'paused' }), 5000)).toBe(0.1);
  expect(activityProgress(activity('media.now_playing', { ...np, durationMs: null }), 5000)).toBeNull();
  expect(activityProgress(activity('media.now_playing', np), 50000)).toBe(1);
});

it('shows countdown and Pomodoro progress using deadlines', () => {
  const timer = activity('timer.running', { timer: { durationMs: 10000, state: { kind: 'running', endsAt: 10000 } }, count: 1 });
  expect(activityProgress(timer, 2500)).toBe(0.25);
  expect(progressRunning(timer)).toBe(true);
  const pomo = activity('pomodoro.running', { phaseMs: 10000, run: { kind: 'paused', remainingMs: 2500 } });
  expect(activityProgress(pomo, 9000)).toBe(0.75);
  expect(progressRunning(pomo)).toBe(false);
});
