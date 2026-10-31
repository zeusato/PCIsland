<script lang="ts">
  // Music: now playing, library, playlists, online sites (docs/modules/full-window.md §5).
  import FolderPlus from '@lucide/svelte/icons/folder-plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Play from '@lucide/svelte/icons/play';
  import ListPlus from '@lucide/svelte/icons/list-plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Shuffle from '@lucide/svelte/icons/shuffle';
  import Repeat from '@lucide/svelte/icons/repeat';
  import Repeat1 from '@lucide/svelte/icons/repeat-1';
  import FileMusic from '@lucide/svelte/icons/file-music';
    import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import { inTauri, playerCommands } from '$lib/ipc/commands';
  import { pickAudioFiles, pickFolder } from '$lib/ipc/dialog';
  import type { Track } from '$lib/ipc/bindings/Track';
  import type { Folder } from '$lib/ipc/bindings/Folder';
  import type { Playlist } from '$lib/ipc/bindings/Playlist';
  import { live } from '$lib/stores/live.svelte';
  import { formatDuration } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';
  import MediaPeek from '../../../features/media/views/Peek.svelte';
  import RadioPanel from '../../../features/local_player/views/RadioPanel.svelte';
  import WebPlayerPanel from '../../../features/web_player/views/Panel.svelte';
  import { pickM3u, saveM3u } from '$lib/ipc/dialog';
  import { nowPlaying } from '../../../features/media/store.svelte';

  let tab = $state<'library' | 'playlists' | 'radio' | 'online'>('library');
  let search = $state('');
  let tracks = $state<Track[]>([]);
  let folders = $state<Folder[]>([]);
  let playlists = $state<Playlist[]>([]);
  let scanning = $state(false);
  let message = $state('');
  let newPlaylist = $state('');

  const np = $derived(nowPlaying());
  const player = $derived(live.player);

  async function refresh() {
    if (!inTauri) return;
    [folders, playlists] = await Promise.all([playerCommands.folders(), playerCommands.playlists()]);
    tracks = await playerCommands.query(search);
  }
  $effect(() => {
    refresh();
  });
  $effect(() => {
    const q = search;
    const id = setTimeout(async () => inTauri && (tracks = await playerCommands.query(q)), 200);
    return () => clearTimeout(id);
  });

  async function addFolder() {
    const path = await pickFolder();
    if (!path) return;
    await playerCommands.addFolder(path);
    await rescan();
  }
  async function rescan() {
    scanning = true;
    try {
      const s = await playerCommands.rescan();
      message = t('music.scanned', { added: s.added, updated: s.updated, removed: s.removed });
      await refresh();
    } finally {
      scanning = false;
    }
  }
  async function openFiles() {
    const files = await pickAudioFiles();
    if (files.length) await playerCommands.playFiles(files);
  }
  async function createPlaylist() {
    if (!newPlaylist.trim()) return;
    await playerCommands.createPlaylist(newPlaylist);
    newPlaylist = '';
    playlists = await playerCommands.playlists();
  }
</script>

<h1>{t('main.nav.media')}</h1>

