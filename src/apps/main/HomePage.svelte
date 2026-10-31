<script lang="ts">
  import { t } from '$lib/i18n/index.svelte';
  import { inTauri, webPlayerCommands } from '$lib/ipc/commands';
  import type { SiteDto } from '$lib/ipc/bindings/SiteDto';
  import MediaPeek from '../../features/media/views/Peek.svelte';
  import { nowPlaying } from '../../features/media/store.svelte';

  const np = $derived(nowPlaying());
  let sites = $state<SiteDto[]>([]);

  $effect(() => {
    if (inTauri) webPlayerCommands.listSites().then((list) => (sites = list));
  });
</script>

<section class="card">
  <h2>{t('main.nav.media')}</h2>
  {#if np}
    <div class="np"><MediaPeek {np} /></div>
  {:else}
    <p class="muted">{t('main.home.empty')}</p>
  {/if}
</section>

<section class="card">
  <h2>{t('main.nav.online')}</h2>
  <div class="sites">
    {#each sites as site (site.id)}
      <button onclick={() => webPlayerCommands.open(site.id)}>{site.name}</button>
    {/each}
  </div>
</section>

<style>
  .card {
    background: var(--c-surface-2);
    border-radius: var(--r-card);
    padding: var(--sp-7);
    margin-bottom: var(--sp-6);
    max-width: 640px;
  }
  h2 {
    margin: 0 0 var(--sp-5);
    font-size: var(--fs-title-s);
    font-weight: var(--fw-semibold);
  }
  .np {
    background: var(--c-island);
    border-radius: var(--r-peek);
    height: 160px;
  }
  .muted {
    color: var(--c-text-3);
  }
  .sites {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-4);
  }
  .sites button {
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-pill);
    background: var(--c-surface-3);
    padding: var(--sp-3) var(--sp-6);
    cursor: pointer;
  }
  .sites button:hover {
    border-color: var(--c-accent);
    color: var(--c-accent-bright);
  }
</style>
