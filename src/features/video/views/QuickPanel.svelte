<script lang="ts">
  // Island launcher panel: paste a link; on success the island shows the video as a media
  // source (docs/modules/video-player.md §3, §5.7).
  import { islandInteraction } from '$lib/components/islandInteraction';
  import { mediaCommands } from '$lib/ipc/commands';
  import OpenPanel from './OpenPanel.svelte';

  let { close }: { close: () => void } = $props();
  const { showPlayer } = islandInteraction();
  function playing() {
    void mediaCommands.selectSource('video').catch(() => {});
    showPlayer('video');
  }
</script>

<div class="quick" onkeydown={(e) => { if (e.key === 'Escape') close(); }} role="presentation">
  <OpenPanel island onplaying={playing} />
</div>

<style>
  .quick {
    padding: var(--sp-2) 0;
  }
</style>
