import { flushSync, mount, unmount } from 'svelte';
import { expect, it, vi } from 'vitest';
import VideoRoot from './VideoRoot.svelte';
import type { PlaybackSource } from '$lib/ipc/bindings/PlaybackSource';
import { videoCommands } from '$lib/ipc/video';

vi.mock('$lib/ipc/commands', () => ({ inTauri: true }));
vi.mock('$lib/ipc/video', () => ({
  onVideoSource: vi.fn(async () => () => {}),
  onVideoCommand: vi.fn(async () => () => {}),
  onVideoState: vi.fn(async () => () => {}),
  videoCommands: {
    current: vi.fn(), state: vi.fn(async () => ({ pinned: false })),
    report: vi.fn(async () => {}), setQuality: vi.fn(),
  },
}));

it('plays the Vietnamese URL, switches audio at the same position and retains English after a quality change', async () => {
  const media = HTMLMediaElement.prototype;
  vi.spyOn(media, 'pause').mockImplementation(() => {});
  vi.spyOn(media, 'load').mockImplementation(() => {});
  vi.spyOn(media, 'play').mockResolvedValue(undefined);
  const english = 'http://vstream.localhost/en';
  const vietnamese = 'http://vstream.localhost/vi';
  const source: PlaybackSource = {
    kind: 'pair', audio: english, video: 'http://vstream.localhost/video',
    audioLanguages: { 'en-US': english, 'vi-VN': vietnamese }, quality: 720,
    autoMaxHeight: 1080, qualities: [2160, 1440, 720, 360], startMs: 0, durationMs: 100000, resumedFromMs: null,
    pageUrl: '', title: '', uploader: null, isLive: false, rev: 1,
  };
  vi.mocked(videoCommands.current).mockResolvedValue(source);
  vi.mocked(videoCommands.setQuality).mockResolvedValue({
    ...source, rev: 2, quality: 2160, startMs: 12000,
    audioLanguages: { 'en-US': english + '2', 'vi-VN': vietnamese + '2' },
  });
  const target = document.createElement('div');
  const app = mount(VideoRoot, { target });
  try {
    flushSync();
    const audio = target.querySelector('audio')!;
    await vi.waitFor(() => expect(audio.src).toBe(vietnamese));
    const select = target.querySelector<HTMLSelectElement>('select.language')!;
    expect(select.value).toBe('vi-VN');
    target.querySelector<HTMLButtonElement>('footer button')!.click();
    audio.currentTime = 12;
    select.value = 'en-US';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    expect(audio.src).toBe(english);
    audio.currentTime = 0;
    audio.dispatchEvent(new Event('loadedmetadata'));
    expect(audio.currentTime).toBe(12);
    expect(media.play).not.toHaveBeenCalled();
    const quality = target.querySelector<HTMLSelectElement>('footer select')!;
    expect([...quality.options].map((option) => option.value)).toEqual(['auto', '2160', '1440', '720', '360']);
    quality.value = '2160';
    quality.dispatchEvent(new Event('change', { bubbles: true }));
    expect(videoCommands.setQuality).toHaveBeenCalledWith(2160, 12000);
    await vi.waitFor(() => expect(audio.src).toBe(english + '2'));
  } finally {
    await unmount(app);
    vi.restoreAllMocks();
  }
});
