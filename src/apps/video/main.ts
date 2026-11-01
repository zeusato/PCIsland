import { mount } from 'svelte';
import '$lib/styles/base.css';
import { inTauri, settingsCommands } from '$lib/ipc/commands';
import { setLocale } from '$lib/i18n/index.svelte';
import VideoRoot from './VideoRoot.svelte';

if (inTauri) {
  settingsCommands
    .get()
    .then((s) => {
      setLocale(s.app.language === 'en' ? 'en' : 'vi');
      document.documentElement.dataset.accent = s.ui.accent;
    })
    .catch(() => {});
}

const target = document.getElementById('app');
if (!target) throw new Error('#app missing');
export default mount(VideoRoot, { target });
