// Content transitions for island morphs (docs/04-design-system.md §6).
import { cubicOut } from 'svelte/easing';
import type { TransitionConfig } from 'svelte/transition';
import { prefersReducedMotion } from './springs';

/** Enter: fade + unblur + slight rise, delayed until the container is ~40% morphed. */
export function contentIn(_node: Element, { delay = 90 }: { delay?: number } = {}): TransitionConfig {
  if (prefersReducedMotion()) return { duration: 120, css: (t) => `opacity:${t}` };
  return {
    delay,
    duration: 180,
    easing: cubicOut,
    css: (t, u) => `opacity:${t}; filter:blur(${4 * u}px); transform:translateY(${4 * u}px)`,
  };
}

/** Exit: quick fade + blur. */
export function contentOut(_node: Element): TransitionConfig {
  if (prefersReducedMotion()) return { duration: 120, css: (t) => `opacity:${t}` };
  return { duration: 120, css: (t, u) => `opacity:${t}; filter:blur(${4 * u}px)` };
}
