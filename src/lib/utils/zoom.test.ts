import { describe, expect, it } from 'vitest';
import { clampZoom, nextZoom, zoomActionFromKeyboard } from './zoom';

describe('application zoom utilities', () => {
  it('detects standard and numpad zoom shortcuts', () => {
    expect(zoomActionFromKeyboard({ key: '+', code: 'Equal', ctrlKey: true, metaKey: false })).toBe(
      'in',
    );
    expect(zoomActionFromKeyboard({ key: '=', code: 'Equal', ctrlKey: true, metaKey: false })).toBe(
      'in',
    );
    expect(
      zoomActionFromKeyboard({ key: 'Add', code: 'NumpadAdd', ctrlKey: true, metaKey: false }),
    ).toBe('in');
    expect(zoomActionFromKeyboard({ key: '-', code: 'Minus', ctrlKey: true, metaKey: false })).toBe(
      'out',
    );
    expect(
      zoomActionFromKeyboard({
        key: 'Subtract',
        code: 'NumpadSubtract',
        ctrlKey: true,
        metaKey: false,
      }),
    ).toBe('out');
    expect(
      zoomActionFromKeyboard({ key: '0', code: 'Digit0', ctrlKey: true, metaKey: false }),
    ).toBe('reset');
  });

  it('ignores zoom keys without Ctrl or Meta', () => {
    expect(
      zoomActionFromKeyboard({ key: '+', code: 'Equal', ctrlKey: false, metaKey: false }),
    ).toBe(null);
  });

  it('clamps and steps zoom values deterministically', () => {
    expect(clampZoom(0.1)).toBe(0.8);
    expect(clampZoom(3)).toBe(2);
    expect(nextZoom(1, 'in')).toBe(1.1);
    expect(nextZoom(1, 'out')).toBe(0.9);
    expect(nextZoom(1.95, 'in')).toBe(2);
    expect(nextZoom(0.81, 'out')).toBe(0.8);
    expect(nextZoom(1.4, 'reset')).toBe(1);
  });
});
