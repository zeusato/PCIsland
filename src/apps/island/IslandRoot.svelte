<script lang="ts">
  // Island root: presentation machine, spring morph, hit regions, activities from the hub
  // (docs/modules/island-window.md §4–§5, docs/modules/activity-hub.md).
  import { setContext, untrack } from 'svelte';
  import { ISLAND_INTERACTION, type IslandInteraction } from '$lib/components/islandInteraction';
  import MediaPlayer from '../../features/media/views/Player.svelte';
  import { nowPlaying } from '../../features/media/store.svelte';
  import { payload } from '$lib/registry/payloads';
  import { Spring } from 'svelte/motion';
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import { activityCommands, inTauri, islandCommands, shelfCommands } from '$lib/ipc/commands';
  import { onDragDrop, onIslandDragHover, onIslandHotkey, onIslandPointer, onIslandVisibility } from '$lib/ipc/events';
  import { morphPreset } from '$lib/motion/springs';
  import { contentIn, contentOut } from '$lib/motion/transitions';
  import { viewFor, isLargeAlert } from '$lib/registry/views';
  import { live } from '$lib/stores/live.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import IdleEyes from '../../features/indicators/views/IdleEyes.svelte';
  import Launcher from './Launcher.svelte';
  import MinimalProgress from './MinimalProgress.svelte';
  import ShelfSlots from '../../features/shelf/views/Slots.svelte';
  import { flashMood } from '$lib/stores/eyes.svelte';
  import { BUBBLE, geometryFor, hitRects, union, type Content, type Geometry } from './geometry';
  import { initialIslandState, islandMachine, type IslandEvent } from './islandMachine';
  import { hasEditor } from './editing';


  let ui = $state(initialIslandState);
  let islandElement: HTMLDivElement | undefined = $state();
  const dispatch = (event: IslandEvent) => (ui = islandMachine(ui, event));

  const hub = $derived(live.island);
  const primary = $derived(hub?.primary ?? null);
  const secondary = $derived(hub?.secondary ?? null);
  const playerTrack = $derived(nowPlaying() ?? (primary?.kind === 'media.now_playing' ? payload(primary, 'media.now_playing') : null));
  const alert = $derived(hub?.alert ?? null);
  const idle = $derived(!primary || primary.kind === 'indicators.idle');
  let launcherInitial = $state<string | null>(null);
  let launcherOpen = $state(false);
  let playerSource = $state<string | null>(null);
  let measured = $state<{ node: HTMLElement; key: string; height: number } | null>(null);
  const interaction: IslandInteraction = {
    measure(node) {
      const key = contentKey;
      return {
        update: (height) => { measured = { node, key, height }; },
        destroy: () => { if (measured?.node === node) measured = null; },
      };
    },
    showPlayer(source) {
      launcherInitial = null;
      launcherOpen = false;
      playerSource = source;
      dispatch({ type: 'click' });
    },
    showFeature(id) {
      launcherInitial = id;
      launcherOpen = true;
      dispatch({ type: 'click' });
    },
  };
  setContext<IslandInteraction>(ISLAND_INTERACTION, interaction);
  // The drop/shelf panel lives here, outside the context consumers: measure it directly.
  function fitDrop(node: HTMLElement) {
    if (typeof ResizeObserver === 'undefined') return;
    const measurement = interaction.measure(node);
    const observer = new ResizeObserver(() => measurement.update(Math.ceil(node.getBoundingClientRect().height)));
    observer.observe(node);
    return { destroy: () => { observer.disconnect(); measurement.destroy(); } };
  }
  $effect(() => {
    if (ui.mode === 'compact' || ui.mode === 'minimal' || ui.mode === 'hidden') {
      launcherOpen = false;
      launcherInitial = null;
      shelfOpen = false;
      // An explicit "show player" lasts one peek; the next hover follows the activity again.
      playerSource = null;
    }
  });
  let nativeDragging = $state(false);
  let dragPreview = $state(false);
  const dragging = $derived(nativeDragging || dragPreview);
  let shelfOpen = $state(false);
  const dropActive = $derived(dragging || shelfOpen);
  let dropError = $state(false);

  // Alerts drive the machine.
  let lastAlertId: string | null = null;
  $effect(() => {
    const id = alert?.id ?? null;
    if (id === lastAlertId) return;
    lastAlertId = id;
    if (id) flashMood('surprised', 400);
    untrack(() => dispatch(id ? { type: 'alertShown', critical: alert ? isLargeAlert(alert) : false } : { type: 'alertEnded' }));
  });

  const content: Content = $derived(
    dropActive
      ? 'drop'
      : ui.mode === 'alert' && alert
        ? isLargeAlert(alert) ? 'alert-large' : 'alert-small'
        : ui.mode === 'peek' && playerSource && !launcherOpen && !launcherInitial
          ? 'media'
        : ui.mode === 'peek' && (idle || launcherOpen || launcherInitial)
          ? 'launcher'
          : idle
            ? 'idle'
            : primary?.source === 'media'
              ? 'media'
              : 'activity',
  );
  const shown = $derived(content === 'alert-large' || content === 'alert-small' ? alert : primary);
  const variant = $derived(content === 'alert-large' || content === 'alert-small' ? 'alert' : ui.mode === 'peek' ? 'peek' : 'compact');
  const contentKey = $derived(content === 'launcher' ? 'launcher' : playerSource && content === 'media' ? `player:${playerSource}:${variant}` : `${content}:${variant}:${shown?.id ?? ''}`);
  const showBubble = $derived(!!secondary && !dropActive && (ui.mode === 'compact' || ui.mode === 'peek'));
  const showSwitcher = $derived(ui.mode === 'peek' && !dropActive && (!idle || !!playerSource || !!nowPlaying()));
  const showPlayer = $derived(ui.mode === 'peek' && content === 'media');
  const canReturnToPlayer = $derived(showSwitcher && !showPlayer && !!nowPlaying());
  const target = $derived(geometryFor(ui.mode, content, content === 'media' ? 'media' : primary?.source ?? '', measured?.key === contentKey ? measured.height : 0, showSwitcher));
  const size = new Spring<Geometry>(geometryFor('compact', 'idle'), morphPreset());
  const live_ = $derived(!idle && (primary?.priority ?? 0) >= 50 && ui.mode !== 'minimal');
  const critical = $derived(content === 'alert-large');
  const focus = $derived(hub?.focusActive ?? false);

  // Morph: report the union of before/after as hit region, then the exact one when settled.
  $effect(() => {
    const next = target;
    const mode = ui.mode;
    const bubble = showBubble;
    let cancelled = false;
    void sendHit(hitRects(union(untrack(() => size.current), next), mode, bubble)).then(async () => {
      if (cancelled) return;
      await size.set(next);
      if (!cancelled) await sendHit(hitRects(next, mode, bubble));
    }).catch(() => {});
    return () => { cancelled = true; };
  });

  async function sendHit(rects: ReturnType<typeof hitRects>) {
    if (inTauri) await islandCommands.setHitRegions(rects).catch(() => {});
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  const hoverDelay = $derived(live.settings?.island.hoverDelayMs ?? 250);
  const collapseDelay = $derived(live.settings?.island.collapseDelayMs ?? 400);
  function onEnter() {
    clearTimeout(timer);
    if (dropActive) return;
    dispatch({ type: 'pointerEnter' });
    if (idle && ui.mode === 'compact') flashMood('happy', 600);
    timer = setTimeout(() => dispatch({ type: 'pointerDwell' }), hoverDelay);
  }
  function onLeave() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      // Check when the delay expires: an editor may have opened after pointerleave.
      if (!dropActive && !launcherInitial && !hasEditor(islandElement)) dispatch({ type: 'pointerLeaveTimeout' });
    }, collapseDelay);
  }
  function onClick() {
    if (ui.mode === 'alert' && alert && !isLargeAlert(alert)) {
      activityCommands.dismiss(alert.id);
      return;
    }
    dispatch({ type: 'click' });
  }
  function closeLauncherPanel() {
    if (launcherInitial) {
      launcherInitial = null;
      launcherOpen = true;
    }
  }

  function toggleLauncher(event: MouseEvent) {
    event.stopPropagation();
    const open = content !== 'launcher';
    shelfOpen = false;
    launcherInitial = null;
    launcherOpen = open;
    dispatch({ type: 'click' });
  }

  $effect(() => {
    if (!inTauri) return;
    const subs = [
      onIslandDragHover((active) => {
        clearTimeout(timer);
        dragPreview = active;
        if (active) dropError = false;
      }),
      onIslandVisibility((v) => dispatch({ type: 'visibility', mode: v.mode, hasLiveActivity: v.hasLiveActivity })),
      onIslandPointer((p) => {
        if (p.clickOutside) {
          launcherInitial = null;
          dispatch({ type: 'clickOutside' });
        }
      }),
      onIslandHotkey((action) => {
        if (action === 'togglePeek') dispatch({ type: 'hotkeyToggle' });
        if (action === 'quickNote') {
          launcherInitial = 'notes.create';
          if (ui.mode !== 'peek') dispatch({ type: 'hotkeyToggle' });
        }
        if (action === 'videoOpen') {
          launcherInitial = 'video.open';
          if (ui.mode !== 'peek') dispatch({ type: 'hotkeyToggle' });
        }
      }),
      onDragDrop((e) => {
        clearTimeout(timer);
        if (e.type === 'leave') {
          nativeDragging = false;
          return;
        }
        if (e.type === 'drop') {
          nativeDragging = false;
          dragPreview = false;
          shelfOpen = true;
          dropError = false;
          dispatch({ type: 'click' });
          void shelfCommands.add(e.paths).catch(() => dropError = true);
        } else {
          nativeDragging = true;
          dropError = false;
        }
      }),
    ];
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && dispatch({ type: 'escape' });
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      subs.forEach((s) => s.then((unlisten) => unlisten()));
    };
  });

