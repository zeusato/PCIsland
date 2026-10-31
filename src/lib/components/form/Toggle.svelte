<script lang="ts">
  let { checked, onchange, label }: { checked: boolean; onchange: (v: boolean) => void; label: string } = $props();
</script>

<button class="toggle" class:on={checked} role="switch" aria-checked={checked} aria-label={label} onclick={() => onchange(!checked)}>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 40px;
    height: 22px;
    border: 0;
    border-radius: var(--r-pill);
    background: var(--c-surface-3);
    cursor: pointer;
    flex: none;
  }
  .toggle.on {
    background: var(--c-accent);
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--c-text-1);
    transition: transform 160ms ease-out;
  }
  .on .knob {
    transform: translateX(18px);
  }
</style>
