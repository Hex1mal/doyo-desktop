import type { Node, Tag } from '../types/node';

export type ProjectionMode =
  'active' | 'completed' | 'deleted' | 'inbox' | 'today' | 'upcoming' | 'overdue' | 'tag';

export type SortMode =
  'manual' | 'title' | 'created' | 'updated' | 'due' | 'priority' | 'completed';

export type GroupMode =
  'none' | 'workspace' | 'group' | 'due' | 'priority' | 'tag' | 'completionPeriod';

export type DensityMode = 'compact' | 'comfortable';

export interface ProjectionFilters {
  completion?: 'active' | 'completed' | 'any';
  dueState?: 'none' | 'due' | 'overdue' | 'today' | 'upcoming';
  dateFrom?: string | null;
  dateTo?: string | null;
  priority?: number | null;
  tagIds?: string[];
  workspaceId?: string | null;
  ancestorId?: string | null;
  pinned?: boolean | null;
  wontDo?: boolean | null;
  text?: string;
}

export interface ProjectionOptions {
  mode?: ProjectionMode;
  filters?: ProjectionFilters;
  sort?: SortMode;
  group?: GroupMode;
  now?: Date;
  tagAssignments?: Map<string, Tag[]>;
  tagId?: string | null;
  upcomingDays?: number;
}

export interface TaskProjectionItem {
  node: Node;
  label: 'Task' | 'Subtask';
  path: Node[];
  workspace: Node | null;
  tags: Tag[];
  dueDay: string | null;
  completionPeriod: string | null;
}

export interface TaskProjectionGroup {
  key: string;
  title: string;
  items: TaskProjectionItem[];
}

function nodeKey(parentId: string | null | undefined) {
  return parentId ?? '__root__';
}

export function buildNodeIndex(nodes: Iterable<Node>) {
  const byId = new Map<string, Node>();
  const children = new Map<string, Node[]>();
  for (const node of nodes) {
    byId.set(node.id, node);
    const key = nodeKey(node.parentId);
    const list = children.get(key) ?? [];
    list.push(node);
    children.set(key, list);
  }
  for (const list of children.values()) {
    list.sort((a, b) => a.position - b.position || a.createdAt.localeCompare(b.createdAt));
  }
  return { byId, children };
}

export function getAncestors(node: Node, byId: Map<string, Node>) {
  const ancestors: Node[] = [];
  let current: Node | undefined = node;
  const seen = new Set<string>();
  while (current.parentId) {
    if (seen.has(current.parentId)) break;
    seen.add(current.parentId);
    const parent = byId.get(current.parentId);
    if (!parent) break;
    ancestors.unshift(parent);
    current = parent;
  }
  return ancestors;
}

export function isTaskNode(node: Node) {
  return node.nodeType === 'Task';
}

export function contextualTaskLabel(node: Node, byId: Map<string, Node>): 'Task' | 'Subtask' {
  const parent = node.parentId ? byId.get(node.parentId) : null;
  return parent?.nodeType === 'Task' ? 'Subtask' : 'Task';
}

export function customTags(node: Node): string[] {
  const custom = node.properties.custom;
  if (!custom || typeof custom !== 'object' || Array.isArray(custom)) return [];
  const tags = custom.tags;
  return Array.isArray(tags)
    ? tags
        .filter((tag): tag is string => typeof tag === 'string')
        .map((tag) => tag.trim())
        .filter(Boolean)
    : [];
}

export function isWontDo(node: Node) {
  const custom = node.properties.custom;
  return Boolean(custom && typeof custom === 'object' && !Array.isArray(custom) && custom.wontDo);
}

export function localDayKey(dateValue: string | null | undefined) {
  if (!dateValue) return null;
  const date = new Date(dateValue);
  if (Number.isNaN(date.getTime())) return null;
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function startOfLocalDay(date: Date) {
  const copy = new Date(date);
  copy.setHours(0, 0, 0, 0);
  return copy;
}

function addDays(date: Date, days: number) {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + days);
  return copy;
}

export function inNextSevenTotalDays(dateValue: string | null | undefined, now = new Date()) {
  if (!dateValue) return false;
  const date = new Date(dateValue);
  if (Number.isNaN(date.getTime())) return false;
  const start = startOfLocalDay(now);
  const endExclusive = addDays(start, 7);
  return date >= start && date < endExclusive;
}

function sameLocalDay(left: Date, right: Date) {
  return (
    left.getFullYear() === right.getFullYear() &&
    left.getMonth() === right.getMonth() &&
    left.getDate() === right.getDate()
  );
}

