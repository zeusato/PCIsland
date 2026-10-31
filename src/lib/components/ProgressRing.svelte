<script lang="ts">
  let {
    value,
    size = 20,
    stroke = 3,
    color = 'var(--c-accent)',
  }: { value: number; size?: number; stroke?: number; color?: string } = $props();
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
</script>

<svg width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true">
  <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--c-surface-3)" stroke-width={stroke} />
  <circle
    cx={size / 2}
    cy={size / 2}
    {r}
    fill="none"
    stroke={color}
    stroke-width={stroke}
    stroke-linecap="round"
    stroke-dasharray={c}
    stroke-dashoffset={c * (1 - Math.min(1, Math.max(0, value)))}
    transform="rotate(-90 {size / 2} {size / 2})"
  />
</svg>
