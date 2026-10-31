// Mirror of the backend media state (docs/modules/media-hub.md §6).
import { inTauri, mediaCommands } from '$lib/ipc/commands';
import { onMediaState } from '$lib/ipc/events';
import type { MediaSnapshot } from '$lib/ipc/bindings/MediaSnapshot';
import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';

export const media = $state<{ snapshot: MediaSnapshot | null }>({ snapshot: null });

let started = false;

/** Subscribe first, then fetch the snapshot; drop events older than it (rev). */
export async function startMediaStore(): Promise<void> {
  if (started || !inTauri) return;
  started = true;
  await onMediaState((snap) => {
    if (!media.snapshot || snap.rev > media.snapshot.rev) media.snapshot = snap;
  });
  const snap = await mediaCommands.getState();
  if (!media.snapshot || snap.rev >= media.snapshot.rev) media.snapshot = snap;
}

export function nowPlaying(): NowPlayingDto | null {
  return media.snapshot?.nowPlaying ?? null;
}

/** Interpolated playback position (docs/02-system-design.md §8). */
export function positionAt(np: NowPlayingDto, now = Date.now()): number | null {
  if (np.positionMs === null) return null;
  const elapsed = np.status === 'playing' ? (now - np.positionAt) * np.rate : 0;
  const pos = np.positionMs + Math.max(0, elapsed);
  return np.durationMs === null ? pos : Math.min(pos, np.durationMs);
}

export function formatTime(ms: number): string {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
}
