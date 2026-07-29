<script lang="ts">
  import type { Node } from '$lib/types/node';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import SidebarNode from './SidebarNode.svelte';

  let { node, depth = 0 }: { node: Node; depth?: number } = $props();

  let children = $derived(nodeStore.getChildren(node.id));
  let isExpanded = $derived(nodeStore.expandedIds.has(node.id));
  let isActive = $derived(nodeStore.selectedId === node.id && nodeStore.viewMode === 'tree');
  let parent = $derived(node.parentId ? nodeStore.get(node.parentId) : null);

  function kindFor(nodeType: string) {
    if (nodeType === 'Workspace') return 'workspace';
    if (nodeType === 'Group' && parent?.nodeType === 'Group') return 'subgroup';
    if (nodeType === 'Group') return 'group';
    if (nodeType === 'Task' && parent?.nodeType === 'Task') return 'subtask';
    if (nodeType === 'Task') return 'task';
    if (nodeType === 'Note') return 'note';
    return 'other';
  }

  function typeLabel(nodeType: string) {
    if (nodeType === 'Workspace') return 'W';
    if (nodeType === 'Group') return 'G';
    if (nodeType === 'Task') return 'T';
    if (nodeType === 'Note') return 'N';
    return '•';
  }

  function childNoun(nodeType: string) {
    if (nodeType === 'Workspace') return 'groups';
    if (nodeType === 'Group') return 'items';
    if (nodeType === 'Task') return 'subtasks';
    return 'items';
  }

  function openNode() {
    uiStore.setActiveModule('workspaces');
    nodeStore.selectInWorkspace(node.id);
  }
</script>

<div class="sidebar-node" style={`--depth: ${depth}`}>
  <div class="row">
    <button
      class="chev"
      aria-label={isExpanded ? 'Collapse' : 'Expand'}
      disabled={children.length === 0}
      onclick={(e) => {
        e.stopPropagation();
        nodeStore.toggleExpand(node.id);
      }}
    >
      {children.length === 0 ? '' : isExpanded ? '▾' : '▸'}
    </button>
    {#if depth > 0}
      <span class="branch" aria-hidden="true">└</span>
    {/if}
    <button
      class="item"
      class:active={isActive}
      onclick={openNode}
      title={node.title || 'Untitled'}
    >
      <span class="type-mark {kindFor(node.nodeType)}">{typeLabel(node.nodeType)}</span>
      <span class="label">{node.title || 'Untitled'}</span>
      {#if children.length > 0}
        <span class="count">{children.length} {childNoun(node.nodeType)}</span>
      {/if}
    </button>
  </div>

  {#if isExpanded}
    {#each children as child (child.id)}
      <SidebarNode node={child} depth={depth + 1} />
    {/each}
  {/if}
</div>

<style>
  .sidebar-node {
    min-width: 0;
  }
  .row {
    display: flex;
    align-items: center;
    padding-left: calc(var(--depth) * 12px);
  }
  .chev {
    width: 20px;
    height: 26px;
    border: none;
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
    flex-shrink: 0;
    font-size: 11px;
  }
  .chev:disabled {
    cursor: default;
    opacity: 0.35;
  }
  .item {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 5px 7px;
    border-radius: 5px;
    font-size: var(--text-sm);
    text-align: left;
  }
  .branch {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: 11px;
    margin-right: 3px;
    flex-shrink: 0;
  }
  .item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .item.active {
    background: var(--bg-active);
    color: var(--accent);
    font-weight: 650;
  }
  .type-mark {
    width: 17px;
    height: 17px;
    border-radius: 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 800;
    flex-shrink: 0;
  }
  .type-mark.workspace {
    background: rgba(99, 102, 241, 0.14);
    color: var(--accent);
  }
  .type-mark.group {
    background: rgba(59, 130, 246, 0.14);
    color: var(--info);
  }
  .type-mark.subgroup {
    background: rgba(14, 165, 233, 0.13);
    color: #0ea5e9;
  }
  .type-mark.task {
    background: rgba(16, 185, 129, 0.12);
    color: var(--success);
  }
  .type-mark.subtask {
    background: rgba(245, 158, 11, 0.14);
    color: var(--warning);
  }
  .type-mark.note {
    background: rgba(139, 92, 246, 0.12);
    color: #8b5cf6;
  }
  .type-mark.other {
    background: var(--bg-hover);
    color: var(--text-tertiary);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    color: var(--text-tertiary);
    font-size: 10px;
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
