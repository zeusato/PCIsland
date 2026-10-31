<script lang="ts">
  // Idle clock: updates once per minute, aligned to the minute (docs/modules/system-indicators.md §5).
  import { formatTime } from '$lib/format';
  import { live } from '$lib/stores/live.svelte';

  let now = $state(Date.now());
  $effect(() => {
    let id: ReturnType<typeof setTimeout>;
    const tick = () => {
      now = Date.now();
      id = setTimeout(tick, 60_000 - (Date.now() % 60_000));
    };
    id = setTimeout(tick, 60_000 - (Date.now() % 60_000));
    return () => clearTimeout(id);
  });
</script>

<span class="clock tabular">{formatTime(now, live.settings?.ui.timeFormat === '12h')}</span>

<style>
  .clock {
    display: grid;
    place-items: center;
    width: 100%;
    font-size: var(--fs-body-island);
    font-weight: var(--fw-semibold);
    color: var(--c-text-1);
  }
</style>
