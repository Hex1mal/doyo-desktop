/**
 * Registry of open overlay surfaces: modals, dialogs, menus and popovers.
 *
 * Doyo registers its shortcuts on `window`, which means they fire no matter
 * what the user is looking at. With a dialog open that is dangerous rather than
 * merely untidy - Delete acted on the selected tree node while the user was
 * interacting with a confirmation dialog.
 *
 * Rather than have every surface stop propagation on its own - which only works
 * if each one remembers to, and silently breaks when a surface is dismissed by
 * an outside click - surfaces register a layer here while they are open. The
 * global handler consults the registry and stands down while anything is open,
 * so the topmost surface owns the keyboard.
 *
 * Layers form a stack, so nested surfaces (a menu inside a dialog) behave
 * correctly: the registry only reports empty once every layer has closed.
 *
 * The stack is deliberately NOT `$state`. Its only consumer is the keydown
 * handler, which is an ordinary function reading the value at the moment a key
 * is pressed. Making it reactive would mean `open()` both reads and writes
 * reactive state from inside the registering `$effect`, so the effect would
 * depend on its own write, re-run, and leave the layer stranded when the
 * surface closed - the keyboard would stay disabled for the rest of the
 * session. If a component ever needs to react to overlay depth, add a separate
 * reactive signal rather than making this stack reactive.
 */

export type OverlayKind = 'modal' | 'menu';

type Layer = {
  /** Unique per registration, so two menus of the same kind cannot collide. */
  token: number;
  name: string;
  kind: OverlayKind;
};

const stack: Layer[] = [];
let nextToken = 0;

export const overlayStore = {
  /** True while any modal, dialog, menu or popover is open. */
  get isAnyOpen(): boolean {
    return stack.length > 0;
  },
  get depth(): number {
    return stack.length;
  },
  /** The most recently opened layer, which owns the keyboard. */
  get top(): Layer | null {
    return stack.length > 0 ? stack[stack.length - 1] : null;
  },
  /** Names of the open layers, outermost first. Intended for tests and debugging. */
  get names(): string[] {
    return stack.map((layer) => layer.name);
  },

  open(name: string, kind: OverlayKind = 'modal'): number {
    const token = ++nextToken;
    stack.push({ token, name, kind });
    return token;
  },

  close(token: number): void {
    const index = stack.findIndex((layer) => layer.token === token);
    if (index !== -1) stack.splice(index, 1);
  },

  /** Test seam. Nothing in the app should need this. */
  reset(): void {
    stack.length = 0;
  },
};

/**
 * Register an overlay layer for as long as `isOpen()` is true.
 *
 * Call at the top level of a component's script. The layer is released when the
 * surface closes and also when the component is destroyed, so a surface that is
 * unmounted while open - closed by an outside click, or by its owner
 * disappearing - cannot strand the keyboard.
 */
export function overlayLayer(
  name: string,
  isOpen: () => boolean,
  kind: OverlayKind = 'modal',
): void {
  $effect(() => {
    if (!isOpen()) return;
    const token = overlayStore.open(name, kind);
    return () => overlayStore.close(token);
  });
}
