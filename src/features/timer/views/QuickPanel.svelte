<script lang="ts">
  // Launcher panel: preset chips, wheel to adjust (docs/modules/timer.md §3) — no typing needed.
  import { timerCommands } from '$lib/ipc/commands';
  import { live } from '$lib/stores/live.svelte';
  import { t } from '$lib/i18n/index.svelte';

  let { close }: { close: () => void } = $props();
  const presets = $derived(live.settings?.timer.presetsMin ?? [1, 3, 5, 10, 15, 25, 45, 60]);
  let minutes = $state(5);

  function wheel(e: WheelEvent) {
    e.preventDefault();
    minutes = Math.min(600, Math.max(1, minutes + (e.deltaY < 0 ? 1 : -1)));
  }
  async function start(m: number) {
    await timerCommands.start(m * 60_000);
    close();
  }
</script>

<div class="panel">
  <div class="chips">
    {#each presets as p (p)}
      <button class="chip" class:active={p === minutes} onclick={() => (minutes = p)}>{p}′</button>
    {/each}
  </div>
  <div class="row">
    <span class="value tabular" onwheel={wheel} role="spinbutton" aria-valuenow={minutes} tabindex="-1">{minutes} {t('timer.minutes')}</span>
    <button class="go" onclick={() => start(minutes)}>{t('timer.start')}</button>
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .chips {
    display: flex;
    gap: var(--sp-2);
    flex-wrap: wrap;
  }
  .chip {
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-pill);
    background: var(--c-surface-2);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--fs-small);
    cursor: pointer;
  }
  .chip.active {
    border-color: var(--c-accent);
    color: var(--c-accent-bright);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .value {
    font-size: var(--fs-title-s);
    font-weight: var(--fw-semibold);
    cursor: ns-resize;
  }
  .go {
    border: 0;
    border-radius: var(--r-pill);
    background: var(--c-accent);
    color: #000;
    padding: var(--sp-3) var(--sp-6);
    font-weight: var(--fw-semibold);
    cursor: pointer;
  }
</style>
