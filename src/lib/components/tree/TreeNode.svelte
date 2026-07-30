<script module lang="ts">
  let activeTreeDragId: string | null = null;
  let pointerDropElement: HTMLElement | null = null;

  function clearPointerDropElement() {
    pointerDropElement?.classList.remove(
      'pointer-drop-before',
      'pointer-drop-after',
      'pointer-drop-inside',
      'pointer-drop-invalid',
    );
    pointerDropElement = null;
  }

  function markPointerDropElement(element: HTMLElement | null, state: string | null) {
    clearPointerDropElement();
    if (!element || !state) return;
    pointerDropElement = element;
    element.classList.add(`pointer-drop-${state}`);
  }
</script>

<script lang="ts">
  import type { Node } from '$lib/types/node';
  import { focusStore } from '$lib/stores/focus.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import { formatDue, isOverdue } from '$lib/utils/date';

  let {
    node,
    depth = 0,
    isSelected = false,
    isEditing = false,
    flat = false,
  }: {
    node: Node;
    depth?: number;
    isSelected?: boolean;
    isEditing?: boolean;
    flat?: boolean;
  } = $props();

  let editTitle = $state('');
  let inputEl: HTMLInputElement | undefined = $state();
  let showMenu = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let dropState = $state<'before' | 'after' | 'inside' | 'invalid' | null>(null);
  let suppressNextClick = $state(false);
  let pendingMove = $state<{
    sourceId: string;
    targetParentId: string | null;
    targetIndex?: number;
  } | null>(null);
  const INDENT = 20;

  let hasChildren = $derived(nodeStore.getChildren(node.id).length > 0);
  let isExpanded = $derived(nodeStore.expandedIds.has(node.id));
  let children = $derived(nodeStore.getChildren(node.id));
  let tags = $derived(nodeStore.getTagObjects(node.id));
  let childCount = $derived(children.length);
  let parent = $derived(node.parentId ? nodeStore.get(node.parentId) : null);
  let kind = $derived.by(() => {
    if (node.nodeType === 'Workspace') return 'workspace';
    if (node.nodeType === 'Group' && parent?.nodeType === 'Group') return 'subgroup';
    if (node.nodeType === 'Group') return 'group';
    if (node.nodeType === 'Task' && parent?.nodeType === 'Task') return 'subtask';
    if (node.nodeType === 'Task') return 'task';
    if (node.nodeType === 'Note') return 'note';
    return 'other';
  });
  let kindLabel = $derived.by(() => {
    if (kind === 'workspace') return 'Workspace';
    if (kind === 'group') return 'Group';
    if (kind === 'subgroup') return 'Subgroup';
    if (kind === 'task') return 'Task';
    if (kind === 'subtask') return 'Subtask';
    if (kind === 'note') return 'Note';
    return node.nodeType;
  });
  let childSummary = $derived.by(() => {
    if (childCount === 0) return '';
    if (kind === 'workspace' || kind === 'group' || kind === 'subgroup') {
      const groups = children.filter((child) => child.nodeType === 'Group').length;
      const tasks = children.filter((child) => child.nodeType === 'Task').length;
      const parts = [];
      if (groups)
        parts.push(
          `${groups} ${kind === 'workspace' ? 'group' : 'subgroup'}${groups === 1 ? '' : 's'}`,
        );
      if (tasks) parts.push(`${tasks} task${tasks === 1 ? '' : 's'}`);
      return parts.join(' · ');
    }
    if (kind === 'task' || kind === 'subtask') {
      return `${childCount} subtask${childCount === 1 ? '' : 's'}`;
    }
    return `${childCount} child${childCount === 1 ? '' : 'ren'}`;
  });
  const colorCycle = ['#6366F1', '#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6'];
  let siblings = $derived(nodeStore.getChildren(node.parentId));
  let siblingIndex = $derived(siblings.findIndex((item) => item.id === node.id));

  $effect(() => {
    if (isEditing) {
      editTitle = node.title;
      queueMicrotask(() => {
        inputEl?.focus();
        inputEl?.select();
      });
    }
  });

  $effect(() => {
    if (showMenu && nodeStore.selectedId !== node.id) {
      showMenu = false;
    }
  });

  function handleWindowKeydown(e: KeyboardEvent) {
    if (pendingMove && e.key === 'Escape') {
      e.preventDefault();
      pendingMove = null;
      return;
    }
    if (showMenu && e.key === 'Escape') {
      e.preventDefault();
      closeMenu();
    }
  }

  function closeMenu() {
    showMenu = false;
  }

  function openMenuAt(clientX: number, clientY: number) {
    const MARGIN = 12;
    const menuW = 260;
    const menuH = 440;
    let x = clientX + 2;
    let y = clientY + 2;
    if (x + menuW > window.innerWidth) x = clientX - menuW;
    if (y + menuH > window.innerHeight) y = clientY - menuH;
    if (x < MARGIN) x = MARGIN;
    if (y < MARGIN) y = MARGIN;
    menuX = x;
    menuY = y;
    showMenu = true;
    nodeStore.select(node.id);
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    openMenuAt(e.clientX, e.clientY);
  }

  async function commitEdit() {
    const t = editTitle.trim();
    if (t && t !== node.title) {
      await nodeStore.rename(node.id, t);
    } else if (!t && !node.title) {
      nodeStore.stopEditing();
    } else {
      nodeStore.stopEditing();
    }
  }

  function handleEditKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      commitEdit().then(() => {
        if (e.shiftKey) nodeStore.createChild('');
        else nodeStore.createSibling('');
      });
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      editTitle = node.title;
      nodeStore.stopEditing();
    } else if (e.key === 'Tab') {
      e.preventDefault();
      e.stopPropagation();
      commitEdit().then(() => {
        if (e.shiftKey) nodeStore.outdent();
        else nodeStore.indent();
      });
    }
  }

  function handleRowKeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      openMenuAt(rect.left + 32, rect.top + rect.height);
    } else if (e.key === 'Enter' || e.key === 'F2') {
      e.preventDefault();
      nodeStore.startEditing(node.id);
    } else if (e.key === ' ') {
      e.preventDefault();
      nodeStore.select(node.id);
      uiStore.toggleInspector();
    }
  }

  async function copyInternalLink() {
    try {
      await navigator.clipboard.writeText(`doyo://node/${node.id}`);
      nodeStore.setStatus('Internal link copied');
    } catch {
      nodeStore.setStatus('Could not copy link');
    }
  }

  function queueCrossParentMove(
    sourceId: string,
    targetParentId: string | null,
    targetIndex?: number,
  ) {
    pendingMove = { sourceId, targetParentId, targetIndex };
  }

  async function confirmPendingMove() {
    if (!pendingMove) return;
    const move = pendingMove;
    pendingMove = null;
    await nodeStore.moveToParentAt(move.sourceId, move.targetParentId, move.targetIndex);
  }

  function pendingMoveTitle(move: typeof pendingMove) {
    if (!move) return '';
    const source = nodeStore.get(move.sourceId);
    const target = move.targetParentId ? nodeStore.get(move.targetParentId) : null;
    const sourceName = source?.title || 'Untitled';
    if (!move.targetParentId) return `Move "${sourceName}" to the workspace root?`;
    return `Move "${sourceName}" into "${target?.title || 'Untitled'}"?`;
  }

  function pendingMoveDetail(move: typeof pendingMove) {
    if (!move) return '';
    const source = nodeStore.get(move.sourceId);
    const target = move.targetParentId ? nodeStore.get(move.targetParentId) : null;
    const sourceKind = source ? nodeStore.getKindLabel(source) : 'Item';
    const targetKind = target ? nodeStore.getKindLabel(target) : 'Root';
    return `${sourceKind} will be moved under ${targetKind}. Existing descendants and order are preserved.`;
  }

  function handleDragStart(event: DragEvent) {
    activeTreeDragId = node.id;
    event.dataTransfer?.setData('application/doyo-node', JSON.stringify({ id: node.id }));
    event.dataTransfer!.effectAllowed = 'move';
  }

  function draggedIdFrom(event: DragEvent) {
    const raw = event.dataTransfer?.getData('application/doyo-node');
    if (!raw) return activeTreeDragId;
    try {
      return (JSON.parse(raw) as { id: string }).id;
    } catch {
      return activeTreeDragId;
    }
  }

  function handleDragOver(event: DragEvent) {
    const draggedId = draggedIdFrom(event);
    if (!draggedId) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    try {
      if (draggedId === node.id) {
        dropState = 'invalid';
        return;
      }
      const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
      const ratio = (event.clientY - rect.top) / Math.max(1, rect.height);
      if (ratio < 0.28) {
        dropState = 'before';
        return;
      }
      if (ratio > 0.72) {
        dropState = 'after';
        return;
      }
      dropState = nodeStore.canMoveTo(draggedId, node.id).ok ? 'inside' : 'invalid';
    } catch {
      dropState = 'invalid';
    }
  }

  async function handleDrop(event: DragEvent) {
    event.preventDefault();
    dropState = null;
    const draggedId = draggedIdFrom(event);
    activeTreeDragId = null;
    if (!draggedId || draggedId === node.id) return;
    const source = nodeStore.get(draggedId);
    if (!source) return;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = (event.clientY - rect.top) / Math.max(1, rect.height);
    if (ratio < 0.28 || ratio > 0.72) {
      if (source.parentId !== node.parentId) {
        const targetParent = node.parentId;
        const validation = nodeStore.canMoveTo(source.id, targetParent);
        if (!validation.ok) {
          nodeStore.setStatus(validation.reason ?? 'Invalid drop target');
          return;
        }
        const targetSiblings = nodeStore.getChildren(node.parentId);
        const targetIndex = targetSiblings.findIndex((item) => item.id === node.id);
        queueCrossParentMove(source.id, targetParent, ratio < 0.28 ? targetIndex : targetIndex + 1);
        return;
      }
      const targetSiblings = nodeStore.getChildren(node.parentId);
      const targetIndex = targetSiblings.findIndex((item) => item.id === node.id);
      await nodeStore.moveToParentAt(
        source.id,
        node.parentId,
        ratio < 0.28 ? targetIndex : targetIndex + 1,
      );
      return;
    }
    const validation = nodeStore.canMoveTo(source.id, node.id);
    if (!validation.ok) {
      nodeStore.setStatus(validation.reason ?? 'Invalid drop target');
      return;
    }
    queueCrossParentMove(source.id, node.id);
  }

  function clearDropState() {
    dropState = null;
  }

  function handleDragEnd() {
    activeTreeDragId = null;
    dropState = null;
  }

  function closestTreeRow(event: PointerEvent) {
    return document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>('[data-tree-node-id]');
  }

  function pointerDropStateFor(sourceId: string, targetId: string, targetEl: HTMLElement) {
    if (sourceId === targetId) return 'invalid';
    const targetNode = nodeStore.get(targetId);
    if (!targetNode) return 'invalid';
    const rect = targetEl.getBoundingClientRect();
    const ratio = eventRatio(targetEl, rect) ?? 0.5;
    if (ratio < 0.28) {
      return nodeStore.canMoveTo(sourceId, targetNode.parentId).ok ? 'before' : 'invalid';
    }
    if (ratio > 0.72) {
      return nodeStore.canMoveTo(sourceId, targetNode.parentId).ok ? 'after' : 'invalid';
    }
    return nodeStore.canMoveTo(sourceId, targetId).ok ? 'inside' : 'invalid';
  }

  function eventRatio(targetEl: HTMLElement, rect = targetEl.getBoundingClientRect()) {
    const y = Number(targetEl.dataset.pointerY);
    if (!Number.isFinite(y)) return null;
    return (y - rect.top) / Math.max(1, rect.height);
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0 || isEditing) return;
    if ((event.target as HTMLElement).closest('button,input,textarea,.context-menu')) return;
    const startX = event.clientX;
    const startY = event.clientY;
    let dragging = false;

    const handleMove = (moveEvent: PointerEvent) => {
      const distance = Math.hypot(moveEvent.clientX - startX, moveEvent.clientY - startY);
      if (!dragging && distance < 6) return;
      dragging = true;
      suppressNextClick = true;
      moveEvent.preventDefault();
      document.body.classList.add('tree-pointer-dragging');
      const targetEl = closestTreeRow(moveEvent);
      if (!targetEl) {
        markPointerDropElement(null, null);
        return;
      }
      targetEl.dataset.pointerY = String(moveEvent.clientY);
      const targetId = targetEl.dataset.treeNodeId;
      if (!targetId) return;
      markPointerDropElement(targetEl, pointerDropStateFor(node.id, targetId, targetEl));
    };

    const handleUp = async (upEvent: PointerEvent) => {
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleUp);
      document.body.classList.remove('tree-pointer-dragging');
      const wasDragging = dragging;
      const targetEl = closestTreeRow(upEvent);
      const targetId = targetEl?.dataset.treeNodeId;
      clearPointerDropElement();
      if (!wasDragging || !targetEl || !targetId || targetId === node.id) return;

      const targetNode = nodeStore.get(targetId);
      if (!targetNode) return;
      const rect = targetEl.getBoundingClientRect();
      const ratio = (upEvent.clientY - rect.top) / Math.max(1, rect.height);
      if (ratio < 0.28 || ratio > 0.72) {
        const targetParentId = targetNode.parentId;
        const validation = nodeStore.canMoveTo(node.id, targetParentId);
        if (!validation.ok) {
          nodeStore.setStatus(validation.reason ?? 'Invalid drop target');
          return;
        }
        const targetSiblings = nodeStore.getChildren(targetParentId);
        const targetIndex = targetSiblings.findIndex((item) => item.id === targetId);
        const nextIndex = ratio < 0.28 ? targetIndex : targetIndex + 1;
        if (node.parentId !== targetParentId) {
          queueCrossParentMove(node.id, targetParentId, nextIndex);
          return;
        }
        await nodeStore.moveToParentAt(node.id, targetParentId, nextIndex);
        return;
      }

      const validation = nodeStore.canMoveTo(node.id, targetId);
      if (!validation.ok) {
        nodeStore.setStatus(validation.reason ?? 'Invalid drop target');
        return;
      }
      queueCrossParentMove(node.id, targetId);
    };

    window.addEventListener('pointermove', handleMove, { passive: false });
    window.addEventListener('pointerup', handleUp);
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div
  class="tree-node"
  class:selected={isSelected}
  class:completed={node.isCompleted}
  class:editing={isEditing}
  class:workspace={kind === 'workspace'}
  class:group={kind === 'group'}
  class:subgroup={kind === 'subgroup'}
  class:task={kind === 'task'}
  class:subtask={kind === 'subtask'}
  class:note={kind === 'note'}
  class:drop-before={dropState === 'before'}
  class:drop-after={dropState === 'after'}
  class:drop-inside={dropState === 'inside'}
  class:drop-invalid={dropState === 'invalid'}
  style={`${flat ? '' : `padding-left: ${depth * INDENT + 8}px;`}${
    node.properties.color ? `--node-color: ${node.properties.color};` : ''
  }`}
  role="treeitem"
  aria-expanded={hasChildren ? isExpanded : undefined}
  aria-level={depth + 1}
  aria-selected={isSelected}
  tabindex={isSelected ? 0 : -1}
  data-tree-node-id={node.id}
  draggable={!isEditing}
  onclick={() => {
    if (suppressNextClick) {
      suppressNextClick = false;
      return;
    }
    showMenu = false;
    nodeStore.select(node.id);
  }}
  onpointerdown={handlePointerDown}
  onkeydown={handleRowKeydown}
  ondblclick={() => nodeStore.startEditing(node.id)}
  oncontextmenu={handleContextMenu}
  ondragstart={handleDragStart}
  ondragover={handleDragOver}
  ondragleave={clearDropState}
  ondrop={handleDrop}
  ondragend={handleDragEnd}
