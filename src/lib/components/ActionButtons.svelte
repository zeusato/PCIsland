<script lang="ts">
  // Quick actions of an activity, routed through ActivityHub (docs/02-system-design.md §5).
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import { activityCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  import Icon from './Icon.svelte';

  let { activity, labels = false }: { activity: Activity; labels?: boolean } = $props();
  let shaking = $state<string | null>(null);

  async function run(actionId: string) {
    try {
      await activityCommands.invoke(activity.id, actionId);
    } catch {
      shaking = actionId;
      setTimeout(() => (shaking = null), 300);
    }
  }
</script>

<div class="actions">
  {#each activity.actions as action (action.id)}
    <button
      class:primary={action.primary}
      class:labels
      class:shake={shaking === action.id}
      aria-label={t(action.labelKey)}
      title={t(action.labelKey)}
      onclick={(e) => {
        e.stopPropagation();
        run(action.id);
      }}
    >
      <Icon name={action.icon} size={16} />
      {#if labels}<span>{t(action.labelKey)}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .actions {
    display: flex;
    gap: var(--sp-4);
    justify-content: center;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-2);
    min-width: 32px;
    height: 32px;
    padding: 0 var(--sp-3);
    border: 0;
    border-radius: var(--r-pill);
    background: var(--c-surface-3);
    cursor: pointer;
    font-size: var(--fs-small);
  }
  button.labels {
    padding: 0 var(--sp-5);
  }
  button:hover {
    color: var(--c-accent-bright);
  }
  .primary {
    background: var(--c-accent-soft);
    color: var(--c-accent-bright);
  }
  .shake {
    animation: shake 300ms;
  }
  @keyframes shake {
    25% {
      transform: translateX(-3px);
    }
    75% {
      transform: translateX(3px);
    }
  }
</style>
