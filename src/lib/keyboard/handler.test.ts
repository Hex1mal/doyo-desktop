import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

/**
 * Doyo registers its shortcuts on `window`, so they fire regardless of what the
 * user is looking at. These tests pin the guards that stop them acting on the
 * tree behind a dialog, or stealing keys from a field the user is typing in.
 */

const nodeStore = {
  selectedId: 'node-1' as string | null,
  editingId: null as string | null,
  expandedIds: new Set<string>(),
  deleteSelected: vi.fn(),
  createSibling: vi.fn(async () => null),
  createChild: vi.fn(),
  indent: vi.fn(),
  outdent: vi.fn(),
  selectNext: vi.fn(),
  selectPrev: vi.fn(),
  select: vi.fn(),
  toggleExpand: vi.fn(),
  toggleComplete: vi.fn(),
  setPriority: vi.fn(),
  startEditing: vi.fn(),
  stopEditing: vi.fn(),
  undo: vi.fn(),
  redo: vi.fn(),
  rename: vi.fn(async () => undefined),
  getSelected: vi.fn(() => ({ id: 'node-1', parentId: null })),
  getChildren: vi.fn(() => []),
};

const uiStore = {
  quickOpenOpen: false,
  dueDatePromptOpen: false,
  focusMode: false,
  openQuickOpen: vi.fn(),
  closeQuickOpen: vi.fn(),
  openDueDatePrompt: vi.fn(),
  closeDueDatePrompt: vi.fn(),
  exitFocusMode: vi.fn(),
  toggleInspector: vi.fn(),
  toggleSidebar: vi.fn(),
  toggleTheme: vi.fn(),
  zoomIn: vi.fn(),
  zoomOut: vi.fn(),
  resetZoom: vi.fn(),
};

const commandPaletteStore = { isOpen: false, open: vi.fn(), close: vi.fn() };

vi.mock('$lib/stores/nodes.svelte', () => ({ nodeStore }));
vi.mock('$lib/stores/ui.svelte', () => ({ uiStore }));
vi.mock('$lib/stores/command-palette.svelte', () => ({ commandPaletteStore }));

/** Every store method that mutates data. None may run from inside a modal. */
const mutators = [
  nodeStore.deleteSelected,
  nodeStore.createSibling,
  nodeStore.createChild,
  nodeStore.indent,
  nodeStore.outdent,
  nodeStore.toggleComplete,
  nodeStore.setPriority,
  nodeStore.undo,
  nodeStore.redo,
  nodeStore.startEditing,
];

/**
 * Minimal DOM stand-ins. The suite has no jsdom, by design - see
 * ui-prefs.test.ts - and the handler only needs `instanceof HTMLElement`,
 * `tagName`, `isContentEditable`, `classList` and `closest`.
 */
class FakeElement {
  tagName: string;
  isContentEditable = false;
  role: string | null = null;
  href: string | null = null;
  classList = { contains: (name: string) => this.classes.has(name) };
  private classes = new Set<string>();

  constructor(tagName: string, opts: { role?: string; href?: string; classes?: string[] } = {}) {
    this.tagName = tagName.toUpperCase();
    this.role = opts.role ?? null;
    this.href = opts.href ?? null;
    for (const c of opts.classes ?? []) this.classes.add(c);
  }

  /** Enough of `closest` for the activatable-control selector the handler uses. */
  closest(selector: string): FakeElement | null {
    const parts = selector.split(',').map((part) => part.trim());
    for (const part of parts) {
      if (part === 'button' && this.tagName === 'BUTTON') return this;
      if (part === 'summary' && this.tagName === 'SUMMARY') return this;
      if (part === 'a[href]' && this.tagName === 'A' && this.href !== null) return this;
      const roleMatch = /^\[role="(.+)"\]$/.exec(part);
      if (roleMatch && this.role === roleMatch[1]) return this;
    }
    return null;
  }
}

vi.stubGlobal('HTMLElement', FakeElement);

type Press = { key: string; ctrlKey?: boolean; metaKey?: boolean; shiftKey?: boolean };

