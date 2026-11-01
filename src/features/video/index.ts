import Clapperboard from '@lucide/svelte/icons/clapperboard';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';

/** Launcher entry; the `videoOpen` hotkey opens the same panel (island-window §4). */
export function registerVideo(): void {
  registerQuickAction({
    id: 'video.open',
    labelKey: 'main.nav.video',
    icon: Clapperboard,
    order: 15,
    panel: QuickPanel,
  });
}
