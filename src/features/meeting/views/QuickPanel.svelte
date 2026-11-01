<script lang="ts">
  import MeetingPanel from './MeetingPanel.svelte';
  import { islandCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  import { holdFocus } from '$lib/ipc/focus';
  import { inTauri } from '$lib/ipc/commands';
  import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
  let {close}:{close:()=>void}=$props();
  $effect(() => { if (!inTauri) return; const focus = holdFocus(); return () => focus.release(); });
</script>
<MeetingPanel compact />
<button class="open-log" onclick={()=>{void islandCommands.openMain('/meeting');close();}}>{t('meeting.openLog')}<ArrowUpRight size={15}/></button>
<style>
  .open-log { display:flex; justify-content:center; align-items:center; gap:var(--sp-3); width:100%; min-height:var(--sp-9); border:0; border-top:1px solid var(--c-border); background:transparent; color:var(--c-text-2); font:inherit; font-size:var(--fs-small); cursor:pointer; padding:var(--sp-4); }
  .open-log:hover { color:var(--c-accent-bright); }
</style>
