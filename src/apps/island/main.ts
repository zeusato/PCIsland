import { mount } from 'svelte';
import { House } from '@lucide/svelte';
import '$lib/styles/base.css';
import './island.css';
import { inTauri, islandCommands } from '$lib/ipc/commands';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import { startMediaStore } from '../../features/media/store.svelte';
import { registerWebPlayer } from '../../features/web_player';
import IslandRoot from './IslandRoot.svelte';

registerWebPlayer();
registerQuickAction({
  id: 'app.open_main',
  labelKey: 'island.launcher.openMain',
  icon: House,
  order: 100,
  run: () => {
    if (inTauri) islandCommands.openMain('/home');
  },
});

startMediaStore().catch((err) => console.error('media store failed', err));

const target = document.getElementById('app');
if (!target) throw new Error('#app missing');
export default mount(IslandRoot, { target });
