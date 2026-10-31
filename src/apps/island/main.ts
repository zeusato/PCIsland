import { mount } from 'svelte';
import House from '@lucide/svelte/icons/house';
import '$lib/styles/base.css';
import './island.css';
import { inTauri, islandCommands } from '$lib/ipc/commands';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import { startLiveStores } from '$lib/stores/live.svelte';
import { startVisualizer } from '$lib/stores/visualizer.svelte';
import { startMediaStore } from '../../features/media/store.svelte';
import { registerWebPlayer } from '../../features/web_player';
import { registerTimer } from '../../features/timer';
import { registerReminders } from '../../features/reminders';
import { registerNotes } from '../../features/notes';
import IslandRoot from './IslandRoot.svelte';

registerTimer();
registerReminders();
registerNotes();
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

startLiveStores().catch((err) => console.error('live stores failed', err));
startVisualizer();
startMediaStore().catch((err) => console.error('media store failed', err));

const target = document.getElementById('app');
if (!target) throw new Error('#app missing');
export default mount(IslandRoot, { target });
