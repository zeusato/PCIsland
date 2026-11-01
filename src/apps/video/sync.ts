// Audio-clock sync for the video+audio pair (docs/modules/video-player.md §5.5, spike S2:
// 16 ms average drift at 1080p60). Pure decision function so it can be unit tested.

export type SyncAction = { kind: 'none' } | { kind: 'rate'; rate: number } | { kind: 'jump' };

/** `drift` = video time − audio time, in seconds (positive = video ahead). */
export function syncStep(drift: number): SyncAction {
  const abs = Math.abs(drift);
  if (abs > 0.4) return { kind: 'jump' };
  // 35 ms: below lip-sync perception; measured steady offset was 40 ms at an 80 ms threshold.
  if (abs > 0.035) return { kind: 'rate', rate: drift > 0 ? 0.97 : 1.03 };
  return { kind: 'none' };
}

export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
}