function key(press: Press, target: unknown) {
  const event = {
    ...press,
    ctrlKey: press.ctrlKey ?? false,
    metaKey: press.metaKey ?? false,
    shiftKey: press.shiftKey ?? false,
    target,
    defaultPrevented: false,
    preventDefault() {
      (this as { defaultPrevented: boolean }).defaultPrevented = true;
    },
    stopPropagation() {},
  };
  return event as unknown as KeyboardEvent & { defaultPrevented: boolean };
}

/** Stands in for "focus is nowhere in particular". */
const background = new FakeElement('div');

let handleGlobalKeydown: (e: KeyboardEvent) => void;
let overlayStore: typeof import('$lib/stores/overlay.svelte').overlayStore;

beforeEach(async () => {
  vi.clearAllMocks();
  nodeStore.selectedId = 'node-1';
  nodeStore.editingId = null;
  uiStore.quickOpenOpen = false;
  uiStore.dueDatePromptOpen = false;
  uiStore.focusMode = false;
  commandPaletteStore.isOpen = false;
  ({ handleGlobalKeydown } = await import('./handler'));
  ({ overlayStore } = await import('$lib/stores/overlay.svelte'));
  overlayStore.reset();
});

afterEach(() => overlayStore.reset());

describe('while a modal or menu is open', () => {
  // The surfaces that register an overlay layer, by the name they register.
  const surfaces = [
    'move-dialog',
    'node-config-dialog',
    'startup-recovery',
    'command-palette',
    'restore-confirm',
    'quick-open',
    'due-date-prompt',
    'tree-node-menu',
    'tree-node-move-confirm',
    'sidebar-node-menu',
    'calendar-block-menu',
  ];

  for (const surface of surfaces) {
    it(`Delete does not touch the selected node behind ${surface}`, () => {
      overlayStore.open(surface);
      handleGlobalKeydown(key({ key: 'Delete' }, background));
      expect(nodeStore.deleteSelected).not.toHaveBeenCalled();
    });
  }

  it('leaves every mutating shortcut alone', () => {
    overlayStore.open('move-dialog');
    const presses: Press[] = [
      { key: 'Delete' },
      { key: 'Enter' },
      { key: 'Enter', shiftKey: true },
      { key: 'Tab' },
      { key: 'Tab', shiftKey: true },
      { key: 'F2' },
      { key: ' ' },
      { key: 'ArrowDown' },
      { key: 'n', ctrlKey: true },
      { key: 'z', ctrlKey: true },
      { key: 'y', ctrlKey: true },
      { key: '1', ctrlKey: true },
      { key: 'Enter', ctrlKey: true },
    ];
    for (const press of presses) handleGlobalKeydown(key(press, background));

    for (const fn of mutators) expect(fn).not.toHaveBeenCalled();
    expect(nodeStore.selectNext).not.toHaveBeenCalled();
    expect(uiStore.toggleInspector).not.toHaveBeenCalled();
  });

  it('does not consume the event, so the surface can handle it', () => {
    overlayStore.open('restore-confirm');
    const event = key({ key: 'Escape' }, background);
    handleGlobalKeydown(event);
    expect(event.defaultPrevented).toBe(false);
    // The background must not react to Escape either.
    expect(nodeStore.stopEditing).not.toHaveBeenCalled();
    expect(commandPaletteStore.close).not.toHaveBeenCalled();
  });

  it('stays closed until every nested layer is released', () => {
    const dialog = overlayStore.open('move-dialog');
    const menu = overlayStore.open('tree-node-menu', 'menu');
    expect(overlayStore.depth).toBe(2);

    overlayStore.close(menu);
    handleGlobalKeydown(key({ key: 'Delete' }, background));
    expect(nodeStore.deleteSelected).not.toHaveBeenCalled();

    overlayStore.close(dialog);
    handleGlobalKeydown(key({ key: 'Delete' }, background));
    expect(nodeStore.deleteSelected).toHaveBeenCalledTimes(1);
  });
});

