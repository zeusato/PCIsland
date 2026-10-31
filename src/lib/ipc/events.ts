import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { VisibilityEvent } from './bindings/VisibilityEvent';
import type { PointerEvent as IslandPointerEvent } from './bindings/PointerEvent';
import type { MediaSnapshot } from './bindings/MediaSnapshot';
import type { WebPlayerSnapshot } from './bindings/WebPlayerSnapshot';

const on =
  <T>(name: string) =>
  (handler: (payload: T) => void): Promise<UnlistenFn> =>
    listen<T>(name, (event) => handler(event.payload));

export const onIslandVisibility = on<VisibilityEvent>('island://visibility');
export const onIslandPointer = on<IslandPointerEvent>('island://pointer');
export const onMediaState = on<MediaSnapshot>('media://now-playing');
export const onWebPlayerState = on<WebPlayerSnapshot>('web_player://state');
