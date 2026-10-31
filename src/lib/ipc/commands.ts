// Only this file and events.ts may talk to Tauri (docs/03-coding-principles.md §4).
import { invoke } from '@tauri-apps/api/core';
import type { HitRect } from './bindings/HitRect';
import type { IslandSnapshot } from './bindings/IslandSnapshot';
import type { VisibilityMode } from './bindings/VisibilityMode';
import type { MediaSnapshot } from './bindings/MediaSnapshot';
import type { WebPlayerSnapshot } from './bindings/WebPlayerSnapshot';
import type { SiteDto } from './bindings/SiteDto';

/** False when the UI runs in a plain browser (e.g. `pnpm dev` preview). */
export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const islandCommands = {
  setHitRegions: (rects: HitRect[]) => invoke<void>('island_set_hit_regions', { rects }),
  getState: () => invoke<IslandSnapshot>('island_get_state'),
  openMain: (route?: string) => invoke<void>('island_open_main', { route: route ?? null }),
  toggleHidden: () => invoke<VisibilityMode>('island_toggle_hidden'),
};

export const mediaCommands = {
  getState: () => invoke<MediaSnapshot>('media_get_state'),
  playPause: () => invoke<void>('media_play_pause'),
  next: () => invoke<void>('media_next'),
  previous: () => invoke<void>('media_previous'),
};

export const webPlayerCommands = {
  getState: () => invoke<WebPlayerSnapshot>('web_player_get_state'),
  listSites: () => invoke<SiteDto[]>('web_player_list_sites'),
  open: (siteId: string) => invoke<void>('web_player_open', { siteId, url: null }),
  show: () => invoke<void>('web_player_show'),
  hide: () => invoke<void>('web_player_hide'),
  close: () => invoke<void>('web_player_close'),
};
