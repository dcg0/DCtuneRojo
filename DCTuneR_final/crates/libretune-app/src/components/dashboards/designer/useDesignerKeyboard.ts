import { useEffect } from 'react';

interface UseDesignerKeyboardArgs {
  onDelete: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onCopy: () => void;
  onPaste: () => void;
  onSave: () => void;
  onDeselect: () => void;
}

/** True while the user is typing into a form field, so designer shortcuts shouldn't fire. */
function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

/**
 * Window-level keyboard shortcuts for the dashboard designer.
 *  - Delete / Backspace: delete selected
 *  - Ctrl/Cmd+Z: undo, Ctrl+Shift+Z / Ctrl+Y: redo
 *  - Ctrl/Cmd+C / V: copy / paste
 *  - Ctrl/Cmd+S: save
 *  - Esc: clear selection
 *
 * Disabled while a form field (e.g. a property editor input) has focus, so
 * e.g. Backspace-ing a position value doesn't delete the selected gauge.
 */
export function useDesignerKeyboard({
  onDelete,
  onUndo,
  onRedo,
  onCopy,
  onPaste,
  onSave,
  onDeselect,
}: UseDesignerKeyboardArgs): void {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isEditableTarget(e.target)) return;

      if (e.key === 'Delete' || e.key === 'Backspace') {
        onDelete();
      } else if (e.ctrlKey || e.metaKey) {
        if (e.key === 'z' && !e.shiftKey) {
          e.preventDefault();
          onUndo();
        } else if ((e.key === 'z' && e.shiftKey) || e.key === 'y') {
          e.preventDefault();
          onRedo();
        } else if (e.key === 'c') {
          e.preventDefault();
          onCopy();
        } else if (e.key === 'v') {
          e.preventDefault();
          onPaste();
        } else if (e.key === 's') {
          e.preventDefault();
          onSave();
        }
      } else if (e.key === 'Escape') {
        onDeselect();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onDelete, onUndo, onRedo, onCopy, onPaste, onSave, onDeselect]);
}
