/** Accept both the quick-note shorthand and Markdown task-list syntax. */
export function checklistLine(line: string): { checked: boolean; text: string } | null {
  const match = line.match(/^\s*(?:- )?\[([ xX]?)\]\s?(.*)$/);
  return match ? { checked: match[1]?.toLowerCase() === 'x', text: match[2] ?? '' } : null;
}

export function appendCheckbox(text: string): string {
  return `${text}${text && !text.endsWith('\n') ? '\n' : ''}[ ] `;
}
