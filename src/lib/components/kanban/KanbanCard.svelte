<script lang="ts">
  import { kanbanStore } from '$lib/stores/kanban.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore, type KanbanMode } from '$lib/stores/ui.svelte';
  import type { TaskProjectionItem } from '$lib/utils/task-projection';

  let { item, mode, columnKey }: { item: TaskProjectionItem; mode: KanbanMode; columnKey: string } =
    $props();
  let dragging = $state(false);
  let press: {
    pointerId: number;
    startX: number;
    startY: number;
  } | null = null;

  function select() {
    nodeStore.select(item.node.id);
    uiStore.setInspectorVisible(true);
  }

  function begin(event: PointerEvent) {
    if (
      event.button !== 0 ||
      (event.target instanceof HTMLElement && event.target.closest('button'))
    )
      return;
    press = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    if (!press || press.pointerId !== event.pointerId) return;
    if (Math.hypot(event.clientX - press.startX, event.clientY - press.startY) > 5) {
      dragging = true;
      document.body.classList.add('calendar-is-dragging');
      event.preventDefault();
    }
  }

  async function end(event: PointerEvent) {
    if (!press || press.pointerId !== event.pointerId) return;
    const wasDragging = dragging;
    press = null;
    dragging = false;
    document.body.classList.remove('calendar-is-dragging');
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {
      // WebView may release capture during pointer cancellation.
    }
    if (!wasDragging) return;
    event.preventDefault();
    const target = document.elementFromPoint(event.clientX, event.clientY);
    const column =
      target instanceof HTMLElement ? target.closest<HTMLElement>('[data-kanban-column]') : null;
    if (column?.dataset.kanbanColumn) {
      await kanbanStore.moveTask(item.node, mode, column.dataset.kanbanColumn, columnKey);
    }
  }
</script>

<div
  class="kanban-card"
  class:dragging
  role="button"
  tabindex="0"
  onpointerdown={begin}
  onpointermove={move}
  onpointerup={end}
  onpointercancel={() => {
    dragging = false;
    press = null;
    document.body.classList.remove('calendar-is-dragging');
  }}
  onclick={() => {
    if (!dragging) select();
  }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') select();
  }}
>
  <div class="card-top">
    <button
      type="button"
      class="check"
      class:done={item.node.isCompleted}
      aria-label={item.node.isCompleted ? 'Reopen task' : 'Complete task'}
      onclick={(e) => {
        e.stopPropagation();
        nodeStore.toggleComplete(item.node.id);
      }}
    >
      {item.node.isCompleted ? '✓' : ''}
    </button>
    <strong>{item.node.title || 'Untitled'}</strong>
    <span>{item.label}</span>
  </div>
  <div class="meta">
    {item.path
      .slice(0, -1)
      .map((node) => node.title || 'Untitled')
      .join(' › ') || 'No path'}
  </div>
  <div class="chips">
    <span>P{item.node.properties.priority ?? 4}</span>
    {#if item.node.properties.dueDate}
      <span>{new Date(item.node.properties.dueDate).toLocaleDateString()}</span>
    {/if}
    {#each item.tags.slice(0, 3) as tag (tag.id)}
      <span>{tag.name}</span>
    {/each}
  </div>
</div>

<style>
  .kanban-card {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    padding: 8px;
    display: grid;
    gap: 6px;
    cursor: grab;
    user-select: none;
    touch-action: none;
  }
  .kanban-card.dragging {
    opacity: 0.7;
    outline: 2px solid var(--accent);
    cursor: grabbing;
  }
  .card-top {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 7px;
    align-items: center;
  }
  .card-top strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-top span,
  .meta,
  .chips span {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .check {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1px solid var(--text-tertiary);
    background: transparent;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 10px;
  }
  .check.done {
    background: var(--success);
    border-color: var(--success);
    color: white;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chips span {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 5px;
  }
</style>
