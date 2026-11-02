// Only this file and events.ts may talk to Tauri (docs/03-coding-principles.md §4).
import { Channel, invoke } from '@tauri-apps/api/core';
import type { HitRect } from './bindings/HitRect';
import type { IslandSnapshot } from './bindings/IslandSnapshot';
import type { VisibilityMode } from './bindings/VisibilityMode';
import type { MediaSnapshot } from './bindings/MediaSnapshot';
import type { WebPlayerSnapshot } from './bindings/WebPlayerSnapshot';
import type { SiteDto } from './bindings/SiteDto';
import type { IslandState } from './bindings/IslandState';
import type { Settings } from './bindings/Settings';
import type { AppInfo } from './bindings/AppInfo';
import type { ResourceSnapshot } from './bindings/ResourceSnapshot';
import type { CleanupResult } from './bindings/CleanupResult';
import type { CleanupProgress } from './bindings/CleanupProgress';
import type { MemoryProcess } from './bindings/MemoryProcess';
import type { MemoryTarget } from './bindings/MemoryTarget';
import type { MemoryReclaim } from './bindings/MemoryReclaim';
import type { MemoryMeasurement } from './bindings/MemoryMeasurement';
import type { MemoryApp } from './bindings/MemoryApp';
import type { AppCloseResult } from './bindings/AppCloseResult';
import type { RecycleBinInfo } from './bindings/RecycleBinInfo';
import type { ElevatedCleanup } from './bindings/ElevatedCleanup';
import type { FocusSoundsStatus } from './bindings/FocusSoundsStatus';
import type { FocusCatalog } from './bindings/FocusCatalog';
import type { FocusDownloadProgress } from './bindings/FocusDownloadProgress';

/** Downloadable focus music (docs/modules/focus-sounds.md §6). */
export const focusSoundsCommands = {
  status: () => invoke<FocusSoundsStatus>('focus_sounds_status'),
  catalog: () => invoke<FocusCatalog>('focus_sounds_catalog'),
  download: (packs: string[], onProgress: (p: FocusDownloadProgress) => void) => {
    const progress = new Channel<FocusDownloadProgress>();
    progress.onmessage = onProgress;
    return invoke<FocusSoundsStatus>('focus_sounds_download', { packs, progress });
  },
  cancel: () => invoke<void>('focus_sounds_cancel'),
  remove: (packs: string[]) => invoke<FocusSoundsStatus>('focus_sounds_remove', { packs }),
};

export const maintenanceCommands = {
  clean: (onProgress: (p: CleanupProgress) => void) => {
    const progress = new Channel<CleanupProgress>();
    progress.onmessage = onProgress;
    return invoke<CleanupProgress>('maintenance_clean', { progress });
  },
  resources: () => invoke<ResourceSnapshot>('maintenance_resources'),
  logs: (clean: boolean) => invoke<CleanupResult>('maintenance_logs', { clean }),
  processes: () => invoke<MemoryProcess[]>('maintenance_processes'),
  reclaim: (targets: MemoryTarget[]) => invoke<MemoryReclaim>('maintenance_reclaim', { targets }),
  measure: (targets: MemoryTarget[]) => invoke<MemoryMeasurement>('maintenance_measure', { targets }),
  open: (tool: 'storage' | 'task_manager') => invoke<void>('maintenance_open', { tool }),
  recycleBin: () => invoke<RecycleBinInfo>('maintenance_recycle_bin'),
  emptyRecycleBin: () => invoke<RecycleBinInfo>('maintenance_empty_recycle_bin'),
  windowsUpdate: () => invoke<ElevatedCleanup>('maintenance_windows_update'),
  apps: () => invoke<MemoryApp[]>('maintenance_apps'),
  closeApp: (targets: MemoryTarget[], allowAudio: boolean) => invoke<AppCloseResult>('maintenance_close_app', { targets, allowAudio }),
  forceClose: (targets: MemoryTarget[]) => invoke<AppCloseResult>('maintenance_force_close', { targets }),
};
import type { SoundId } from './bindings/SoundId';
import type { TimerSnapshot } from './bindings/TimerSnapshot';
import type { Countdown } from './bindings/Countdown';
import type { PomodoroStats } from './bindings/PomodoroStats';
import type { Reminder } from './bindings/Reminder';
import type { ReminderInput } from './bindings/ReminderInput';
import type { ParsedPreview } from './bindings/ParsedPreview';
import type { Note } from './bindings/Note';
import type { NoteTask } from './bindings/NoteTask';
import type { TaskAlarmInput } from './bindings/TaskAlarmInput';
import type { ShelfItem } from './bindings/ShelfItem';
import type { MeetingSnapshot } from './bindings/MeetingSnapshot';

