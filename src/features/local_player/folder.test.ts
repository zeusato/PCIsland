import { afterEach, expect, it, vi } from 'vitest';
import { playerCommands } from '$lib/ipc/commands';
import { playFolder } from './folder';
const track = (id: string, path: string) => ({ id, path, title: id, artist: null, album: null, durationMs: null, hasArtwork: false });
afterEach(() => vi.restoreAllMocks());

it('scans the folder and queues its descendants across pages, excluding sibling folders', async () => {
  vi.spyOn(playerCommands, 'addFolder').mockResolvedValue({ id: 'folder', path: 'D:\\Music', lastScanAt: null });
  vi.spyOn(playerCommands, 'rescan').mockResolvedValue({ added: 2, updated: 0, removed: 0 });
  const query = vi.spyOn(playerCommands, 'query').mockResolvedValueOnce([
    track('inside', 'd:/MUSIC/a.mp3'), ...Array.from({ length: 499 }, (_, i) => track(`other${i}`, `D:/Music-other/${i}.mp3`)),
  ]).mockResolvedValueOnce([track('nested', 'D:\\Music\\Album\\b.flac')]);
  const play = vi.spyOn(playerCommands, 'playTracks').mockResolvedValue();
  expect(await playFolder('D:\\Music\\')).toBe(true);
  expect(query).toHaveBeenLastCalledWith('', 500, 500);
  expect(play).toHaveBeenCalledWith(['inside', 'nested']);
});

it('does not replace the queue when the selected folder contains no music', async () => {
  vi.spyOn(playerCommands, 'addFolder').mockResolvedValue({ id: 'folder', path: 'D:/Empty', lastScanAt: null });
  vi.spyOn(playerCommands, 'rescan').mockResolvedValue({ added: 0, updated: 0, removed: 0 });
  vi.spyOn(playerCommands, 'query').mockResolvedValue([track('other', 'D:/Other/a.mp3')]);
  const play = vi.spyOn(playerCommands, 'playTracks').mockResolvedValue();
  expect(await playFolder('D:/Empty')).toBe(false);
  expect(play).not.toHaveBeenCalled();
});
