// Presentation state machine of the island — docs/modules/island-window.md §4.
// Pure reducer: timers and side effects live in IslandRoot.svelte.

export type Presentation = 'compact' | 'peek' | 'alert' | 'minimal' | 'hidden';
export type VisibilityMode = 'hidden' | 'minimal' | 'normal';

export interface IslandUiState {
  mode: Presentation;
  /** Peek opened by click stays open until dismissed explicitly. */
  pinned: boolean;
  /** Where to return after an alert or after becoming visible again. */
  resumeTo: 'compact' | 'peek';
  /** A critical alert blocks collapsing until acknowledged. */
  critical: boolean;
}

export type IslandEvent =
  | { type: 'pointerDwell' }
  | { type: 'pointerLeaveTimeout' }
  | { type: 'pointerEnter' }
  | { type: 'click' }
  | { type: 'clickOutside' }
  | { type: 'escape' }
  | { type: 'hotkeyToggle' }
  | { type: 'alertShown'; critical: boolean }
  | { type: 'alertEnded' }
  | { type: 'visibility'; mode: VisibilityMode; hasLiveActivity: boolean };

export const initialIslandState: IslandUiState = {
  mode: 'compact',
  pinned: false,
  resumeTo: 'compact',
  critical: false,
};

export function islandMachine(state: IslandUiState, event: IslandEvent): IslandUiState {
  switch (event.type) {
    case 'visibility':
      return applyVisibility(state, event.mode, event.hasLiveActivity);

    case 'alertShown':
      if (state.mode === 'hidden' && !event.critical) return state;
      return {
        ...state,
        mode: 'alert',
        critical: event.critical,
        resumeTo: state.mode === 'peek' ? 'peek' : state.mode === 'alert' ? state.resumeTo : 'compact',
      };

    case 'alertEnded':
      if (state.mode !== 'alert') return { ...state, critical: false };
      return { ...state, mode: state.resumeTo, critical: false };
  }

  // Alerts own the island until they end; hidden ignores pointer input.
  if (state.mode === 'alert' || state.mode === 'hidden') return state;

  switch (event.type) {
    case 'pointerEnter':
      return state.mode === 'minimal' ? { ...state, mode: 'compact' } : state;

    case 'pointerDwell':
      return state.mode === 'compact' ? { ...state, mode: 'peek' } : state;

    case 'click':
      if (state.mode === 'compact' || state.mode === 'minimal') return { ...state, mode: 'peek', pinned: true };
      if (state.mode === 'peek') return { ...state, pinned: true };
      return state;

    case 'pointerLeaveTimeout':
      return state.mode === 'peek' && !state.pinned ? { ...state, mode: 'compact' } : state;

    case 'clickOutside':
    case 'escape':
      return state.mode === 'peek' ? { ...state, mode: 'compact', pinned: false } : state;

    case 'hotkeyToggle':
      if (state.mode === 'peek') return { ...state, mode: 'compact', pinned: false };
      return { ...state, mode: 'peek', pinned: true };
  }
}

function applyVisibility(
  state: IslandUiState,
  mode: VisibilityMode,
  hasLiveActivity: boolean,
): IslandUiState {
  if (mode === 'hidden') {
    if (state.mode === 'hidden') return state;
    const resumeTo = state.mode === 'peek' ? 'peek' : 'compact';
    return { ...state, mode: 'hidden', resumeTo, pinned: state.mode === 'peek' && state.pinned };
  }
  if (mode === 'minimal' && !hasLiveActivity) {
    if (state.mode === 'alert' || state.mode === 'peek') return state;
    return { ...state, mode: 'minimal' };
  }
  // normal (or minimal suppressed by a live activity)
  if (state.mode === 'hidden') return { ...state, mode: state.resumeTo };
  if (state.mode === 'minimal') return { ...state, mode: 'compact' };
  return state;
}