export const meetingCommands = {
  state: () => invoke<MeetingSnapshot>('meeting_state'),
  devices: () => invoke<Array<{id: string; name: string; system: boolean}>>('meeting_devices'),
  start: (title: string, mic: boolean, system: boolean, transcribe: boolean, micDevice: string | null = null, systemDevice: string | null = null) => invoke<string>('meeting_start', {title,mic,system,transcribe,micDevice,systemDevice}),
  stop: () => invoke<void>('meeting_stop'),
  mark: () => invoke<void>('meeting_mark'),
  notes: (id: string, text: string, revision: number) => invoke<void>('meeting_notes', {id,text,revision}),
  retry: (id: string, clipId: string) => invoke<void>('meeting_retry', {id,clipId}),
  editTranscript: (id: string, clipId: string, text: string) => invoke<void>('meeting_transcript_edit', {id,clipId,text}),
  export: (id: string, destination: string) => invoke<void>('meeting_export', {id,destination}),
};
import type { LocalPlayerSnapshot } from './bindings/LocalPlayerSnapshot';
import type { Track } from './bindings/Track';
import type { Folder } from './bindings/Folder';
import type { Playlist } from './bindings/Playlist';
import type { ScanSummary } from './bindings/ScanSummary';
import type { LyricsDto } from './bindings/LyricsDto';
import type { UpdateInfo } from './bindings/UpdateInfo';
import type { ShortcutError } from './bindings/ShortcutError';
import type { ModuleState } from './bindings/ModuleState';
import type { Station } from './bindings/Station';

/** False when the UI runs in a plain browser (e.g. `pnpm dev` preview). */
export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const appCommands = {
  toast: (title: string, body: string) => invoke<void>('app_show_toast', { title, body }),
  copyLink: (url: string) => invoke<void>('app_copy_link', { url }),
  info: () => invoke<AppInfo>('app_get_info'),
  quit: () => invoke<void>('app_quit'),
  openDataFolder: () => invoke<void>('app_open_data_folder'),
  playSound: (sound: SoundId) => invoke<void>('app_play_sound', { sound }),
  pauseAlerts: (minutes: number | null) => invoke<void>('app_pause_alerts', { minutes }),
  checkUpdateNow: () => invoke<UpdateInfo | null>('app_check_update_now'),
  openUrl: (url: string) => invoke<void>('app_open_url', { url }),
  modules: () => invoke<Record<string, ModuleState>>('app_modules'),
  shortcutErrors: () => invoke<ShortcutError[]>('app_shortcut_errors'),
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
  idleSeconds: () => invoke<number>('island_idle_seconds'),
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
  tasks: (id: string) => invoke<NoteTask[]>('notes_tasks', { id }),
  setTaskAlarm: (taskId: string, input: TaskAlarmInput | null) => invoke<void>('notes_set_task_alarm', { taskId, input }),
  list: (query?: string) => invoke<Note[]>('notes_list', { query: query ?? null }),
  save: (text: string, id?: string, expectedUpdatedAt?: number) => invoke<Note>('notes_save', { id: id ?? null, text, expectedUpdatedAt: expectedUpdatedAt ?? null }),
  desktop: (id: string) => invoke<void>('notes_desktop_open', { id }),
  desktopState: () => invoke<{id: string; font: number; topmost: boolean}>('notes_desktop_state'),
  desktopOptions: (font: number, topmost: boolean, id?: string) => invoke<void>('notes_desktop_options', { font, topmost, id: id ?? null }),
  remove: (id: string) => invoke<void>('notes_delete', { id }),
  /** Writes all notes to a Markdown file; resolves to the count written. */
  exportMarkdown: (path: string) => invoke<number>('notes_export', { path }),
  restore: (id: string) => invoke<void>('notes_restore', { id }),
  pin: (id: string | null) => invoke<void>('notes_pin', { id }),
  toggleCheck: (id: string, line: number) => invoke<Note>('notes_toggle_check', { id, line }),
};

