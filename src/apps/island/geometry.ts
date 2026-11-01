// Island geometry per presentation (docs/04-design-system.md §5), logical px.
import type { Presentation } from './islandMachine';

export interface Geometry {
  w: number;
  h: number;
  /** Bottom corner radius (top corners are square: attached style). */
  r: number;
}

export const WINDOW_W = 480;
export const MAX_PEEK_H = 600;
export const SWITCHER_H = 28;
/** Invisible hover strip above a minimal island. */
export const MINIMAL_HOVER = { w: 160, h: 14 };
export const BUBBLE = { w: 32, h: 32, gap: 6 };

export type Content = 'idle' | 'activity' | 'media' | 'drop' | 'alert-small' | 'alert-large' | 'launcher';

const PEEK_HEIGHT: Record<string, number> = {
  media: 172,
  timer: 168,
  reminders: 140,
  notes: 176,
  shelf: 196,
  launcher: 220,
};

export function geometryFor(mode: Presentation, content: Content, source = '', contentHeight = 0, showSwitcher = true): Geometry {
  if (content === 'drop') return { w: 400, h: 220, r: 28 };
  switch (mode) {
    case 'minimal':
      return { w: 96, h: 4, r: 2 };
    case 'peek':
      return { w: 400, h: Math.min(MAX_PEEK_H, Math.max(PEEK_HEIGHT[content === 'launcher' ? 'launcher' : source] ?? 160, contentHeight) + (showSwitcher ? SWITCHER_H : 0)), r: 28 };
    case 'alert':
      return content === 'alert-large' ? { w: 400, h: 150, r: 28 } : { w: 260, h: 40, r: 20 };
    case 'compact':
    case 'hidden':
      if (content === 'idle') return { w: 120, h: 30, r: 15 };
      return { w: content === 'media' ? 200 : 180, h: 32, r: 16 };
  }
}

/** Hit rectangles for the island (and the secondary tab) anchored at the top centre. */
export function hitRects(g: Geometry, mode: Presentation, bubble: boolean) {
  const w = mode === 'minimal' ? Math.max(g.w, MINIMAL_HOVER.w) : g.w;
  const h = mode === 'minimal' ? Math.max(g.h, MINIMAL_HOVER.h) : g.h;
  const rects = [{ x: (WINDOW_W - w) / 2, y: 0, width: w, height: h }];
  if (bubble) rects.push({ x: (WINDOW_W + g.w) / 2 + BUBBLE.gap, y: 0, width: BUBBLE.w, height: BUBBLE.h });
  return rects;
}

export function union(a: Geometry, b: Geometry): Geometry {
  return { w: Math.max(a.w, b.w), h: Math.max(a.h, b.h), r: Math.max(a.r, b.r) };
}
