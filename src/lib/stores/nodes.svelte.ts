import {
  treeGetFull,
  nodeCreate,
  nodeUpdate,
  nodeReplaceProperties,
  nodeDelete,
  nodeMove,
  nodeMoveOrdered,
  nodeReorder,
  nodeReorderRoot,
  nodeSetPriority,
  nodeSetDueDate,
  nodeSetCompletion,
  nodeIncompleteDescendantCount,
  nodeDuplicate,
  tagAssign,
  tagCreate,
  tagDelete,
  tagGetForNode,
  tagList,
  tagRemove,
  tagRename,
  tagSyncLegacy,
  trashEmpty,
  trashGetNodes,
  trashRestore,
  searchQuery as apiSearchQuery,
  undo as apiUndo,
  redo as apiRedo,
} from '$lib/api/client';
import type { Node, SearchResult, Tag } from '$lib/types/node';
import { toast } from '$lib/stores/toast.svelte';
import { uiStore } from '$lib/stores/ui.svelte';
import { completionCascadeMessage } from '$lib/utils/completion-policy';
import {
  projectTasks,
  type ProjectionFilters,
  type ProjectionMode,
  type SortMode,
} from '$lib/utils/task-projection';

export type ViewMode =
  | 'tree'
  | 'today'
  | 'inbox'
  | 'upcoming'
  | 'completed'
  | 'trash'
  | 'tag'
  | 'filter'
  | 'search'
  | 'favorites';

const state = $state({
  nodes: new Map<string, Node>(),
  trashNodes: new Map<string, Node>(),
  tags: [] as Tag[],
  tagAssignments: new Map<string, Tag[]>(),
  selectedTagId: null as string | null,
  filterDraft: {
    completion: 'active',
  } as ProjectionFilters,
  childrenByParent: new Map<string, string[]>(),
  expandedIds: new Set<string>(),
  selectedId: null as string | null,
  focusRootId: null as string | null,
  editingId: null as string | null,
  isLoading: false,
  isSearchLoading: false,
  viewMode: 'tree' as ViewMode,
  searchQuery: '',
  searchResults: [] as SearchResult[],
  statusMessage: 'Ready',
  _rev: 0,
  _searchSeq: 0,
});

function sortByPosition(a: Node, b: Node) {
  return a.position - b.position;
}

function parentKey(parentId: string | null | undefined) {
  return parentId ?? '__root__';
}

function buildChildrenIndex(nodes: Map<string, Node>) {
  const index = new Map<string, string[]>();
  for (const node of nodes.values()) {
    if (node.deletedAt) continue;
    const key = parentKey(node.parentId);
    const ids = index.get(key) ?? [];
    ids.push(node.id);
    index.set(key, ids);
  }
  return index;
}

export type NodeKind = 'workspace' | 'group' | 'subgroup' | 'task' | 'subtask' | 'note' | 'other';

function semanticKind(node: Node, nodes = state.nodes): NodeKind {
  const parent = node.parentId ? nodes.get(node.parentId) : null;
  if (node.nodeType === 'Workspace') return 'workspace';
  if (node.nodeType === 'Group' && parent?.nodeType === 'Group') return 'subgroup';
  if (node.nodeType === 'Group') return 'group';
  if (node.nodeType === 'Task' && parent?.nodeType === 'Task') return 'subtask';
  if (node.nodeType === 'Task') return 'task';
  if (node.nodeType === 'Note') return 'note';
  return 'other';
}

function semanticLabel(kind: NodeKind) {
  switch (kind) {
    case 'workspace':
      return 'Workspace';
    case 'group':
      return 'Group';
    case 'subgroup':
      return 'Subgroup';
    case 'task':
      return 'Task';
    case 'subtask':
      return 'Subtask';
    case 'note':
      return 'Note';
    default:
      return 'Node';
  }
}

function canContainGroup(parent: Node | null | undefined) {
  return parent?.nodeType === 'Workspace' || parent?.nodeType === 'Group';
}

function canContainTask(parent: Node | null | undefined) {
  return parent?.nodeType === 'Workspace' || parent?.nodeType === 'Group';
}

function canContainSubtask(parent: Node | null | undefined) {
  return parent?.nodeType === 'Task';
}

function descendantsOf(id: string) {
  const result: Node[] = [];
  const walk = (parentId: string) => {
    const childIds = state.childrenByParent.get(parentKey(parentId)) ?? [];
    for (const childId of childIds) {
      const child = state.nodes.get(childId);
      if (!child || child.deletedAt) continue;
      result.push(child);
      walk(child.id);
    }
  };
  walk(id);
  return result;
}

function rebuildFromList(list: Node[]) {
  const map = new Map<string, Node>();
  for (const n of list) {
    if (n.deletedAt) continue;
    map.set(n.id, n);
  }
  state.nodes = map;
  state.childrenByParent = buildChildrenIndex(map);
  // Keep previously expanded ids that still exist. On first load the set is
  // empty, so every workspace starts collapsed rather than auto-expanding a
  // fixed depth of the hierarchy.
  const nextExpanded = new Set<string>();
  for (const id of state.expandedIds) {
    if (map.has(id)) nextExpanded.add(id);
  }
  if (state.focusRootId && !map.has(state.focusRootId)) {
    state.focusRootId = null;
  }
  state.expandedIds = nextExpanded;
  state._rev++;
}

