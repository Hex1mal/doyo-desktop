import { describe, expect, it } from 'vitest';
import {
  calculateParetoScore,
  buildDailyBuckets,
  countdownDelta,
  flowtimeBreakSuggestion,
  isWithinRange,
  localDateKey,
  normalizeFilterDefinition,
  rangeStart,
} from './productivity';

describe('productivity utilities', () => {
  it('removes empty filter fields without losing meaningful false values', () => {
    expect(
      normalizeFilterDefinition({
        text: '',
        priority: null,
        pinned: false,
        tagIds: [],
        completion: 'active',
      }),
    ).toEqual({ pinned: false, completion: 'active' });
  });

  it('calculates bounded Pareto scores', () => {
    expect(calculateParetoScore(80, 20)).toBe(400);
    expect(calculateParetoScore(120, 0)).toBe(10_000);
    expect(calculateParetoScore(Number.NaN, 20)).toBe(0);
  });

  it('suggests flexible Flowtime breaks from session length', () => {
    expect(flowtimeBreakSuggestion(10 * 60)).toBe(5);
    expect(flowtimeBreakSuggestion(35 * 60)).toBe(8);
    expect(flowtimeBreakSuggestion(60 * 60)).toBe(12);
    expect(flowtimeBreakSuggestion(100 * 60)).toBe(20);
  });

  it('computes countdown and countup deltas', () => {
    const now = new Date('2026-07-28T00:00:00Z');
    expect(countdownDelta('2026-07-30T00:00:00Z', 'countdown', now)).toEqual({
      days: 2,
      label: '2 days left',
    });
    expect(countdownDelta('2026-07-26T00:00:00Z', 'countup', now)).toEqual({
      days: 2,
      label: '2 days since',
    });
  });

  it('formats local date keys', () => {
    expect(localDateKey(new Date(2026, 6, 8))).toBe('2026-07-08');
  });

  it('calculates range starts and membership', () => {
    const now = new Date(2026, 6, 28, 15, 0, 0);
    expect(localDateKey(rangeStart('week', now))).toBe('2026-07-22');
    expect(isWithinRange(new Date(2026, 6, 22, 0, 0, 0).toISOString(), 'week', now)).toBe(true);
    expect(isWithinRange(new Date(2026, 6, 21, 23, 59, 59).toISOString(), 'week', now)).toBe(false);
  });

  it('builds daily trend buckets', () => {
    const now = new Date(2026, 6, 28, 15, 0, 0);
    const buckets = buildDailyBuckets(
      'week',
      [
        { date: new Date(2026, 6, 28, 1, 0, 0).toISOString() },
        { date: new Date(2026, 6, 28, 2, 0, 0).toISOString(), amount: 2 },
        { date: new Date(2026, 6, 27, 0, 0, 0).toISOString() },
      ],
      now,
    );
    expect(buckets).toHaveLength(7);
    expect(buckets.at(-1)).toEqual({ key: '2026-07-28', value: 3 });
    expect(buckets.at(-2)).toEqual({ key: '2026-07-27', value: 1 });
  });
});
