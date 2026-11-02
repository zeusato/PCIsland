import { maintenanceCommands as api } from '$lib/ipc/commands';
import type { ResourceSnapshot } from '$lib/ipc/bindings/ResourceSnapshot';
import type { CleanupResult } from '$lib/ipc/bindings/CleanupResult';
import type { CleanupProgress } from '$lib/ipc/bindings/CleanupProgress';
import type { MemoryProcess } from '$lib/ipc/bindings/MemoryProcess';
import type { MemoryReclaim } from '$lib/ipc/bindings/MemoryReclaim';
import type { MemoryMeasurement } from '$lib/ipc/bindings/MemoryMeasurement';
import type { MemoryApp } from '$lib/ipc/bindings/MemoryApp';
import type { AppCloseResult } from '$lib/ipc/bindings/AppCloseResult';
import type { RecycleBinInfo } from '$lib/ipc/bindings/RecycleBinInfo';
import type { ElevatedCleanup } from '$lib/ipc/bindings/ElevatedCleanup';

/** Pending confirmation; every irreversible or disruptive action passes through one. */
export type Confirm = { kind: 'recycle' } | { kind: 'update' } | { kind: 'close' | 'force'; name: string; audio: boolean };

export const bytes = (n: number) => {
  const unit = n >= 1024 ** 3 ? 'GiB' : n >= 1024 ** 2 ? 'MiB' : n >= 1024 ? 'KiB' : 'B';
  const divisor = { GiB: 1024 ** 3, MiB: 1024 ** 2, KiB: 1024, B: 1 }[unit];
  return `${(n / divisor).toFixed(unit === 'B' ? 0 : 1)} ${unit}`;
};

/** Shared by every panel in a window: work started in the island keeps its progress and
 * result when the panel unmounts. Only resource polling stops with the last panel. */
