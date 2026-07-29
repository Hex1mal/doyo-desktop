<script lang="ts">
  import { calendarStore } from '$lib/stores/calendar.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import type { Node, TimeBlock } from '$lib/types/node';
  import { itemDurationMinutes } from '$lib/utils/calendar';

  let {
    task,
    block,
    compact = false,
  }: {
    task?: Node;
    block?: TimeBlock;
    compact?: boolean;
  } = $props();

  let resizing = $state(false);
  let pointerDrag = $state(false);
  let suppressClick = false;
  let press: {
    pointerId: number;
    startX: number;
    startY: number;
    payload: { type: 'task' | 'block'; id: string };
  } | null = null;

  function selectTask(node: Node) {
    nodeStore.select(node.id);
    uiStore.setInspectorVisible(true);
  }

  function dragPayload() {
    if (task) return JSON.stringify({ type: 'task', id: task.id });
    if (block) return JSON.stringify({ type: 'block', id: block.id });
    return '';
  }

  function payload() {
    if (task) return { type: 'task' as const, id: task.id };
    if (block) return { type: 'block' as const, id: block.id };
    return null;
  }

  function isInteractiveTarget(target: EventTarget | null) {
    return (
      target instanceof HTMLElement && Boolean(target.closest('button, input, select, textarea, a'))
    );
  }

  function beginPointerDrag(event: PointerEvent) {
    if (event.button !== 0 || isInteractiveTarget(event.target)) return;
    const nextPayload = payload();
    if (!nextPayload) return;
    press = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      payload: nextPayload,
    };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function movePointerDrag(event: PointerEvent) {
    if (!press || press.pointerId !== event.pointerId) return;
    const distance = Math.hypot(event.clientX - press.startX, event.clientY - press.startY);
    if (!pointerDrag && distance > 5) {
      pointerDrag = true;
      suppressClick = true;
      calendarStore.beginDrag(press.payload);
    }
    if (pointerDrag) {
      event.preventDefault();
    }
  }

  async function endPointerDrag(event: PointerEvent) {
    if (!press || press.pointerId !== event.pointerId) return;
    const activePress = press;
    press = null;
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {
      // Pointer capture may already be released by WebView cancellation.
    }
    if (pointerDrag) {
      event.preventDefault();
      pointerDrag = false;
      await calendarStore.finishPointerDrop(activePress.payload, event.clientX, event.clientY);
      calendarStore.clearDrag();
      window.setTimeout(() => {
        suppressClick = false;
      }, 0);
    }
  }

  function cancelPointerDrag() {
    press = null;
    pointerDrag = false;
    calendarStore.clearDrag();
    window.setTimeout(() => {
      suppressClick = false;
    }, 0);
  }

  function beginResize(event: PointerEvent) {
    if (!block) return;
    event.stopPropagation();
    event.preventDefault();
    const target = event.currentTarget as HTMLElement;
    resizing = true;
    target.setPointerCapture(event.pointerId);
    const startY = event.clientY;
    const handleMove = (move: PointerEvent) => {
      if (move.pointerId !== event.pointerId) return;
      move.preventDefault();
    };
    const handleUp = async (up: PointerEvent) => {
      if (up.pointerId !== event.pointerId) return;
      const delta = Math.round((up.clientY - startY) / 24) * 30;
      resizing = false;
      target.removeEventListener('pointermove', handleMove);
      target.removeEventListener('pointercancel', handleCancel);
      window.removeEventListener('pointerup', handleUp);
      try {
        target.releasePointerCapture(event.pointerId);
      } catch {
        // Pointer capture may already be released.
      }
      if (delta !== 0) await calendarStore.resizeBlock(block.id, delta);
    };
    const handleCancel = () => {
      resizing = false;
      target.removeEventListener('pointermove', handleMove);
      target.removeEventListener('pointercancel', handleCancel);
      window.removeEventListener('pointerup', handleUp);
    };
    target.addEventListener('pointermove', handleMove);
    target.addEventListener('pointercancel', handleCancel);
    window.addEventListener('pointerup', handleUp);
  }

  function selectOnKey(event: KeyboardEvent, node: Node) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      selectTask(node);
    }
  }
</script>

