<script lang="ts">
  // Idle screen: a pair of eyes that blink now and then (docs/04-design-system.md §11).
  // One setTimeout schedules each blink; no frames are drawn in between.
  import { prefersReducedMotion } from '$lib/motion/springs';

  let { active = true }: { active?: boolean } = $props();
  let blinking = $state(false);

  $effect(() => {
    if (!active || prefersReducedMotion()) return;
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
      timer = setTimeout(() => blink(Math.random() < 0.2), 3000 + Math.random() * 4000);
    };
    schedule();
    return () => clearTimeout(timer);
  });
</script>

<div class="eyes" class:blinking aria-hidden="true">
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
    transform-origin: 50% 50%;
  }
  .blinking .eye {
    animation: blink 170ms linear;
  }
  @keyframes blink {
    0% {
      transform: scaleY(1);
      animation-timing-function: ease-in;
    }
    41% {
      transform: scaleY(0.1);
    }
    59% {
      transform: scaleY(0.1);
      animation-timing-function: ease-out;
    }
    100% {
      transform: scaleY(1);
    }
  }
</style>
