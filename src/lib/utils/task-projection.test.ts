import { describe, expect, it } from 'vitest';
import type { Node, Tag } from '../types/node';
import {
  groupProjection,
  inNextSevenTotalDays,
  parseFilters,
  projectTasks,
  serializeFilters,
} from './task-projection';

function node(partial: Partial<Node> & Pick<Node, 'id' | 'title' | 'nodeType'>): Node {
  return {
    parentId: null,
    position: 0,
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

describe('task projections', () => {
  it('excludes workspaces and groups from task-only projections', () => {
    const ws = node({ id: 'w', title: 'Work', nodeType: 'Workspace' });
    const group = node({ id: 'g', title: 'Group', nodeType: 'Group', parentId: 'w' });
    const task = node({ id: 't', title: 'Task', nodeType: 'Task', parentId: 'g' });

    const projected = projectTasks([ws, group, task], { mode: 'active' });
    expect(projected.map((item) => item.node.id)).toEqual(['t']);
    expect(projected[0].label).toBe('Task');
    expect(projected[0].path.map((item) => item.title)).toEqual(['Work', 'Group', 'Task']);
  });

  it('labels nested tasks as subtasks without limiting depth', () => {
    const ws = node({ id: 'w', title: 'Work', nodeType: 'Workspace' });
    const root = node({ id: 't0', title: 'Root', nodeType: 'Task', parentId: 'w' });
    const sub = node({ id: 't1', title: 'Sub', nodeType: 'Task', parentId: 't0' });
    const deep = node({ id: 't2', title: 'Deep', nodeType: 'Task', parentId: 't1' });

    const projected = projectTasks([ws, root, sub, deep], { mode: 'active' });
    expect(projected.map((item) => item.label)).toEqual(['Task', 'Subtask', 'Subtask']);
  });

  it('uses seven total local calendar days for upcoming', () => {
    const now = new Date('2026-07-28T12:00:00+08:00');
    expect(inNextSevenTotalDays('2026-07-28T00:00:00+08:00', now)).toBe(true);
    expect(inNextSevenTotalDays('2026-08-03T23:59:59+08:00', now)).toBe(true);
    expect(inNextSevenTotalDays('2026-08-04T00:00:00+08:00', now)).toBe(false);
  });

  it('handles end of month and end of year boundaries', () => {
    expect(
      inNextSevenTotalDays('2026-03-01T00:00:00+08:00', new Date('2026-02-28T12:00:00+08:00')),
    ).toBe(true);
    expect(
      inNextSevenTotalDays('2027-01-01T00:00:00+08:00', new Date('2026-12-31T12:00:00+08:00')),
    ).toBe(true);
  });

  it('filters by normalized tags, priority, workspace, and text', () => {
    const ws = node({ id: 'w', title: 'Polyglot', nodeType: 'Workspace' });
    const task = node({
      id: 't',
      title: 'Study present perfect',
      nodeType: 'Task',
      parentId: 'w',
      properties: { priority: 1 },
    });
    const tag: Tag = { id: 'tag-study', name: 'Study', color: null, createdAt: '' };
    const tagAssignments = new Map([[task.id, [tag]]]);

    const projected = projectTasks([ws, task], {
      mode: 'active',
      tagAssignments,
      filters: {
        priority: 1,
        tagIds: ['tag-study'],
        workspaceId: 'w',
        text: 'perfect',
      },
    });
    expect(projected).toHaveLength(1);
  });

  it('groups completed tasks by completion period', () => {
    const now = new Date('2026-07-28T12:00:00+08:00');
    const ws = node({ id: 'w', title: 'Work', nodeType: 'Workspace' });
    const today = node({
      id: 'today',
      title: 'Today done',
      nodeType: 'Task',
      parentId: 'w',
      isCompleted: true,
      completedAt: '2026-07-28T02:00:00Z',
    });
    const old = node({
      id: 'old',
      title: 'Old done',
      nodeType: 'Task',
      parentId: 'w',
      isCompleted: true,
      completedAt: '2026-07-01T02:00:00Z',
    });

    const projected = projectTasks([ws, today, old], { mode: 'completed', now });
    const groups = groupProjection(projected, 'completionPeriod');
    expect(groups.map((group) => group.title)).toContain('Today');
    expect(groups.map((group) => group.title)).toContain('Earlier');
  });

  it('serializes simple filter definitions instead of storing task ids', () => {
    const filters = { completion: 'active' as const, priority: 1, text: 'study' };
    expect(parseFilters(serializeFilters(filters))).toEqual(filters);
  });
});
