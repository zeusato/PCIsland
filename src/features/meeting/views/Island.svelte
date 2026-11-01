<script lang="ts">
  import type {Activity} from '$lib/ipc/bindings/Activity';
  import {islandCommands} from '$lib/ipc/commands';
  import {t} from '$lib/i18n/index.svelte';
  import Mic from '@lucide/svelte/icons/mic';
  let {activity,variant}:{activity:Activity;variant:string}=$props();
</script>
<button title={t('meeting.recording')} aria-label={t('meeting.openLog')} onclick={e=>{e.stopPropagation();void islandCommands.openMain('/meeting');}}>
  <Mic size={16}/>{#if variant!=='bubble'}<span>{t('meeting.recording')} · {new Date(activity.startedAt).toLocaleTimeString()}</span>{/if}
</button>
<style>button{display:flex;gap:var(--sp-3);align-items:center;justify-content:center;width:100%;height:100%;border:0;background:transparent;color:var(--c-accent-bright);cursor:pointer;font:inherit;}</style>
