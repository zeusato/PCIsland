<script lang="ts">
  // Island root: presentation machine, spring morph, hit regions
  // (docs/modules/island-window.md §4–§5, docs/04-design-system.md §5–§6).
  import { untrack } from 'svelte';
  import { Spring } from 'svelte/motion';
  import { inTauri, islandCommands } from '$lib/ipc/commands';
  import { onIslandPointer, onIslandVisibility } from '$lib/ipc/events';
  import { morphPreset } from '$lib/motion/springs';
  import { contentIn, contentOut } from '$lib/motion/transitions';
  import IdleEyes from '../../features/indicators/views/IdleEyes.svelte';
  import MediaCompact from '../../features/media/views/Compact.svelte';
  import MediaPeek from '../../features/media/views/Peek.svelte';
  import { nowPlaying } from '../../features/media/store.svelte';
  import Launcher from './Launcher.svelte';
  import { geometryFor, hitRect, union, type Geometry } from './geometry';
  import { initialIslandState, islandMachine, type IslandEvent } from './islandMachine';

  const HOVER_DELAY_MS = 250; // island.hover_delay_ms
  const COLLAPSE_DELAY_MS = 400; // island.collapse_delay_ms

  let ui = $state(initialIslandState);
  const dispatch = (event: IslandEvent) => (ui = islandMachine(ui, event));

  const np = $derived(nowPlaying());
  const target = $derived(geometryFor(ui.mode, np !== null));
  const size = new Spring<Geometry>(geometryFor('compact', false), morphPreset());
  const live = $derived(np?.status === 'playing');

  // Morph: report the union of before/after as hit region, then the exact one when settled.
  $effect(() => {
    const next = target;
    const mode = ui.mode;
    // untrack: reading size.current must not re-run this effect on every spring frame.
    sendHit(hitRect(union(untrack(() => size.current), next), mode));
    size
      .set(next)
      .then(() => {
        if (untrack(() => ui.mode) === mode) sendHit(hitRect(next, mode));
      })
      .catch(() => {}); // superseded by a newer target: that run reports its own region
  });

  function sendHit(rect: ReturnType<typeof hitRect>) {
    if (inTauri) islandCommands.setHitRegions([rect]).catch(() => {});
  }

  // Hover dwell / collapse timers drive the pure machine.
  let timer: ReturnType<typeof setTimeout> | undefined;
  function onEnter() {
    clearTimeout(timer);
    dispatch({ type: 'pointerEnter' });
    timer = setTimeout(() => dispatch({ type: 'pointerDwell' }), HOVER_DELAY_MS);
  }
  function onLeave() {
    clearTimeout(timer);
    timer = setTimeout(() => dispatch({ type: 'pointerLeaveTimeout' }), COLLAPSE_DELAY_MS);
  }

  $effect(() => {
    if (!inTauri) return;
    const subs = [
      onIslandVisibility((v) =>
        dispatch({ type: 'visibility', mode: v.mode, hasLiveActivity: v.hasLiveActivity }),
      ),
      onIslandPointer((p) => {
        if (p.clickOutside) dispatch({ type: 'clickOutside' });
      }),
    ];
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && dispatch({ type: 'escape' });
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      subs.forEach((s) => s.then((unlisten) => unlisten()));
    };
  });

  const contentKey = $derived(`${ui.mode}:${np ? 'media' : 'idle'}`);
</script>

<div
  class="island"
  class:live
  class:minimal={ui.mode === 'minimal'}
  class:hidden={ui.mode === 'hidden'}
  style:width="{size.current.w}px"
  style:height="{size.current.h}px"
  style:--r="{size.current.r}px"
  role="button"
  tabindex="-1"
  onpointerenter={onEnter}
  onpointerleave={onLeave}
  onclick={() => dispatch({ type: 'click' })}
  ondblclick={() => inTauri && islandCommands.openMain(np ? '/media' : '/home')}
  onkeydown={() => {}}
>
  {#key contentKey}
    <div class="content" in:contentIn out:contentOut>
      {#if ui.mode === 'peek'}
        {#if np}<MediaPeek {np} />{:else}<Launcher />{/if}
      {:else if ui.mode === 'compact'}
        {#if np}<MediaCompact {np} />{:else}<IdleEyes />{/if}
      {/if}
    </div>
  {/key}
</div>

<style>
  .island {
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    background: var(--c-island);
    border-radius: 0 0 var(--r) var(--r);
    overflow: visible;
    cursor: default;
    outline: none;
  }
  /* Concave "shoulders" joining the island to the screen edge (attached style). */
  .island::before,
  .island::after {
    content: '';
    position: absolute;
    top: 0;
    width: var(--r-shoulder);
    height: var(--r-shoulder);
    pointer-events: none;
  }
  .island::before {
    left: calc(-1 * var(--r-shoulder));
    background: radial-gradient(circle at 0 100%, transparent var(--r-shoulder), var(--c-island) calc(var(--r-shoulder) + 0.5px));
  }
  .island::after {
    right: calc(-1 * var(--r-shoulder));
    background: radial-gradient(circle at 100% 100%, transparent var(--r-shoulder), var(--c-island) calc(var(--r-shoulder) + 0.5px));
  }
  .island.minimal::before,
  .island.minimal::after {
    display: none;
  }
  .island.live {
    box-shadow: var(--glow-live);
  }
  .island.hidden {
    opacity: 0;
  }
  .content {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: inherit;
    display: grid;
  }
</style>
