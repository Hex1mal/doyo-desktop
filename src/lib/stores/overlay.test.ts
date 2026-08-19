import { describe, expect, it, beforeEach } from 'vitest';
import { overlayStore } from './overlay.svelte';

describe('overlay layer stack', () => {
  beforeEach(() => overlayStore.reset());

  it('reports open only while a layer is held', () => {
    expect(overlayStore.isAnyOpen).toBe(false);
    const token = overlayStore.open('dialog');
    expect(overlayStore.isAnyOpen).toBe(true);
    expect(overlayStore.top?.name).toBe('dialog');
    overlayStore.close(token);
    expect(overlayStore.isAnyOpen).toBe(false);
  });

  it('keeps nested layers independent and ordered', () => {
    const dialog = overlayStore.open('dialog');
    const menu = overlayStore.open('menu-in-dialog', 'menu');
    expect(overlayStore.names).toEqual(['dialog', 'menu-in-dialog']);
    expect(overlayStore.top?.kind).toBe('menu');

    overlayStore.close(menu);
    expect(overlayStore.isAnyOpen).toBe(true);
    expect(overlayStore.top?.name).toBe('dialog');

    overlayStore.close(dialog);
    expect(overlayStore.isAnyOpen).toBe(false);
  });

  it('releases layers closed out of order', () => {
    const outer = overlayStore.open('outer');
    const inner = overlayStore.open('inner');
    overlayStore.close(outer);
    expect(overlayStore.names).toEqual(['inner']);
    overlayStore.close(inner);
    expect(overlayStore.isAnyOpen).toBe(false);
  });

  it('gives every registration its own token', () => {
    const a = overlayStore.open('tree-node-menu', 'menu');
    const b = overlayStore.open('tree-node-menu', 'menu');
    expect(a).not.toBe(b);
    overlayStore.close(a);
    expect(overlayStore.depth).toBe(1);
    overlayStore.close(b);
    expect(overlayStore.depth).toBe(0);
  });

  it('ignores a token that is already closed', () => {
    const token = overlayStore.open('dialog');
    overlayStore.close(token);
    overlayStore.close(token);
    expect(overlayStore.depth).toBe(0);
  });
});
