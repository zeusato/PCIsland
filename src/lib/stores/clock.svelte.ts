// A shared "now" that ticks only while some view needs it (docs/02-system-design.md §13).
const state = $state({ now: Date.now() });
let users = 0;
let timer: ReturnType<typeof setInterval> | undefined;
let period = 1000;

/** Call inside $effect; returns the cleanup. Ticks are aligned to the period. */
export function useTicker(ms = 1000): () => void {
  users += 1;
  if (ms < period || !timer) {
    period = Math.min(ms, period);
    clearInterval(timer);
    state.now = Date.now();
    timer = setInterval(() => (state.now = Date.now()), period);
  }
  return () => {
    users -= 1;
    if (users === 0) {
      clearInterval(timer);
      timer = undefined;
      period = 1000;
    }
  };
}

export function now(): number {
  return state.now;
}
