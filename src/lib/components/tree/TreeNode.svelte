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

  function nextColor() {
    const current = node.properties.color;
    const index = current ? colorCycle.indexOf(current) : -1;
    return colorCycle[(index + 1) % colorCycle.length];
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
  style={flat ? '' : `padding-left: ${depth * INDENT + 8}px`}
  role="treeitem"
  aria-expanded={hasChildren ? isExpanded : undefined}
  aria-level={depth + 1}
  aria-selected={isSelected}
  tabindex={isSelected ? 0 : -1}
  onclick={() => {
    showMenu = false;
    nodeStore.select(node.id);
  }}
  onkeydown={handleRowKeydown}
  ondblclick={() => nodeStore.startEditing(node.id)}
  oncontextmenu={handleContextMenu}
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
    <span class="type-dot {kind}" aria-hidden="true"></span>
  {/if}

  {#if !flat && depth > 0}
    <span class="branch" aria-hidden="true">└──</span>
  {/if}

  <span class="type-chip {kind}">{kindLabel}</span>

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
        <button
          class="menu-item"
          onclick={() => {
            nodeStore.setColor(node.id, nextColor());
            closeMenu();
          }}
        >
          Change Color
        </button>
      {/if}
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
    min-width: 0;
    gap: 4px;
    position: relative;
  }
  .tree-node:hover {
    background: var(--bg-hover);
  }
  .tree-node.selected {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .tree-node.workspace {
    border-left-color: rgba(99, 102, 241, 0.18);
  }
  .tree-node.group {
    border-left-color: rgba(59, 130, 246, 0.18);
  }
  .tree-node.subgroup {
    border-left-color: rgba(14, 165, 233, 0.18);
  }
  .tree-node.task {
    border-left-color: rgba(16, 185, 129, 0.16);
  }
  .tree-node.subtask {
    border-left-color: rgba(245, 158, 11, 0.18);
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
    background: var(--accent);
  }
  .type-dot.group {
    background: var(--info);
  }
  .type-dot.subgroup {
    background: #0ea5e9;
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
    color: var(--accent);
    background: rgba(99, 102, 241, 0.12);
  }
  .type-chip.group {
    color: var(--info);
    background: rgba(59, 130, 246, 0.12);
  }
  .type-chip.subgroup {
    color: #0ea5e9;
    background: rgba(14, 165, 233, 0.13);
  }
  .type-chip.task {
    color: var(--success);
    background: rgba(16, 185, 129, 0.12);
  }
  .type-chip.subtask {
    color: var(--warning);
    background: rgba(245, 158, 11, 0.14);
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
</style>
