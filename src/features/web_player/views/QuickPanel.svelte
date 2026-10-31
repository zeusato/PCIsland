<script lang="ts">
  // Launcher panel: pick a music site to open in the web player (docs/modules/web-player.md §3).
  import { inTauri, webPlayerCommands } from '$lib/ipc/commands';
  import type { SiteDto } from '$lib/ipc/bindings/SiteDto';

  let { close }: { close: () => void } = $props();
  let sites = $state<SiteDto[]>([]);

  $effect(() => {
    if (inTauri) webPlayerCommands.listSites().then((list) => (sites = list));
  });

  async function open(site: SiteDto) {
    await webPlayerCommands.open(site.id);
    close();
  }
</script>

<div class="sites">
  {#each sites as site (site.id)}
    <button class="chip" onclick={() => open(site)}>{site.name}</button>
  {/each}
</div>

<style>
  .sites {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-4);
  }
  .chip {
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-pill);
    background: var(--c-surface-2);
    padding: var(--sp-3) var(--sp-5);
    font-size: var(--fs-small);
    cursor: pointer;
  }
  .chip:hover {
    border-color: var(--c-accent);
    color: var(--c-accent-bright);
  }
</style>