{#if np}
  <Card title={t('music.nowPlaying')}>
    <div class="np"><MediaPeek {np} /></div>
    {#if player && np.sourceId === 'local'}
      <div class="modes">
        <Button variant={player.shuffle ? 'primary' : 'default'} onclick={() => playerCommands.control(player.shuffle ? 'shuffle_off' : 'shuffle_on')}><Shuffle size={14} />{t('music.shuffle')}</Button>
        <Button
          variant={player.repeat === 'off' ? 'default' : 'primary'}
          onclick={() => playerCommands.control(player.repeat === 'off' ? 'repeat_all' : player.repeat === 'all' ? 'repeat_one' : 'repeat_off')}
        >
          {#if player.repeat === 'one'}<Repeat1 size={14} />{:else}<Repeat size={14} />{/if}{t(`music.repeat.${player.repeat}`)}
        </Button>
        <label class="volume">
          {t('music.volume')}
          <input type="range" min="0" max="1" step="0.01" value={player.volume} oninput={(e) => playerCommands.setVolume(Number((e.currentTarget as HTMLInputElement).value))} />
        </label>
      </div>
    {/if}
  </Card>
{/if}

<div class="tabs">
  <button class:active={tab === 'library'} onclick={() => (tab = 'library')}>{t('music.library')}</button>
  <button class:active={tab === 'playlists'} onclick={() => (tab = 'playlists')}>{t('music.playlists')}</button>
  <button class:active={tab === 'radio'} onclick={() => (tab = 'radio')}>{t('radio.title')}</button>
  <button class:active={tab === 'online'} onclick={() => (tab = 'online')}>{t('main.nav.online')}</button>
</div>

{#if tab === 'library'}
  <Card>
    {#snippet actions()}
      <Button onclick={openFiles}><FileMusic size={14} />{t('music.openFiles')}</Button>
      <Button onclick={addFolder}><FolderPlus size={14} />{t('music.addFolder')}</Button>
      <Button onclick={rescan} disabled={scanning || !folders.length}><RefreshCw size={14} />{scanning ? t('music.scanning') : t('music.rescan')}</Button>
    {/snippet}
    {#if folders.length}
      <div class="folders">
        {#each folders as f (f.id)}
          <span class="folder">{f.path}
            <button aria-label={t('common.remove')} onclick={async () => { await playerCommands.removeFolder(f.id); refresh(); }}><Trash2 size={12} /></button>
          </span>
        {/each}
      </div>
    {/if}
    {#if message}<p class="muted">{message}</p>{/if}
    <input class="search" placeholder={t('music.search')} bind:value={search} />
    {#if tracks.length === 0}
      <p class="muted">{folders.length ? t('music.noResults') : t('music.empty')}</p>
    {:else}
      <table>
        <tbody>
          {#each tracks as track, i (track.id)}
            <tr class:current={player?.queue[player?.index ?? -1]?.id === track.id}>
              <td class="title">
                <button class="link" onclick={() => playerCommands.playTracks(tracks.map((x) => x.id), i)}>{track.title}</button>
              </td>
              <td class="muted">{track.artist ?? ''}</td>
              <td class="muted">{track.album ?? ''}</td>
              <td class="muted tabular">{track.durationMs ? formatDuration(track.durationMs) : ''}</td>
              <td class="row-actions">
                <button title={t('music.playNext')} onclick={() => playerCommands.enqueue([track.id], true)}><ListPlus size={14} /></button>
                {#if playlists.length}
                  <select aria-label={t('music.addToPlaylist')} onchange={async (e) => { const v = (e.currentTarget as HTMLSelectElement).value; if (v) { await playerCommands.addToPlaylist(v, [track.id]); playlists = await playerCommands.playlists(); } (e.currentTarget as HTMLSelectElement).value = ''; }}>
                    <option value="">+</option>
                    {#each playlists as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
                  </select>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card>
{:else if tab === 'playlists'}
  <Card>
    <div class="new">
      <input placeholder={t('music.newPlaylist')} bind:value={newPlaylist} onkeydown={(e) => e.key === 'Enter' && createPlaylist()} />
      <Button onclick={createPlaylist}>{t('common.create')}</Button>
      <Button onclick={async () => { const p = await pickM3u(); if (p) { await playerCommands.importPlaylist(p); playlists = await playerCommands.playlists(); } }}>{t('music.importM3u')}</Button>
    </div>
    {#each playlists as p (p.id)}
      <div class="playlist">
        <span>{p.name} <span class="muted">· {t('music.trackCount', { n: p.count })}</span></span>
        <span>
          <Button variant="ghost" onclick={() => playerCommands.playPlaylist(p.id)}><Play size={14} /></Button>
          <Button variant="ghost" title={t('music.exportM3u')} onclick={async () => { const dest = await saveM3u(p.name); if (dest) await playerCommands.exportPlaylist(p.id, dest); }}>m3u</Button>
          <Button variant="ghost" onclick={async () => { await playerCommands.deletePlaylist(p.id); playlists = await playerCommands.playlists(); }}><Trash2 size={14} /></Button>
        </span>
      </div>
    {:else}
      <p class="muted">{t('music.noPlaylists')}</p>
    {/each}
  </Card>
{:else if tab === 'radio'}
  <Card><RadioPanel /></Card>
{:else}
  <Card><WebPlayerPanel /></Card>
{/if}

<style>
  h1 {
    margin: 0 0 var(--sp-6);
    font-size: var(--fs-title);
  }
  .np {
    background: var(--c-island);
    border-radius: var(--r-peek);
    height: 172px;
    max-width: 420px;
  }
  .modes {
    display: flex;
    gap: var(--sp-4);
    align-items: center;
    margin-top: var(--sp-5);
  }
  .volume {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    color: var(--c-text-2);
    font-size: var(--fs-small);
  }
  .tabs {
    display: flex;
    gap: var(--sp-2);
    margin-bottom: var(--sp-5);
  }
  .tabs button {
    border: 0;
    background: none;
    padding: var(--sp-3) var(--sp-5);
    border-radius: var(--r-pill);
    color: var(--c-text-2);
    cursor: pointer;
  }
  .tabs .active {
    background: var(--c-surface-2);
    color: var(--c-accent-bright);
  }
  .folders {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-3);
    margin-bottom: var(--sp-4);
  }
  .folder {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    background: var(--c-surface-3);
    border-radius: var(--r-pill);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--fs-small);
  }
  .folder button,
  .row-actions button {
    border: 0;
    background: none;
    cursor: pointer;
    color: var(--c-text-3);
  }
  .search,
  .new input {
    width: 100%;
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-4) var(--sp-5);
    margin-bottom: var(--sp-5);
    font: inherit;
    user-select: text;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-small);
  }
  td {
    padding: var(--sp-3) var(--sp-3);
    border-bottom: 1px solid var(--c-border);
  }
  tr.current .link {
    color: var(--c-accent-bright);
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .link:hover {
    color: var(--c-accent-bright);
  }
  .row-actions {
    text-align: right;
    white-space: nowrap;
  }
  .row-actions select {
    background: var(--c-surface-3);
    color: var(--c-text-1);
    border: 0;
    border-radius: 6px;
  }
  .muted {
    color: var(--c-text-3);
  }
  .new {
    display: flex;
    gap: var(--sp-4);
  }
  .new input {
    margin: 0;
  }
  .playlist {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--sp-3) 0;
    border-bottom: 1px solid var(--c-border);
  }
</style>
