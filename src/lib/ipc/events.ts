import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import type { VisibilityEvent } from './bindings/VisibilityEvent';
import type { PointerEvent as IslandPointerEvent } from './bindings/PointerEvent';
import type { MediaSnapshot } from './bindings/MediaSnapshot';
import type { WebPlayerSnapshot } from './bindings/WebPlayerSnapshot';
import type { IslandState } from './bindings/IslandState';
import type { Settings } from './bindings/Settings';
import type { TimerSnapshot } from './bindings/TimerSnapshot';
import type { RemindersSnapshot } from './bindings/RemindersSnapshot';
import type { Note } from './bindings/Note';
import type { ShelfItem } from './bindings/ShelfItem';
import type { LocalPlayerSnapshot } from './bindings/LocalPlayerSnapshot';
import type { LyricsDto } from './bindings/LyricsDto';

const on =
  <T>(name: string) =>
  (handler: (payload: T) => void): Promise<UnlistenFn> =>
    listen<T>(name, (event) => handler(event.payload));

export const onIslandVisibility = on<VisibilityEvent>('island://visibility');
export const onIslandPointer = on<IslandPointerEvent>('island://pointer');
export const onIslandHotkey = on<string>('island://hotkey');
export const onIslandState = on<IslandState>('island://state');
export const onSettings = on<Settings>('settings://changed');
export const onMediaState = on<MediaSnapshot>('media://now-playing');
export const onWebPlayerState = on<WebPlayerSnapshot>('web_player://state');
export const onTimerState = on<TimerSnapshot>('timer://state');
export const onRemindersState = on<RemindersSnapshot>('reminders://state');
export const onNotesState = on<Note | null>('notes://state');
export const onShelfState = on<ShelfItem[]>('shelf://state');
export const onPlayerState = on<LocalPlayerSnapshot>('local_player://state');
export const onLyrics = on<LyricsDto | null>('lyrics://current');

export type DragDrop =
  | { type: 'enter' | 'over'; x: number; y: number; paths: string[] }
  | { type: 'drop'; x: number; y: number; paths: string[] }
  | { type: 'leave' };

/** OS file drag-and-drop onto the current window (positions in physical px). */
export function onDragDrop(handler: (e: DragDrop) => void): Promise<UnlistenFn> {
  let paths: string[] = [];
  return getCurrentWebview().onDragDropEvent((event) => {
    const p = event.payload;
    if (p.type === 'leave') return handler({ type: 'leave' });
    if ('paths' in p) paths = p.paths;
    handler({ type: p.type, x: p.position.x, y: p.position.y, paths });
  });
}
