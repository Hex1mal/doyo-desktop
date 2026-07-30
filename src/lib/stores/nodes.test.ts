import { describe, expect, it, vi, beforeEach } from 'vitest';
import type { Node } from '$lib/types/node';

// Mock Tauri webview APIs before any store module that depends on zoom is loaded.
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: vi.fn(() => ({
    setZoom: vi.fn(),
  })),
}));

// Mirror of nodes held by the mocked backend. Tests populate this and then
// call nodeStore.load() to simulate application startup.
const nodeMap = new Map<string, Node>();

vi.mock('$lib/api/client', () => {
  const mockFn = vi.fn;
  const mocks: Record<string, unknown> = {
    treeGetFull: vi.fn(async () => [...nodeMap.values()]),
    nodeCreate: mockFn(),
    nodeUpdate: vi.fn(async (id: string, changes: Record<string, unknown>) => {
      const existing = nodeMap.get(id) as Node;
      const updated = { ...existing, ...changes } as Node;
      nodeMap.set(id, updated);
      return updated;
    }),
    nodeReplaceProperties: mockFn(),
    nodeDelete: mockFn(),
    nodeMove: mockFn(),
    nodeMoveOrdered: mockFn(),
    nodeReorder: mockFn(),
    nodeReorderRoot: mockFn(),
    nodeSetPriority: mockFn(),
    nodeSetDueDate: mockFn(),
    nodeSetCompletion: mockFn(),
    nodeIncompleteDescendantCount: mockFn(),
    nodeDuplicate: mockFn(),
    tagAssign: mockFn(),
    tagCreate: mockFn(),
    tagDelete: mockFn(),
    tagGetForNode: vi.fn(async () => []),
    tagList: vi.fn(async () => []),
    tagRemove: mockFn(),
    tagRename: mockFn(),
    tagSyncLegacy: vi.fn(async () => 0),
    trashEmpty: mockFn(),
    trashGetNodes: vi.fn(async () => []),
    trashRestore: mockFn(),
    searchQuery: mockFn(),
    undo: mockFn(),
    redo: mockFn(),
  };
  return mocks;
});

function createNode(
  id: string,
  parentId: string | null,
  nodeType: Node['nodeType'],
  title: string,
  position: number,
): Node {
  return {
    id,
    parentId,
    position,
    nodeType,
    title,
    body: '',
    properties: {},
    isCollapsed: true,
    isCompleted: false,
    completedAt: null,
    deletedAt: null,
    version: 1,
    createdAt: '',
    updatedAt: '',
  };
}

describe('nodeStore expansion state', () => {
  beforeEach(() => {
    nodeMap.clear();
    vi.resetModules();
  });

  async function loadStore() {
    const { nodeStore } = await import('./nodes.svelte');
    return nodeStore;
  }

  it('starts with all workspaces collapsed after load', async () => {
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('ws2', createNode('ws2', null, 'Workspace', 'Workspace 2', 1));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();

    expect([...nodeStore.expandedIds]).toEqual([]);
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1', 'ws2']);
  });

  it('does not show groups, tasks, or subtasks on startup', async () => {
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));
    nodeMap.set('t1', createNode('t1', 'g1', 'Task', 'Task 1', 0));
    nodeMap.set('st1', createNode('st1', 't1', 'Task', 'Subtask 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();

    const visibleIds = nodeStore.getFlatVisibleList().map((item) => item.node.id);
    expect(visibleIds).toEqual(['ws1']);
    expect(visibleIds).not.toContain('g1');
    expect(visibleIds).not.toContain('t1');
    expect(visibleIds).not.toContain('st1');
  });

  it('does not perform a default depth-three expansion', async () => {
    // Build a three-level deep hierarchy to ensure no level is auto-expanded.
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));
    nodeMap.set('sg1', createNode('sg1', 'g1', 'Group', 'Subgroup 1', 0));
    nodeMap.set('t1', createNode('t1', 'sg1', 'Task', 'Task 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();

    expect([...nodeStore.expandedIds]).toEqual([]);
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1']);
  });

  it('manually expanding a workspace reveals only its direct children', async () => {
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));
    nodeMap.set('sg1', createNode('sg1', 'g1', 'Group', 'Subgroup 1', 0));
    nodeMap.set('t1', createNode('t1', 'ws1', 'Task', 'Task 1', 1));

    const nodeStore = await loadStore();
    await nodeStore.load();
    nodeStore.expand('ws1');

    const visibleIds = nodeStore.getFlatVisibleList().map((item) => item.node.id);
    expect(visibleIds).toEqual(['ws1', 'g1', 't1']);
    expect(visibleIds).not.toContain('sg1');
  });

  it('manually expanding nested nodes still works', async () => {
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));
    nodeMap.set('sg1', createNode('sg1', 'g1', 'Group', 'Subgroup 1', 0));
    nodeMap.set('t1', createNode('t1', 'sg1', 'Task', 'Task 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();

    nodeStore.expand('ws1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1', 'g1']);

    nodeStore.expand('g1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual([
      'ws1',
      'g1',
      'sg1',
    ]);

    nodeStore.expand('sg1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual([
      'ws1',
      'g1',
      'sg1',
      't1',
    ]);
  });

  it('collapsing a node hides its descendants', async () => {
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));
    nodeMap.set('t1', createNode('t1', 'g1', 'Task', 'Task 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();

    nodeStore.expand('ws1');
    nodeStore.expand('g1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1', 'g1', 't1']);

    await nodeStore.toggleExpand('g1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1', 'g1']);

    await nodeStore.toggleExpand('ws1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1']);
  });

  it('selecting a node does not recursively expand the hierarchy', async () => {
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));
    nodeMap.set('t1', createNode('t1', 'g1', 'Task', 'Task 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();

    nodeStore.select('ws1');
    expect([...nodeStore.expandedIds]).toEqual([]);
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1']);
  });

  it('preserves node ordering after load', async () => {
    nodeMap.set('ws2', createNode('ws2', null, 'Workspace', 'Workspace 2', 100));
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g2', createNode('g2', 'ws1', 'Group', 'Group 2', 100));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();

    nodeStore.expand('ws1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual([
      'ws1',
      'g1',
      'g2',
      'ws2',
    ]);
  });

  it('reinitializing the store produces collapsed roots', async () => {
    nodeMap.set('ws1', createNode('ws1', null, 'Workspace', 'Workspace 1', 0));
    nodeMap.set('g1', createNode('g1', 'ws1', 'Group', 'Group 1', 0));

    const nodeStore = await loadStore();
    await nodeStore.load();
    nodeStore.expand('ws1');
    expect(nodeStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1', 'g1']);

    // Simulate a fresh load (e.g. after app restart) without preserving UI state.
    vi.resetModules();
    const freshStore = (await import('./nodes.svelte')).nodeStore;
    await freshStore.load();

    expect([...freshStore.expandedIds]).toEqual([]);
    expect(freshStore.getFlatVisibleList().map((item) => item.node.id)).toEqual(['ws1']);
  });
});
