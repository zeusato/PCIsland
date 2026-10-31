// Formatting helpers (docs/04-design-system.md §9).
import { getLocale } from '$lib/i18n/index.svelte';

export function formatDuration(ms: number, tenths = false): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, '0');
  const t = tenths ? `.${Math.floor((ms % 1000) / 100)}` : '';
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}${t}` : `${m}:${s}${t}`;
}

/** Remaining ms rounded up so a countdown shows 0:01 until it really ends. */
export function ceilSeconds(ms: number): number {
  return Math.ceil(Math.max(0, ms) / 1000) * 1000;
}

export function formatTime(epochMs: number, hour12 = false): string {
  return new Date(epochMs).toLocaleTimeString(getLocale() === 'vi' ? 'vi-VN' : 'en-GB', {
    hour: '2-digit',
    minute: '2-digit',
    hour12,
  });
}

export function formatDay(epochMs: number): string {
  const d = new Date(epochMs);
  const today = new Date();
  const tomorrow = new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1);
  const vi = getLocale() === 'vi';
  if (d.toDateString() === today.toDateString()) return vi ? 'Hôm nay' : 'Today';
  if (d.toDateString() === tomorrow.toDateString()) return vi ? 'Ngày mai' : 'Tomorrow';
  return d.toLocaleDateString(vi ? 'vi-VN' : 'en-GB', { weekday: 'short', day: '2-digit', month: '2-digit' });
}

export function formatWhen(epochMs: number, hour12 = false): string {
  return `${formatDay(epochMs)} ${formatTime(epochMs, hour12)}`;
}
