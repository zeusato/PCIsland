// Spring presets — docs/04-design-system.md §6. Components never set spring params themselves.
export interface SpringPreset {
  stiffness: number;
  damping: number;
}

export const springs = {
  /** Island size/radius morph; slight overshoot. */
  morph: { stiffness: 0.12, damping: 0.62 },
  /** Buttons, hover, chips; no overshoot. */
  snappy: { stiffness: 0.2, damping: 0.8 },
  /** Progress bars, numeric transitions. */
  gentle: { stiffness: 0.06, damping: 0.9 },
} as const satisfies Record<string, SpringPreset>;

export function prefersReducedMotion(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Morph preset, degraded to `snappy` when reduced motion is on. */
export function morphPreset(reduced = prefersReducedMotion()): SpringPreset {
  return reduced ? springs.snappy : springs.morph;
}
