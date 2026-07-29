import { describe, expect, it } from 'vitest';
import type { Node, Tag } from '../types/node';
import type { TaskProjectionItem } from './task-projection';
import { groupKanbanItems, kanbanColumns, mergeCustomStatus, taskStatus } from './kanban';

function node(partial: Partial<Node>): Node {
  return {
    id: partial.id ?? 'task',
    parentId: partial.parentId ?? 'workspace',
    position: 0,
    nodeType: partial.nodeType ?? 'Task',
    title: partial.title ?? 'Task',
    body: '',
    properties: partial.properties ?? {},
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

function item(task: Node, tags: Tag[] = [], path: Node[] = [task]): TaskProjectionItem {
  return {
    node: task,
    label: 'Task',
    path,
    workspace: path.find((entry) => entry.nodeType === 'Workspace') ?? null,
    tags,
    dueDay: null,
    completionPeriod: null,
  };
}

describe('kanban utilities', () => {
  it('groups tasks by status and preserves configured columns', () => {
    const todo = node({ id: 'a', properties: { custom: { status: 'Next' } } });
    const doing = node({ id: 'b', properties: { custom: { status: 'Doing' } } });
    const columns = kanbanColumns('status', [item(todo), item(doing)], {
      statusColumns: ['Inbox', 'Next'],
      tags: [],
      nodes: [],
    });
    const groups = groupKanbanItems(columns, 'status', [item(todo), item(doing)]);
    expect(groups.map((group) => group.key)).toEqual(['Inbox', 'Next', 'Doing']);
    expect(groups.find((group) => group.key === 'Doing')?.items[0].node.id).toBe('b');
  });

  it('groups by priority and tag', () => {
    const study: Tag = { id: 'tag-study', name: 'Study', color: null, createdAt: '' };
    const high = item(node({ id: 'high', properties: { priority: 1 } }), [study]);
    const none = item(node({ id: 'none', properties: {} }), []);
    expect(groupKanbanItems(kanbanColumns('priority', [high, none], { statusColumns: [], tags: [], nodes: [] }), 'priority', [high, none]).find((group) => group.key === '1')?.items).toHaveLength(1);
    expect(groupKanbanItems(kanbanColumns('tag', [high, none], { statusColumns: [], tags: [study], nodes: [] }), 'tag', [high, none]).find((group) => group.key === 'tag-study')?.items).toHaveLength(1);
  });

  it('merges status without losing other custom values', () => {
    const task = node({ properties: { custom: { frog: true, status: 'Inbox' } } });
    expect(taskStatus(task)).toBe('Inbox');
    expect(mergeCustomStatus(task, 'Done')).toEqual({ frog: true, status: 'Done' });
  });
});
