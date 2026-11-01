import { getContext } from 'svelte';

export const ISLAND_INTERACTION = Symbol('island-interaction');
export interface IslandInteraction {
  measure: (node: HTMLElement) => { update: (height: number) => void; destroy: () => void };
  showPlayer: (source: string) => void;
  showFeature?: (id: string) => void;
}

/** Observe intrinsic content, never the animated island container. */
export function islandInteraction() {
  const context = getContext<IslandInteraction | undefined>(ISLAND_INTERACTION);
  return {
    showPlayer: (source: string) => context?.showPlayer(source),
    showFeature: (id: string) => context?.showFeature?.(id),
    fit: (node: HTMLElement, enabled = true) => {
      let stop = () => {};
      const update = (enabled: boolean) => {
        stop();
        if (!enabled || !context || typeof ResizeObserver === 'undefined') return;
        const measurement = context.measure(node);
        const observer = new ResizeObserver(() => measurement.update(Math.ceil(node.getBoundingClientRect().height)));
        observer.observe(node);
        measurement.update(Math.ceil(node.getBoundingClientRect().height));
        stop = () => { observer.disconnect(); measurement.destroy(); };
      };
      update(enabled);
      return { update, destroy: () => stop() };
    },
  };
}