function tagsFromCustom(custom: unknown): string[] {
  if (!custom || typeof custom !== 'object' || Array.isArray(custom)) return [];
  const tags = (custom as Record<string, unknown>).tags;
  if (!Array.isArray(tags)) return [];
  return tags.filter((tag): tag is string => typeof tag === 'string');
}

function tagKey(name: string) {
  return name.trim().toLowerCase();
}

function mergedCustom(node: Node, patch: Record<string, unknown>) {
  const existing =
    node.properties.custom && typeof node.properties.custom === 'object'
      ? node.properties.custom
      : {};
  return { ...existing, ...patch };
}

function cleanedProperties(properties: Node['properties']): Node['properties'] {
  return Object.fromEntries(
    Object.entries(properties).filter(([, value]) => value !== undefined && value !== ''),
  ) as Node['properties'];
}

function selectionOrFocusNode() {
  if (state.selectedId) return state.nodes.get(state.selectedId) ?? null;
  if (state.focusRootId) return state.nodes.get(state.focusRootId) ?? null;
  return null;
}

export const nodeStore = {
  get nodes() {
    state._rev;
    return state.nodes;
  },
  get expandedIds() {
    return state.expandedIds;
  },
  get selectedId() {
    return state.selectedId;
  },
  get focusRootId() {
    return state.focusRootId;
  },
  get editingId() {
    return state.editingId;
  },
  get isLoading() {
    return state.isLoading;
  },
  get isSearchLoading() {
    return state.isSearchLoading;
  },
  get viewMode() {
    return state.viewMode;
  },
  get searchQuery() {
    return state.searchQuery;
  },
  get searchResults() {
    return state.searchResults;
  },
  get trashNodes() {
    state._rev;
    return [...state.trashNodes.values()].sort((a, b) =>
      (b.deletedAt ?? '').localeCompare(a.deletedAt ?? ''),
    );
  },
  get tags() {
    state._rev;
    return state.tags;
  },
  get tagAssignments() {
    state._rev;
    return state.tagAssignments;
  },
  get selectedTagId() {
    return state.selectedTagId;
  },
  get filterDraft() {
    return state.filterDraft;
  },
  get statusMessage() {
    return state.statusMessage;
  },
  get count() {
    state._rev;
    let c = 0;
    for (const n of state.nodes.values()) if (!n.deletedAt) c++;
    return c;
  },

  setStatus(msg: string) {
    state.statusMessage = msg;
  },

  setViewMode(mode: ViewMode) {
    state.viewMode = mode;
  },

  setSelectedTag(id: string | null) {
    state.selectedTagId = id;
    state.viewMode = id ? 'tag' : state.viewMode;
  },

  setFilterDraft(filters: ProjectionFilters) {
    state.filterDraft = filters;
    state.viewMode = 'filter';
  },

  setFocusRoot(id: string | null) {
    state.focusRootId = id;
    if (id) {
      this.expandAncestors(id);
      this.expand(id);
    }
  },

  setSearchQuery(q: string) {
    state.searchQuery = q;
  },

  async runSearch(q = state.searchQuery) {
    state.searchQuery = q;
    const query = q.trim();
    const seq = ++state._searchSeq;
    if (!query) {
      state.searchResults = [];
      state.isSearchLoading = false;
      return;
    }

    state.isSearchLoading = true;
    try {
      const results = await apiSearchQuery(query, {});
      if (seq !== state._searchSeq) return;
      state.searchResults = results;
      for (const result of results) {
        this.upsert(result.node);
      }
    } catch (e) {
      if (seq === state._searchSeq) {
        state.searchResults = [];
        toast.error('Search failed');
      }
      console.error(e);
    } finally {
      if (seq === state._searchSeq) {
        state.isSearchLoading = false;
      }
    }
  },

  async load(): Promise<boolean> {
    state.isLoading = true;
    try {
      const list = await treeGetFull(null);
      rebuildFromList(list);
      await this.loadTags();
      if (state.viewMode === 'trash') await this.loadTrash();
      state.statusMessage = `${state.nodes.size} nodes`;
      return true;
    } catch (e) {
      toast.error('Failed to load workspace');
      console.error(e);
      return false;
    } finally {
      state.isLoading = false;
    }
  },

  async loadTags(): Promise<boolean> {
    try {
      await tagSyncLegacy();
      const tags = await tagList();
      const assignments = new Map<string, Tag[]>();
      const taskIds = [...state.nodes.values()]
        .filter((node) => node.nodeType === 'Task')
        .map((node) => node.id);
      await Promise.all(
        taskIds.map(async (id) => {
          assignments.set(id, await tagGetForNode(id));
        }),
      );
      state.tags = tags;
      state.tagAssignments = assignments;
      state._rev++;
      return true;
    } catch (e) {
      console.error(e);
      toast.error('Failed to load tags');
      return false;
    }
  },

  async loadTrash(): Promise<boolean> {
    try {
      const deleted = await trashGetNodes();
      state.trashNodes = new Map(deleted.map((node) => [node.id, node]));
      state._rev++;
      return true;
    } catch (e) {
      console.error(e);
      toast.error('Failed to load Trash');
      return false;
    }
  },

  get(id: string): Node | undefined {
    state._rev;
    return state.nodes.get(id);
  },

  getSelected(): Node | null {
    return state.selectedId ? (state.nodes.get(state.selectedId) ?? null) : null;
  },

  getFocusedRoot(): Node | null {
    return state.focusRootId ? (state.nodes.get(state.focusRootId) ?? null) : null;
  },

  getKind(node: Node): NodeKind {
    state._rev;
    return semanticKind(node);
  },

  getKindLabel(node: Node): string {
    return semanticLabel(this.getKind(node));
  },

  getRoots(): Node[] {
    return [...state.nodes.values()]
      .filter((n) => !n.parentId && !n.deletedAt)
      .sort(sortByPosition);
  },

  getChildren(parentId: string | null): Node[] {
    state._rev;
    const ids = state.childrenByParent.get(parentKey(parentId)) ?? [];
    return ids
      .map((id) => state.nodes.get(id))
      .filter((n): n is Node => Boolean(n && !n.deletedAt))
      .sort(sortByPosition);
  },

  getDescendants(id: string): Node[] {
    state._rev;
    return descendantsOf(id);
  },

  getPath(id: string): string {
    const node = state.nodes.get(id);
    if (!node) return '';
    return [...this.getAncestors(id), node].map((n) => n.title || 'Untitled').join(' > ');
  },

  getFlatVisibleList(rootId = state.focusRootId): Array<{ node: Node; depth: number }> {
    const result: Array<{ node: Node; depth: number }> = [];
    const walk = (parentId: string | null, depth: number) => {
      for (const child of this.getChildren(parentId)) {
        result.push({ node: child, depth });
        if (state.expandedIds.has(child.id)) {
          walk(child.id, depth + 1);
        }
      }
    };
    if (rootId && state.nodes.has(rootId)) {
      walk(rootId, 0);
    } else {
      walk(null, 0);
    }
    return result;
  },

  getAncestors(id: string): Node[] {
    const result: Node[] = [];
    let current = state.nodes.get(id);
    while (current?.parentId) {
      const parent = state.nodes.get(current.parentId);
      if (!parent) break;
      result.unshift(parent);
      current = parent;
    }
    return result;
  },

  select(id: string | null) {
    state.selectedId = id;
    state.editingId = null;
  },

  startEditing(id: string) {
    state.selectedId = id;
    state.editingId = id;
  },

  selectInWorkspace(id: string) {
    this.setViewMode('tree');
    const node = state.nodes.get(id);
    const ancestors = node ? this.getAncestors(id) : [];
    const workspace =
      node?.nodeType === 'Workspace' ? node : ancestors.find((n) => n.nodeType === 'Workspace');
    this.setFocusRoot(id);
    if (workspace) this.expand(workspace.id);
    this.select(id);
  },

  openFavoritesView() {
    uiStore.setActiveModule('workspaces');
    state.viewMode = 'favorites';
    state.selectedId = null;
    state.focusRootId = null;
    state.selectedTagId = null;
  },

  revealNode(id: string) {
    const node = state.nodes.get(id);
    if (!node || node.deletedAt) return;
    const ancestors = this.getAncestors(id);
    const workspace =
      node.nodeType === 'Workspace' ? node : ancestors.find((n) => n.nodeType === 'Workspace');
    this.setViewMode('tree');
    this.setFocusRoot(workspace?.id ?? id);
    this.expandAncestors(id);
    this.select(id);
  },

  stopEditing() {
    state.editingId = null;
  },

  upsert(node: Node) {
    const next = new Map(state.nodes);
    const previous = next.get(node.id);
    next.set(node.id, node);
    state.nodes = next;
    const nextChildren = new Map(state.childrenByParent);

    if (previous && previous.parentId !== node.parentId) {
      const oldKey = parentKey(previous.parentId);
      nextChildren.set(
        oldKey,
        (nextChildren.get(oldKey) ?? []).filter((id) => id !== node.id),
      );
    }

    const newKey = parentKey(node.parentId);
    const ids = (nextChildren.get(newKey) ?? []).filter((id) => id !== node.id);
    if (!node.deletedAt) ids.push(node.id);
    nextChildren.set(newKey, ids);
    state.childrenByParent = nextChildren;
    state._rev++;
  },

  removeSubtree(id: string) {
    const toRemove = new Set<string>();
    const collect = (nid: string) => {
      toRemove.add(nid);
      for (const childId of state.childrenByParent.get(parentKey(nid)) ?? []) {
        if (!toRemove.has(childId)) {
          collect(childId);
        }
      }
    };
    collect(id);
    const next = new Map(state.nodes);
    const nextExpanded = new Set(state.expandedIds);
    for (const rid of toRemove) {
      next.delete(rid);
      nextExpanded.delete(rid);
    }
    state.nodes = next;
    state.childrenByParent = buildChildrenIndex(next);
    state.expandedIds = nextExpanded;
    state._rev++;
    if (state.selectedId && toRemove.has(state.selectedId)) {
      state.selectedId = null;
    }
  },

  async toggleExpand(id: string) {
    const next = new Set(state.expandedIds);
    const node = state.nodes.get(id);
    if (next.has(id)) {
      next.delete(id);
      if (node) {
        try {
          const updated = await nodeUpdate(id, { isCollapsed: true });
          this.upsert(updated);
        } catch {
          /* local only */
        }
      }
    } else {
      next.add(id);
      if (node) {
        try {
          const updated = await nodeUpdate(id, { isCollapsed: false });
          this.upsert(updated);
        } catch {
          /* local only */
        }
      }
    }
    state.expandedIds = next;
  },

  expand(id: string) {
    if (!state.expandedIds.has(id)) {
      const next = new Set(state.expandedIds);
      next.add(id);
      state.expandedIds = next;
    }
  },

  expandAncestors(id: string) {
    const next = new Set(state.expandedIds);
    let current = state.nodes.get(id);
    while (current?.parentId) {
      next.add(current.parentId);
      current = state.nodes.get(current.parentId);
    }
    state.expandedIds = next;
  },

  selectNext() {
    const list = this.getFlatVisibleList();
    if (list.length === 0) return;
    if (!state.selectedId) {
      this.select(list[0].node.id);
      return;
    }
    const idx = list.findIndex((x) => x.node.id === state.selectedId);
    if (idx < list.length - 1) this.select(list[idx + 1].node.id);
  },

  selectPrev() {
    const list = this.getFlatVisibleList();
    if (list.length === 0) return;
    if (!state.selectedId) {
      this.select(list[list.length - 1].node.id);
      return;
    }
    const idx = list.findIndex((x) => x.node.id === state.selectedId);
    if (idx > 0) this.select(list[idx - 1].node.id);
  },

  async createSibling(title = ''): Promise<Node | null> {
    const selected = this.getSelected();
    const parentId = selected?.parentId ?? state.focusRootId;
    const nodeType = selected?.nodeType ?? (state.focusRootId ? 'Task' : 'Workspace');
    try {
      const node = await nodeCreate(parentId, nodeType, title);
      this.upsert(node);
      if (parentId) this.expand(parentId);
      this.startEditing(node.id);
      state.statusMessage = `Created ${nodeType.toLowerCase()}`;
      return node;
    } catch (e) {
      toast.error('Failed to create');
      console.error(e);
      return null;
    }
  },

  async createChild(title = ''): Promise<Node | null> {
    const parent = selectionOrFocusNode();
    if (!parent) return this.createWorkspace(title || 'My Workspace');
    if (canContainGroup(parent)) return this.createSubgroupUnder(parent.id, title);
    if (canContainSubtask(parent)) return this.createSubtaskUnder(parent.id, title);
    toast.error('This node cannot contain children');
    return null;
  },

  async createGroupUnder(parentId: string, title = 'New Group'): Promise<Node | null> {
    const parent = state.nodes.get(parentId);
    if (!parent || parent.nodeType !== 'Workspace') {
      toast.error('Groups can only be created inside a workspace');
      return null;
    }
    try {
      const node = await nodeCreate(parentId, 'Group', title || 'New Group');
      this.upsert(node);
      this.expand(parentId);
      this.startEditing(node.id);
      state.statusMessage = 'Created group';
      return node;
    } catch (e) {
      toast.error('Failed to create group');
      console.error(e);
      return null;
    }
  },

  async createSubgroupUnder(parentId: string, title = 'New Subgroup'): Promise<Node | null> {
    const parent = state.nodes.get(parentId);
    if (!canContainGroup(parent)) {
      toast.error('Subgroups can only be created inside a group or subgroup');
      return null;
    }
    try {
      const defaultTitle = parent?.nodeType === 'Workspace' ? 'New Group' : 'New Subgroup';
      const node = await nodeCreate(parentId, 'Group', title || defaultTitle);
      this.upsert(node);
      this.expand(parentId);
      this.startEditing(node.id);
      state.statusMessage = parent?.nodeType === 'Workspace' ? 'Created group' : 'Created subgroup';
      return node;
    } catch (e) {
      toast.error('Failed to create subgroup');
      console.error(e);
      return null;
    }
  },

  async createTaskUnder(parentId: string, title = 'New Task'): Promise<Node | null> {
    const parent = state.nodes.get(parentId);
    if (!canContainTask(parent)) {
      toast.error('Tasks can only be created inside a workspace, group, or subgroup');
      return null;
    }
    try {
      const node = await nodeCreate(parentId, 'Task', title || 'New Task');
      this.upsert(node);
      this.expand(parentId);
      this.startEditing(node.id);
      state.statusMessage = 'Created task';
      return node;
    } catch (e) {
      toast.error('Failed to create task');
      console.error(e);
      return null;
    }
  },

  async createSubtaskUnder(parentId: string, title = 'New Subtask'): Promise<Node | null> {
    const parent = state.nodes.get(parentId);
    if (!canContainSubtask(parent)) {
      toast.error('Subtasks can only be created inside a task or subtask');
      return null;
    }
    try {
      const node = await nodeCreate(parentId, 'Task', title || 'New Subtask');
      this.upsert(node);
      this.expand(parentId);
      this.startEditing(node.id);
      state.statusMessage = 'Created subtask';
      return node;
    } catch (e) {
      toast.error('Failed to create subtask');
      console.error(e);
      return null;
    }
  },

  async createGroup(): Promise<Node | null> {
    const parent = selectionOrFocusNode();
    if (!parent) {
      toast.error('Select a workspace first');
      return null;
    }
    if (parent.nodeType === 'Workspace') return this.createGroupUnder(parent.id);
    if (parent.nodeType === 'Group') return this.createSubgroupUnder(parent.id);
    toast.error('A task cannot contain groups');
    return null;
  },

  async createTask(): Promise<Node | null> {
    const parent = selectionOrFocusNode();
    if (!parent) {
      toast.error('Select a workspace, group, or subgroup first');
      return null;
    }
    return this.createTaskUnder(parent.id);
  },

  async createSubtask(): Promise<Node | null> {
    const parent = selectionOrFocusNode();
    if (!parent) {
      toast.error('Select a task first');
      return null;
    }
    return this.createSubtaskUnder(parent.id);
  },

  async duplicate(id: string): Promise<Node | null> {
    try {
      const dup = await nodeDuplicate(id);
      this.upsert(dup);
      const parentId = dup.parentId;
      if (parentId) this.expand(parentId);
      this.select(dup.id);
      state.statusMessage = 'Duplicated';
      return dup;
    } catch (e) {
      toast.error('Failed to duplicate');
      console.error(e);
      return null;
    }
  },

  async createWorkspace(title = 'Personal'): Promise<Node | null> {
    try {
      const node = await nodeCreate(null, 'Workspace', title);
      this.upsert(node);
      this.expand(node.id);
      this.setFocusRoot(node.id);
      this.startEditing(node.id);
      return node;
    } catch (e) {
      toast.error('Failed to create workspace');
      return null;
    }
  },

  async rename(id: string, title: string) {
    try {
      const updated = await nodeUpdate(id, { title });
      this.upsert(updated);
      this.stopEditing();
    } catch (e) {
      toast.error('Failed to rename');
    }
  },

  async toggleComplete(id: string) {
    const node = state.nodes.get(id);
    if (!node || node.nodeType !== 'Task') {
      toast.error('Only tasks and subtasks can be completed');
      return;
    }
    const completed = !node.isCompleted;
    let cascade = false;

    if (completed) {
      if (uiStore.completionPolicy === 'cascade') {
        cascade = true;
      } else if (uiStore.completionPolicy === 'ask') {
        try {
          const count = await nodeIncompleteDescendantCount(id);
          if (count > 0) {
            cascade = window.confirm(completionCascadeMessage(count));
          }
        } catch (e) {
          toast.error('Failed to inspect subtasks');
          console.error(e);
          return;
        }
      }
    }

    try {
      const updated = await nodeSetCompletion(id, completed, cascade);
      await this.load();
      this.select(updated.id);
      state.statusMessage = updated.isCompleted ? 'Completed' : 'Reopened';
    } catch (e) {
      toast.error('Failed to toggle complete');
    }
  },

  async moveTo(id: string, parentId: string): Promise<{ ok: boolean; error?: string }> {
    try {
      await nodeMove(id, parentId, 999999);
      await this.load();
      this.expand(parentId);
      this.expandAncestors(id);
      this.select(id);
      state.statusMessage = 'Moved';
      return { ok: true };
    } catch (e) {
      const error = String(e);
      toast.error(`Move failed: ${error}`);
      console.error(e);
      return { ok: false, error };
    }
  },

  async configureNode(
    id: string,
    input: {
      title: string;
      icon?: string;
      color?: string;
      defaultView?: string;
      parentId?: string | null;
    },
  ): Promise<{ ok: boolean; error?: string }> {
    const node = state.nodes.get(id);
    if (!node) return { ok: false, error: 'Node not found' };
    const title = input.title.trim();
    if (!title) return { ok: false, error: 'Title is required' };
    try {
      const custom = mergedCustom(node, { defaultView: input.defaultView ?? 'list' });
      if (node.nodeType === 'Group' && input.parentId && input.parentId !== node.parentId) {
        await nodeMove(id, input.parentId, 999999);
      }
      await nodeUpdate(id, {
        title,
        properties: {
          icon: input.icon,
          color: input.color,
          custom,
        },
      });
      await this.load();
      this.expandAncestors(id);
      this.select(id);
      state.statusMessage = 'Configuration saved';
      return { ok: true };
    } catch (e) {
      const error = String(e);
      toast.error(`Configuration failed: ${error}`);
      return { ok: false, error };
    }
  },

  async setPriority(id: string, priority: number) {
    try {
      const updated = await nodeSetPriority(id, priority);
      this.upsert(updated);
      state.statusMessage = `Priority P${priority}`;
    } catch (e) {
      toast.error('Failed to set priority');
    }
  },

  async setDueDate(id: string, dueDate: string | null) {
    try {
      const updated = await nodeSetDueDate(id, dueDate);
      this.upsert(updated);
      state.statusMessage = dueDate ? 'Due date set' : 'Due date cleared';
    } catch (e) {
      toast.error('Failed to set due date');
    }
  },

  async setFavorite(id: string, favorite: boolean) {
    try {
      const updated = await nodeUpdate(id, { properties: { favorite } });
      this.upsert(updated);
      state.statusMessage = favorite ? 'Added to favorites' : 'Removed from favorites';
    } catch (e) {
      toast.error('Failed to update favorite');
    }
  },

  async setPinned(id: string, pinned: boolean) {
    try {
      const updated = await nodeUpdate(id, { properties: { pinned } });
      this.upsert(updated);
      state.statusMessage = pinned ? 'Pinned' : 'Unpinned';
    } catch (e) {
      toast.error('Failed to update pin');
    }
  },

  async replaceProperties(id: string, properties: Node['properties']) {
    try {
      const updated = await nodeReplaceProperties(id, cleanedProperties(properties));
      this.upsert(updated);
      return updated;
    } catch (e) {
      toast.error('Failed to save properties');
      console.error(e);
      return null;
    }
  },

  async setColor(id: string, color: string | null) {
    const node = state.nodes.get(id);
    if (!node) return;
    try {
      const next = { ...node.properties };
      if (color) next.color = color;
      else delete next.color;
      const updated = await nodeReplaceProperties(id, cleanedProperties(next));
      this.upsert(updated);
      state.statusMessage = color ? 'Color updated' : 'Color cleared';
    } catch (e) {
      toast.error('Failed to update color');
    }
  },

  async saveScheduling(
    id: string,
    input: {
      dueDate: string | null;
      reminders?: Node['properties']['reminders'];
      recurrence?: Node['properties']['recurrence'];
      estimatedDurationMinutes?: number;
    },
  ) {
    const node = state.nodes.get(id);
    if (!node || node.nodeType !== 'Task') {
      toast.error('Select a task or subtask first');
      return null;
    }
    try {
      const next = { ...node.properties };
      if (input.dueDate) next.dueDate = input.dueDate;
      else delete next.dueDate;
      if (input.reminders && input.reminders.length) next.reminders = input.reminders;
      else delete next.reminders;
      if (input.recurrence) next.recurrence = input.recurrence;
      else delete next.recurrence;
      if (input.estimatedDurationMinutes && input.estimatedDurationMinutes > 0) {
        next.estimatedDurationMinutes = input.estimatedDurationMinutes;
      } else {
        delete next.estimatedDurationMinutes;
      }
      const updated = await nodeReplaceProperties(id, cleanedProperties(next));
      this.upsert(updated);
      state.statusMessage = 'Schedule saved';
      return updated;
    } catch (e) {
      toast.error(`Schedule failed: ${String(e)}`);
      return null;
    }
  },

  async setWontDo(id: string, wontDo: boolean) {
    const node = state.nodes.get(id);
    if (!node) return;
    try {
      const updated = await nodeUpdate(id, {
        properties: {
          custom: mergedCustom(node, { wontDo }),
        },
      });
      this.upsert(updated);
      state.statusMessage = wontDo ? "Marked won't do" : "Cleared won't do";
    } catch (e) {
      toast.error("Failed to update won't do");
    }
  },

  async setTaskCustom(id: string, patch: Record<string, unknown>) {
    const node = state.nodes.get(id);
    if (!node || node.nodeType !== 'Task') {
      toast.error('Select a task or subtask first');
      return null;
    }
    try {
      const updated = await nodeUpdate(id, {
        properties: {
          custom: mergedCustom(node, patch),
        },
      });
      this.upsert(updated);
      state.statusMessage = 'Task metadata saved';
      return updated;
    } catch (e) {
      toast.error(`Task metadata failed: ${String(e)}`);
      return null;
    }
  },

  async convertTaskToNote(id: string) {
    const node = state.nodes.get(id);
    if (!node || node.nodeType !== 'Task') {
      toast.error('Only tasks and subtasks can be converted to notes');
      return null;
    }
    try {
      const updated = await nodeUpdate(id, {
        nodeType: 'Note',
        properties: {
          custom: mergedCustom(node, {
            convertedFrom: 'Task',
            convertedAt: new Date().toISOString(),
          }),
        },
      });
      await this.load();
      this.select(updated.id);
      state.statusMessage = 'Converted to note';
      return updated;
    } catch (e) {
      toast.error(`Convert to note failed: ${String(e)}`);
      return null;
    }
  },

  isWontDo(id: string): boolean {
    const node = state.nodes.get(id);
    const custom = node?.properties.custom;
    return Boolean(custom && typeof custom === 'object' && !Array.isArray(custom) && custom.wontDo);
  },

  async setTags(id: string, tags: string[]) {
    const node = state.nodes.get(id);
    if (!node || node.nodeType !== 'Task') return;
    const cleanTags = [...new Set(tags.map((tag) => tag.trim()).filter(Boolean))];
    try {
      const known = new Map(state.tags.map((tag) => [tagKey(tag.name), tag]));
      const desired: Tag[] = [];
      for (const tagName of cleanTags) {
        const key = tagKey(tagName);
        let tag = known.get(key);
        if (!tag) {
          tag = await tagCreate(tagName, null);
          known.set(key, tag);
          state.tags = [...state.tags, tag].sort((a, b) => a.name.localeCompare(b.name));
        }
        desired.push(tag);
      }

      const current = state.tagAssignments.get(id) ?? [];
      const desiredIds = new Set(desired.map((tag) => tag.id));
      const currentIds = new Set(current.map((tag) => tag.id));
      for (const tag of desired) {
        if (!currentIds.has(tag.id)) await tagAssign(id, tag.id);
      }
      for (const tag of current) {
        if (!desiredIds.has(tag.id)) await tagRemove(id, tag.id);
      }

      state.tagAssignments = new Map(state.tagAssignments).set(id, desired);
      state._rev++;
      state.statusMessage = cleanTags.length ? `Tagged ${cleanTags.length}` : 'Tags cleared';
    } catch (e) {
      console.error(e);
      toast.error('Failed to update tags');
    }
  },

  getTags(id: string): string[] {
    const node = state.nodes.get(id);
    if (state.tagAssignments.has(id)) {
      return (state.tagAssignments.get(id) ?? []).map((tag) => tag.name);
    }
    return tagsFromCustom(node?.properties.custom);
  },

  getTagObjects(id: string): Tag[] {
    state._rev;
    if (state.tagAssignments.has(id)) return state.tagAssignments.get(id) ?? [];
    const node = state.nodes.get(id);
    return tagsFromCustom(node?.properties.custom).map((name) => ({
      id: `legacy:${tagKey(name)}`,
      name,
      color: null,
      createdAt: '',
    }));
  },

  async createTag(name: string, color: string | null = null): Promise<Tag | null> {
    try {
      const tag = await tagCreate(name, color);
      state.tags = [...state.tags, tag].sort((a, b) => a.name.localeCompare(b.name));
      state._rev++;
      return tag;
    } catch (e) {
      toast.error(`Tag create failed: ${String(e)}`);
      return null;
    }
  },

  async renameTag(id: string, name: string, color: string | null = null): Promise<Tag | null> {
    try {
      const tag = await tagRename(id, name, color);
      state.tags = state.tags.map((existing) => (existing.id === id ? tag : existing));
      const assignments = new Map(state.tagAssignments);
      for (const [nodeId, list] of assignments) {
        assignments.set(
          nodeId,
          list.map((existing) => (existing.id === id ? tag : existing)),
        );
      }
      state.tagAssignments = assignments;
      state._rev++;
      return tag;
    } catch (e) {
      toast.error(`Tag rename failed: ${String(e)}`);
      return null;
    }
  },

  async deleteTag(id: string) {
    if (!window.confirm('Delete this tag? Task records will remain.')) return;
    try {
      await tagDelete(id);
      state.tags = state.tags.filter((tag) => tag.id !== id);
      const assignments = new Map(state.tagAssignments);
      for (const [nodeId, list] of assignments) {
        assignments.set(
          nodeId,
          list.filter((tag) => tag.id !== id),
        );
      }
      state.tagAssignments = assignments;
      if (state.selectedTagId === id) state.selectedTagId = null;
      state._rev++;
    } catch (e) {
      toast.error(`Tag delete failed: ${String(e)}`);
    }
  },

  async restoreFromTrash(id: string, destinationParentId: string | null = null) {
    try {
      const restored = await trashRestore(id, destinationParentId);
      await this.load();
      await this.loadTrash();
      this.expandAncestors(restored.id);
      this.select(restored.id);
      state.statusMessage = 'Restored from Trash';
    } catch (e) {
      toast.error(`Restore failed: ${String(e)}`);
    }
  },

  async permanentlyDelete(id: string) {
    if (!window.confirm('Permanently delete this item and descendants? This cannot be undone.')) {
      return;
    }
    try {
      await nodeDelete(id, true);
      state.trashNodes.delete(id);
      state._rev++;
      state.statusMessage = 'Permanently deleted';
    } catch (e) {
      toast.error(`Permanent delete failed: ${String(e)}`);
    }
  },

  async emptyTrash() {
    if (!window.confirm('Empty Trash permanently? This cannot be undone.')) return;
    try {
      const count = await trashEmpty();
      state.trashNodes = new Map();
      state._rev++;
      state.statusMessage = `Emptied Trash (${count})`;
    } catch (e) {
      toast.error(`Empty Trash failed: ${String(e)}`);
    }
  },

  getFavorites(): Node[] {
    state._rev;
    return [...state.nodes.values()]
      .filter((n) => !n.deletedAt && n.properties.favorite)
      .sort(sortByPosition);
  },

  async updateBody(id: string, body: string) {
    try {
      const updated = await nodeUpdate(id, { body });
      this.upsert(updated);
    } catch (e) {
      toast.error('Failed to save body');
    }
  },

  async deleteSelected() {
    const id = state.selectedId;
    if (!id) return;
    const list = this.getFlatVisibleList();
    const idx = list.findIndex((x) => x.node.id === id);
    const neighbor = list[idx + 1]?.node.id ?? list[idx - 1]?.node.id ?? null;
    try {
      await nodeDelete(id, false);
      this.removeSubtree(id);
      await this.loadTrash();
      this.select(neighbor);
      state.statusMessage = 'Deleted (undo with Ctrl+Z)';
      toast.info('Deleted — Ctrl+Z to undo');
    } catch (e) {
      toast.error('Failed to delete');
    }
  },

  async indent() {
    const selected = this.getSelected();
    if (!selected) return;
    const siblings = this.getChildren(selected.parentId);
    const idx = siblings.findIndex((s) => s.id === selected.id);
    if (idx <= 0) return;
    const newParent = siblings[idx - 1];
    try {
      await nodeMove(selected.id, newParent.id, 999999);
      const updated = { ...selected, parentId: newParent.id };
      this.upsert(updated);
      this.expand(newParent.id);
      // reload for correct positions
      await this.load();
      this.select(selected.id);
      state.statusMessage = 'Indented';
    } catch (e) {
      toast.error('Cannot indent');
    }
  },

  async outdent() {
    const selected = this.getSelected();
    if (!selected?.parentId) return;
    const parent = state.nodes.get(selected.parentId);
    if (!parent) return;
    try {
      await nodeMove(selected.id, parent.parentId, parent.position + 500);
      await this.load();
      this.select(selected.id);
      state.statusMessage = 'Outdented';
    } catch (e) {
      toast.error('Cannot outdent');
    }
  },

  canMoveTo(id: string, targetParentId: string | null): { ok: boolean; reason?: string } {
    const node = state.nodes.get(id);
    const target = targetParentId ? state.nodes.get(targetParentId) : null;
    if (!node) return { ok: false, reason: 'Node not found' };
    if (node.id === targetParentId) return { ok: false, reason: 'A node cannot contain itself' };
    if (node.nodeType === 'Workspace' && targetParentId !== null) {
      return { ok: false, reason: 'Workspace remains a root item' };
    }
    if (
      targetParentId &&
      descendantsOf(id).some((descendant) => descendant.id === targetParentId)
    ) {
      return { ok: false, reason: 'Cannot move into a descendant' };
    }
    if (
      node.nodeType === 'Group' &&
      !(target?.nodeType === 'Workspace' || target?.nodeType === 'Group')
    ) {
      return { ok: false, reason: 'Groups can only move into workspaces or groups' };
    }
    if (
      node.nodeType === 'Task' &&
      !(
        target?.nodeType === 'Workspace' ||
        target?.nodeType === 'Group' ||
        target?.nodeType === 'Task'
      )
    ) {
      return { ok: false, reason: 'Tasks can only move into workspaces, groups, or tasks' };
    }
    if (node.nodeType !== 'Workspace' && !targetParentId) {
      return { ok: false, reason: 'Only workspaces can be placed at the root' };
    }
    return { ok: true };
  },

  async moveSibling(id: string, direction: -1 | 1) {
    const node = state.nodes.get(id);
    if (!node) return false;
    const siblings = this.getChildren(node.parentId);
    const index = siblings.findIndex((item) => item.id === id);
    const nextIndex = index + direction;
    if (index < 0 || nextIndex < 0 || nextIndex >= siblings.length) return false;
    const ids = siblings.map((item) => item.id);
    [ids[index], ids[nextIndex]] = [ids[nextIndex], ids[index]];
    try {
      if (node.parentId) await nodeReorder(node.parentId, ids);
      else await nodeReorderRoot(ids);
      await this.load();
      this.select(id);
      state.statusMessage = direction < 0 ? 'Moved up' : 'Moved down';
      return true;
    } catch (e) {
      toast.error(`Reorder failed: ${String(e)}`);
      return false;
    }
  },

  async moveToParentAt(id: string, targetParentId: string | null, targetIndex = 999999) {
    const validation = this.canMoveTo(id, targetParentId);
    if (!validation.ok) {
      toast.error(validation.reason ?? 'Invalid move');
      return false;
    }
    try {
      await nodeMoveOrdered(id, targetParentId, targetIndex);
      await this.load();
      if (targetParentId) this.expand(targetParentId);
      this.expandAncestors(id);
      this.select(id);
      state.statusMessage = 'Moved';
      return true;
    } catch (e) {
      toast.error(`Move failed: ${String(e)}`);
      return false;
    }
  },

  async undo() {
    try {
      const desc = await apiUndo();
      await this.load();
      state.statusMessage = `Undo: ${desc}`;
      toast.info(`Undo: ${desc}`);
    } catch {
      toast.info('Nothing to undo');
    }
  },

  async redo() {
    try {
      const desc = await apiRedo();
      await this.load();
      state.statusMessage = `Redo: ${desc}`;
      toast.info(`Redo: ${desc}`);
    } catch {
      toast.info('Nothing to redo');
    }
  },

  getTaskProjection(mode: ProjectionMode, sort: SortMode = 'manual') {
    state._rev;
    const source =
      mode === 'deleted'
        ? [...state.nodes.values(), ...state.trashNodes.values()]
        : [...state.nodes.values()];
    return projectTasks(source, {
      mode,
      sort,
      tagAssignments: state.tagAssignments,
      tagId: state.selectedTagId,
    });
  },

  getFilteredProjection(sort: SortMode = 'manual') {
    state._rev;
    return projectTasks([...state.nodes.values()], {
      mode: 'active',
      sort,
      filters: state.filterDraft,
      tagAssignments: state.tagAssignments,
    });
  },

  getTodayNodes(): Node[] {
    return this.getTaskProjection('today', 'priority').map((item) => item.node);
  },

  getUpcomingNodes(): Node[] {
    return this.getTaskProjection('upcoming', 'due').map((item) => item.node);
  },

  getCompletedNodes(): Node[] {
    return this.getTaskProjection('completed', 'completed').map((item) => item.node);
  },

  getInboxNodes(): Node[] {
    return this.getTaskProjection('inbox', 'manual').map((item) => item.node);
  },

  getOverdueCount(): number {
    return this.getTaskProjection('overdue', 'due').length;
  },
};
