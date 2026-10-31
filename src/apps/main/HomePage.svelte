<script lang="ts">
  import { Timer, Bell } from '@lucide/svelte';
  import Card from '$lib/components/Card.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { live } from '$lib/stores/live.svelte';
  import { formatWhen } from '$lib/format';
  import MediaPeek from '../../features/media/views/Peek.svelte';
  import { nowPlaying } from '../../features/media/store.svelte';

  const np = $derived(nowPlaying());
  const upcoming = $derived((live.reminders?.upcoming ?? []).slice(0, 5));
  const timers = $derived(live.timer?.timers.length ?? 0);
</script>

<h1>{t('main.nav.home')}</h1>

<div class="grid">
  <Card title={t('music.nowPlaying')}>
    {#if np}
      <div class="np"><MediaPeek {np} /></div>
    {:else}
      <p class="muted">{t('main.home.empty')}</p>
    {/if}
  </Card>

  <Card title={t('main.nav.reminders')}>
    {#each upcoming as r (r.id)}
      <div class="line"><Bell size={14} /><span class="grow">{r.title}</span><span class="muted">{formatWhen(r.snoozedUntil ?? r.dueAt)}</span></div>
    {:else}
      <p class="muted">{t('reminders.empty')}</p>
    {/each}
  </Card>

  <Card title={t('main.nav.timer')}>
    <div class="line"><Timer size={14} /><span>{t('home.timers', { n: timers })}</span></div>
    {#if live.timer?.pomodoro}<div class="line">Pomodoro · {t(`pomodoro.phase.${live.timer.pomodoro.phase}`)}</div>{/if}
  </Card>
</div>

<style>
  h1 {
    margin: 0 0 var(--sp-6);
    font-size: var(--fs-title);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 0 var(--sp-6);
  }
  .np {
    background: var(--c-island);
    border-radius: var(--r-peek);
    height: 172px;
  }
  .line {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) 0;
  }
  .grow {
    flex: 1;
  }
  .muted {
    color: var(--c-text-3);
    font-size: var(--fs-small);
  }
</style>