{#if task}
  <div
    class="calendar-item task"
    class:compact
    class:pointer-drag={pointerDrag}
    role="button"
    tabindex="0"
    draggable="false"
    data-calendar-draggable="task"
    ondragstart={(e) => e.dataTransfer?.setData('application/doyo-calendar', dragPayload())}
    onpointerdown={beginPointerDrag}
    onpointermove={movePointerDrag}
    onpointerup={endPointerDrag}
    onpointercancel={cancelPointerDrag}
    onclick={(e) => {
      if (suppressClick) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }
      selectTask(task);
    }}
    onkeydown={(e) => selectOnKey(e, task)}
    title={nodeStore.getPath(task.id)}
  >
    <button
      type="button"
      class="check"
      class:done={task.isCompleted}
      aria-label={task.isCompleted ? 'Reopen task' : 'Complete task'}
      onclick={(e) => {
        e.stopPropagation();
        nodeStore.toggleComplete(task.id);
      }}
    >
      {task.isCompleted ? '✓' : ''}
    </button>
    <span class="item-title">{task.title || 'Untitled'}</span>
    {#if task.properties.dueDate}
      <span class="time"
        >{new Date(task.properties.dueDate).toLocaleTimeString([], {
          hour: '2-digit',
          minute: '2-digit',
        })}</span
      >
    {/if}
    {#if task.properties.priority && task.properties.priority < 4}
      <span class="priority">P{task.properties.priority}</span>
    {/if}
    {#each nodeStore.getTagObjects(task.id).slice(0, 2) as tag (tag.id)}
      <span class="tag">{tag.name}</span>
    {/each}
  </div>
{:else if block}
  <div
    class="calendar-item block"
    class:compact
    class:pointer-drag={pointerDrag}
    role="group"
    draggable="false"
    data-calendar-draggable="block"
    ondragstart={(e) => e.dataTransfer?.setData('application/doyo-calendar', dragPayload())}
    onpointerdown={beginPointerDrag}
    onpointermove={movePointerDrag}
    onpointerup={endPointerDrag}
    onpointercancel={cancelPointerDrag}
    title={block.notes}
  >
    <strong>{block.title || 'Planning block'}</strong>
    <span class="time">
      {new Date(block.startTime).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
      -
      {new Date(block.endTime).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
    </span>
    <span>{itemDurationMinutes(block)}m</span>
    <div class="block-actions">
      <button title="Link selected task" onclick={() => calendarStore.linkSelectedTask(block.id)}
        >Link</button
      >
      <button title="Unlink task" onclick={() => calendarStore.unlinkBlock(block.id)}>Unlink</button
      >
      <button title="Shorten by 30 minutes" onclick={() => calendarStore.resizeBlock(block.id, -30)}
        >-30</button
      >
      <button title="Extend by 30 minutes" onclick={() => calendarStore.resizeBlock(block.id, 30)}
        >+30</button
      >
      <button title="Delete block" onclick={() => calendarStore.deleteBlock(block.id)}
        >Delete</button
      >
    </div>
    <button
      class="resize-handle"
      class:resizing
      title="Drag to resize"
      aria-label="Drag to resize time block"
      onpointerdown={beginResize}>↕</button
    >
  </div>
{/if}

<style>
  .calendar-item {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-input);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 6px;
    font-size: 11px;
    text-align: left;
    cursor: pointer;
    position: relative;
    touch-action: none;
    user-select: none;
  }
  .calendar-item.pointer-drag {
    opacity: 0.72;
    outline: 2px solid var(--accent);
    z-index: 4;
  }
  .calendar-item.compact {
    padding: 2px 5px;
  }
  .calendar-item.task {
    background: rgba(16, 185, 129, 0.1);
  }
  .calendar-item.block {
    background: rgba(99, 102, 241, 0.12);
    flex-wrap: wrap;
  }
  .item-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .check {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid var(--text-tertiary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 9px;
  }
  .check.done {
    background: var(--success);
    border-color: var(--success);
    color: white;
  }
  .time,
  .priority,
  .tag {
    color: var(--text-tertiary);
    font-size: 10px;
    flex-shrink: 0;
  }
  .priority {
    color: var(--warning);
    font-weight: 800;
  }
  .tag {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 4px;
  }
  .block-actions {
    display: flex;
    gap: 4px;
    margin-left: auto;
    margin-right: 24px;
  }
  .block-actions button {
    border: 1px solid var(--border);
    background: var(--bg-panel);
    color: var(--text-tertiary);
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
  }
  .resize-handle {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    width: 20px;
    height: 24px;
    cursor: ns-resize;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-panel);
    color: var(--text-tertiary);
    font-size: 12px;
    line-height: 1;
    touch-action: none;
    z-index: 10;
  }
  .resize-handle:hover,
  .resize-handle.resizing {
    background: var(--accent);
    color: white;
  }
</style>
