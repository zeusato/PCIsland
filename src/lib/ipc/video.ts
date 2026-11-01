// IPC of the floating video player (docs/modules/video-player.md §6).
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { LinkKind } from './bindings/LinkKind';
import type { PlaybackSource } from './bindings/PlaybackSource';
import type { ToolsStatus } from './bindings/ToolsStatus';
import type { VideoCommand } from './bindings/VideoCommand';
import type { VideoOpenResult } from './bindings/VideoOpenResult';
import type { VideoReport } from './bindings/VideoReport';
import type { VideoSnapshot } from './bindings/VideoSnapshot';

export const videoCommands = {
  classify: (url: string) => invoke<LinkKind | null>('video_classify', { url }),
  clipboard: () => invoke<string | null>('video_clipboard'),
  open: (url: string, embed = false) => invoke<VideoOpenResult>('video_open', { url, embed }),
  openStream: (id: string) => invoke<void>('video_open_stream', { id }),
  state: () => invoke<VideoSnapshot>('video_get_state'),
  toolsStatus: () => invoke<ToolsStatus>('video_tools_status'),
  installTools: () => invoke<ToolsStatus>('video_tools_install'),
  /** Island/main → player page: play_pause, pause, play, mute, back, forward, seek (ms). */
  control: (action: string, value?: number) => invoke<void>('video_control', { action, value: value ?? null }),
  /** pin, unpin, reset, show, close. */
  window: (action: 'pin' | 'unpin' | 'reset' | 'show' | 'close') => invoke<void>('video_window', { action }),
  openInBrowser: () => invoke<void>('video_open_in_browser'),
  // Video window only.
  current: () => invoke<PlaybackSource | null>('video_current'),
  report: (report: VideoReport) => invoke<void>('video_report', { report }),
  refresh: (positionMs: number) => invoke<PlaybackSource>('video_refresh', { positionMs }),
  setQuality: (height: number | null, positionMs: number) =>
    invoke<PlaybackSource>('video_set_quality', { height, positionMs }),
};

export const onVideoSource = (cb: (s: PlaybackSource) => void): Promise<UnlistenFn> =>
  listen<PlaybackSource>('video://source', (e) => cb(e.payload));
export const onVideoCommand = (cb: (c: VideoCommand) => void): Promise<UnlistenFn> =>
  listen<VideoCommand>('video://command', (e) => cb(e.payload));
export const onVideoState = (cb: (s: VideoSnapshot) => void): Promise<UnlistenFn> =>
  listen<VideoSnapshot>('video://state', (e) => cb(e.payload));

/** Fullscreen of the video window itself. */
export const videoWindow = {
  isFullscreen: () => getCurrentWindow().isFullscreen(),
  setFullscreen: (on: boolean) => getCurrentWindow().setFullscreen(on),
};
