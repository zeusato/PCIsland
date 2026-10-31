// Island geometry per presentation (docs/04-design-system.md §5), logical px.
import type { Presentation } from './islandMachine';

export interface Geometry {
  w: number;
  h: number;
  /** Bottom corner radius (top corners are square: attached style). */
  r: number;
}

export const WINDOW_W = 480;
/** Invisible hover strip above a minimal island. */
export const MINIMAL_HOVER = { w: 160, h: 14 };

export function geometryFor(mode: Presentation, hasMedia: boolean): Geometry {
  switch (mode) {
    case 'minimal':
      return { w: 96, h: 4, r: 2 };
    case 'peek':
      return hasMedia ? { w: 400, h: 160, r: 28 } : { w: 400, h: 150, r: 28 };
    case 'alert':
      return { w: 260, h: 40, r: 20 };
    case 'compact':
    case 'hidden':
      return hasMedia ? { w: 200, h: 32, r: 16 } : { w: 120, h: 30, r: 15 };
  }
}

/** Hit rectangle for the island anchored at the top centre of the window. */
export function hitRect(g: Geometry, mode: Presentation) {
  const w = mode === 'minimal' ? Math.max(g.w, MINIMAL_HOVER.w) : g.w;
  const h = mode === 'minimal' ? Math.max(g.h, MINIMAL_HOVER.h) : g.h;
  return { x: (WINDOW_W - w) / 2, y: 0, width: w, height: h };
}

export function union(a: Geometry, b: Geometry): Geometry {
  return { w: Math.max(a.w, b.w), h: Math.max(a.h, b.h), r: Math.max(a.r, b.r) };
}
