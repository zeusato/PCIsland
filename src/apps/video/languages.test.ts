import { describe, expect, it } from 'vitest';
import { languageLabel, manifestLanguage, preferredTrack } from './languages';

const tracks = [
  { lang: 'en', name: 'English' },
  { lang: 'vi-VN', name: 'Vietnamese' },
];

describe('video audio language', () => {
  it('opens in Vietnamese and respects an explicit switch to English', () => {
    expect(preferredTrack(tracks, manifestLanguage, 'vi')).toBe(tracks[1]);
    expect(preferredTrack(tracks, manifestLanguage, 'en')).toBe(tracks[0]);
  });

  it('leaves the source default intact when Vietnamese is missing', () => {
    expect(preferredTrack(tracks.slice(0, 1), manifestLanguage, 'vi')).toBeUndefined();
    expect(preferredTrack([], manifestLanguage, 'vi')).toBeUndefined();
  });

  it('recognizes an HLS rendition labeled by name only', () => {
    expect(manifestLanguage({ name: 'Tiếng Việt', lang: 'und' })).toBe('vi');
    expect(manifestLanguage({ name: 'Vietnamese (dubbed)' })).toBe('vi');
  });

  it('displays language tags in the UI locale and keeps free-form track names', () => {
    expect(languageLabel('vi', 'vi', '')).toMatch(/việt/i);
    expect(languageLabel('English (original)', 'vi', '')).toBe('English (original)');
    expect(languageLabel('', 'vi', 'Mặc định')).toBe('Mặc định');
  });
});
