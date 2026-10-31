// Native file/folder pickers (docs/02-system-design.md §12: no fs access in the frontend).
import { open } from '@tauri-apps/plugin-dialog';

export async function pickFolder(): Promise<string | null> {
  const r = await open({ directory: true, multiple: false });
  return typeof r === 'string' ? r : null;
}

export async function pickAudioFiles(): Promise<string[]> {
  const r = await open({ multiple: true, filters: [{ name: 'Audio', extensions: ['mp3', 'flac', 'wav', 'ogg', 'm4a', 'aac'] }] });
  return Array.isArray(r) ? r : typeof r === 'string' ? [r] : [];
}
