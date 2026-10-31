// Quick action registry: buttons shown by the island launcher (docs/modules/island-window.md §4).
// Features register their actions in their index.ts; the island never imports features directly.
import type { Component } from 'svelte';

export interface QuickAction {
  id: string;
  labelKey: string;
  icon: Component<{ size?: number }>;
  order: number;
  /** Runs immediately when clicked… */
  run?: () => void;
  state?: () => 'active' | 'break' | 'paused' | null;
  status?: () => string;
  busy?: () => boolean;
  /** …or opens this panel inside the peek (features/<module>/views/QuickPanel.svelte). */
  panel?: Component<{ close: () => void }>;
}

const registry = $state<{ actions: QuickAction[] }>({ actions: [] });

export function registerQuickAction(action: QuickAction): void {
  registry.actions = [...registry.actions.filter((a) => a.id !== action.id), action].sort(
    (a, b) => a.order - b.order,
  );
}

export function quickActions(): QuickAction[] {
  return registry.actions;
}
