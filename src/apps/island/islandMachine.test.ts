import { describe, expect, it } from 'vitest';
import { initialIslandState, islandMachine, type IslandEvent, type IslandUiState } from './islandMachine';

const run = (events: IslandEvent[], from: IslandUiState = initialIslandState) =>
  events.reduce(islandMachine, from);

const visible = { type: 'visibility', mode: 'normal', hasLiveActivity: false } as const;

describe('islandMachine', () => {
  it('opens peek after hover dwell and collapses after leave timeout', () => {
    const peek = run([{ type: 'pointerDwell' }]);
    expect(peek.mode).toBe('peek');
    expect(run([{ type: 'pointerLeaveTimeout' }], peek).mode).toBe('compact');
  });

  it('keeps a clicked peek open when the pointer leaves', () => {
    const s = run([{ type: 'click' }, { type: 'pointerLeaveTimeout' }]);
    expect(s).toMatchObject({ mode: 'peek', pinned: true });
  });

  it('closes a pinned peek on click outside, escape or hotkey', () => {
    for (const close of ['clickOutside', 'escape', 'hotkeyToggle'] as const) {
      const s = run([{ type: 'click' }, { type: close }]);
      expect(s).toMatchObject({ mode: 'compact', pinned: false });
    }
  });

  it('returns to peek after an alert that interrupted a peek', () => {
    const s = run([{ type: 'click' }, { type: 'alertShown', critical: false }]);
    expect(s.mode).toBe('alert');
    expect(run([{ type: 'alertEnded' }], s).mode).toBe('peek');
  });

  it('ignores pointer input while an alert is shown', () => {
    const s = run([{ type: 'alertShown', critical: true }, { type: 'pointerLeaveTimeout' }, { type: 'escape' }]);
    expect(s).toMatchObject({ mode: 'alert', critical: true });
  });

  it('drops non-critical alerts while hidden but shows critical ones', () => {
    const hidden = run([{ type: 'visibility', mode: 'hidden', hasLiveActivity: false }]);
    expect(run([{ type: 'alertShown', critical: false }], hidden).mode).toBe('hidden');
    expect(run([{ type: 'alertShown', critical: true }], hidden).mode).toBe('alert');
  });

  it('restores the previous presentation when visible again', () => {
    const s = run([{ type: 'click' }, { type: 'visibility', mode: 'hidden', hasLiveActivity: false }, visible]);
    expect(s.mode).toBe('peek');
  });

  it('honors backend minimal policy even with a live activity', () => {
    expect(run([{ type: 'visibility', mode: 'minimal', hasLiveActivity: false }]).mode).toBe('minimal');
    expect(run([{ type: 'visibility', mode: 'minimal', hasLiveActivity: true }]).mode).toBe('minimal');
  });

  it('wakes from minimal on pointer enter', () => {
    const s = run([{ type: 'visibility', mode: 'minimal', hasLiveActivity: false }, { type: 'pointerEnter' }]);
    expect(s.mode).toBe('compact');
  });

  it('does not shrink an open peek into minimal', () => {
    const s = run([{ type: 'click' }, { type: 'visibility', mode: 'minimal', hasLiveActivity: false }]);
    expect(s.mode).toBe('peek');
  });

  it('returns to minimal after hovering, closing a peek, or ending an alert', () => {
    const minimal = run([{ type: 'visibility', mode: 'minimal', hasLiveActivity: true }]);
    expect(run([{ type: 'pointerEnter' }, { type: 'pointerLeaveTimeout' }], minimal).mode).toBe('minimal');
    expect(run([{ type: 'click' }, { type: 'escape' }], minimal).mode).toBe('minimal');
    expect(run([{ type: 'alertShown', critical: false }, { type: 'alertEnded' }], minimal).mode).toBe('minimal');
  });
});
