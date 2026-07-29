<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { timelineStore } from '$lib/stores/timeline.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import type { TaskProjectionItem } from '$lib/utils/task-projection';
  import { taskTimelineRange, timelineDayIndex } from '$lib/utils/timeline';

  let {
    item,
    days,
    dayWidth,
  }: {
    item: TaskProjectionItem;
    days: Date[];
    dayWidth: number;
  } = $props();

  let range = $derived(taskTimelineRange(item.node));
  let startIndex = $derived(range ? Math.max(0, timelineDayIndex(range.start, days)) : -1);
  let endIndex = $derived(range ? Math.max(startIndex, timelineDayIndex(range.end, days)) : -1);
  let visible = $derived(Boolean(range && startIndex >= 0 && endIndex >= 0));
  let left = $derived(startIndex * dayWidth);
  let width = $derived(Math.max(92, (endIndex - startIndex + 1) * dayWidth - 8));
  let dragging = $state(false);
  let press:
    | {
        pointerId: number;
        x: number;
        mode: 'move' | 'start' | 'end';
      }
    | null = null;

  function select() {
    nodeStore.select(item.node.id);
    uiStore.setInspectorVisible(true);
  }

  function begin(event: PointerEvent, mode: 'move' | 'start' | 'end') {
    if (event.button !== 0) return;
    event.stopPropagation();
    press = { pointerId: event.pointerId, x: event.clientX, mode };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    if (!press || press.pointerId !== event.pointerId) return;
    if (Math.abs(event.clientX - press.x) > 4) {
      dragging = true;
      document.body.classList.add('calendar-is-dragging');
      event.preventDefault();
    }
  }

  async function end(event: PointerEvent) {
    if (!press || press.pointerId !== event.pointerId) return;
    const active = press;
    press = null;
    const delta = Math.round((event.clientX - active.x) / dayWidth);
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {
      // Already released by WebView.
    }
    document.body.classList.remove('calendar-is-dragging');
    const wasDragging = dragging;
    dragging = false;
    if (!wasDragging || delta === 0) return;
    if (active.mode === 'move') await timelineStore.moveTask(item.node, delta);
    if (active.mode === 'start') await timelineStore.resizeStart(item.node, delta);
    if (active.mode === 'end') await timelineStore.resizeEnd(item.node, delta);
  }
</script>

{#if visible}
  <article class="timeline-item" class:dragging style={`left: ${left}px; width: ${width}px`}>
    <button
      class="handle start"
      aria-label="Resize start date"
      title="Resize start"
      onpointerdown={(e) => begin(e, 'start')}
      onpointermove={move}
      onpointerup={end}
    ></button>
    <div
      class="body"
      role="button"
      tabindex="0"
      onpointerdown={(e) => begin(e, 'move')}
      onpointermove={move}
      onpointerup={end}
      onclick={() => !dragging && select()}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') select();
      }}
    >
      <strong>{item.node.title || 'Untitled'}</strong>
      <span>{item.label} · P{item.node.properties.priority ?? 4}</span>
      <span>{item.path.slice(0, -1).map((node) => node.title || 'Untitled').join(' › ') || 'No path'}</span>
    </div>
    <button
      class="handle end"
      aria-label="Resize due date"
      title="Resize end"
      onpointerdown={(e) => begin(e, 'end')}
      onpointermove={move}
      onpointerup={end}
    ></button>
  </article>
{/if}

<style>
  .timeline-item {
    position: absolute;
    top: 7px;
    height: 58px;
    border: 1px solid rgba(16, 185, 129, 0.45);
    border-radius: 6px;
    background: rgba(16, 185, 129, 0.16);
    display: grid;
    grid-template-columns: 12px minmax(0, 1fr) 12px;
    overflow: hidden;
    user-select: none;
    touch-action: none;
  }
  .timeline-item.dragging {
    opacity: 0.72;
    outline: 2px solid var(--accent);
  }
  .body {
    min-width: 0;
    display: grid;
    align-content: center;
    gap: 2px;
    padding: 4px 6px;
    cursor: grab;
  }
  .body strong,
  .body span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .body strong {
    font-size: var(--text-sm);
  }
  .body span {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .handle {
    border: none;
    background: rgba(16, 185, 129, 0.22);
    cursor: ew-resize;
    padding: 0;
  }
  .handle:hover,
  .handle:focus-visible {
    background: var(--accent);
    outline: none;
  }
</style>