</script>

<div
  bind:this={islandElement}
  class="island"
  class:live={live_ && !critical}
  class:critical
  class:focus
  class:minimal={ui.mode === 'minimal' && !dropActive}
  class:hidden={ui.mode === 'hidden' && !dropActive}
  style:width="{size.current.w}px"
  style:height="{size.current.h}px"
  style:--r="{size.current.r}px"
  role="button"
  tabindex="-1"
  onpointerenter={onEnter}
  onpointerleave={onLeave}
  onclick={onClick}
  onkeydown={() => {}}
>
  {#key contentKey}
    <div class="content" class:withSwitcher={showSwitcher} in:contentIn out:contentOut>
      {#if content === 'drop'}
        <div class="drop" use:fitDrop>
          <ShelfSlots items={live.shelf} receiving={dragging} />
          {#if dropError}<p role="alert">{t('shelf.addFailed')}</p>{/if}
          {#if !dragging}<button onclick={(event) => { event.stopPropagation(); shelfOpen = false; }}>{t('common.back')}</button>{/if}
        </div>
      {:else if ui.mode === 'hidden'}
        <span></span>
      {:else if content === 'launcher'}
        <Launcher initial={launcherInitial} onclose={closeLauncherPanel} onopen={() => { launcherOpen = true; }} />
      {:else if ui.mode === 'minimal'}
        <MinimalProgress activity={primary} />
      {:else if showPlayer}
        <!-- The player below stays mounted while another feature is open. -->
      {:else if shown}
        {@const View = viewFor(shown)}
        <View activity={shown} {variant} />
      {:else}
        <IdleEyes />
      {/if}
      {#if content === 'alert-large' && (hub?.criticalWaiting ?? 0) > 0}
        <span class="more">+{hub?.criticalWaiting}</span>
      {/if}
    </div>
  {/key}
  {#if playerSource || playerTrack}
    <div class="content player-content" class:withSwitcher={showSwitcher} hidden={!showPlayer}>
      <MediaPlayer np={nowPlaying() ? undefined : playerTrack ?? undefined} source={playerSource ?? playerTrack?.sourceId} active={showPlayer} />
    </div>
  {/if}
  {#if canReturnToPlayer}
    <button class="switcher return-player" onclick={(event) => { event.stopPropagation(); launcherInitial = null; launcherOpen = false; playerSource = nowPlaying()!.sourceId; }}>{t('music.backToPlayer')}</button>
  {/if}
  {#if showSwitcher}
    <button class="switcher" onclick={toggleLauncher} ondblclick={(event) => event.stopPropagation()} aria-label={t(content === 'launcher' ? 'island.backToActivity' : 'island.features')}>
      {#if content === 'launcher'}<ChevronLeft size={14} />{:else}<LayoutGrid size={14} />{/if}
      {t(content === 'launcher' ? 'island.backToActivity' : 'island.features')}
    </button>
  {/if}
</div>

{#if showBubble && secondary}
  {@const Bubble = viewFor(secondary)}
  <button
    class="bubble"
    style:left="calc(50% + {size.current.w / 2 + BUBBLE.gap}px)"
    aria-label={t('island.pinSecondary')}
    onclick={() => activityCommands.pin(secondary.id)}
  >
    <Bubble activity={secondary} variant="bubble" />
  </button>
{/if}

<style>
  .island {
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    background: var(--c-island);
    border-radius: 0 0 var(--r) var(--r);
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
  .island.focus {
    box-shadow: 0 0 0 1px var(--c-accent);
  }
  .island.critical {
    animation: breathe 2s ease-in-out infinite;
  }
  @keyframes breathe {
    0%,
    100% {
      box-shadow: 0 0 0 1px rgba(255, 106, 26, 0.35), 0 0 14px rgba(255, 106, 26, 0.3);
    }
    50% {
      box-shadow: 0 0 0 1px rgba(255, 106, 26, 0.75), 0 0 30px rgba(255, 106, 26, 0.6);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .island.critical {
      animation: none;
      box-shadow: var(--glow-live);
    }
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
  .content.withSwitcher { top: var(--island-switcher-h); }
  .player-content[hidden] { display: none; }
  .return-player { left: var(--sp-6); right: auto; }
  .switcher {
    position: absolute;
    top: var(--sp-2);
    right: var(--sp-6);
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    height: var(--sp-8);
    padding: 0 var(--sp-4);
    border: 0;
    border-radius: var(--r-control);
    background: var(--c-surface-2);
    color: var(--c-text-2);
    font-size: var(--fs-caption);
    cursor: pointer;
  }
  .switcher:hover { color: var(--c-accent-bright); background: var(--c-surface-3); }
  .more {
    position: absolute;
    top: var(--sp-4);
    right: var(--sp-6);
    font-size: var(--fs-caption);
    color: var(--c-text-2);
  }
  .drop {
    display: flex;
    flex-direction: column;
    align-items: center;
    align-self: start;
    width: 100%;
    max-height: var(--island-panel-max-h);
    overflow-y: auto;
  }
  .bubble {
    position: absolute;
    top: 0;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 0 0 16px 16px;
    background: var(--c-island);
    cursor: pointer;
  }
</style>
