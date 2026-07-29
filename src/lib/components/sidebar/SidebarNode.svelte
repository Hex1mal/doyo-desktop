<script module lang="ts">
  let activeSidebarDragId: string | null = null;
  let pointerDropElement: HTMLElement | null = null;

  function clearPointerDropElement() {
    pointerDropElement?.classList.remove('pointer-drop-before', 'pointer-drop-after');
    pointerDropElement = null;
  }

  function markPointerDropElement(
    element: HTMLElement | null,
    position: 'before' | 'after' | null,
  ) {
    clearPointerDropElement();
    if (!element || !position) return;
    pointerDropElement = element;
    element.classList.add(`pointer-drop-${position}`);
  }
</script>

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
  let showMenu = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let dropPosition = $state<'before' | 'after' | null>(null);
  let suppressNextClick = $state(false);
  const colorCycle = ['#6366F1', '#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6'];
  let siblings = $derived(nodeStore.getChildren(node.parentId));
  let siblingIndex = $derived(siblings.findIndex((item) => item.id === node.id));

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
    showMenu = false;
    uiStore.setActiveModule('workspaces');
    nodeStore.selectInWorkspace(node.id);
  }

  function openMenuAt(clientX: number, clientY: number) {
    const menuW = 230;
    const menuH = 380;
    menuX = Math.max(8, Math.min(clientX + 2, window.innerWidth - menuW - 8));
    menuY = Math.max(8, Math.min(clientY + 2, window.innerHeight - menuH - 8));
    showMenu = true;
    uiStore.setActiveModule('workspaces');
    nodeStore.selectInWorkspace(node.id);
  }

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    openMenuAt(event.clientX, event.clientY);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
      openMenuAt(rect.left + 24, rect.top + rect.height);
    }
  }

  function onDragStart(event: DragEvent) {
    if (node.nodeType !== 'Workspace') return;
    activeSidebarDragId = node.id;
    event.dataTransfer?.setData('application/doyo-node', JSON.stringify({ id: node.id }));
    event.dataTransfer!.effectAllowed = 'move';
  }

  function draggedIdFrom(event: DragEvent) {
    const raw = event.dataTransfer?.getData('application/doyo-node');
    if (!raw) return activeSidebarDragId;
    try {
      return (JSON.parse(raw) as { id: string }).id;
    } catch {
      return activeSidebarDragId;
    }
  }

  function onDragOver(event: DragEvent) {
    if (node.nodeType !== 'Workspace') return;
    const draggedId = draggedIdFrom(event);
    if (!draggedId) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    dropPosition = event.clientY < rect.top + rect.height / 2 ? 'before' : 'after';
  }

  async function onDrop(event: DragEvent) {
    event.preventDefault();
    dropPosition = null;
    const draggedId = draggedIdFrom(event);
    activeSidebarDragId = null;
    if (!draggedId || node.nodeType !== 'Workspace') return;
    if (draggedId === node.id) return;
    const roots = nodeStore.getRoots();
    const from = roots.findIndex((item) => item.id === draggedId);
    const to = roots.findIndex((item) => item.id === node.id);
    if (from < 0 || to < 0) return;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const targetIndex = event.clientY < rect.top + rect.height / 2 ? to : to + 1;
    const adjusted = from < targetIndex ? targetIndex - 1 : targetIndex;
    const direction = adjusted < from ? -1 : 1;
    for (let i = 0; i < Math.abs(adjusted - from); i += 1) {
      await nodeStore.moveSibling(draggedId, direction as -1 | 1);
    }
  }

  function clearDrop() {
    dropPosition = null;
  }

  function onDragEnd() {
    activeSidebarDragId = null;
    dropPosition = null;
  }

  function closestSidebarItem(event: PointerEvent) {
    return document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>('[data-sidebar-node-id]');
  }

  function handlePointerDown(event: PointerEvent) {
    if (node.nodeType !== 'Workspace' || event.button !== 0) return;
    if ((event.target as HTMLElement).closest('.chev,input,textarea,.context-menu')) return;
    const startX = event.clientX;
    const startY = event.clientY;
    let dragging = false;

    const handleMove = (moveEvent: PointerEvent) => {
      const distance = Math.hypot(moveEvent.clientX - startX, moveEvent.clientY - startY);
      if (!dragging && distance < 6) return;
      dragging = true;
      suppressNextClick = true;
      moveEvent.preventDefault();
      document.body.classList.add('sidebar-pointer-dragging');
      const targetEl = closestSidebarItem(moveEvent);
      if (!targetEl) {
        markPointerDropElement(null, null);
        return;
      }
      const targetId = targetEl.dataset.sidebarNodeId;
      const targetNode = targetId ? nodeStore.get(targetId) : null;
      if (targetId === node.id || targetNode?.nodeType !== 'Workspace') {
        markPointerDropElement(null, null);
        return;
      }
      const rect = targetEl.getBoundingClientRect();
      markPointerDropElement(
        targetEl,
        moveEvent.clientY < rect.top + rect.height / 2 ? 'before' : 'after',
      );
    };

    const handleUp = async (upEvent: PointerEvent) => {
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleUp);
      document.body.classList.remove('sidebar-pointer-dragging');
      const wasDragging = dragging;
      const targetEl = closestSidebarItem(upEvent);
      clearPointerDropElement();
      if (!wasDragging || !targetEl) return;
      const targetId = targetEl.dataset.sidebarNodeId;
      const roots = nodeStore.getRoots();
      const from = roots.findIndex((item) => item.id === node.id);
      const to = roots.findIndex((item) => item.id === targetId);
      if (from < 0 || to < 0 || from === to) return;
      const rect = targetEl.getBoundingClientRect();
      const targetIndex = upEvent.clientY < rect.top + rect.height / 2 ? to : to + 1;
      const adjusted = from < targetIndex ? targetIndex - 1 : targetIndex;
      const direction = adjusted < from ? -1 : 1;
      for (let i = 0; i < Math.abs(adjusted - from); i += 1) {
        await nodeStore.moveSibling(node.id, direction as -1 | 1);
      }
    };

    window.addEventListener('pointermove', handleMove, { passive: false });
    window.addEventListener('pointerup', handleUp);
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
      class:drop-before={dropPosition === 'before'}
      class:drop-after={dropPosition === 'after'}
      data-sidebar-node-id={node.id}
      draggable={node.nodeType === 'Workspace'}
      onclick={() => {
        if (suppressNextClick) {
          suppressNextClick = false;
          return;
        }
        openNode();
      }}
      title={node.title || 'Untitled'}
      onpointerdown={handlePointerDown}
      ondragstart={onDragStart}
      ondragover={onDragOver}
      ondragleave={clearDrop}
      ondrop={onDrop}
      ondragend={onDragEnd}
      oncontextmenu={handleContextMenu}
      onkeydown={handleKeydown}
    >
      <span
        class="type-mark {kindFor(node.nodeType)}"
        style={node.properties.color ? `--node-color: ${node.properties.color}` : ''}
        >{typeLabel(node.nodeType)}</span
      >
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

