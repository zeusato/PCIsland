import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { VisibilityEvent } from './bindings/VisibilityEvent';
import type { PointerEvent as IslandPointerEvent } from './bindings/PointerEvent';
import type { MediaSnapshot } from './bindings/MediaSnapshot';
import type { WebPlayerSnapshot } from './bindings/WebPlayerSnapshot';
import type { IslandState } from './bindings/IslandState';
import type { Settings } from './bindings/Settings';
import type { TimerSnapshot } from './bindings/TimerSnapshot';
import type { RemindersSnapshot } from './bindings/RemindersSnapshot';
import type { ShortcutError } from './bindings/ShortcutError';
import type { Note } from './bindings/Note';
import type { ShelfItem } from './bindings/ShelfItem';
import type { MeetingSnapshot } from './bindings/MeetingSnapshot';
import type { LocalPlayerSnapshot } from './bindings/LocalPlayerSnapshot';
import type { LyricsDto } from './bindings/LyricsDto';

const on =
  <T>(name: string) =>
  (handler: (payload: T) => void): Promise<UnlistenFn> =>
    listen<T>(name, (event) => handler(event.payload));

export const onIslandVisibility = on<VisibilityEvent>('island://visibility');
export const onIslandPointer = on<IslandPointerEvent>('island://pointer');
export const onIslandDragHover = on<boolean>('island://drag-hover');
export const onIslandHotkey = on<string>('island://hotkey');
export const onIslandState = on<IslandState>('island://state');
export const onSettings = on<Settings>('settings://changed');
export const onMediaState = on<MediaSnapshot>('media://now-playing');
export const onWebPlayerState = on<WebPlayerSnapshot>('web_player://state');
export const onTimerState = on<TimerSnapshot>('timer://state');
export const onRemindersState = on<RemindersSnapshot>('reminders://state');
export const onNotesState = on<Note | null>('notes://state');
export const onShelfState = on<ShelfItem[]>('shelf://state');
export const onMeetingState = on<MeetingSnapshot>('meeting://state');
export const onPlayerState = on<LocalPlayerSnapshot>('local_player://state');
export const onLyrics = on<LyricsDto | null>('lyrics://current');

export type DragDrop =
  | { type: 'enter' | 'over'; x: number; y: number; paths: string[] }
  | { type: 'drop'; x: number; y: number; paths: string[] }
  | { type: 'leave' };

/** Matches platform/webview_drop.rs. */
const DROP_MESSAGE = 'pcisland:drop-files';
type WebView2Bridge = { postMessageWithAdditionalObjects?: (message: string, objects: FileList) => void };

/**
 * OS file drag-and-drop onto the island (docs/modules/island-window.md §5). Chromium takes
 * the drop; the page forwards the File objects to the host, which answers with real paths
 * on `island://files-dropped`. Positions are CSS px.
 */
export function onDragDrop(handler: (e: DragDrop) => void): Promise<UnlistenFn> {
  const hasFiles = (e: DragEvent) => !!e.dataTransfer && [...e.dataTransfer.types].includes('Files');
  let depth = 0;
  let pending: ReturnType<typeof setTimeout> | undefined;
  const enter = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    if (depth++ === 0) handler({ type: 'enter', x: e.clientX, y: e.clientY, paths: [] });
  };
  const over = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    e.dataTransfer!.dropEffect = 'copy';
    handler({ type: 'over', x: e.clientX, y: e.clientY, paths: [] });
  };
  const leave = (e: DragEvent) => {
    if (!hasFiles(e) || --depth > 0) return;
    depth = 0;
    handler({ type: 'leave' });
  };
  const drop = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    depth = 0;
    const bridge = (window as unknown as { chrome?: { webview?: WebView2Bridge } }).chrome?.webview;
    if (!bridge?.postMessageWithAdditionalObjects || !e.dataTransfer!.files.length) return handler({ type: 'leave' });
    bridge.postMessageWithAdditionalObjects(DROP_MESSAGE, e.dataTransfer!.files);
    // The host answers within milliseconds; never leave the drop zone armed forever.
    clearTimeout(pending);
    pending = setTimeout(() => handler({ type: 'leave' }), 3000);
  };
  const types = { dragenter: enter, dragover: over, dragleave: leave, drop } as const;
  for (const [type, fn] of Object.entries(types)) window.addEventListener(type, fn as EventListener, true);
  return listen<string[]>('island://files-dropped', (event) => {
    clearTimeout(pending);
    handler({ type: 'drop', x: 0, y: 0, paths: event.payload });
  }).then((unlisten) => () => {
    unlisten();
    clearTimeout(pending);
    for (const [type, fn] of Object.entries(types)) window.removeEventListener(type, fn as EventListener, true);
  });
}

/** Visualizer frames (16 bands, 0–255), pushed only while the backend captures audio. */
export function onVisualizerFrame(handler: (bands: number[]) => void): Promise<void> {
  const channel = new Channel<number[]>();
  channel.onmessage = handler;
  return invoke<void>('visualizer_subscribe', { channel });
}

/** Eye gaze toward the cursor, x/y in -100..100 (only when the setting is on). */
export const onShortcutErrors = on<ShortcutError[]>('app://shortcut-errors');
export const onIslandGaze = on<[number, number]>('island://gaze');

export const onDesktopNoteSelect = on<string>('notes://desktop-select');