function completionPeriod(completedAt: string | null, now = new Date()) {
  if (!completedAt) return null;
  const completed = new Date(completedAt);
  if (Number.isNaN(completed.getTime())) return 'Earlier';
  const today = startOfLocalDay(now);
  const yesterday = addDays(today, -1);
  const weekStart = addDays(today, -6);
  const completedDay = startOfLocalDay(completed);
  if (sameLocalDay(completedDay, today)) return 'Today';
  if (sameLocalDay(completedDay, yesterday)) return 'Yesterday';
  if (completedDay >= weekStart && completedDay < yesterday) return 'Earlier this week';
  return 'Earlier';
}

function textMatches(item: TaskProjectionItem, query: string) {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return (
    item.node.title.toLowerCase().includes(q) ||
    item.node.body.toLowerCase().includes(q) ||
    item.path.some((node) => node.title.toLowerCase().includes(q)) ||
    item.tags.some((tag) => tag.name.toLowerCase().includes(q))
  );
}

function hasAncestor(item: TaskProjectionItem, ancestorId: string) {
  return item.node.id === ancestorId || item.path.some((node) => node.id === ancestorId);
}

function workspaceOf(path: Node[]) {
  return path.find((node) => node.nodeType === 'Workspace') ?? null;
}

function tagsForNode(node: Node, assignments?: Map<string, Tag[]>) {
  const normalized = assignments?.get(node.id) ?? [];
  const byName = new Map(normalized.map((tag) => [tag.name.trim().toLowerCase(), tag]));
  for (const tag of customTags(node)) {
    const key = tag.toLowerCase();
    if (!byName.has(key)) {
      byName.set(key, {
        id: `legacy:${key}`,
        name: tag,
        color: null,
        createdAt: '',
      });
    }
  }
  return [...byName.values()].sort((a, b) => a.name.localeCompare(b.name));
}

export function projectTasks(nodes: Node[], options: ProjectionOptions = {}): TaskProjectionItem[] {
  const mode = options.mode ?? 'active';
  const now = options.now ?? new Date();
  const { byId } = buildNodeIndex(nodes);
  const items: TaskProjectionItem[] = [];

  for (const node of nodes) {
    if (!isTaskNode(node)) continue;
    const deleted = Boolean(node.deletedAt);
    if (mode === 'deleted' ? !deleted : deleted) continue;
    if (mode === 'active' && node.isCompleted) continue;
    if (mode === 'completed' && !node.isCompleted) continue;
    if (
      (mode === 'today' || mode === 'upcoming' || mode === 'overdue' || mode === 'inbox') &&
      node.isCompleted
    ) {
      continue;
    }

    const path = [...getAncestors(node, byId), node];
    const tags = tagsForNode(node, options.tagAssignments);
    const item: TaskProjectionItem = {
      node,
      label: contextualTaskLabel(node, byId),
      path,
      workspace: workspaceOf(path),
      tags,
      dueDay: localDayKey(node.properties.dueDate),
      completionPeriod: completionPeriod(node.completedAt, now),
    };

    if (mode === 'today') {
      const due = node.properties.dueDate ? new Date(node.properties.dueDate) : null;
      if (!due || due >= addDays(startOfLocalDay(now), 1)) continue;
    }
    if (mode === 'upcoming' && !inNextSevenTotalDays(node.properties.dueDate, now)) continue;
    if (mode === 'overdue') {
      const due = node.properties.dueDate ? new Date(node.properties.dueDate) : null;
      if (!due || due >= startOfLocalDay(now)) continue;
    }
    if (mode === 'inbox') {
      const parent = node.parentId ? byId.get(node.parentId) : null;
      const parentIsInbox = parent?.title.trim().toLowerCase() === 'inbox';
      if (node.parentId && !parentIsInbox) continue;
    }
    if (mode === 'tag' && options.tagId && !tags.some((tag) => tag.id === options.tagId)) continue;

    items.push(item);
  }

  return sortProjection(applyFilters(items, options.filters, now), options.sort ?? 'manual');
}

