import type { Node, Tag } from '../types/node';
import type { TaskProjectionItem } from './task-projection';

export type KanbanGroupMode = 'status' | 'priority' | 'tag' | 'workspace' | 'group';

export interface KanbanColumn {
  key: string;
  title: string;
  editable?: boolean;
}

export interface KanbanColumnGroup extends KanbanColumn {
  items: TaskProjectionItem[];
}

export function taskStatus(node: Node) {
  const custom = node.properties.custom;
  if (!custom || typeof custom !== 'object' || Array.isArray(custom)) return 'Inbox';
  const status = custom.status;
  return typeof status === 'string' && status.trim() ? status.trim() : 'Inbox';
}

export function kanbanColumns(
  mode: KanbanGroupMode,
  items: TaskProjectionItem[],
  options: {
    statusColumns: string[];
    tags: Tag[];
    nodes: Node[];
  },
): KanbanColumn[] {
  if (mode === 'status') {
    const configured = options.statusColumns.map((status) => status.trim()).filter(Boolean);
    const discovered = items.map((item) => taskStatus(item.node));
    return [...new Set([...configured, ...discovered])].map((status) => ({
      key: status,
      title: status,
      editable: true,
    }));
  }

  if (mode === 'priority') {
    return [1, 2, 3, 4].map((priority) => ({
      key: String(priority),
      title: priority === 4 ? 'P4 / None' : `P${priority}`,
    }));
  }

  if (mode === 'tag') {
    return [
      { key: 'none', title: 'No tags' },
      ...options.tags.map((tag) => ({ key: tag.id, title: tag.name })),
    ];
  }

  const containers = options.nodes
    .filter((node) => {
      if (node.deletedAt) return false;
      if (mode === 'workspace') return node.nodeType === 'Workspace';
      return node.nodeType === 'Group';
    })
    .sort((a, b) => a.title.localeCompare(b.title));
  return containers.map((node) => ({ key: node.id, title: node.title || 'Untitled' }));
}

export function groupKanbanItems(
  columns: KanbanColumn[],
  mode: KanbanGroupMode,
  items: TaskProjectionItem[],
) {
  const groups = new Map(
    columns.map((column) => [column.key, { ...column, items: [] as TaskProjectionItem[] }]),
  );
  const ensure = (key: string, title: string) => {
    const group = groups.get(key) ?? { key, title, items: [] };
    groups.set(key, group);
    return group;
  };

  for (const item of items) {
    if (mode === 'status') {
      ensure(taskStatus(item.node), taskStatus(item.node)).items.push(item);
    } else if (mode === 'priority') {
      ensure(
        String(item.node.properties.priority ?? 4),
        `P${item.node.properties.priority ?? 4}`,
      ).items.push(item);
    } else if (mode === 'tag') {
      if (item.tags.length === 0) ensure('none', 'No tags').items.push(item);
      for (const tag of item.tags) ensure(tag.id, tag.name).items.push(item);
    } else if (mode === 'workspace') {
      ensure(item.workspace?.id ?? 'none', item.workspace?.title ?? 'No workspace').items.push(
        item,
      );
    } else if (mode === 'group') {
      const group = [...item.path].reverse().find((node) => node.nodeType === 'Group');
      ensure(group?.id ?? 'none', group?.title ?? 'No group').items.push(item);
    }
  }

  return [...groups.values()];
}
