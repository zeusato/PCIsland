// Launcher tiles the user can show or hide in Settings (docs/modules/island-window.md §4).
// The main window does not load the island's feature registrations, so this list mirrors
// the IDs passed to registerQuickAction; Launcher.test.ts keeps the two in sync.
export const LAUNCHER_CATALOG: ReadonlyArray<{ id: string; labelKey: string }> = [
  { id: 'web_player.open', labelKey: 'main.nav.media' },
  { id: 'video.open', labelKey: 'main.nav.video' },
  { id: 'notes.create', labelKey: 'island.launcher.note' },
  { id: 'reminders.create', labelKey: 'island.launcher.reminder' },
  { id: 'pomodoro.start', labelKey: 'island.launcher.pomodoro' },
  { id: 'timer.start', labelKey: 'island.launcher.timer' },
  { id: 'stopwatch.start', labelKey: 'island.launcher.stopwatch' },
  { id: 'shelf.open', labelKey: 'main.nav.shelf' },
  { id: 'maintenance.open', labelKey: 'maintenance.title' },
  { id: 'meeting.open', labelKey: 'main.nav.meeting' },
  { id: 'app.open_main', labelKey: 'island.launcher.openMain' },
];