describe('shortcuts resume once the modal closes', () => {
  it('restores tree editing keys', () => {
    const token = overlayStore.open('move-dialog');
    handleGlobalKeydown(key({ key: 'Delete' }, background));
    expect(nodeStore.deleteSelected).not.toHaveBeenCalled();

    overlayStore.close(token);

    handleGlobalKeydown(key({ key: 'Delete' }, background));
    handleGlobalKeydown(key({ key: 'Enter' }, background));
    handleGlobalKeydown(key({ key: 'Tab' }, background));
    handleGlobalKeydown(key({ key: 'ArrowDown' }, background));

    expect(nodeStore.deleteSelected).toHaveBeenCalledTimes(1);
    expect(nodeStore.createSibling).toHaveBeenCalledTimes(1);
    expect(nodeStore.indent).toHaveBeenCalledTimes(1);
    expect(nodeStore.selectNext).toHaveBeenCalledTimes(1);
  });
});

describe('typing in a field never runs task or tree commands', () => {
  const fields: [string, FakeElement][] = [
    ['input', new FakeElement('input')],
    ['textarea', new FakeElement('textarea')],
    ['select', new FakeElement('select')],
    ['contenteditable', Object.assign(new FakeElement('div'), { isContentEditable: true })],
  ];

  for (const [label, field] of fields) {
    it(`plain keys stay with the ${label}`, () => {
      for (const k of ['Delete', 'Enter', 'Tab', ' ', 'F2', 'ArrowDown', 'ArrowUp']) {
        handleGlobalKeydown(key({ key: k }, field));
      }
      for (const fn of mutators) expect(fn).not.toHaveBeenCalled();
      expect(nodeStore.selectNext).not.toHaveBeenCalled();
    });
  }

  it('Backspace and Delete edit the text rather than the tree', () => {
    const input = new FakeElement('input');
    handleGlobalKeydown(key({ key: 'Backspace' }, input));
    handleGlobalKeydown(key({ key: 'Delete' }, input));
    expect(nodeStore.deleteSelected).not.toHaveBeenCalled();
  });

  it('Ctrl+Z undoes the text, not the last node operation', () => {
    const input = new FakeElement('input');
    handleGlobalKeydown(key({ key: 'z', ctrlKey: true }, input));
    handleGlobalKeydown(key({ key: 'y', ctrlKey: true }, input));
    expect(nodeStore.undo).not.toHaveBeenCalled();
    expect(nodeStore.redo).not.toHaveBeenCalled();
  });

  it('Ctrl+N does not create a node while typing', () => {
    const input = new FakeElement('input');
    handleGlobalKeydown(key({ key: 'n', ctrlKey: true }, input));
    expect(nodeStore.createSibling).not.toHaveBeenCalled();
  });

  it('still allows the app chrome chords', () => {
    const input = new FakeElement('input');
    handleGlobalKeydown(key({ key: 'k', ctrlKey: true }, input));
    expect(commandPaletteStore.open).toHaveBeenCalledTimes(1);
  });
});

describe('focused controls keep their own keys', () => {
  it('Enter and Space activate a button instead of the tree', () => {
    const button = new FakeElement('button');
    handleGlobalKeydown(key({ key: 'Enter' }, button));
    handleGlobalKeydown(key({ key: ' ' }, button));
    expect(nodeStore.createSibling).not.toHaveBeenCalled();
    expect(uiStore.toggleInspector).not.toHaveBeenCalled();
  });

  it('but arrows still navigate the tree from a button', () => {
    const button = new FakeElement('button');
    handleGlobalKeydown(key({ key: 'ArrowDown' }, button));
    expect(nodeStore.selectNext).toHaveBeenCalledTimes(1);
  });
});

describe('Escape with nothing open', () => {
  it('stops inline editing', () => {
    nodeStore.editingId = 'node-1';
    const event = key({ key: 'Escape' }, background);
    handleGlobalKeydown(event);
    expect(nodeStore.stopEditing).toHaveBeenCalledTimes(1);
    expect(event.defaultPrevented).toBe(true);
  });

  it('leaves focus mode', () => {
    uiStore.focusMode = true;
    handleGlobalKeydown(key({ key: 'Escape' }, background));
    expect(uiStore.exitFocusMode).toHaveBeenCalledTimes(1);
  });
});