export function createMaintenance() {
  const state = $state({ resources: null as ResourceSnapshot | null, diskBefore: null as number | null,
    logs: null as CleanupResult | null, cleaned: null as CleanupResult | null,
    cleanup: null as CleanupProgress | null,
    processes: [] as MemoryProcess[], selected: [] as number[], scanned: false,
    reclaim: null as MemoryReclaim | null, later: null as MemoryMeasurement | null,
    ramBefore: null as number | null, ramAfter: null as number | null,
    busy: false, error: false, samplingError: false, waiting: false,
    recycle: null as RecycleBinInfo | null, recycled: null as RecycleBinInfo | null,
    update: null as ElevatedCleanup | null, updating: false,
    apps: [] as MemoryApp[], appsScanned: false,
    confirm: null as Confirm | null, closed: null as { name: string; result: AppCloseResult } | null });
  let mounted = 0; let sampling = false; let poll: ReturnType<typeof setInterval> | undefined;
  let laterTimer: ReturnType<typeof setTimeout> | undefined;
  async function refresh() {
    if (!mounted || sampling || document.hidden) return;
    sampling = true;
    try { const snapshot = await api.resources(); state.resources = snapshot; state.samplingError = false; }
    catch { state.samplingError = true; }
    finally { sampling = false; }
  }
  async function run(task: () => Promise<void>) {
    if (state.busy) return;
    state.busy = true; state.error = false;
    try { await task(); } catch { state.error = true; }
    finally { state.busy = false; }
  }
  async function loadRecycle() {
    try { const info = await api.recycleBin(); state.recycle = info; } catch { state.recycle = null; }
  }
  const visibility = () => { if (!document.hidden) void refresh(); };
  return { state, refresh,
    clean: () => run(async () => {
      state.cleanup = { phase: 'scan', category: 'temp', scanned: 0, total: 0, processed: 0, files: 0, bytes: 0, folders: 0, skipped: 0, busyApps: 0, limited: false, groups: [] };
      try {
        const result = await api.clean(progress => { state.cleanup = progress; });
        state.cleanup = result;
      } catch (error) {
        if (state.cleanup) state.cleanup.phase = 'failed';
        throw error;
      }
      void loadRecycle(); await refresh();
    }),
    start() {
      if (mounted++ === 0) { poll = setInterval(() => void refresh(), 5000); document.addEventListener('visibilitychange', visibility); }
      void refresh(); if (!state.busy) void loadRecycle();
      return () => { if (--mounted === 0) { clearInterval(poll); document.removeEventListener('visibilitychange', visibility); } };
    },
    scan: () => run(async () => { const rows = await api.processes(); { state.processes = rows; state.selected = []; state.scanned = true; } }),
    select(pid: number, checked: boolean) { if (state.busy) return; state.selected = state.selected.filter(p => p !== pid); if (checked && state.processes.some(p => p.target.pid === pid && p.reason === 'candidate')) state.selected.push(pid); },
    selectCandidates() { if (!state.busy) state.selected = state.processes.filter(p => p.reason === 'candidate').slice(0, 128).map(p => p.target.pid); },
    reclaim: () => run(async () => {
      const targets = state.processes.filter(p => state.selected.includes(p.target.pid) && p.reason === 'candidate').map(p => p.target);
      if (!targets.length) return;
      clearTimeout(laterTimer); state.later = null; state.waiting = false;
      state.ramBefore = (await api.resources()).ramAvailable;
      const result = await api.reclaim(targets);
            state.reclaim = result; state.selected = []; state.ramAfter = (await api.resources()).ramAvailable;
            state.waiting = result.processed > 0;
      if (state.waiting) laterTimer = setTimeout(() => {
        void api.measure(result.targets).then(m => { state.later = m; }).catch(() => { state.error = true; }).finally(() => { state.waiting = false; });
      }, 30000);
      await refresh();
    }),
    scanLogs: () => run(async () => { const result = await api.logs(false); state.logs = result; }),
    cleanLogs: () => run(async () => { const result = await api.logs(true); { state.cleaned = result; state.logs = null; } await refresh(); }),
    ask(confirm: Confirm) { if (!state.busy) state.confirm = confirm; },
    cancel() { state.confirm = null; },
    /** Irreversible: only runs from the confirmation step. */
    emptyRecycle: () => run(async () => {
      if (state.confirm?.kind !== 'recycle') return;
      state.confirm = null;
      const freed = await api.emptyRecycleBin();
      state.recycled = freed;
      await loadRecycle(); await refresh();
    }),
    windowsUpdate: () => run(async () => {
      if (state.confirm?.kind !== 'update') return;
      state.confirm = null; state.update = null; state.updating = true;
      try { const result = await api.windowsUpdate(); state.update = result; }
      finally { state.updating = false; }
      await refresh();
    }),
    scanApps: () => run(async () => { const rows = await api.apps(); { state.apps = rows; state.appsScanned = true; state.closed = null; state.confirm = null; } }),
    /** Step one (graceful) or step two (force), each only after its own confirmation. */
    closeApp: () => run(async () => {
      const confirm = state.confirm;
      if (confirm?.kind !== 'close' && confirm?.kind !== 'force') return;
      state.confirm = null;
      const app = state.apps.find(a => a.name === confirm.name);
      let result: AppCloseResult;
      if (confirm.kind === 'force') {
        const remaining = state.closed?.name === confirm.name ? state.closed.result.remaining : [];
        if (!remaining.length) return;
        result = await api.forceClose(remaining);
      } else {
        if (!app) return;
        result = await api.closeApp(app.processes, confirm.audio);
      }
            // Audio started after the scan: ask again with the audio warning instead of acting.
      if (result.status === 'audio') { state.confirm = { kind: 'close', name: confirm.name, audio: true }; return; }
      state.closed = { name: confirm.name, result };
      if (result.status === 'closed') state.apps = state.apps.filter(a => a.name !== confirm.name);
      await refresh();
    }),
    open: (tool: 'storage' | 'task_manager') => run(async () => { if (tool === 'storage') state.diskBefore = (await api.resources()).diskFree; await api.open(tool); }),
  };
}


let shared: ReturnType<typeof createMaintenance> | undefined;
/** The window-wide controller used by panels (island and Settings). */
export const useMaintenance = () => (shared ??= createMaintenance());
/** Tests only: drop the shared controller between cases. */
export const resetMaintenance = () => { shared = undefined; };
