// Ephemeral UI feedback survives changes to the island's primary activity.
export const feedback = $state<{ message: string; revision: number }>({ message: '', revision: 0 });
export function showFeedback(message: string): void {
  feedback.message = message;
  feedback.revision++;
}
