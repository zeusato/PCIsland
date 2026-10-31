<script lang="ts">
  let {
    value,
    options,
    onchange,
    label,
  }: { value: string; options: { value: string; label: string }[]; onchange: (v: string) => void; label: string } = $props();
</script>

<select aria-label={label} {value} onchange={(e) => onchange((e.currentTarget as HTMLSelectElement).value)}>
  {#each options as o (o.value)}
    <option value={o.value}>{o.label}</option>
  {/each}
</select>

<style>
  select {
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-3) var(--sp-5);
    font: inherit;
  }
</style>
