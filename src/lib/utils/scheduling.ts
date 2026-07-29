import type { RecurrenceConfig, ReminderConfig } from '$lib/types/node';
import { addDays, localDayKey, parseLocalDayKey, startOfLocalDay } from './calendar';

export function normalizeTypedTime(input: string): string | null {
  const value = input.trim();
  if (!value) return '';
  const colon = /^(\d{1,2}):(\d{2})$/.exec(value);
  const compact = /^(\d{1,2})(\d{2})$/.exec(value);
  const match = colon ?? compact;
  if (!match) return null;
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (!Number.isInteger(hour) || !Number.isInteger(minute)) return null;
  if (hour < 0 || hour > 23 || minute < 0 || minute > 59) return null;
  return `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`;
}

export function parseDurationMinutes(input: string): number | null {
  const value = input.trim().toLowerCase();
  if (!value) return null;
  const compact = /^(\d+)\s*m?$/.exec(value);
  if (compact) {
    const minutes = Number(compact[1]);
    return minutes > 0 ? minutes : null;
  }
  const match = /^(?:(\d+)\s*h)?\s*(?:(\d+)\s*m)?$/.exec(value);
  if (!match || (!match[1] && !match[2])) return null;
  const minutes = Number(match[1] ?? 0) * 60 + Number(match[2] ?? 0);
  return minutes > 0 ? minutes : null;
}

export function formatDurationInput(minutes?: number | null): string {
  if (!minutes || minutes <= 0) return '';
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (!hours) return `${rest}m`;
  if (!rest) return `${hours}h`;
  return `${hours}h ${rest}m`;
}

export function quickDateKey(action: string, now = new Date()) {
  const today = startOfLocalDay(now);
  if (action === 'today') return localDayKey(today);
  if (action === 'tomorrow') return localDayKey(addDays(today, 1));
  if (action === '3days') return localDayKey(addDays(today, 3));
  if (action === 'sunday') {
    const delta = (7 - today.getDay()) % 7;
    return localDayKey(addDays(today, delta));
  }
  if (action === 'none') return '';
  return localDayKey(today);
}

export function dateTimeIso(dayKey: string, time: string) {
  const day = parseLocalDayKey(dayKey);
  if (!day) return null;
  const normalized = normalizeTypedTime(time);
  if (normalized === null) return null;
  if (normalized) {
    const [hour, minute] = normalized.split(':').map(Number);
    day.setHours(hour, minute, 0, 0);
  }
  return day.toISOString();
}

export function recurrenceFromChoice(choice: string): RecurrenceConfig | null {
  if (choice === 'none') return null;
  if (choice === 'daily') return { pattern: 'daily', interval: 1 };
  if (choice === 'weekly') return { pattern: 'weekly', interval: 1 };
  if (choice === 'monthly') return { pattern: 'monthly', interval: 1 };
  return null;
}

export function recurrenceChoice(recurrence?: RecurrenceConfig | null) {
  if (!recurrence) return 'none';
  if (['daily', 'weekly', 'monthly'].includes(recurrence.pattern)) return recurrence.pattern;
  return 'custom';
}

export function reminderFromChoice(choice: string, dueDate: string | null): ReminderConfig[] {
  if (choice === 'none') return [];
  if (!dueDate) throw new Error('A reminder requires a date.');
  const due = new Date(dueDate);
  if (Number.isNaN(due.getTime())) throw new Error('A reminder requires a valid date.');
  if (choice === 'at-time')
    return [{ time: due.toISOString(), offsetMinutes: null, type: 'absolute' }];
  const offset = Number(choice);
  if (!Number.isFinite(offset)) return [];
  return [{ time: null, offsetMinutes: offset, type: 'relative' }];
}
