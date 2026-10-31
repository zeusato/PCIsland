import { mount } from 'svelte';
import '$lib/styles/base.css';
import { startMediaStore } from '../../features/media/store.svelte';
import MainRoot from './MainRoot.svelte';

startMediaStore().catch((err) => console.error('media store failed', err));

const target = document.getElementById('app');
if (!target) throw new Error('#app missing');
export default mount(MainRoot, { target });
