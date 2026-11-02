// Share link of the playing track (docs/modules/media-hub.md §5). Only online sources have
// a public link: the web player's current track (or its page) and the pasted video link.
import { appCommands, webPlayerCommands } from '$lib/ipc/commands';
import { videoCommands } from '$lib/ipc/video';

export const shareable = (sourceId: string | undefined): boolean => sourceId === 'web' || sourceId === 'video';

export async function shareLink(sourceId: string): Promise<string | null> {
  if (sourceId === 'web') {
    const state = await webPlayerCommands.getState();
    const session = state.history.find((s) => s.siteId === state.siteId);
    return session?.trackUrl ?? state.url ?? session?.pageUrl ?? null;
  }
  if (sourceId === 'video') return (await videoCommands.current())?.pageUrl ?? null;
  return null;
}

/** Copies the link; resolves false when the source has no link to share. */
export async function copyShareLink(sourceId: string): Promise<boolean> {
  const url = await shareLink(sourceId);
  if (!url) return false;
  await appCommands.copyLink(url);
  return true;
}
