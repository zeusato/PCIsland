import Music from '@lucide/svelte/icons/music';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';

export function registerWebPlayer(): void {
  registerQuickAction({
    id: 'web_player.open',
    labelKey: 'main.nav.media',
    icon: Music,
    order: 10,
    panel: QuickPanel,
  });
}
