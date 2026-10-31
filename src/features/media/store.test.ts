import { describe, expect, it } from 'vitest';
import { formatTime, positionAt } from './store.svelte';
import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';

const base: NowPlayingDto = {
  sourceId: 'external:Spotify.exe',
  appName: 'Spotify',
  title: 'Song',
  artist: null,
  album: null,
  artworkUrl: null,
  status: 'playing',
  positionMs: 10_000,
  positionAt: 1_000,
  durationMs: 60_000,
  rate: 1,
  caps: { playPause: true, next: true, previous: true, seek: true },
  isAd: false,
};

describe('positionAt', () => {
  it('interpolates while playing', () => {
    expect(positionAt(base, 6_000)).toBe(15_000);
  });
  it('does not move while paused', () => {
    expect(positionAt({ ...base, status: 'paused' }, 6_000)).toBe(10_000);
  });
  it('clamps to duration and honours rate', () => {
    expect(positionAt({ ...base, rate: 2 }, 101_000)).toBe(60_000);
  });
  it('returns null without a timeline', () => {
    expect(positionAt({ ...base, positionMs: null })).toBeNull();
  });
});

describe('formatTime', () => {
  it('formats minutes and hours', () => {
    expect(formatTime(65_000)).toBe('1:05');
    expect(formatTime(3_725_000)).toBe('1:02:05');
  });
});
