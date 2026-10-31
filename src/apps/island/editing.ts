const EDITABLE_INPUTS = new Set(['text', 'search', 'email', 'password', 'tel', 'url', 'number', 'date', 'datetime-local', 'time', 'month', 'week']);

/** Keep a composing form open even before typing, after blur, or while saving. */
export function hasEditor(root: HTMLElement | undefined): boolean {
  return [...(root?.querySelectorAll('input, textarea, [contenteditable]') ?? [])].some((element) => {
    if (element.closest('[hidden], [inert], [aria-hidden="true"]')) return false;
    if (element instanceof HTMLTextAreaElement) return !element.readOnly;
    if (element instanceof HTMLInputElement) return !element.readOnly && EDITABLE_INPUTS.has(element.type);
    return ['', 'true', 'plaintext-only'].includes(element.getAttribute('contenteditable') ?? 'false');
  });
}
