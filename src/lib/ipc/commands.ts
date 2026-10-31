// Only this file and events.ts may talk to Tauri (docs/03-coding-principles.md §4).
import { invoke } from '@tauri-apps/api/core';
import type { HitRect } from './bindings/HitRect';
import type { IslandSnapshot } from './bindings/IslandSnapshot';
import type { VisibilityMode } from './bindings/VisibilityMode';
import type { MediaSnapshot } from './bindings/MediaSnapshot';
import type { WebPlayerSnapshot } from './bindings/WebPlayerSnapshot';
import type { SiteDto } from './bindings/SiteDto';
import type { IslandState } from './bindings/IslandState';
import type { Settings } from './bindings/Settings';
import type { AppInfo } from './bindings/AppInfo';
import type { SoundId } from './bindings/SoundId';
import type { TimerSnapshot } from './bindings/TimerSnapshot';
import type { Countdown } from './bindings/Countdown';
import type { PomodoroStats } from './bindings/PomodoroStats';
import type { Reminder } from './bindings/Reminder';
import type { ReminderInput } from './bindings/ReminderInput';
import type { ParsedPreview } from './bindings/ParsedPreview';
import type { Note } from './bindings/Note';
import type { ShelfItem } from './bindings/ShelfItem';
import type { LocalPlayerSnapshot } from './bindings/LocalPlayerSnapshot';
import type { Track } from './bindings/Track';
import type { Folder } from './bindings/Folder';
import type { Playlist } from './bindings/Playlist';
import type { ScanSummary } from './bindings/ScanSummary';
import type { LyricsDto } from './bindings/LyricsDto';

/** False when the UI runs in a plain browser (e.g. `pnpm dev` preview). */
export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const appCommands = {
  info: () => invoke<AppInfo>('app_get_info'),
  quit: () => invoke<void>('app_quit'),
  openDataFolder: () => invoke<void>('app_open_data_folder'),
  playSound: (sound: SoundId) => invoke<void>('app_play_sound', { sound }),
  pauseAlerts: (minutes: number | null) => invoke<void>('app_pause_alerts', { minutes }),
};

export const settingsCommands = {
  get: () => invoke<Settings>('settings_get'),
  update: (section: string, patch: Record<string, unknown>) =>
    invoke<Settings>('settings_update', { section, patch }),
};

export const activityCommands = {
  getState: () => invoke<IslandState>('activity_get_state'),
  invoke: (activityId: string, actionId: string) => invoke<void>('activity_invoke', { activityId, actionId }),
  dismiss: (activityId: string) => invoke<void>('activity_dismiss', { activityId }),
  pin: (activityId: string | null) => invoke<void>('activity_pin', { activityId }),
};

export const islandCommands = {
  setHitRegions: (rects: HitRect[]) => invoke<void>('island_set_hit_regions', { rects }),
  getState: () => invoke<IslandSnapshot>('island_get_state'),
  openMain: (route?: string) => invoke<void>('island_open_main', { route: route ?? null }),
  toggleHidden: () => invoke<VisibilityMode>('island_toggle_hidden'),
  requestFocus: () => invoke<void>('island_request_focus'),
  releaseFocus: () => invoke<void>('island_release_focus'),
};

export const mediaCommands = {
  getState: () => invoke<MediaSnapshot>('media_get_state'),
  playPause: () => invoke<void>('media_play_pause'),
  next: () => invoke<void>('media_next'),
  previous: () => invoke<void>('media_previous'),
  selectSource: (sourceId: string | null) => invoke<void>('media_select_source', { sourceId }),
};

export const timerCommands = {
  getState: () => invoke<TimerSnapshot>('timer_get_state'),
  start: (durationMs: number, label?: string) =>
    invoke<Countdown>('timer_start', { durationMs, label: label ?? null }),
  control: (id: string, action: string) => invoke<void>('timer_control', { id, action }),
  stopwatch: (action: 'start' | 'pause' | 'reset' | 'lap') => invoke<void>('stopwatch_control', { action }),
  pomodoro: (action: 'start' | 'pause' | 'resume' | 'skip' | 'stop' | 'next') =>
    invoke<void>('pomodoro_control', { action }),
  stats: (since: number) => invoke<PomodoroStats>('pomodoro_stats', { since }),
  focus: (active: boolean) => invoke<void>('focus_set', { active }),
};

