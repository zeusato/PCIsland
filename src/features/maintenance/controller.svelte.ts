import { maintenanceCommands as api } from '$lib/ipc/commands';
import type { ResourceSnapshot } from '$lib/ipc/bindings/ResourceSnapshot';
import type { CleanupResult } from '$lib/ipc/bindings/CleanupResult';
import type { CleanupProgress } from '$lib/ipc/bindings/CleanupProgress';
import type { MemoryProcess } from '$lib/ipc/bindings/MemoryProcess';
import type { MemoryReclaim } from '$lib/ipc/bindings/MemoryReclaim';
import type { MemoryMeasurement } from '$lib/ipc/bindings/MemoryMeasurement';

export const bytes = (n: number) => {
  const unit = n >= 1024 ** 3 ? 'GiB' : n >= 1024 ** 2 ? 'MiB' : n >= 1024 ? 'KiB' : 'B';
  const divisor = { GiB: 1024 ** 3, MiB: 1024 ** 2, KiB: 1024, B: 1 }[unit];
  return `${(n / divisor).toFixed(unit === 'B' ? 0 : 1)} ${unit}`;
};

/** One controller per mounted panel. Timers stop on unmount/hidden window. */
export function createMaintenance() {
  const state = $state({ resources: null as ResourceSnapshot | null, diskBefore: null as number | null,
    logs: null as CleanupResult | null, cleaned: null as CleanupResult | null,
    cleanup: null as CleanupProgress | null,
    processes: [] as MemoryProcess[], selected: [] as number[], scanned: false,
    reclaim: null as MemoryReclaim | null, later: null as MemoryMeasurement | null,
    ramBefore: null as number | null, ramAfter: null as number | null,
    busy: false, error: false, samplingError: false, waiting: false });
  let alive = false; let sampling = false; let poll: ReturnType<typeof setInterval> | undefined;
  let laterTimer: ReturnType<typeof setTimeout> | undefined;
  async function refresh() {
    if (!alive || sampling || document.hidden) return;
    sampling = true;
    try { const snapshot = await api.resources(); if (alive) { state.resources = snapshot; state.samplingError = false; } }
    catch { if (alive) state.samplingError = true; }
    finally { sampling = false; }
  }
  async function run(task: () => Promise<void>) {
    if (!alive || state.busy) return;
    state.busy = true; state.error = false;
    try { await task(); } catch { if (alive) state.error = true; }
    finally { if (alive) state.busy = false; }
  }
  const visibility = () => { if (!document.hidden) void refresh(); };
  return { state, refresh,
    clean: () => run(async () => {
      state.cleanup = { phase: 'scan', category: 'temp', scanned: 0, total: 0, processed: 0, files: 0, bytes: 0, skipped: 0, busyApps: 0, limited: false };
      try {
        const result = await api.clean(progress => { if (alive) state.cleanup = progress; });
        if (alive) state.cleanup = result;
      } catch (error) {
        if (alive && state.cleanup) state.cleanup.phase = 'failed';
        throw error;
      }
      await refresh();
    }),
    start() { alive = true; void refresh(); poll = setInterval(() => void refresh(), 5000); document.addEventListener('visibilitychange', visibility);
      return () => { alive = false; clearInterval(poll); clearTimeout(laterTimer); document.removeEventListener('visibilitychange', visibility); };
    },
    scan: () => run(async () => { const rows = await api.processes(); if (alive) { state.processes = rows; state.selected = []; state.scanned = true; } }),
    select(pid: number, checked: boolean) { if (state.busy) return; state.selected = state.selected.filter(p => p !== pid); if (checked && state.processes.some(p => p.target.pid === pid && p.reason === 'candidate')) state.selected.push(pid); },
    selectCandidates() { if (!state.busy) state.selected = state.processes.filter(p => p.reason === 'candidate').slice(0, 128).map(p => p.target.pid); },
    reclaim: () => run(async () => {
      const targets = state.processes.filter(p => state.selected.includes(p.target.pid) && p.reason === 'candidate').map(p => p.target);
      if (!targets.length) return;
      clearTimeout(laterTimer); state.later = null; state.waiting = false;
      state.ramBefore = (await api.resources()).ramAvailable;
      const result = await api.reclaim(targets);
      if (!alive) return;
      state.reclaim = result; state.selected = []; state.ramAfter = (await api.resources()).ramAvailable;
      if (!alive) return;
      state.waiting = result.processed > 0;
      if (state.waiting) laterTimer = setTimeout(() => {
        void api.measure(result.targets).then(m => { if (alive) state.later = m; }).catch(() => { if (alive) state.error = true; }).finally(() => { if (alive) state.waiting = false; });
      }, 30000);
      await refresh();
    }),
    scanLogs: () => run(async () => { const result = await api.logs(false); if (alive) state.logs = result; }),
    cleanLogs: () => run(async () => { const result = await api.logs(true); if (alive) { state.cleaned = result; state.logs = null; } await refresh(); }),
    open: (tool: 'storage' | 'task_manager') => run(async () => { if (tool === 'storage') state.diskBefore = (await api.resources()).diskFree; await api.open(tool); }),
  };
}
