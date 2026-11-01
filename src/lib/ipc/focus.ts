// Shared keyboard focus for the island. Several views can need typing at once (the note
// panel and its alarm editor); the backend gives focus back on the first release, so
// requests are counted here and only the last holder releases it.
import { inTauri, islandCommands } from './commands';

let holders = 0;

/** Takes island focus; call the returned function once to give it back. */
export function holdFocus(): { ready: Promise<void>; release: () => void } {
  if (!inTauri) return { ready: Promise.resolve(), release: () => {} };
  holders += 1;
  const ready = holders === 1 ? islandCommands.requestFocus().catch(() => {}) : Promise.resolve();
  let released = false;
  return {
    ready,
    release: () => {
      if (released) return;
      released = true;
      holders = Math.max(0, holders - 1);
      if (holders === 0) void islandCommands.releaseFocus().catch(() => {});
    },
  };
}
