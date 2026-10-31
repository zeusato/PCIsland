<script lang="ts">
  // Idle screen: eyes that blink, follow the cursor and show moods
  // (docs/04-design-system.md §11). One setTimeout schedules each blink; nothing is
  // drawn between blinks, gaze updates only arrive while the cursor is near.
  import { prefersReducedMotion } from '$lib/motion/springs';
  import { inTauri, islandCommands } from '$lib/ipc/commands';
  import { onIslandGaze } from '$lib/ipc/events';
  import { live } from '$lib/stores/live.svelte';
  import { eyesMood } from '$lib/stores/eyes.svelte';

  let { active = true }: { active?: boolean } = $props();
  let blinking = $state(false);
  let gaze = $state<[number, number]>([0, 0]);
  let sleepy = $state(false);

  const moods = $derived(live.settings?.indicators.eyesMoods ?? false);
  const reduced = prefersReducedMotion();
  const mood = $derived(!moods || reduced ? 'neutral' : eyesMood.mood !== 'neutral' ? eyesMood.mood : sleepy ? 'sleepy' : 'neutral');

  // Blink scheduler; sleepy eyes blink slower.
  $effect(() => {
    if (!active || reduced) return;
    const slow = mood === 'sleepy';
    let timer: ReturnType<typeof setTimeout>;
    const blink = (thenDouble: boolean) => {
      blinking = true;
      timer = setTimeout(() => {
        blinking = false;
        timer = thenDouble ? setTimeout(() => blink(false), 120) : setTimeout(schedule, 0);
      }, 170);
    };
    const schedule = () => {
      if (document.hidden) {
        timer = setTimeout(schedule, 1000);
        return;
      }
      const wait = slow ? 8000 + Math.random() * 4000 : 3000 + Math.random() * 4000;
      timer = setTimeout(() => blink(!slow && Math.random() < 0.2), wait);
    };
    schedule();
    return () => clearTimeout(timer);
  });

  // Sleepy: no input for 10 minutes, or between 23:00 and 06:00. Checked once a minute.
  $effect(() => {
    if (!moods || !inTauri) return;
    const check = async () => {
      const h = new Date().getHours();
      const idle = await islandCommands.idleSeconds().catch(() => 0);
      sleepy = h >= 23 || h < 6 || idle >= 600;
    };
    check();
    const id = setInterval(check, 60_000);
    return () => clearInterval(id);
  });

  // Gaze from the backend cursor poller (only when the setting is on).
  $effect(() => {
    if (!inTauri || reduced) return;
    const sub = onIslandGaze(([x, y]) => (gaze = [x / 100, y / 100]));
    return () => void sub.then((u) => u());
  });
</script>

<div
  class="eyes {mood}"
  class:blinking
  style:--gx="{gaze[0] * 3}px"
  style:--gy="{Math.max(-0.5, gaze[1]) * 3}px"
  aria-hidden="true"
>
  <span class="eye"></span>
  <span class="eye"></span>
</div>

<style>
  .eyes {
    display: flex;
    gap: var(--eye-gap);
    align-items: center;
    justify-content: center;
  }
  .eye {
    width: var(--eye-w);
    height: var(--eye-h);
    border-radius: calc(var(--eye-w) / 2);
    background: var(--c-accent);
    opacity: 0.9;
    transform: translate(var(--gx, 0), var(--gy, 0));
    transition:
      transform 180ms ease-out,
      height 250ms ease-out,
      border-radius 250ms ease-out;
  }
  /* Happy: curved "^ ^" eyes. */
  .happy .eye {
    height: 6px;
    border-radius: 7px 7px 2px 2px;
  }
  /* Sleepy: half-closed. */
  .sleepy .eye {
    height: 5px;
    opacity: 0.7;
  }
  /* Surprised: wide open. */
  .surprised .eye {
    transform: translate(var(--gx, 0), var(--gy, 0)) scale(1.2);
  }
  .blinking .eye {
    animation: blink 170ms linear;
  }
  @keyframes blink {
    0% {
      transform: translate(var(--gx, 0), var(--gy, 0)) scaleY(1);
      animation-timing-function: ease-in;
    }
    41% {
      transform: translate(var(--gx, 0), var(--gy, 0)) scaleY(0.1);
    }
    59% {
      transform: translate(var(--gx, 0), var(--gy, 0)) scaleY(0.1);
      animation-timing-function: ease-out;
    }
    100% {
      transform: translate(var(--gx, 0), var(--gy, 0)) scaleY(1);
    }
  }
</style>