{#if showMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="menu-backdrop"
    onclick={() => (showMenu = false)}
    oncontextmenu={(event) => {
      event.preventDefault();
      showMenu = false;
    }}
  ></div>
  <div class="context-menu" style="left: {menuX}px; top: {menuY}px;">
    <div class="menu-title">{nodeStore.getKindLabel(node)}</div>
    <div class="menu-section">
      <div class="menu-section-label">Create</div>
      {#if node.nodeType === 'Workspace'}
        <button onclick={() => (nodeStore.createGroupUnder(node.id), (showMenu = false))}
          >New Group</button
        >
        <button onclick={() => (nodeStore.createTaskUnder(node.id), (showMenu = false))}
          >New Task</button
        >
      {:else if node.nodeType === 'Group'}
        <button onclick={() => (nodeStore.createSubgroupUnder(node.id), (showMenu = false))}
          >New Subgroup</button
        >
        <button onclick={() => (nodeStore.createTaskUnder(node.id), (showMenu = false))}
          >New Task</button
        >
      {:else if node.nodeType === 'Task'}
        <button onclick={() => (nodeStore.createSubtaskUnder(node.id), (showMenu = false))}
          >New Subtask</button
        >
      {/if}
    </div>
    <div class="menu-section">
      <div class="menu-section-label">Hierarchy</div>
      <button
        disabled={siblingIndex <= 0}
        onclick={() => (nodeStore.moveSibling(node.id, -1), (showMenu = false))}>Move up</button
      >
      <button
        disabled={siblingIndex < 0 || siblingIndex >= siblings.length - 1}
        onclick={() => (nodeStore.moveSibling(node.id, 1), (showMenu = false))}>Move down</button
      >
    </div>
    <div class="menu-section">
      <div class="menu-section-label">Organization</div>
      <button
        onclick={() => (
          nodeStore.setFavorite(node.id, !node.properties.favorite),
          (showMenu = false)
        )}>{node.properties.favorite ? 'Remove Favorite' : 'Add Favorite'}</button
      >
      {#if node.nodeType === 'Workspace' || node.nodeType === 'Group'}
        <button onclick={() => (uiStore.openConfigDialog(node.id), (showMenu = false))}
          >Configure {nodeStore.getKindLabel(node)}</button
        >
      {/if}
      {#if node.nodeType === 'Task'}
        <button onclick={() => (uiStore.openDueDatePrompt(), (showMenu = false))}
          >Schedule...</button
        >
      {/if}
      <div class="color-menu" aria-label="Change color">
        {#each colorCycle as color}
          <button
            class="color-choice"
            class:active={node.properties.color === color}
            style={`--choice-color: ${color}`}
            title={`Use ${color}`}
            onclick={() => (nodeStore.setColor(node.id, color), (showMenu = false))}
          ></button>
        {/each}
        <button
          class="color-default"
          onclick={() => (nodeStore.setColor(node.id, null), (showMenu = false))}>Default</button
        >
      </div>
    </div>
  </div>
{/if}

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
  .item.drop-before,
  .item.drop-after,
  .item.pointer-drop-before,
  .item.pointer-drop-after {
    position: relative;
  }
  .item.drop-before::before,
  .item.drop-after::after,
  .item.pointer-drop-before::before,
  .item.pointer-drop-after::after {
    content: '';
    position: absolute;
    left: 4px;
    right: 4px;
    height: 2px;
    background: var(--accent);
    border-radius: 999px;
  }
  .item.drop-before::before {
    top: 0;
  }
  .item.pointer-drop-before::before {
    top: 0;
  }
  .item.drop-after::after,
  .item.pointer-drop-after::after {
    bottom: 0;
  }
  :global(body.sidebar-pointer-dragging) {
    cursor: grabbing;
    user-select: none;
    -webkit-user-select: none;
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
    background: color-mix(in srgb, var(--node-color, var(--accent)) 18%, transparent);
    color: var(--node-color, var(--accent));
  }
  .type-mark.group {
    background: color-mix(in srgb, var(--node-color, var(--info)) 18%, transparent);
    color: var(--node-color, var(--info));
  }
  .type-mark.subgroup {
    background: color-mix(in srgb, var(--node-color, #0ea5e9) 18%, transparent);
    color: var(--node-color, #0ea5e9);
  }
  .type-mark.task {
    background: color-mix(in srgb, var(--node-color, var(--success)) 16%, transparent);
    color: var(--node-color, var(--success));
  }
  .type-mark.subtask {
    background: color-mix(in srgb, var(--node-color, var(--warning)) 16%, transparent);
    color: var(--node-color, var(--warning));
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
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 99;
  }
  .context-menu {
    position: fixed;
    z-index: 100;
    width: 230px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
    box-shadow: var(--shadow-lg);
  }
  .menu-title,
  .menu-section-label {
    padding: 5px 6px;
    color: var(--text-tertiary);
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
  }
  .menu-section {
    border-top: 1px solid var(--border);
    padding-top: 4px;
    margin-top: 4px;
  }
  .menu-section:first-child {
    border-top: none;
  }
  .context-menu button {
    width: 100%;
    min-height: 28px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    cursor: pointer;
    padding: 5px 7px;
    font-size: var(--text-xs);
  }
  .context-menu button:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .context-menu button:disabled {
    cursor: default;
    opacity: 0.38;
  }
  .color-menu {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 5px;
    padding: 5px 6px;
  }
  .color-choice {
    height: 24px;
    background: var(--choice-color) !important;
    border: 2px solid transparent !important;
  }
  .color-choice.active {
    border-color: var(--text-primary) !important;
  }
  .color-default {
    grid-column: span 2;
    text-align: center !important;
    border: 1px solid var(--border) !important;
  }
</style>
