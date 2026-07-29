import { describe, expect, it } from 'vitest';
import type { Node } from '../types/node';
import {
  localDayKey,
  monthGrid,
  moveTaskDate,
  parseLocalDayKey,
  hourFromPointerY,
  tasksByDay,
  validateTimeRange,
  visibleRange,
  weekStart,
} from './calendar';

function task(partial: Partial<Node>): Node {
  return {
    id: 'task',
    parentId: 'workspace',
    position: 0,
    nodeType: 'Task',
    title: 'Task',
    body: '',
    properties: {},
    isCollapsed: false,
    isCompleted: false,
    completedAt: null,
    deletedAt: null,
    version: 1,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...partial,
  };
}

describe('calendar utilities', () => {
  it('builds a stable month grid across month and year boundaries', () => {
    const jan = monthGrid(new Date(2027, 0, 1), 1);
    expect(jan).toHaveLength(42);
    expect(localDayKey(jan[0])).toBe('2026-12-28');
    expect(localDayKey(jan[41])).toBe('2027-02-07');
  });

  it('handles leap-year February', () => {
    const feb = monthGrid(new Date(2028, 1, 10), 0);
    expect(feb.some((day) => localDayKey(day) === '2028-02-29')).toBe(true);
  });

  it('calculates week starts with configured first day', () => {
    expect(localDayKey(weekStart(new Date(2026, 6, 28), 1))).toBe('2026-07-27');
    expect(localDayKey(weekStart(new Date(2026, 6, 28), 0))).toBe('2026-07-26');
  });

  it('uses local day boundaries for day range', () => {
    const range = visibleRange('day', new Date(2026, 6, 28, 23, 30), 1);
    expect(localDayKey(range.start)).toBe('2026-07-28');
    expect(localDayKey(range.end)).toBe('2026-07-29');
  });

  it('parses calendar drop day keys as local dates', () => {
    const parsed = parseLocalDayKey('2026-07-28');
    expect(parsed).not.toBeNull();
    expect(parsed?.getFullYear()).toBe(2026);
    expect(parsed?.getMonth()).toBe(6);
    expect(parsed?.getDate()).toBe(28);
    expect(parseLocalDayKey('2026-02-31')).toBeNull();
    expect(parseLocalDayKey('not-a-day')).toBeNull();
  });

  it('maps pointer y-position to quarter-hour slot times', () => {
    const slot = {
      dataset: { calendarHour: '9' },
      getBoundingClientRect: () => ({ top: 100, height: 80 }),
    } as unknown as HTMLElement;
    expect(hourFromPointerY(100, slot)).toEqual({ hour: 9, minute: 0 });
    expect(hourFromPointerY(120, slot)).toEqual({ hour: 9, minute: 15 });
    expect(hourFromPointerY(150, slot)).toEqual({ hour: 9, minute: 45 });
    expect(hourFromPointerY(179, slot)).toEqual({ hour: 10, minute: 0 });
  });

  it('moves tasks without changing hierarchy fields', () => {
    const original = task({ parentId: 'group', properties: { priority: 1 } });
    const moved = moveTaskDate(original, new Date(2026, 7, 1), 14);
    expect(new Date(moved.dueDate).getHours()).toBe(14);
    expect(original.parentId).toBe('group');
  });

  it('rejects invalid time ranges', () => {
    const start = new Date('2026-07-28T10:00:00Z');
    expect(validateTimeRange(start, new Date('2026-07-28T10:00:00Z'))).toBe(false);
    expect(validateTimeRange(start, new Date('2026-07-28T09:59:00Z'))).toBe(false);
    expect(validateTimeRange(start, new Date('2026-07-28T10:15:00Z'))).toBe(true);
  });

  it('excludes deleted tasks and optionally includes completed tasks', () => {
    const active = task({ id: 'a', properties: { dueDate: '2026-07-28T00:00:00Z' } });
    const completed = task({
      id: 'c',
      isCompleted: true,
      properties: { dueDate: '2026-07-28T00:00:00Z' },
    });
    const deleted = task({
      id: 'd',
      deletedAt: '2026-07-28T00:00:00Z',
      properties: { dueDate: '2026-07-28T00:00:00Z' },
    });

    expect(tasksByDay([active, completed, deleted], false).get('2026-07-28')?.map((n) => n.id)).toEqual(['a']);
    expect(tasksByDay([active, completed, deleted], true).get('2026-07-28')?.map((n) => n.id)).toEqual(['a', 'c']);
  });
});
