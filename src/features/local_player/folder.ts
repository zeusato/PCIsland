import { playerCommands } from '$lib/ipc/commands';

/** Scan, then queue only tracks inside the selected folder, including subfolders. */
export async function playFolder(path: string): Promise<boolean> {
  await playerCommands.addFolder(path);
  await playerCommands.rescan();
  const normalize = (value: string) => value.replaceAll('\\', '/').toLowerCase();
  const prefix = normalize(path).replace(/\/+$/, '') + '/';
  const ids: string[] = [];
  for (let offset = 0; ; offset += 500) {
    const tracks = await playerCommands.query('', 500, offset);
    ids.push(...tracks.filter((track) => normalize(track.path).startsWith(prefix)).map((track) => track.id));
    if (tracks.length < 500) break;
  }
  if (!ids.length) return false;
  await playerCommands.playTracks(ids);
  return true;
}
