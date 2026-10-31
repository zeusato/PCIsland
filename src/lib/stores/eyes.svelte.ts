// Short-lived moods set by the island root (hover → happy, new alert → surprised).
export const eyesMood = $state<{ mood: 'neutral' | 'happy' | 'surprised' }>({ mood: 'neutral' });

let timer: ReturnType<typeof setTimeout> | undefined;
export function flashMood(mood: 'happy' | 'surprised', ms: number): void {
  clearTimeout(timer);
  eyesMood.mood = mood;
  timer = setTimeout(() => (eyesMood.mood = 'neutral'), ms);
}
