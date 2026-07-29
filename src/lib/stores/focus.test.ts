import { describe, expect, it } from 'vitest';
import { formatFocusDuration } from '$lib/utils/focus';

describe('focus formatting', () => {
  it('formats stopwatch and pomodoro durations', () => {
    expect(formatFocusDuration(0)).toBe('0:00');
    expect(formatFocusDuration(65)).toBe('1:05');
    expect(formatFocusDuration(3661)).toBe('1:01:01');
  });

  it('clamps negative values', () => {
    expect(formatFocusDuration(-10)).toBe('0:00');
  });
});
