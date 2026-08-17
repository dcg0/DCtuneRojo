import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useDesignerKeyboard } from '../useDesignerKeyboard';

function fireKeyDown(target: EventTarget, key: string, opts: Partial<KeyboardEventInit> = {}) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...opts });
  target.dispatchEvent(event);
  return event;
}

function renderDesignerKeyboard() {
  const handlers = {
    onDelete: vi.fn(),
    onUndo: vi.fn(),
    onRedo: vi.fn(),
    onCopy: vi.fn(),
    onPaste: vi.fn(),
    onSave: vi.fn(),
    onDeselect: vi.fn(),
  };
  renderHook(() => useDesignerKeyboard(handlers));
  return handlers;
}

describe('useDesignerKeyboard', () => {
  it('deletes the selection on Backspace when focus is on the window/canvas', () => {
    const handlers = renderDesignerKeyboard();
    fireKeyDown(window, 'Backspace');
    expect(handlers.onDelete).toHaveBeenCalledTimes(1);
  });

  it('does not delete the selection on Backspace while typing in a text input', () => {
    const input = document.createElement('input');
    input.type = 'text';
    document.body.appendChild(input);

    const handlers = renderDesignerKeyboard();
    fireKeyDown(input, 'Backspace');

    expect(handlers.onDelete).not.toHaveBeenCalled();
    document.body.removeChild(input);
  });

  it('does not delete the selection on Delete while typing in a textarea', () => {
    const textarea = document.createElement('textarea');
    document.body.appendChild(textarea);

    const handlers = renderDesignerKeyboard();
    fireKeyDown(textarea, 'Delete');

    expect(handlers.onDelete).not.toHaveBeenCalled();
    document.body.removeChild(textarea);
  });

  it('still fires Ctrl+S (save) when focus is on the canvas, not a field', () => {
    const handlers = renderDesignerKeyboard();
    fireKeyDown(window, 's', { ctrlKey: true });
    expect(handlers.onSave).toHaveBeenCalledTimes(1);
  });

  it('does not hijack Ctrl+S while typing in a field (lets the browser/field handle it)', () => {
    const input = document.createElement('input');
    document.body.appendChild(input);

    const handlers = renderDesignerKeyboard();
    fireKeyDown(input, 's', { ctrlKey: true });

    expect(handlers.onSave).not.toHaveBeenCalled();
    document.body.removeChild(input);
  });
});
