<script lang="ts">
  // Maps backend icon ids (ActivityAction.icon) to Lucide components.
  import {
    AlarmClock,
    Check,
    Flag,
    Pause,
    PinOff,
    Play,
    Plus,
    RotateCcw,
    SkipBack,
    SkipForward,
    Square,
    Trash2,
    X,
    Circle,
  } from '@lucide/svelte';
  import type { Component } from 'svelte';

  let { name, size = 18 }: { name: string; size?: number } = $props();

  const icons: Record<string, Component<{ size?: number }>> = {
    'alarm-clock': AlarmClock,
    check: Check,
    flag: Flag,
    pause: Pause,
    'pin-off': PinOff,
    play: Play,
    plus: Plus,
    'rotate-ccw': RotateCcw,
    'skip-back': SkipBack,
    'skip-forward': SkipForward,
    square: Square,
    'trash-2': Trash2,
    x: X,
  };
  const Comp = $derived(icons[name] ?? Circle);
</script>

<Comp {size} />
