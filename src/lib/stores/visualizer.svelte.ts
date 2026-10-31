// Real visualizer frames (docs/modules/lyrics-visualizer.md §B). The backend only sends
// frames while it captures; `fresh` tells views whether to use them or the CSS fallback.
import { inTauri } from '$lib/ipc/commands';
import { onVisualizerFrame } from '$lib/ipc/events';

export const viz = $state<{ bands: number[]; at: number }>({ bands: [], at: 0 });

let subscribed = false;
export function startVisualizer(): void {
  if (subscribed || !inTauri) return;
  subscribed = true;
  onVisualizerFrame((frame) => {
    viz.bands = frame;
    viz.at = Date.now();
  }).catch(() => (subscribed = false));
}

/** Frames newer than 250 ms with any energy: the real analyzer is running. */
export function fresh(now = Date.now()): boolean {
  return now - viz.at < 250 && viz.bands.some((b) => b > 0);
}

/** Collapses 16 bands to `n` bars (0..1). */
export function bars(n: number): number[] {
  const out: number[] = [];
  const per = Math.max(1, Math.floor(viz.bands.length / n));
  for (let i = 0; i < n; i++) {
    const slice = viz.bands.slice(i * per, (i + 1) * per);
    out.push(slice.length ? Math.max(...slice) / 255 : 0);
  }
  return out;
}
