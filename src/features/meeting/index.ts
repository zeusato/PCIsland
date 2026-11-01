import Mic from '@lucide/svelte/icons/mic';
import { registerQuickAction } from '$lib/registry/quickActions.svelte';
import { t } from '$lib/i18n/index.svelte';
import { meetingStore, startMeetingStore } from './store.svelte';
import QuickPanel from './views/QuickPanel.svelte';
export function registerMeeting() {
  registerQuickAction({id:'meeting.open',labelKey:'main.nav.meeting',icon:Mic,order:85,panel:QuickPanel,
    state:()=>meetingStore.snapshot?.active ? 'active' : null,
    status:()=>meetingStore.snapshot?.active ? t('meeting.recording') : t('meeting.ready'),
  });
  void startMeetingStore();
}
