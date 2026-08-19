import { commandPaletteStore } from '$lib/stores/command-palette.svelte';
import { uiStore } from '$lib/stores/ui.svelte';
import { nodeStore } from '$lib/stores/nodes.svelte';
import { overlayStore } from '$lib/stores/overlay.svelte';
import { zoomActionFromKeyboard } from '$lib/utils/zoom';

/** Somewhere the user is typing, so plain keys and text editing belong to it. */
function isTextEntryTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  // A select consumes typing, arrows and Space to choose an option.
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

/**
 * A control that owns Enter and Space itself. Stealing those turns "press the
 * focused button" into "create a sibling" or "toggle the inspector".
 */
function isActivatableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return Boolean(target.closest('button, summary, a[href], [role="button"], [role="menuitem"]'));
}

export function handleGlobalKeydown(e: KeyboardEvent) {
  // A modal, dialog, menu or popover owns the keyboard while it is open. These
  // shortcuts are registered on window, so without this they act on the tree
  // behind whatever the user is actually looking at. Every surface handles its
  // own keys, Escape included.
  if (overlayStore.isAnyOpen) return;

  const inInput = isTextEntryTarget(e.target);
  const ctrl = e.ctrlKey || e.metaKey;

  // Always allow Escape
  if (e.key === 'Escape') {
    if (commandPaletteStore.isOpen) {
      commandPaletteStore.close();
      e.preventDefault();
      return;
    }
    if (uiStore.quickOpenOpen) {
      uiStore.closeQuickOpen();
      e.preventDefault();
      return;
    }
    if (uiStore.dueDatePromptOpen) {
      uiStore.closeDueDatePrompt();
      e.preventDefault();
      return;
    }
    if (uiStore.focusMode) {
      uiStore.exitFocusMode();
      e.preventDefault();
      return;
    }
    if (nodeStore.editingId) {
      nodeStore.stopEditing();
      e.preventDefault();
      return;
    }
    return;
  }

  // Ctrl/Cmd chords always work — even in inputs
  if (ctrl) {
    const key = e.key.toLowerCase();
    const zoomAction = zoomActionFromKeyboard(e);
    if (zoomAction) {
      e.preventDefault();
      if (zoomAction === 'in') uiStore.zoomIn();
      else if (zoomAction === 'out') uiStore.zoomOut();
      else uiStore.resetZoom();
      return;
    }
    if (key === 'k') {
      e.preventDefault();
      commandPaletteStore.open();
      return;
    }
    if (key === 'p') {
      e.preventDefault();
      uiStore.openQuickOpen();
      return;
    }
    if (key === 'f') {
      e.preventDefault();
      uiStore.openQuickOpen();
      return;
    }
    if (key === 'n' && !e.shiftKey && !inInput) {
      e.preventDefault();
      nodeStore.createSibling('');
      return;
    }
    if (key === 'n' && e.shiftKey && !inInput) {
      e.preventDefault();
      nodeStore.createSibling('').then((n) => {
        if (n) nodeStore.rename(n.id, '').catch(() => {});
      });
      // quick capture = sibling at root if nothing selected
      return;
    }
    if (key === 'z' && !e.shiftKey && !inInput) {
      e.preventDefault();
      nodeStore.undo();
      return;
    }
    if (((key === 'z' && e.shiftKey) || key === 'y') && !inInput) {
      e.preventDefault();
      nodeStore.redo();
      return;
    }
    if (key === 'i') {
      e.preventDefault();
      uiStore.toggleInspector();
      return;
    }
    if (key === '\\') {
      e.preventDefault();
      uiStore.toggleSidebar();
      return;
    }
    if (key === 'd' && e.shiftKey) {
      e.preventDefault();
      uiStore.toggleTheme();
      return;
    }
    if (key === 'd' && !e.shiftKey && !inInput) {
      e.preventDefault();
      if (nodeStore.selectedId) uiStore.openDueDatePrompt();
      return;
    }
    if (key === 'enter' && !inInput) {
      e.preventDefault();
      if (nodeStore.selectedId) nodeStore.toggleComplete(nodeStore.selectedId);
      return;
    }
    if (['1', '2', '3', '4'].includes(key) && !inInput) {
      e.preventDefault();
      if (nodeStore.selectedId) nodeStore.setPriority(nodeStore.selectedId, Number(key));
      return;
    }
    if (key === 'enter' && inInput && (e.target as HTMLElement).classList.contains('title-input')) {
      // handled by input itself
      return;
    }
  }

  // While typing in inputs, don't steal plain keys
  if (inInput) return;

  // A focused button or link owns Enter and Space. Without this, activating a
  // toolbar button with the keyboard creates a sibling or toggles the
  // inspector instead of pressing the button.
  if ((e.key === 'Enter' || e.key === ' ') && isActivatableTarget(e.target)) return;

  // Tree navigation / editing keys
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault();
      nodeStore.selectNext();
      break;
    case 'ArrowUp':
      e.preventDefault();
      nodeStore.selectPrev();
      break;
    case 'ArrowRight': {
      e.preventDefault();
      const sel = nodeStore.getSelected();
      if (!sel) break;
      const children = nodeStore.getChildren(sel.id);
      if (children.length > 0 && !nodeStore.expandedIds.has(sel.id)) {
        nodeStore.toggleExpand(sel.id);
      } else if (children.length > 0) {
        nodeStore.select(children[0].id);
      }
      break;
    }
    case 'ArrowLeft': {
      e.preventDefault();
      const sel = nodeStore.getSelected();
      if (!sel) break;
      if (nodeStore.expandedIds.has(sel.id) && nodeStore.getChildren(sel.id).length > 0) {
        nodeStore.toggleExpand(sel.id);
      } else if (sel.parentId) {
        nodeStore.select(sel.parentId);
      }
      break;
    }
    case 'Enter':
      e.preventDefault();
      if (e.shiftKey) {
        nodeStore.createChild('');
      } else {
        nodeStore.createSibling('');
      }
      break;
    case 'Tab':
      e.preventDefault();
      if (e.shiftKey) nodeStore.outdent();
      else nodeStore.indent();
      break;
    case 'Delete':
      e.preventDefault();
      nodeStore.deleteSelected();
      break;
    case 'F2':
      e.preventDefault();
      if (nodeStore.selectedId) nodeStore.startEditing(nodeStore.selectedId);
      break;
    case ' ':
      e.preventDefault();
      if (nodeStore.selectedId) {
        uiStore.toggleInspector();
      }
      break;
    default:
      break;
  }
}
