// Mirrors of backend state shared by island and main windows.
// Pattern: subscribe first, then fetch the snapshot, keep the newest `rev`
// (docs/02-system-design.md §7).
import {
  activityCommands,
  inTauri,
  lyricsCommands,
  playerCommands,
  settingsCommands,
  timerCommands,
} from '$lib/ipc/commands';
import {
  onIslandState,
  onLyrics,
  onPlayerState,
  onRemindersState,
  onSettings,
  onShelfState,
  onTimerState,
} from '$lib/ipc/events';
import type { IslandState } from '$lib/ipc/bindings/IslandState';
import type { Settings } from '$lib/ipc/bindings/Settings';
import type { TimerSnapshot } from '$lib/ipc/bindings/TimerSnapshot';
import type { RemindersSnapshot } from '$lib/ipc/bindings/RemindersSnapshot';
import type { ShelfItem } from '$lib/ipc/bindings/ShelfItem';
import type { LocalPlayerSnapshot } from '$lib/ipc/bindings/LocalPlayerSnapshot';
import type { LyricsDto } from '$lib/ipc/bindings/LyricsDto';
import { setLocale } from '$lib/i18n/index.svelte';

export const live = $state<{
  island: IslandState | null;
  settings: Settings | null;
  timer: TimerSnapshot | null;
  reminders: RemindersSnapshot | null;
  shelf: ShelfItem[];
  player: LocalPlayerSnapshot | null;
  lyrics: LyricsDto | null;
}>({ island: null, settings: null, timer: null, reminders: null, shelf: [], player: null, lyrics: null });

const newer = <T extends { rev: number | bigint }>(cur: T | null, next: T) =>
  !cur || Number(next.rev) >= Number(cur.rev);

function applySettings(s: Settings) {
  live.settings = s;
  setLocale(s.app.language === 'en' ? 'en' : 'vi');
  document.documentElement.dataset.accent = s.ui.accent;
  document.documentElement.style.setProperty('--glow-scale', String(s.ui.glowIntensity));
}

let started = false;
export async function startLiveStores(): Promise<void> {
  if (started || !inTauri) return;
  started = true;
  await Promise.all([
    onIslandState((s) => {
      if (newer(live.island, s)) live.island = s;
    }),
    onSettings(applySettings),
    onTimerState((s) => {
      if (newer(live.timer, s)) live.timer = s;
    }),
    onRemindersState((s) => {
      live.reminders = s;
    }),
    onShelfState((s) => {
      live.shelf = s;
    }),
    onPlayerState((s) => {
      if (newer(live.player, s)) live.player = s;
    }),
    onLyrics((l) => {
      live.lyrics = l;
    }),
  ]);
  const [island, settings, timer, player, lyrics] = await Promise.all([
    activityCommands.getState(),
    settingsCommands.get(),
    timerCommands.getState(),
    playerCommands.getState(),
    lyricsCommands.current(),
  ]);
  if (newer(live.island, island)) live.island = island;
  applySettings(settings);
  if (newer(live.timer, timer)) live.timer = timer;
  if (newer(live.player, player)) live.player = player;
  live.lyrics = lyrics;
}

/** Updates one settings section; the backend validates and broadcasts the result. */
export async function updateSettings(section: string, patch: Record<string, unknown>) {
  applySettings(await settingsCommands.update(section, patch));
}
