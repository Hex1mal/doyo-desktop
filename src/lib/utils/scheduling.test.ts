import { describe, expect, it } from 'vitest';
import {
  dateTimeIso,
  formatDurationInput,
  normalizeTypedTime,
  parseDurationMinutes,
  quickDateKey,
  recurrenceChoice,
  recurrenceFromChoice,
  reminderFromChoice,
} from './scheduling';

describe('scheduling utilities', () => {
  it('normalizes typed 24-hour times', () => {
    expect(normalizeTypedTime('9:00')).toBe('09:00');
    expect(normalizeTypedTime('930')).toBe('09:30');
    expect(normalizeTypedTime('23:45')).toBe('23:45');
    expect(normalizeTypedTime('24:00')).toBeNull();
    expect(normalizeTypedTime('12:99')).toBeNull();
  });

  it('parses and formats duration estimates', () => {
    expect(parseDurationMinutes('15m')).toBe(15);
    expect(parseDurationMinutes('1h')).toBe(60);
    expect(parseDurationMinutes('1h 30m')).toBe(90);
    expect(parseDurationMinutes('90m')).toBe(90);
    expect(parseDurationMinutes('0m')).toBeNull();
    expect(formatDurationInput(90)).toBe('1h 30m');
  });

  it('calculates quick dates using local dates', () => {
    const now = new Date(2026, 6, 30, 23, 30);
    expect(quickDateKey('today', now)).toBe('2026-07-30');
    expect(quickDateKey('tomorrow', now)).toBe('2026-07-31');
    expect(quickDateKey('3days', now)).toBe('2026-08-02');
  });

  it('combines date and time into a local datetime', () => {
    const iso = dateTimeIso('2028-02-29', '930');
    expect(iso).not.toBeNull();
    const parsed = new Date(iso as string);
    expect(parsed.getFullYear()).toBe(2028);
    expect(parsed.getMonth()).toBe(1);
    expect(parsed.getDate()).toBe(29);
    expect(parsed.getHours()).toBe(9);
    expect(parsed.getMinutes()).toBe(30);
  });

  it('maps recurrence choices to the existing model', () => {
    expect(recurrenceFromChoice('daily')).toEqual({ pattern: 'daily', interval: 1 });
    expect(recurrenceChoice({ pattern: 'weekly', interval: 1 })).toBe('weekly');
    expect(recurrenceChoice(null)).toBe('none');
  });

  it('validates reminder requirements', () => {
    expect(() => reminderFromChoice('at-time', null)).toThrow(/requires a date/);
    expect(reminderFromChoice('none', null)).toEqual([]);
    expect(reminderFromChoice('at-time', '2026-07-30T09:00:00.000Z')).toHaveLength(1);
    expect(reminderFromChoice('-30', '2026-07-30T09:00:00.000Z')[0].offsetMinutes).toBe(-30);
  });
});