export const reminderCommands = {
  list: (filter: 'upcoming' | 'missed' | 'done') => invoke<Reminder[]>('reminders_list', { filter }),
  create: (input: ReminderInput) => invoke<Reminder>('reminders_create', { input }),
  update: (id: string, input: ReminderInput) => invoke<void>('reminders_update', { id, input }),
  remove: (id: string) => invoke<void>('reminders_delete', { id }),
  complete: (id: string) => invoke<void>('reminders_complete', { id }),
  snooze: (id: string, minutes: number) => invoke<void>('reminders_snooze', { id, minutes }),
  parse: (text: string) => invoke<ParsedPreview>('reminders_parse', { text }),
};

export const noteCommands = {
  list: (query?: string) => invoke<Note[]>('notes_list', { query: query ?? null }),
  save: (text: string, id?: string) => invoke<Note>('notes_save', { id: id ?? null, text }),
  remove: (id: string) => invoke<void>('notes_delete', { id }),
  restore: (id: string) => invoke<void>('notes_restore', { id }),
  pin: (id: string | null) => invoke<void>('notes_pin', { id }),
  toggleCheck: (id: string, line: number) => invoke<Note>('notes_toggle_check', { id, line }),
};

export const shelfCommands = {
  list: () => invoke<ShelfItem[]>('shelf_list'),
  add: (paths: string[]) => invoke<number>('shelf_add', { paths }),
  remove: (id: string) => invoke<void>('shelf_remove', { id }),
  clear: () => invoke<void>('shelf_clear'),
  open: (path: string, reveal = false) => invoke<void>('shelf_open', { path, reveal }),
};

export const playerCommands = {
  getState: () => invoke<LocalPlayerSnapshot>('local_player_get_state'),
  playTracks: (trackIds: string[], start = 0) => invoke<void>('local_player_play_tracks', { trackIds, start }),
  playFiles: (paths: string[]) => invoke<void>('local_player_play_files', { paths }),
  enqueue: (trackIds: string[], next: boolean) => invoke<void>('local_player_enqueue', { trackIds, next }),
  control: (action: string, value?: number) => invoke<void>('local_player_control', { action, value: value ?? null }),
  setVolume: (volume: number) => invoke<void>('local_player_set_volume', { volume }),
  folders: () => invoke<Folder[]>('local_player_folders'),
  addFolder: (path: string) => invoke<Folder>('local_player_add_folder', { path }),
  removeFolder: (id: string) => invoke<void>('local_player_remove_folder', { id }),
  rescan: () => invoke<ScanSummary>('local_player_rescan'),
  query: (search: string, limit = 200, offset = 0) => invoke<Track[]>('local_player_query', { search, limit, offset }),
  playlists: () => invoke<Playlist[]>('local_player_playlists'),
  createPlaylist: (name: string) => invoke<Playlist>('local_player_playlist_create', { name }),
  deletePlaylist: (id: string) => invoke<void>('local_player_playlist_delete', { id }),
  addToPlaylist: (id: string, trackIds: string[]) => invoke<void>('local_player_playlist_add', { id, trackIds }),
  playPlaylist: (id: string) => invoke<void>('local_player_playlist_play', { id }),
};

export const lyricsCommands = {
  current: () => invoke<LyricsDto | null>('lyrics_get_current'),
};

export const webPlayerCommands = {
  getState: () => invoke<WebPlayerSnapshot>('web_player_get_state'),
  listSites: () => invoke<SiteDto[]>('web_player_list_sites'),
  open: (siteId: string) => invoke<void>('web_player_open', { siteId, url: null }),
  show: () => invoke<void>('web_player_show'),
  hide: () => invoke<void>('web_player_hide'),
  close: () => invoke<void>('web_player_close'),
};
