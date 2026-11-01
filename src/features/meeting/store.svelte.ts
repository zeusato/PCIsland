import { inTauri, meetingCommands } from '$lib/ipc/commands';
import { onMeetingState } from '$lib/ipc/events';
import type { MeetingSnapshot } from '$lib/ipc/bindings/MeetingSnapshot';
export const meetingStore = $state<{snapshot: MeetingSnapshot | null}>({snapshot:null});
let started = false;
export async function startMeetingStore() {
  if (!inTauri || started) return;
  started = true;
  let changed = false;
  await onMeetingState(s=>{changed=true;meetingStore.snapshot=s;});
  const snapshot = await meetingCommands.state();
  if (!changed) meetingStore.snapshot = snapshot;
}
