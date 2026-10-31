// Drag files out of PCisland into Explorer, mail, chat… (docs/modules/file-shelf.md §3).
import { startDrag } from '@crabnebula/tauri-plugin-drag';
import { shelfCommands } from './commands';

let icon: string | null = null;

export async function dragFilesOut(paths: string[]): Promise<void> {
  icon ??= await shelfCommands.dragIcon();
  await startDrag({ item: paths, icon, mode: 'copy' });
}
