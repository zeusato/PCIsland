<script lang="ts">
  let {
    value,
    min,
    max,
    step = 1,
    onchange,
    label,
    suffix = '',
  }: { value: number; min: number; max: number; step?: number; onchange: (v: number) => void; label: string; suffix?: string } = $props();
</script>

<span class="wrap">
  <input
    type="number"
    aria-label={label}
    {min}
    {max}
    {step}
    {value}
    onchange={(e) => {
      const v = Number((e.currentTarget as HTMLInputElement).value);
      if (Number.isFinite(v)) onchange(Math.min(max, Math.max(min, v)));
    }}
  />
  {#if suffix}<span class="suffix">{suffix}</span>{/if}
</span>

<style>
  .wrap {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-3);
  }
  input {
    width: 80px;
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-3) var(--sp-4);
    font: inherit;
    user-select: text;
  }
  .suffix {
    color: var(--c-text-3);
    font-size: var(--fs-small);
  }
</style>
