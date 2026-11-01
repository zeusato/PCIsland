import Package from '@lucide/svelte/icons/package';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import { live } from '$lib/stores/live.svelte';
import { t } from '$lib/i18n/index.svelte';
import QuickPanel from './views/QuickPanel.svelte';

export function registerShelf(): void {
  registerQuickAction({ id: 'shelf.open', labelKey: 'main.nav.shelf', icon: Package, order: 70, panel: QuickPanel,
    status: () => t('shelf.itemCount', { count: live.shelf.length }),
  });
}
