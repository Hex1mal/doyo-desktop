export interface Keybinding {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: string;
  context?: 'global' | 'tree' | 'editor' | 'modal';
  description: string;
}

export const DEFAULT_KEYBINDINGS: Keybinding[] = [
  { key: 'k', ctrl: true, action: 'command-palette.open', context: 'global', description: 'Open command palette' },
  { key: 'p', ctrl: true, action: 'quick-open.open', context: 'global', description: 'Quick open' },
  { key: 'n', ctrl: true, action: 'node.create', context: 'tree', description: 'Create new node' },
  { key: 'N', ctrl: true, shift: true, action: 'quick-capture', context: 'global', description: 'Quick capture to inbox' },
  { key: 'Enter', action: 'node.create-sibling', context: 'tree', description: 'Create sibling' },
  { key: 'Enter', ctrl: true, action: 'node.toggle-complete', context: 'tree', description: 'Toggle complete' },
  { key: 'Tab', action: 'node.indent', context: 'tree', description: 'Indent' },
  { key: 'Tab', shift: true, action: 'node.outdent', context: 'tree', description: 'Outdent' },
  { key: 'ArrowUp', action: 'tree.move-up', context: 'tree', description: 'Move up' },
  { key: 'ArrowDown', action: 'tree.move-down', context: 'tree', description: 'Move down' },
  { key: 'ArrowLeft', action: 'tree.collapse', context: 'tree', description: 'Collapse' },
  { key: 'ArrowRight', action: 'tree.expand', context: 'tree', description: 'Expand' },
  { key: 'z', ctrl: true, action: 'undo', context: 'global', description: 'Undo' },
  { key: 'Z', ctrl: true, shift: true, action: 'redo', context: 'global', description: 'Redo' },
  { key: 'f', ctrl: true, action: 'search.focus', context: 'global', description: 'Search' },
  { key: 'i', ctrl: true, action: 'inspector.toggle', context: 'global', description: 'Toggle inspector' },
  { key: '\\', ctrl: true, action: 'sidebar.toggle', context: 'global', description: 'Toggle sidebar' },
  { key: 'D', ctrl: true, shift: true, action: 'theme.toggle', context: 'global', description: 'Toggle dark mode' },
  { key: 'd', ctrl: true, action: 'node.set-due-date', context: 'tree', description: 'Set due date' },
  { key: '1', ctrl: true, action: 'node.set-priority-1', context: 'tree', description: 'Priority 1' },
  { key: '2', ctrl: true, action: 'node.set-priority-2', context: 'tree', description: 'Priority 2' },
  { key: '3', ctrl: true, action: 'node.set-priority-3', context: 'tree', description: 'Priority 3' },
  { key: '4', ctrl: true, action: 'node.set-priority-4', context: 'tree', description: 'Priority 4' },
  { key: 'Escape', action: 'escape', context: 'global', description: 'Close/Escape' },
  { key: 'Delete', action: 'node.delete', context: 'tree', description: 'Delete node' },
  { key: 'Backspace', action: 'node.delete', context: 'tree', description: 'Delete node' },
];