export const shelfCommands = {
  copy: (ids: string[]) => invoke<void>('shelf_copy', { ids }),
  removeMany: (ids: string[]) => invoke<void>('shelf_remove_many', { ids }),
  cancel: () => invoke<void>('shelf_cancel'),
  zip: (ids: string[], destination: string, onProgress: (p: {completed: number; total: number; name: string}) => void) => {
    const progress = new Channel<{completed: number; total: number; name: string}>(); progress.onmessage = onProgress;
    return invoke<string>('shelf_zip', { ids, destination, progress });
  },
  convert: (id: string, destination: string, format: string, maxEdge: number | null) => invoke<string>('shelf_convert', { id, destination, format, maxEdge }),
  list: () => invoke<ShelfItem[]>('shelf_list'),
  add: (paths: string[]) => invoke<number>('shelf_add', { paths }),
  remove: (id: string) => invoke<void>('shelf_remove', { id }),
  clear: () => invoke<void>('shelf_clear'),
  open: (path: string, reveal = false) => invoke<void>('shelf_open', { path, reveal }),
  dragIcon: () => invoke<string>('shelf_drag_icon'),
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
  importPlaylist: (path: string) => invoke<Playlist>('local_player_playlist_import', { path }),
  exportPlaylist: (id: string, path: string) => invoke<number>('local_player_playlist_export', { id, path }),
  stations: () => invoke<Station[]>('local_player_stations'),
  addStation: (name: string, url: string) => invoke<Station[]>('local_player_station_add', { name, url }),
  removeStation: (id: string) => invoke<Station[]>('local_player_station_remove', { id }),
  playStation: (id: string) => invoke<void>('local_player_play_station', { id }),
};

export const lyricsCommands = {
  current: () => invoke<LyricsDto | null>('lyrics_get_current'),
  /** Shifts the current lyrics by deltaMs; 0 resets the offset. */
  adjustOffset: (deltaMs: number) => invoke<LyricsDto | null>('lyrics_adjust_offset', { deltaMs }),
};

export const webPlayerCommands = {
  setRepeat: (mode: 'off' | 'all' | 'one') => invoke<void>('web_player_set_repeat', { mode }),
  search: (siteId: string, query: string) => invoke<import('./bindings/WebQueueItem').WebQueueItem[]>('web_player_search', { siteId, query }),
  playResult: (siteId: string, url: string) => invoke<void>('web_player_play_result', { siteId, url }),
  requestQueue: () => invoke<void>('web_player_request_queue'),
  resume: (siteId: string) => invoke<void>('web_player_resume', { siteId }),
  selectQueue: (url: string) => invoke<void>('web_player_select_queue', { url }),
  getState: () => invoke<WebPlayerSnapshot>('web_player_get_state'),
  listSites: () => invoke<SiteDto[]>('web_player_list_sites'),
  open: (siteId: string) => invoke<void>('web_player_open', { siteId, url: null }),
  show: () => invoke<void>('web_player_show'),
  hide: () => invoke<void>('web_player_hide'),
  close: () => invoke<void>('web_player_close'),
  navigate: (action: 'back' | 'forward' | 'reload' | 'home') => invoke<void>('web_player_navigate', { action }),
  clearData: () => invoke<void>('web_player_clear_data'),
};
