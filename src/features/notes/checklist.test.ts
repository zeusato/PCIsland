import { describe, expect, it } from 'vitest';
import { appendCheckbox, checklistLine } from './checklist';

describe('note checklists', () => {
  it('recognizes existing shorthand, Markdown and uppercase checks', () => {
    for (const prefix of ['[]', '[ ]', '  - [ ]']) expect(checklistLine(`${prefix} việc`)).toEqual({ checked: false, text: 'việc' });
    for (const prefix of ['[x]', '[X]', '- [X]']) expect(checklistLine(`${prefix} việc`)).toEqual({ checked: true, text: 'việc' });
    expect(checklistLine('văn bản [ ] ở giữa')).toBeNull();
  });
  it('adds a checkbox on its own line without erasing the draft', () => {
    expect(appendCheckbox('')).toBe('[ ] ');
    expect(appendCheckbox('ghi chú')).toBe('ghi chú\n[ ] ');
    expect(appendCheckbox('ghi chú\n')).toBe('ghi chú\n[ ] ');
  });
});
