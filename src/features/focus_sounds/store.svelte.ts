// Downloadable focus music (docs/modules/focus-sounds.md). One controller per window, shared
// by Settings and the first-run welcome so a running download keeps its progress.
import { focusSoundsCommands as api, inTauri } from '$lib/ipc/commands';
import { live, updateSettings } from '$lib/stores/live.svelte';
import type { FocusSoundsStatus } from '$lib/ipc/bindings/FocusSoundsStatus';
import type { FocusCatalog } from '$lib/ipc/bindings/FocusCatalog';
import type { FocusDownloadProgress } from '$lib/ipc/bindings/FocusDownloadProgress';

export const focusSounds = $state({
  status: null as FocusSoundsStatus | null,
  catalog: null as FocusCatalog | null,
  loadingCatalog: false,
  catalogError: false,
  busy: false,
  progress: null as FocusDownloadProgress | null,
  error: '',
});

export async function loadStatus() {
  if (!inTauri) return;
  try { focusSounds.status = await api.status(); } catch { /* Disk read only; shown as nothing installed. */ }
}

export async function loadCatalog() {
  if (focusSounds.loadingCatalog) return;
  focusSounds.loadingCatalog = true; focusSounds.catalogError = false;
  try { focusSounds.catalog = await api.catalog(); }
  catch { focusSounds.catalogError = true; }
  finally { focusSounds.loadingCatalog = false; }
}

const selectedPacks = () => live.settings?.pomodoro.focusSoundPacks ?? [];
const union = (a: string[], b: string[]) => [...a, ...b.filter((x) => !a.includes(x))];

/** Downloads packs and turns them on for Pomodoro; resolves false on error or cancel. */
export async function downloadPacks(ids: string[]): Promise<boolean> {
  if (focusSounds.busy || !ids.length) return false;
  focusSounds.busy = true; focusSounds.error = ''; focusSounds.progress = null;
  try {
    focusSounds.status = await api.download(ids, (p) => { focusSounds.progress = p; });
    const done = ids.filter((id) => focusSounds.status?.packs.some((p) => p.id === id && p.complete));
    const next = union(selectedPacks(), done);
    if (next.length !== selectedPacks().length) await updateSettings('pomodoro', { focusSoundPacks: next });
    await loadCatalog();
    return true;
  } catch (e) {
    focusSounds.error = typeof e === 'object' && e && 'message' in e ? String(e.message) : String(e);
    return false;
  } finally {
    focusSounds.busy = false; focusSounds.progress = null;
  }
}

export const cancelDownload = () => api.cancel();

export async function removePack(id: string) {
  if (focusSounds.busy) return;
  try {
    focusSounds.status = await api.remove([id]);
    await updateSettings('pomodoro', { focusSoundPacks: selectedPacks().filter((x) => x !== id) });
    if (focusSounds.catalog) await loadCatalog();
  } catch { focusSounds.error = 'remove'; }
}

export function usePack(id: string, on: boolean) {
  const current = selectedPacks();
  void updateSettings('pomodoro', { focusSoundPacks: on ? union(current, [id]) : current.filter((x) => x !== id) });
}

export const mib = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(bytes < 10 * 1024 * 1024 ? 1 : 0)} MB`;
