import { mount } from 'svelte';
import '$lib/styles/base.css';
import { settingsCommands } from '$lib/ipc/commands';
import { setLocale } from '$lib/i18n/index.svelte';
import Root from './Root.svelte';
void settingsCommands.get().then(s=>setLocale(s.app.language === 'en' ? 'en' : 'vi'));
mount(Root, {target: document.getElementById('app')!});
