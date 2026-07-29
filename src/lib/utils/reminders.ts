import type { Countdown, Habit } from '$lib/types/node';
import { localDateKey } from '$lib/utils/productivity';

export interface DueReminder {
  key: string;
  title: string;
  body: string;
}

function timeKey(date: Date) {
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
}

function isHabitDueToday(habit: Habit, now: Date) {
  if (habit.archived || !habit.reminderTime) return false;
  const start = new Date(`${habit.startDate}T00:00:00`);
  if (!Number.isNaN(start.getTime()) && start > now) return false;
  if (habit.frequency === 'weekly' && habit.days.length > 0 && !habit.days.includes(now.getDay()))
    return false;
  return habit.reminderTime <= timeKey(now);
}

export function dueHabitReminders(
  habits: Habit[],
  now = new Date(),
  sentKeys = new Set<string>(),
): DueReminder[] {
  const today = localDateKey(now);
  return habits
    .filter((habit) => isHabitDueToday(habit, now))
    .map((habit) => ({
      key: `habit:${habit.id}:${today}`,
      title: `Habit reminder: ${habit.title}`,
      body:
        habit.goal > 1
          ? `Target: ${habit.goal} ${habit.goalUnit}`
          : 'Log this habit when you are done.',
    }))
    .filter((reminder) => !sentKeys.has(reminder.key));
}

export function dueCountdownReminders(
  countdowns: Countdown[],
  now = new Date(),
  sentKeys = new Set<string>(),
): DueReminder[] {
  return countdowns
    .filter((countdown) => !countdown.archived && countdown.reminderAt)
    .filter((countdown) => {
      const reminderAt = new Date(countdown.reminderAt ?? '');
      return !Number.isNaN(reminderAt.getTime()) && reminderAt <= now;
    })
    .map((countdown) => ({
      key: `countdown:${countdown.id}:${countdown.reminderAt}`,
      title: `Countdown reminder: ${countdown.title}`,
      body: countdown.mode === 'countup' ? 'Count-up event reminder.' : 'Countdown event reminder.',
    }))
    .filter((reminder) => !sentKeys.has(reminder.key));
}
