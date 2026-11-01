import { describe, expect, it } from 'vitest';
import { formatTime, syncStep } from './sync';

describe('syncStep', () => {
  it('leaves small drift alone', () => {
    expect(syncStep(0.016)).toEqual({ kind: 'none' });
    expect(syncStep(-0.03)).toEqual({ kind: 'none' });
  });
  it('nudges the video rate toward the audio', () => {
    expect(syncStep(0.04)).toEqual({ kind: 'rate', rate: 0.97 });
    expect(syncStep(-0.2)).toEqual({ kind: 'rate', rate: 1.03 });
  });
  it('jumps when far apart', () => {
    expect(syncStep(1.5)).toEqual({ kind: 'jump' });
    expect(syncStep(-0.41)).toEqual({ kind: 'jump' });
  });
});

describe('formatTime', () => {
  it('formats minutes and hours', () => {
    expect(formatTime(0)).toBe('0:00');
    expect(formatTime(75.9)).toBe('1:15');
    expect(formatTime(3723)).toBe('1:02:03');
    expect(formatTime(Number.NaN)).toBe('0:00');
  });
});
