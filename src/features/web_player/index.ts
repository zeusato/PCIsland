import { Globe } from '@lucide/svelte';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import QuickPanel from './views/QuickPanel.svelte';

export function registerWebPlayer(): void {
  registerQuickAction({
    id: 'web_player.open',
    labelKey: 'island.launcher.online',
    icon: Globe,
    order: 60,
    panel: QuickPanel,
  });
}