>
  {#if !flat}
    {#if depth > 0}
      <span class="guide" style="left: {depth * INDENT + 1}px"></span>
    {/if}
    {#if hasChildren}
      <button
        class="toggle"
        onclick={(e) => {
          e.stopPropagation();
          nodeStore.toggleExpand(node.id);
        }}
        aria-label={isExpanded ? 'Collapse' : 'Expand'}
      >
        {isExpanded ? '▼' : '▶'}
      </button>
    {:else}
      <span class="toggle-placeholder"></span>
    {/if}
  {/if}

  {#if node.nodeType === 'Task'}
    <button
      class="checkbox"
      class:done={node.isCompleted}
      onclick={(e) => {
        e.stopPropagation();
        nodeStore.toggleComplete(node.id);
      }}
      aria-label={node.isCompleted ? 'Mark incomplete' : 'Mark complete'}
    >
      {node.isCompleted ? '✓' : ''}
    </button>
  {:else}
    <span
      class="type-dot {kind}"
      style={node.properties.color ? `--node-color: ${node.properties.color}` : ''}
      aria-hidden="true"
    ></span>
  {/if}

  {#if !flat && depth > 0}
    <span class="branch" aria-hidden="true">└──</span>
  {/if}

  <span
    class="type-chip {kind}"
    style={node.properties.color ? `--node-color: ${node.properties.color}` : ''}>{kindLabel}</span
  >

  {#if node.properties.priority && node.properties.priority < 4}
    <span class="priority p{node.properties.priority}" title="P{node.properties.priority}"></span>
  {/if}

  {#if isEditing}
    <input
      class="title-input"
      bind:this={inputEl}
      bind:value={editTitle}
      onkeydown={handleEditKeydown}
      onblur={commitEdit}
      placeholder="Untitled"
    />
  {:else}
    <span class="title" class:strikethrough={node.isCompleted}>
      {node.title || 'Untitled'}
    </span>
  {/if}

  <span class="meta">
    {#if childSummary}
      <span class="count">{childSummary}</span>
    {/if}
    {#if node.properties.dueDate}
      <span class="due" class:overdue={isOverdue(node.properties.dueDate)}>
        {formatDue(node.properties.dueDate)}
      </span>
    {/if}
    {#if node.nodeType === 'Task' && tags.length > 0}
      {#each tags.slice(0, 2) as tag (tag.id)}
        <span class="tag" style={tag.color ? `--tag-color: ${tag.color}` : ''}>{tag.name}</span>
      {/each}
    {/if}
  </span>
</div>

{#if pendingMove}
  <button
    type="button"
    class="confirm-backdrop"
    aria-label="Cancel move"
    onclick={() => {
      pendingMove = null;
    }}
  ></button>
  <div
    class="move-confirm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="move-confirm-title-{node.id}"
  >
    <h3 id="move-confirm-title-{node.id}">{pendingMoveTitle(pendingMove)}</h3>
    <p>{pendingMoveDetail(pendingMove)}</p>
    <div class="confirm-actions">
      <button
        type="button"
        class="secondary"
        onclick={() => {
          pendingMove = null;
        }}
      >
        Cancel
      </button>
      <button type="button" class="primary" onclick={confirmPendingMove}>Move</button>
    </div>
  </div>
{/if}

<!-- CONTEXT MENU -->
{#if showMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="menu-backdrop"
    onclick={closeMenu}
    oncontextmenu={(e) => {
      e.preventDefault();
      closeMenu();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        closeMenu();
      }
    }}
  ></div>
  <div class="context-menu" style="position: fixed; left: {menuX}px; top: {menuY}px;">
    <div class="menu-title">{kindLabel}</div>

    <div class="menu-section">
      <div class="menu-section-label">Create</div>
      {#if kind === 'workspace'}
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.createGroupUnder(node.id);
            closeMenu();
          }}
        >
          New Group
        </button>
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.createTaskUnder(node.id);
            closeMenu();
          }}
        >
          New Task
        </button>
      {:else if kind === 'group' || kind === 'subgroup'}
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.createSubgroupUnder(node.id);
            closeMenu();
          }}
        >
          New Subgroup
        </button>
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.createTaskUnder(node.id);
            closeMenu();
          }}
        >
          New Task
        </button>
      {:else if kind === 'task' || kind === 'subtask'}
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.createSubtaskUnder(node.id);
            closeMenu();
          }}
        >
          New Subtask
        </button>
      {/if}
    </div>

    {#if kind === 'task' || kind === 'subtask'}
      <div class="menu-section">
        <div class="menu-section-label">Scheduling</div>
        <button
          class="menu-item"
          onclick={() => {
            uiStore.openDueDatePrompt();
            closeMenu();
          }}
        >
          Set Due Date...
        </button>
        <div class="priority-menu" aria-label="Set priority">
          {#each [1, 2, 3, 4] as p}
            <button
              class="priority-choice p{p}"
              class:active={node.properties.priority === p}
              title="Priority P{p}"
              onclick={() => {
                nodeStore.setPriority(node.id, p);
                closeMenu();
              }}
            >
              P{p}
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="menu-section">
      <div class="menu-section-label">Hierarchy</div>
      {#if hasChildren}
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.toggleExpand(node.id);
            closeMenu();
          }}
        >
          {isExpanded ? 'Collapse' : 'Expand'}
        </button>
      {/if}
      {#if kind !== 'workspace'}
        <button
          class="menu-item"
          onclick={() => {
            uiStore.openMoveDialog(node.id);
            closeMenu();
          }}
        >
          Move...
        </button>
      {/if}
      <button
        class="menu-item"
        disabled={siblingIndex <= 0}
        onclick={() => {
          nodeStore.moveSibling(node.id, -1);
          closeMenu();
        }}
      >
        Move up
      </button>
      <button
        class="menu-item"
        disabled={siblingIndex < 0 || siblingIndex >= siblings.length - 1}
        onclick={() => {
          nodeStore.moveSibling(node.id, 1);
          closeMenu();
        }}
      >
        Move down
      </button>
      <button
        class="menu-item"
        onclick={() => {
          nodeStore.duplicate(node.id);
          closeMenu();
        }}
      >
        Duplicate
      </button>
    </div>

    <div class="menu-section">
      <div class="menu-section-label">Organization</div>
      <button
        class="menu-item"
        onclick={() => {
          nodeStore.setPinned(node.id, !node.properties.pinned);
          closeMenu();
        }}
      >
        {node.properties.pinned ? 'Unpin' : 'Pin'}
      </button>
      {#if kind === 'workspace' || kind === 'group' || kind === 'subgroup'}
        <button
          class="menu-item"
          onclick={() => {
            uiStore.openConfigDialog(node.id);
            closeMenu();
          }}
        >
          Configure {kindLabel}
        </button>
      {/if}
      <div class="menu-section-label">Change color</div>
      <div class="color-menu" aria-label="Change color">
        {#each colorCycle as color}
          <button
            class="color-choice"
            class:active={node.properties.color === color}
            style={`--choice-color: ${color}`}
            title={`Use ${color}`}
            onclick={() => {
              nodeStore.setColor(node.id, color);
              closeMenu();
            }}
          ></button>
        {/each}
        <button
          class="color-default"
          title="Default color"
          onclick={() => {
            nodeStore.setColor(node.id, null);
            closeMenu();
          }}
        >
          Default
        </button>
      </div>
      {#if kind === 'task' || kind === 'subtask'}
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.setWontDo(node.id, !nodeStore.isWontDo(node.id));
            closeMenu();
          }}
        >
          {nodeStore.isWontDo(node.id) ? "Clear Won't Do" : "Mark Won't Do"}
        </button>
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.select(node.id);
            closeMenu();
          }}
        >
          Add Tags
        </button>
      {/if}
    </div>

    {#if kind === 'task' || kind === 'subtask'}
      <div class="menu-section">
        <div class="menu-section-label">Task</div>
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.toggleComplete(node.id);
            closeMenu();
          }}
        >
          {node.isCompleted ? 'Reopen' : 'Complete'}
        </button>
        <button
          class="menu-item"
          onclick={() => {
            focusStore.requestTaskFocus(node.id);
            uiStore.setActiveModule('productivity');
            closeMenu();
          }}
        >
          Start Focus Session
        </button>
      </div>
    {/if}

    <div class="menu-section">
      <div class="menu-section-label">Utilities</div>
      <button
        class="menu-item"
        onclick={() => {
          nodeStore.startEditing(node.id);
          closeMenu();
        }}
      >
        Rename
      </button>
      <button
        class="menu-item"
        onclick={() => {
          copyInternalLink();
          closeMenu();
        }}
      >
        Copy Internal Link
      </button>
      {#if kind === 'task' || kind === 'subtask'}
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.convertTaskToNote(node.id);
            closeMenu();
          }}
        >
          Convert to Note
        </button>
      {/if}
    </div>

    <div class="menu-section destructive">
      <button
        class="menu-item danger"
        onclick={() => {
          nodeStore.deleteSelected();
          closeMenu();
        }}
      >
        Delete
      </button>
    </div>
  </div>
{/if}

<style>
  .tree-node {
    display: flex;
    align-items: center;
    height: var(--tree-row-height);
    padding-right: 12px;
    cursor: pointer;
    border-left: 3px solid transparent;
    font-size: var(--text-sm);
    user-select: none;
    -webkit-user-select: none;
    -webkit-user-drag: element;
    min-width: 0;
    gap: 4px;
    position: relative;
  }
  .tree-node * {
    user-select: none;
    -webkit-user-select: none;
  }
  :global(body.tree-pointer-dragging) {
    cursor: grabbing;
    user-select: none;
    -webkit-user-select: none;
  }
  .tree-node:hover {
    background: var(--bg-hover);
  }
  .tree-node[draggable='true'] {
    cursor: grab;
  }
  .tree-node[draggable='true']:active {
    cursor: grabbing;
  }
  .tree-node.selected {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .tree-node.workspace {
    border-left-color: color-mix(in srgb, var(--node-color, var(--accent)) 24%, transparent);
  }
  .tree-node.group {
    border-left-color: color-mix(in srgb, var(--node-color, var(--info)) 24%, transparent);
  }
  .tree-node.subgroup {
    border-left-color: color-mix(in srgb, var(--node-color, #0ea5e9) 24%, transparent);
  }
  .tree-node.task {
    border-left-color: color-mix(in srgb, var(--node-color, var(--success)) 22%, transparent);
  }
  .tree-node.subtask {
    border-left-color: color-mix(in srgb, var(--node-color, var(--warning)) 22%, transparent);
  }
  .tree-node.note {
    border-left-color: rgba(139, 92, 246, 0.18);
  }
  .tree-node.selected.workspace,
  .tree-node.selected.group,
  .tree-node.selected.task,
  .tree-node.selected.subtask,
  .tree-node.selected.note {
    border-left-color: var(--accent);
  }
  .tree-node.editing {
    background: var(--bg-active);
  }
  .tree-node.drop-before::before,
  .tree-node.drop-after::after,
  .tree-node.pointer-drop-before::before,
  .tree-node.pointer-drop-after::after {
    content: '';
    position: absolute;
    left: 8px;
    right: 8px;
    height: 2px;
    background: var(--accent);
    border-radius: 999px;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 18%, transparent);
    z-index: 2;
  }
  .tree-node.drop-before::before,
  .tree-node.pointer-drop-before::before {
    top: 0;
  }
  .tree-node.drop-after::after,
  .tree-node.pointer-drop-after::after {
    bottom: 0;
  }
  .tree-node.drop-inside,
  .tree-node.pointer-drop-inside {
    background: color-mix(in srgb, var(--accent) 14%, var(--bg-active));
    outline: 1px solid var(--accent);
    outline-offset: -2px;
  }
  .tree-node.drop-invalid,
  .tree-node.pointer-drop-invalid {
    cursor: not-allowed;
    background: color-mix(in srgb, var(--danger) 12%, var(--bg-hover));
    outline: 1px dashed var(--danger);
    outline-offset: -2px;
  }

  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
    border: 0;
    padding: 0;
    background: color-mix(in srgb, var(--bg-base) 35%, transparent);
  }
  .move-confirm {
    position: fixed;
    left: 50%;
    top: 50%;
    z-index: 151;
    width: min(420px, calc(100vw - 32px));
    transform: translate(-50%, -50%);
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-panel);
    box-shadow: var(--shadow-lg);
  }
  .move-confirm h3 {
    margin: 0 0 8px;
    color: var(--text-primary);
    font-size: var(--text-lg);
  }
  .move-confirm p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: 1.45;
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
  .confirm-actions button {
    min-width: 76px;
    min-height: 34px;
    border-radius: 6px;
    border: 1px solid var(--border);
    cursor: pointer;
    font-weight: 700;
  }
  .confirm-actions .secondary {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }
  .confirm-actions .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }

  .guide {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--border);
    pointer-events: none;
  }

  .toggle,
  .checkbox {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: none;
    cursor: pointer;
    flex-shrink: 0;
    padding: 0;
    color: var(--text-tertiary);
    border-radius: 4px;
  }
  .toggle {
    font-size: 8px;
  }
  .toggle:hover,
  .checkbox:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .toggle-placeholder {
    width: 20px;
    flex-shrink: 0;
  }
  .checkbox {
    border: 1.5px solid var(--text-tertiary);
    border-radius: 50%;
    width: 16px;
    height: 16px;
    margin: 0 2px;
    font-size: 10px;
  }
  .checkbox.done {
    background: var(--success);
    border-color: var(--success);
    color: white;
  }
  .type-dot {
    width: 12px;
    height: 12px;
    border-radius: 4px;
    flex-shrink: 0;
  }
  .type-dot.workspace {
    background: var(--node-color, var(--accent));
  }
  .type-dot.group {
    background: var(--node-color, var(--info));
  }
  .type-dot.subgroup {
    background: var(--node-color, #0ea5e9);
  }
  .type-dot.note {
    background: #8b5cf6;
  }
  .type-dot.other {
    background: var(--text-tertiary);
  }
  .type-chip {
    flex-shrink: 0;
    min-width: 48px;
    text-align: center;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    color: var(--text-secondary);
    background: var(--bg-hover);
  }
  .type-chip.workspace {
    color: var(--node-color, var(--accent));
    background: color-mix(in srgb, var(--node-color, var(--accent)) 12%, transparent);
  }
  .type-chip.group {
    color: var(--node-color, var(--info));
    background: color-mix(in srgb, var(--node-color, var(--info)) 12%, transparent);
  }
  .type-chip.subgroup {
    color: var(--node-color, #0ea5e9);
    background: color-mix(in srgb, var(--node-color, #0ea5e9) 13%, transparent);
  }
  .type-chip.task {
    color: var(--node-color, var(--success));
    background: color-mix(in srgb, var(--node-color, var(--success)) 12%, transparent);
  }
  .type-chip.subtask {
    color: var(--node-color, var(--warning));
    background: color-mix(in srgb, var(--node-color, var(--warning)) 14%, transparent);
  }
  .type-chip.note {
    color: #8b5cf6;
    background: rgba(139, 92, 246, 0.12);
  }
  .branch {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: 11px;
    margin: 0 2px 0 -2px;
    flex-shrink: 0;
  }
  .priority {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .priority.p1 {
    background: var(--priority-p1);
  }
  .priority.p2 {
    background: var(--priority-p2);
  }
  .priority.p3 {
    background: var(--priority-p3);
  }
  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary);
  }
  .title.strikethrough {
    text-decoration: line-through;
    opacity: 0.5;
  }
  .title-input {
    flex: 1;
    border: none;
    background: var(--bg-input);
    font: inherit;
    outline: 2px solid var(--accent);
    padding: 2px 6px;
    height: 26px;
    border-radius: 4px;
    color: var(--text-primary);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
    margin-left: 8px;
  }
  .count {
    font-size: 10px;
    color: var(--text-tertiary);
    background: var(--bg-hover);
    padding: 1px 6px;
    border-radius: 8px;
    white-space: nowrap;
  }
  .due {
    font-size: 11px;
    color: var(--text-tertiary);
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--bg-hover);
  }
  .due.overdue {
    color: var(--danger);
    background: rgba(239, 68, 68, 0.12);
    font-weight: 600;
  }
  .tag {
    max-width: 88px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 10px;
    color: var(--tag-color, var(--accent));
    background: color-mix(in srgb, var(--tag-color, var(--accent)) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--tag-color, var(--accent)) 28%, transparent);
    padding: 1px 6px;
    border-radius: 8px;
  }

  /* CONTEXT MENU */
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 900;
  }
  .context-menu {
    z-index: 901;
    background: var(--bg-modal);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
    padding: 6px;
    min-width: 240px;
    max-width: 280px;
    font-size: var(--text-sm);
  }
  .menu-title {
    padding: 7px 10px 6px;
    color: var(--text-primary);
    font-weight: 800;
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .menu-section {
    padding: 4px 0;
    border-top: 1px solid var(--border);
  }
  .menu-section:first-of-type {
    border-top: none;
  }
  .menu-section-label {
    padding: 5px 10px 3px;
    color: var(--text-tertiary);
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .menu-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px 12px;
    border: none;
    background: none;
    color: var(--text-primary);
    cursor: pointer;
    border-radius: 4px;
    font-size: var(--text-sm);
  }
  .menu-item:hover {
    background: var(--bg-hover);
  }
  .menu-item.disabled,
  .menu-item:disabled {
    cursor: not-allowed;
    color: var(--text-tertiary);
    opacity: 0.65;
  }
  .menu-item.disabled:hover,
  .menu-item:disabled:hover {
    background: none;
  }
  .menu-item.danger {
    color: var(--danger);
  }
  .menu-item.danger:hover {
    background: rgba(239, 68, 68, 0.1);
  }
  .menu-divider {
    border: none;
    border-top: 1px solid var(--border);
    margin: 4px 0;
  }
  .priority-menu {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    padding: 4px 8px 6px;
  }
  .priority-choice {
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 5px 0;
    background: var(--bg-input);
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 800;
    cursor: pointer;
  }
  .priority-choice:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .priority-choice.p1.active {
    background: var(--priority-p1);
    border-color: var(--priority-p1);
    color: white;
  }
  .priority-choice.p2.active {
    background: var(--priority-p2);
    border-color: var(--priority-p2);
    color: white;
  }
  .priority-choice.p3.active {
    background: var(--priority-p3);
    border-color: var(--priority-p3);
    color: black;
  }
  .priority-choice.p4.active {
    background: var(--text-tertiary);
    border-color: var(--text-tertiary);
    color: white;
  }
  .color-menu {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 5px;
    padding: 4px 8px 8px;
  }
  .color-choice {
    height: 24px;
    border-radius: 5px;
    border: 1px solid color-mix(in srgb, var(--choice-color) 70%, var(--border));
    background: var(--choice-color);
    cursor: pointer;
  }
  .color-choice.active {
    outline: 2px solid var(--text-primary);
    outline-offset: 1px;
  }
  .color-default {
    grid-column: span 2;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-input);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 700;
  }
</style>
