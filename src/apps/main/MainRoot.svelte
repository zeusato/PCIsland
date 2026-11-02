<script lang="ts">
  // Full window shell: title bar, sidebar, hash router (docs/modules/full-window.md).
  import Bell from '@lucide/svelte/icons/bell';
  import Clock from '@lucide/svelte/icons/clock';
  import House from '@lucide/svelte/icons/house';
  import Minus from '@lucide/svelte/icons/minus';
  import Music from '@lucide/svelte/icons/music';
  import Clapperboard from '@lucide/svelte/icons/clapperboard';
  import NotebookPen from '@lucide/svelte/icons/notebook-pen';
  import Package from '@lucide/svelte/icons/package';
  import Settings from '@lucide/svelte/icons/settings';
  import Square from '@lucide/svelte/icons/square';
  import X from '@lucide/svelte/icons/x';
  import { t } from '$lib/i18n/index.svelte';
  import { inTauri } from '$lib/ipc/commands';
  import { currentWindow } from '$lib/ipc/window';
  import HomePage from './HomePage.svelte';
  import MusicPage from './pages/MusicPage.svelte';
  import TimerPage from './pages/TimerPage.svelte';
  import RemindersPage from './pages/RemindersPage.svelte';
  import NotesPage from './pages/NotesPage.svelte';
  import ShelfPage from './pages/ShelfPage.svelte';
  import SettingsPage from './pages/SettingsPage.svelte';
  import VideoPage from './pages/VideoPage.svelte';
  import MeetingPage from './pages/MeetingPage.svelte';
  import Welcome from './Welcome.svelte';
  import { live } from '$lib/stores/live.svelte';
  import Mic from '@lucide/svelte/icons/mic';

  const nav = [
    { route: '/home', key: 'main.nav.home', icon: House },
    { route: '/media', key: 'main.nav.media', icon: Music },
    { route: '/video', key: 'main.nav.video', icon: Clapperboard },
    { route: '/timer', key: 'main.nav.timer', icon: Clock },
    { route: '/reminders', key: 'main.nav.reminders', icon: Bell },
    { route: '/notes', key: 'main.nav.notes', icon: NotebookPen },
    { route: '/shelf', key: 'main.nav.shelf', icon: Package },
    { route: '/meeting', key: 'main.nav.meeting', icon: Mic },
    { route: '/settings', key: 'main.nav.settings', icon: Settings },
  ];

  const readRoute = () => location.hash.replace(/^#/, '') || '/home';
  let route = $state(readRoute());

  $effect(() => {
    const onHash = () => (route = readRoute());
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  });
</script>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>PCisland</span>
    {#if inTauri}
      <div class="controls">
        <button aria-label={t('main.titlebar.minimize')} onclick={() => currentWindow.minimize()}><Minus size={14} /></button>
        <button aria-label={t('main.titlebar.maximize')} onclick={() => currentWindow.toggleMaximize()}><Square size={12} /></button>
        <button class="close" aria-label={t('main.titlebar.close')} onclick={() => currentWindow.close()}><X size={14} /></button>
      </div>
    {/if}
  </header>

  <nav class="sidebar">
    {#each nav as item (item.route)}
      <a href={`#${item.route}`} class:active={route.startsWith(item.route)}>
        <item.icon size={18} />
        <span>{t(item.key)}</span>
      </a>
    {/each}
  </nav>

  <main class="content">
    {#if route.startsWith('/media') || route.startsWith('/online')}
      <MusicPage />
    {:else if route.startsWith('/video')}
      <VideoPage />
    {:else if route.startsWith('/timer')}
      <TimerPage />
    {:else if route.startsWith('/reminders')}
      <RemindersPage />
    {:else if route.startsWith('/notes')}
      <NotesPage />
    {:else if route.startsWith('/shelf')}
      <ShelfPage />
    {:else if route.startsWith('/meeting')}
      <MeetingPage />
    {:else if route.startsWith('/settings')}
      <SettingsPage />
    {:else}
      <HomePage />
    {/if}
  </main>
</div>
{#if live.settings && !live.settings.app.welcomeDone}<Welcome />{/if}

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    grid-template-rows: 40px 1fr;
    height: 100vh;
    background: var(--c-surface-1);
  }
  .titlebar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: var(--sp-6);
    border-bottom: 1px solid var(--c-border);
  }
  .brand {
    font-weight: var(--fw-semibold);
    color: var(--c-accent);
  }
  .controls {
    display: flex;
    height: 100%;
  }
  .controls button {
    width: 46px;
    height: 100%;
    border: 0;
    background: transparent;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .controls button:hover {
    background: var(--c-surface-3);
  }
  .controls .close:hover {
    background: var(--c-danger);
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    padding: var(--sp-5) var(--sp-4);
    border-right: 1px solid var(--c-border);
  }
  .sidebar a {
    display: flex;
    align-items: center;
    gap: var(--sp-5);
    padding: var(--sp-4) var(--sp-5);
    border-radius: var(--r-control);
    color: var(--c-text-2);
    text-decoration: none;
    border-left: 3px solid transparent;
  }
  .sidebar a:hover {
    background: var(--c-surface-2);
  }
  .sidebar a.active {
    color: var(--c-text-1);
    background: var(--c-surface-2);
    border-left-color: var(--c-accent);
  }
  .content {
    padding: var(--sp-9);
    overflow: auto;
  }
</style>
