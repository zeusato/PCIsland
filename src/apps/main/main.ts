import { mount } from 'svelte';
import '$lib/styles/base.css';
import { startMediaStore } from '../../features/media/store.svelte';
import { startLiveStores } from '$lib/stores/live.svelte';
import MainRoot from './MainRoot.svelte';

startMediaStore().catch((err) => console.error('media store failed', err));
startLiveStores().catch((err) => console.error('live stores failed', err));

const target = document.getElementById('app');
if (!target) throw new Error('#app missing');
export default mount(MainRoot, { target });