export function applyFilters(
  items: TaskProjectionItem[],
  filters: ProjectionFilters | undefined,
  now = new Date(),
) {
  if (!filters) return items;
  return items.filter((item) => {
    if (filters.completion === 'active' && item.node.isCompleted) return false;
    if (filters.completion === 'completed' && !item.node.isCompleted) return false;
    if (filters.priority && item.node.properties.priority !== filters.priority) return false;
    if (filters.workspaceId && item.workspace?.id !== filters.workspaceId) return false;
    if (filters.ancestorId && !hasAncestor(item, filters.ancestorId)) return false;
    if (
      filters.pinned !== null &&
      filters.pinned !== undefined &&
      Boolean(item.node.properties.pinned) !== filters.pinned
    ) {
      return false;
    }
    if (
      filters.wontDo !== null &&
      filters.wontDo !== undefined &&
      isWontDo(item.node) !== filters.wontDo
    ) {
      return false;
    }
    if (
      filters.tagIds?.length &&
      !filters.tagIds.every((id) => item.tags.some((tag) => tag.id === id))
    ) {
      return false;
    }
    if (filters.text && !textMatches(item, filters.text)) return false;
    if (
      filters.dateFrom &&
      (!item.node.properties.dueDate ||
        new Date(item.node.properties.dueDate) < new Date(filters.dateFrom))
    ) {
      return false;
    }
    if (
      filters.dateTo &&
      (!item.node.properties.dueDate ||
        new Date(item.node.properties.dueDate) > new Date(filters.dateTo))
    ) {
      return false;
    }
    if (filters.dueState === 'overdue') {
      const due = item.node.properties.dueDate ? new Date(item.node.properties.dueDate) : null;
      if (!due || due >= startOfLocalDay(now)) return false;
    }
    if (filters.dueState === 'today') {
      const due = item.node.properties.dueDate ? new Date(item.node.properties.dueDate) : null;
      if (!due || due < startOfLocalDay(now) || due >= addDays(startOfLocalDay(now), 1))
        return false;
    }
    if (
      filters.dueState === 'upcoming' &&
      !inNextSevenTotalDays(item.node.properties.dueDate, now)
    ) {
      return false;
    }
    if (filters.dueState === 'due' && !item.node.properties.dueDate) return false;
    if (filters.dueState === 'none' && item.node.properties.dueDate) return false;
    return true;
  });
}

export function sortProjection(items: TaskProjectionItem[], sort: SortMode) {
  const indexed = items.map((item, index) => ({ item, index }));
  const compareNullable = (a: string | null | undefined, b: string | null | undefined) => {
    if (!a && !b) return 0;
    if (!a) return 1;
    if (!b) return -1;
    return a.localeCompare(b);
  };
  indexed.sort((a, b) => {
    let result = 0;
    if (sort === 'title') result = a.item.node.title.localeCompare(b.item.node.title);
    if (sort === 'created') result = a.item.node.createdAt.localeCompare(b.item.node.createdAt);
    if (sort === 'updated') result = b.item.node.updatedAt.localeCompare(a.item.node.updatedAt);
    if (sort === 'due')
      result = compareNullable(a.item.node.properties.dueDate, b.item.node.properties.dueDate);
    if (sort === 'priority')
      result = (a.item.node.properties.priority ?? 4) - (b.item.node.properties.priority ?? 4);
    if (sort === 'completed')
      result = compareNullable(b.item.node.completedAt, a.item.node.completedAt);
    if (sort === 'manual') {
      result = 0;
    }
    return result || a.index - b.index;
  });
  return indexed.map(({ item }) => item);
}

export function groupProjection(
  items: TaskProjectionItem[],
  group: GroupMode,
): TaskProjectionGroup[] {
  if (group === 'none') return [{ key: 'all', title: 'Tasks', items }];
  const groups = new Map<string, TaskProjectionGroup>();
  const add = (key: string, title: string, item: TaskProjectionItem) => {
    const group = groups.get(key) ?? { key, title, items: [] };
    group.items.push(item);
    groups.set(key, group);
  };
  for (const item of items) {
    if (group === 'workspace')
      add(item.workspace?.id ?? 'none', item.workspace?.title ?? 'No workspace', item);
    if (group === 'group') {
      const container = [...item.path].reverse().find((node) => node.nodeType === 'Group');
      add(container?.id ?? 'none', container?.title ?? 'No group', item);
    }
    if (group === 'due') add(item.dueDay ?? 'none', item.dueDay ?? 'No due date', item);
    if (group === 'priority')
      add(
        String(item.node.properties.priority ?? 4),
        `P${item.node.properties.priority ?? 4}`,
        item,
      );
    if (group === 'tag') {
      if (item.tags.length === 0) add('none', 'No tags', item);
      for (const tag of item.tags) add(tag.id, tag.name, item);
    }
    if (group === 'completionPeriod') {
      const period = item.completionPeriod ?? 'Earlier';
      add(period, period, item);
    }
  }
  return [...groups.values()];
}

export function serializeFilters(filters: ProjectionFilters) {
  return JSON.stringify(filters);
}

export function parseFilters(raw: string): ProjectionFilters {
  const parsed = JSON.parse(raw);
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Filter definition must be an object');
  }
  return parsed as ProjectionFilters;
}
