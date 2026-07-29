import { describe, expect, it } from 'vitest';
import { dueCountdownReminders, dueHabitReminders } from './reminders';
import type { Countdown, Habit } from '$lib/types/node';

function habit(partial: Partial<Habit>): Habit {
  return {
    id: 'habit-1',
    title: 'Read',
    icon: '',
    color: null,
    frequency: 'daily',
    days: [],
    goal: 1,
    goalUnit: 'count',
    startDate: '2026-07-01',
    reminderTime: '08:00',
    archived: false,
    position: 0,
    createdAt: '',
    updatedAt: '',
    ...partial,
  };
}

function countdown(partial: Partial<Countdown>): Countdown {
  return {
    id: 'countdown-1',
    title: 'Exam',
    targetDate: '2026-08-01T00:00:00Z',
    mode: 'countdown',
    icon: '',
    color: null,
    recurrence: null,
    reminderAt: '2026-07-29T00:00:00Z',
    archived: false,
    position: 0,
    createdAt: '',
    updatedAt: '',
    ...partial,
  };
}

describe('reminder utilities', () => {
  it('returns due habit reminders once per local day', () => {
    const now = new Date(2026, 6, 29, 8, 30, 0);
    const reminders = dueHabitReminders([habit({})], now);
    expect(reminders).toHaveLength(1);
    expect(dueHabitReminders([habit({})], now, new Set([reminders[0].key]))).toHaveLength(0);
  });

  it('respects weekly habit day selection', () => {
    const monday = new Date(2026, 6, 27, 9, 0, 0);
    expect(dueHabitReminders([habit({ frequency: 'weekly', days: [1] })], monday)).toHaveLength(1);
    expect(dueHabitReminders([habit({ frequency: 'weekly', days: [2] })], monday)).toHaveLength(0);
  });

  it('returns countdown reminders after their reminder time', () => {
    const now = new Date('2026-07-29T00:01:00Z');
    expect(dueCountdownReminders([countdown({})], now)).toHaveLength(1);
    expect(dueCountdownReminders([countdown({ archived: true })], now)).toHaveLength(0);
  });
});
