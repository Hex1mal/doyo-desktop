import { describe, expect, it } from 'vitest';
import type { FocusSession, HabitLog, Node } from '$lib/types/node';
import { focusStatistics, habitStatistics, taskStatistics, uniqueActiveTaskRecords } from './statistics';

function node(partial: Partial<Node> & Pick<Node, 'id' | 'nodeType'>): Node {
  return {
    parentId: null,
    position: 0,
    title: '',
    body: '',
    properties: {},
    isCollapsed: false,
    isCompleted: false,
    completedAt: null,
    deletedAt: null,
    version: 1,
    createdAt: '2026-07-01T00:00:00Z',
    updatedAt: '2026-07-01T00:00:00Z',
    ...partial,
  };
}

function focus(partial: Partial<FocusSession>): FocusSession {
  return {
    id: 'f',
    taskId: null,
    taskTitle: '',
    method: 'pomodoro',
    state: 'completed',
    pomodoroPhase: 'focus',
    pomodoroCycle: 1,
    plannedSeconds: 1500,
    accumulatedSeconds: 0,
    elapsedSeconds: 0,
    durationSeconds: 1200,
    interruptions: 0,
    note: '',
    startedAt: '2026-07-29T01:00:00Z',
    lastStartedAt: null,
    endedAt: '2026-07-29T01:20:00Z',
    createdAt: '',
    updatedAt: '',
    ...partial,
  };
}

function log(partial: Partial<HabitLog>): HabitLog {
  return {
    id: 'l',
    habitId: 'h',
    logDate: '2026-07-29',
    status: 'completed',
    value: 1,
    note: '',
    createdAt: '',
    updatedAt: '',
    ...partial,
  };
}

describe('statistics utilities', () => {
  it('counts each task record once and excludes deleted non-task nodes', () => {
    const parent = node({ id: 't1', nodeType: 'Task' });
    const subtask = node({ id: 't2', nodeType: 'Task', parentId: 't1' });
    const duplicateParent = { ...parent };
    const group = node({ id: 'g', nodeType: 'Group' });
    const deleted = node({ id: 't3', nodeType: 'Task', deletedAt: '2026-07-29T00:00:00Z' });
    expect(uniqueActiveTaskRecords([parent, subtask, duplicateParent, group, deleted]).map((task) => task.id)).toEqual([
      't1',
      't2',
    ]);
  });

  it('builds task totals without recursive descendant double counting', () => {
    const now = new Date('2026-07-29T12:00:00Z');
    const stats = taskStatistics(
      [
        node({ id: 'parent', nodeType: 'Task', isCompleted: true, completedAt: '2026-07-29T01:00:00Z' }),
        node({ id: 'child', nodeType: 'Task', parentId: 'parent', isCompleted: true, completedAt: '2026-07-29T02:00:00Z' }),
      ],
      'day',
      now,
    );
    expect(stats.totalTasks).toBe(2);
    expect(stats.completedInRange).toBe(2);
    expect(stats.completionRate).toBe(100);
  });

  it('separates focus methods and planned versus actual time', () => {
    const now = new Date('2026-07-29T12:00:00Z');
    const stats = focusStatistics(
      [
        focus({ id: 'p', method: 'pomodoro', plannedSeconds: 1500, durationSeconds: 1200 }),
        focus({ id: 's', method: 'stopwatch', plannedSeconds: 0, durationSeconds: 600 }),
        focus({ id: 'fl', method: 'flowtime', plannedSeconds: 0, durationSeconds: 900 }),
      ],
      'day',
      now,
    );
    expect(stats.pomodoroSeconds).toBe(1200);
    expect(stats.stopwatchSeconds).toBe(600);
    expect(stats.flowtimeSeconds).toBe(900);
    expect(stats.plannedSeconds).toBe(1500);
    expect(stats.actualSeconds).toBe(2700);
  });

  it('calculates habit completion rates from real logs', () => {
    const now = new Date('2026-07-29T12:00:00Z');
    const stats = habitStatistics(
      [log({ id: 'a', status: 'completed' }), log({ id: 'b', status: 'partial' }), log({ id: 'c', status: 'skipped' })],
      'day',
      now,
    );
    expect(stats.logCount).toBe(3);
    expect(stats.completed).toBe(1);
    expect(stats.completionRate).toBe(33);
  });
});

