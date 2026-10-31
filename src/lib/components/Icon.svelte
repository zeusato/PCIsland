<script lang="ts">
  // Maps backend icon ids (ActivityAction.icon) to Lucide components.
  import AlarmClock from '@lucide/svelte/icons/alarm-clock';
  import Check from '@lucide/svelte/icons/check';
  import Flag from '@lucide/svelte/icons/flag';
  import Pause from '@lucide/svelte/icons/pause';
  import PinOff from '@lucide/svelte/icons/pin-off';
  import Play from '@lucide/svelte/icons/play';
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import SkipBack from '@lucide/svelte/icons/skip-back';
  import SkipForward from '@lucide/svelte/icons/skip-forward';
  import Square from '@lucide/svelte/icons/square';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import X from '@lucide/svelte/icons/x';
  import Circle from '@lucide/svelte/icons/circle';
  import Download from '@lucide/svelte/icons/download';
  import type { Component } from 'svelte';

  let { name, size = 18 }: { name: string; size?: number } = $props();

  const icons: Record<string, Component<{ size?: number }>> = {
    'alarm-clock': AlarmClock,
    check: Check,
    download: Download,
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
