import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { mediaCommands } from '$lib/ipc/commands';
import { pickFolder } from '$lib/ipc/dialog';
import { playFolder } from '../../local_player/folder';
import QuickPanel from './QuickPanel.svelte';
const interaction = vi.hoisted(() => ({ showPlayer: vi.fn() }));
vi.mock('$lib/components/islandInteraction', () => ({ islandInteraction: () => ({ ...interaction, fit: () => ({}) }) }));
vi.mock('$lib/ipc/dialog', () => ({ pickFolder: vi.fn() }));
vi.mock('../../local_player/folder', () => ({ playFolder: vi.fn() }));
afterEach(() => { vi.restoreAllMocks(); vi.clearAllMocks(); });

it.each([null, 'D:/Music'])('uses a folder picker and opens the local player only after a selection (%s)', async (path) => {
  vi.mocked(pickFolder).mockResolvedValue(path);
  vi.mocked(playFolder).mockResolvedValue(true);
  const select = vi.spyOn(mediaCommands, 'selectSource').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(QuickPanel, { target, props: { close: vi.fn() } });
  const button = (text: string) => [...target.querySelectorAll('button')].find((node) => node.textContent === text)!;
  try {
    flushSync(); button('Nhạc trên máy').click(); flushSync();
    button('Chọn thư mục').click();
    await vi.waitFor(() => { flushSync(); expect(button('Chọn thư mục')).toBeDefined(); });
    expect(pickFolder).toHaveBeenCalledOnce();
    if (path) {
      expect(playFolder).toHaveBeenCalledWith(path);
      expect(select).toHaveBeenCalledWith('local');
      expect(interaction.showPlayer).toHaveBeenCalledWith('local');
    } else {
      expect(playFolder).not.toHaveBeenCalled();
      expect(interaction.showPlayer).not.toHaveBeenCalled();
    }
  } finally { await unmount(component); }
});
