<script lang="ts">
  // Web player controls in the full window (docs/modules/web-player.md §3): the P0
  // single-webview player has no toolbar of its own, so navigation lives here.
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import House from '@lucide/svelte/icons/house';
  import Eye from '@lucide/svelte/icons/eye';
  import EyeOff from '@lucide/svelte/icons/eye-off';
  import Power from '@lucide/svelte/icons/power';
  import Globe from '@lucide/svelte/icons/globe';
  import Button from '$lib/components/Button.svelte';
  import { inTauri, webPlayerCommands } from '$lib/ipc/commands';
  import { onWebPlayerState } from '$lib/ipc/events';
  import type { SiteDto } from '$lib/ipc/bindings/SiteDto';
  import type { WebPlayerSnapshot } from '$lib/ipc/bindings/WebPlayerSnapshot';
  import { t } from '$lib/i18n/index.svelte';

  let sites = $state<SiteDto[]>([]);
  let wp = $state<WebPlayerSnapshot | null>(null);
  let confirmClear = $state(false);
  let message = $state('');

  $effect(() => {
    if (!inTauri) return;
    webPlayerCommands.listSites().then((s) => (sites = s));
    webPlayerCommands.getState().then((s) => (wp = s));
    const sub = onWebPlayerState((s) => (wp = s));
    return () => void sub.then((u) => u());
  });

  const open = $derived(wp && wp.lifecycle !== 'closed');
  const siteName = $derived(sites.find((s) => s.id === wp?.siteId)?.name ?? '');

  async function clear() {
    try {
      await webPlayerCommands.clearData();
      message = t('webPlayer.cleared');
    } catch {
      message = t('errors.unknown');
    }
    confirmClear = false;
  }
</script>

<p class="muted">{t('music.onlineHint')}</p>
<div class="sites">
  {#each sites as site (site.id)}
    <Button variant={wp?.siteId === site.id && open ? 'primary' : 'default'} onclick={() => webPlayerCommands.open(site.id)}>
      <Globe size={14} />{site.name}
    </Button>
  {/each}
</div>

{#if open}
  <div class="bar">
    <span class="status">{siteName} · {t(`webPlayer.state.${wp?.lifecycle}`)}</span>
    <Button variant="ghost" title={t('webPlayer.toolbar.back')} onclick={() => webPlayerCommands.navigate('back')}><ArrowLeft size={14} /></Button>
    <Button variant="ghost" title={t('webPlayer.toolbar.forward')} onclick={() => webPlayerCommands.navigate('forward')}><ArrowRight size={14} /></Button>
    <Button variant="ghost" title={t('webPlayer.toolbar.reload')} onclick={() => webPlayerCommands.navigate('reload')}><RotateCw size={14} /></Button>
    <Button variant="ghost" title={t('webPlayer.toolbar.home')} onclick={() => webPlayerCommands.navigate('home')}><House size={14} /></Button>
    {#if wp?.lifecycle === 'hidden'}
      <Button onclick={() => webPlayerCommands.show()}><Eye size={14} />{t('webPlayer.show')}</Button>
    {:else}
      <Button onclick={() => webPlayerCommands.hide()}><EyeOff size={14} />{t('webPlayer.toolbar.collapse')}</Button>
    {/if}
    <Button variant="danger" onclick={() => webPlayerCommands.close()}><Power size={14} />{t('webPlayer.toolbar.close')}</Button>
  </div>
{/if}

<div class="clear">
  {#if confirmClear}
    <span>{t('webPlayer.clearConfirm')}</span>
    <Button variant="danger" onclick={clear}>{t('webPlayer.clear')}</Button>
    <Button variant="ghost" onclick={() => (confirmClear = false)}>{t('common.cancel')}</Button>
  {:else}
    <Button variant="ghost" onclick={() => (confirmClear = true)}>{t('webPlayer.clear')}</Button>
  {/if}
  {#if message}<span class="muted">{message}</span>{/if}
</div>

<style>
  .muted {
    color: var(--c-text-3);
    font-size: var(--fs-small);
  }
  .sites,
  .bar,
  .clear {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-4);
    margin-top: var(--sp-5);
  }
  .status {
    flex: 1;
    color: var(--c-accent-bright);
  }
</style>
