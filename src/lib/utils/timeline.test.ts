import { describe, expect, it } from 'vitest';
import type { Node } from '../types/node';
import {
  moveTimelineRange,
  resizeTimelineEnd,
  resizeTimelineStart,
  taskTimelineRange,
  timelineVisibleRange,
  validateTimelineRange,
} from './timeline';

function task(properties: Node['properties']): Node {
  return {
    id: 'task',
    parentId: 'workspace',
    position: 0,
    nodeType: 'Task',
    title: 'Task',
    body: '',
    properties,
    isCollapsed: false,
    isCompleted: false,
    completedAt: null,
    deletedAt: null,
    version: 1,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  };
}

describe('timeline utilities', () => {
  it('builds zoom ranges with stable boundaries', () => {
    expect(timelineVisibleRange(new Date(2026, 6, 28), 'day').days).toHaveLength(7);
    expect(timelineVisibleRange(new Date(2026, 6, 28), 'week').days).toHaveLength(28);
    expect(timelineVisibleRange(new Date(2026, 6, 28), 'month').days).toHaveLength(90);
  });

  it('supports due-only tasks and start/due ranges', () => {
    expect(taskTimelineRange(task({ dueDate: '2026-07-28T00:00:00Z' }))).not.toBeNull();
    expect(taskTimelineRange(task({ startDate: '2026-07-27T00:00:00Z', dueDate: '2026-07-30T00:00:00Z' }))).not.toBeNull();
  });

  it('moves and resizes ranges without changing hierarchy', () => {
    const node = task({ startDate: '2026-07-27T00:00:00Z', dueDate: '2026-07-30T00:00:00Z' });
    expect(new Date(moveTimelineRange(node, 2)?.dueDate ?? '').getDate()).toBe(1);
    expect(new Date(resizeTimelineStart(node, 1)?.startDate ?? '').getDate()).toBe(28);
    expect(new Date(resizeTimelineEnd(node, 1)?.dueDate ?? '').getDate()).toBe(31);
    expect(node.parentId).toBe('workspace');
  });

  it('rejects invalid and negative ranges', () => {
    expect(validateTimelineRange(new Date('2026-07-29'), new Date('2026-07-28'))).toBe(false);
    const node = task({ startDate: '2026-07-27T00:00:00Z', dueDate: '2026-07-30T00:00:00Z' });
    expect(resizeTimelineStart(node, 5)).toBeNull();
    expect(taskTimelineRange(task({ startDate: '2026-07-31T00:00:00Z', dueDate: '2026-07-30T00:00:00Z' }))).toBeNull();
  });
});
