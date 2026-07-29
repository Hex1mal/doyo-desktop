<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import type { Node } from '$lib/types/node';

  type QuadrantKey =
    | 'urgent_important'
    | 'not_urgent_important'
    | 'urgent_not_important'
    | 'not_urgent_not_important';

  const QUADRANTS: Array<{ key: QuadrantKey; label: string; desc: string }> = [
    { key: 'urgent_important', label: 'I. Do First', desc: 'Urgent & Important' },
    { key: 'not_urgent_important', label: 'II. Schedule', desc: 'Not Urgent & Important' },
    { key: 'urgent_not_important', label: 'III. Delegate', desc: 'Urgent & Not Important' },
    { key: 'not_urgent_not_important', label: 'IV. Eliminate', desc: 'Not Urgent & Not Important' },
  ];

  let dragTaskId = $state<string | null>(null);
  let hoverQuadrant = $state<QuadrantKey | 'unclassified' | null>(null);
  let pointerDrag = $state<{
    taskId: string;
    pointerId: number;
    startX: number;
    startY: number;
  } | null>(null);
  let pointerDragging = $state(false);
  let suppressClick = $state(false);

  let allTasks = $derived(
    [...nodeStore.nodes.values()]
      .filter((n) => n.nodeType === 'Task' && !n.deletedAt && !n.isCompleted)
      .sort((a, b) => a.position - b.position),
  );

  function quadrantKey(node: Node): string | null {
    const custom = (node.properties.custom as Record<string, unknown> | undefined) ?? {};
    const q = custom.eisenhowerQuadrant;
    return typeof q === 'string' ? q : null;
  }

  function tasksFor(key: QuadrantKey | null): Node[] {
    return allTasks.filter((t) => quadrantKey(t) === key);
  }

  let unclassified = $derived(tasksFor(null));
  let q1 = $derived(tasksFor('urgent_important'));
  let q2 = $derived(tasksFor('not_urgent_important'));
  let q3 = $derived(tasksFor('urgent_not_important'));
  let q4 = $derived(tasksFor('not_urgent_not_important'));

  function handleDragStart(e: DragEvent, taskId: string) {
    dragTaskId = taskId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', taskId);
    }
  }

  function handleDragOver(e: DragEvent, zone: QuadrantKey | 'unclassified') {
    e.preventDefault();
    hoverQuadrant = zone;
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  }

  function handleDragLeave() {
    hoverQuadrant = null;
  }

  async function handleDrop(e: DragEvent, quadrant: string | null) {
    e.preventDefault();
    hoverQuadrant = null;
    if (!dragTaskId) return;
    const taskId = dragTaskId;
    dragTaskId = null;
    await moveTaskToQuadrant(taskId, quadrant);
  }

  function beginPointerDrag(e: PointerEvent, taskId: string) {
    if (e.button !== 0) return;
    pointerDrag = { taskId, pointerId: e.pointerId, startX: e.clientX, startY: e.clientY };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function updatePointerDrag(e: PointerEvent) {
    if (!pointerDrag || pointerDrag.pointerId !== e.pointerId) return;
    if (Math.hypot(e.clientX - pointerDrag.startX, e.clientY - pointerDrag.startY) > 5) {
      pointerDragging = true;
      document.body.classList.add('calendar-is-dragging');
      e.preventDefault();
      const target = document.elementFromPoint(e.clientX, e.clientY);
      const drop =
        target instanceof HTMLElement ? target.closest<HTMLElement>('[data-matrix-drop]') : null;
      hoverQuadrant =
        (drop?.dataset.matrixDrop as QuadrantKey | 'unclassified' | undefined) ?? null;
    }
  }

  async function endPointerDrag(e: PointerEvent) {
    if (!pointerDrag || pointerDrag.pointerId !== e.pointerId) return;
    const active = pointerDrag;
    const wasDragging = pointerDragging;
    pointerDrag = null;
    pointerDragging = false;
    document.body.classList.remove('calendar-is-dragging');
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Pointer capture may already be released by the WebView.
    }
    if (!wasDragging) return;
    e.preventDefault();
    suppressClick = true;
    setTimeout(() => (suppressClick = false), 0);
    const target = document.elementFromPoint(e.clientX, e.clientY);
    const drop =
      target instanceof HTMLElement ? target.closest<HTMLElement>('[data-matrix-drop]') : null;
    const zone = drop?.dataset.matrixDrop;
    hoverQuadrant = null;
    if (!zone) return;
    await moveTaskToQuadrant(active.taskId, zone === 'unclassified' ? null : zone);
  }

  function cancelPointerDrag() {
    pointerDrag = null;
    pointerDragging = false;
    hoverQuadrant = null;
    document.body.classList.remove('calendar-is-dragging');
  }

  async function moveTaskToQuadrant(taskId: string, quadrant: string | null) {
    await nodeStore.setTaskCustom(taskId, { eisenhowerQuadrant: quadrant });
    toast.info(
      `Task moved to ${quadrant ? (QUADRANTS.find((q) => q.key === quadrant)?.label ?? quadrant) : 'Unclassified'}`,
    );
  }

  function priorityBadge(p: number | undefined) {
    if (!p || p <= 0) return '';
    return `P${p}`;
  }

  function dueLabel(due: string | undefined | null) {
    if (!due) return '';
    const d = new Date(due);
    const now = new Date();
    const diff = Math.ceil((d.getTime() - now.getTime()) / 86400000);
    if (diff < 0) return `${Math.abs(diff)}d overdue`;
    if (diff === 0) return 'Today';
    if (diff === 1) return 'Tomorrow';
    return `${diff}d`;
  }

  function clearAll() {
    if (!window.confirm('Remove all matrix classifications? Tasks are not deleted.')) return;
    for (const task of allTasks) {
      if (quadrantKey(task)) {
        nodeStore.setTaskCustom(task.id, { eisenhowerQuadrant: null });
      }
    }
    toast.info('All matrix classifications cleared');
  }
</script>

<section class="matrix-view" aria-label="Eisenhower Matrix">
  <div class="matrix-toolbar">
    <h2>Eisenhower Matrix</h2>
    <p>Drag tasks between quadrants. No tasks are duplicated — each task lives in one quadrant.</p>
    <button onclick={clearAll} class="clear-btn">Clear all classifications</button>
  </div>

  <div class="grid-2x2">
    {#each QUADRANTS as quad (quad.key)}
      {@const tasks =
        quad.key === 'urgent_important'
          ? q1
          : quad.key === 'not_urgent_important'
            ? q2
            : quad.key === 'urgent_not_important'
              ? q3
              : q4}
      <div
        class="quadrant"
        class:hovered={hoverQuadrant === quad.key}
        data-matrix-drop={quad.key}
        role="region"
        aria-label={`Eisenhower ${quad.label} quadrant`}
        ondragover={(e) => handleDragOver(e, quad.key)}
        ondragleave={handleDragLeave}
        ondrop={(e) => handleDrop(e, quad.key)}
      >
        <div class="quad-header">
          <strong>{quad.label}</strong>
          <span>{quad.desc}</span>
          <span class="count">{tasks.length}</span>
        </div>
        <div class="quad-tasks">
          {#each tasks as task (task.id)}
            <div
              class="task-card"
              role="button"
              tabindex="0"
              draggable="true"
              ondragstart={(e) => handleDragStart(e, task.id)}
              onpointerdown={(e) => beginPointerDrag(e, task.id)}
              onpointermove={updatePointerDrag}
              onpointerup={endPointerDrag}
              onpointercancel={cancelPointerDrag}
              onclick={() => {
                if (!suppressClick) {
                  nodeStore.select(task.id);
                  nodeStore.setViewMode('tree');
                }
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  nodeStore.select(task.id);
                  nodeStore.setViewMode('tree');
                  e.preventDefault();
                }
              }}
            >
              <span class="task-title">{task.title || 'Untitled'}</span>
              <div class="task-meta">
                {#if task.properties.priority}<span class="priority"
                    >{priorityBadge(task.properties.priority)}</span
                  >{/if}
                {#if task.properties.dueDate}<span class="due"
                    >{dueLabel(task.properties.dueDate)}</span
                  >{/if}
              </div>
            </div>
          {/each}
          {#if tasks.length === 0}
            <div class="drop-hint">Drop tasks here</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <div
    class="unclassified"
    class:hovered={hoverQuadrant === 'unclassified'}
    data-matrix-drop="unclassified"
    role="region"
    aria-label="Unclassified tasks"
    ondragover={(e) => handleDragOver(e, 'unclassified')}
    ondragleave={handleDragLeave}
    ondrop={(e) => handleDrop(e, null)}
  >
    <div class="quad-header">
      <strong>Unclassified</strong>
      <span>No quadrant assigned</span>
      <span class="count">{unclassified.length}</span>
    </div>
    <div class="quad-tasks horizontal">
      {#each unclassified as task (task.id)}
        <div
          class="task-card"
          role="button"
          tabindex="0"
          draggable="true"
          ondragstart={(e) => handleDragStart(e, task.id)}
          onpointerdown={(e) => beginPointerDrag(e, task.id)}
          onpointermove={updatePointerDrag}
          onpointerup={endPointerDrag}
          onpointercancel={cancelPointerDrag}
          onclick={() => {
            if (!suppressClick) {
              nodeStore.select(task.id);
              nodeStore.setViewMode('tree');
            }
          }}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              nodeStore.select(task.id);
              nodeStore.setViewMode('tree');
              e.preventDefault();
            }
          }}
        >
          <span class="task-title">{task.title || 'Untitled'}</span>
          <div class="task-meta">
            {#if task.properties.priority}<span class="priority"
                >{priorityBadge(task.properties.priority)}</span
              >{/if}
            {#if task.properties.dueDate}<span class="due">{dueLabel(task.properties.dueDate)}</span
              >{/if}
          </div>
        </div>
      {/each}
      {#if unclassified.length === 0}
        <div class="drop-hint">All tasks are classified</div>
      {/if}
    </div>
  </div>
</section>

<style>
  .matrix-view {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--bg-app);
    padding: 14px 18px;
  }
  .matrix-toolbar {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 14px;
  }
  .matrix-toolbar h2 {
    margin: 0;
  }
  .matrix-toolbar p {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }
  .clear-btn {
    margin-left: auto;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 6px 12px;
    font-size: var(--text-sm);
  }
  .clear-btn:hover {
    border-color: var(--danger);
    color: var(--danger);
  }
  .grid-2x2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 10px;
    min-height: 400px;
  }
  .quadrant {
    border: 2px solid var(--border);
    border-radius: 10px;
    padding: 10px;
    background: var(--bg-panel);
    display: flex;
    flex-direction: column;
    min-height: 180px;
    transition:
      border-color 0.15s,
      background 0.15s;
  }
  .quadrant.hovered {
    border-color: var(--accent);
    background: var(--accent-subtle, rgba(37, 99, 235, 0.05));
  }
  .unclassified {
    border: 2px solid var(--border);
    border-radius: 10px;
    padding: 10px;
    background: var(--bg-panel);
    margin-top: 10px;
    transition:
      border-color 0.15s,
      background 0.15s;
  }
  .unclassified.hovered {
    border-color: var(--accent);
    background: var(--accent-subtle, rgba(37, 99, 235, 0.05));
  }
  .quad-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border);
  }
  .quad-header strong {
    font-size: var(--text-sm);
    white-space: nowrap;
  }
  .quad-header span {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .quad-header .count {
    margin-left: auto;
    background: var(--bg-active);
    padding: 1px 7px;
    border-radius: 8px;
    font-size: 11px;
  }
  .quad-tasks {
    flex: 1;
    overflow-y: auto;
    display: grid;
    gap: 5px;
    align-content: start;
    min-height: 40px;
  }
  .quad-tasks.horizontal {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .task-card {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 9px;
    background: var(--bg-input);
    cursor: grab;
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .task-card:hover {
    border-color: var(--accent);
  }
  .task-card:active {
    cursor: grabbing;
  }
  .task-title {
    font-size: var(--text-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .task-meta {
    display: flex;
    gap: 6px;
  }
  .priority {
    font-size: 10px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--accent);
    color: white;
    font-weight: 700;
  }
  .due {
    font-size: 10px;
    color: var(--text-tertiary);
  }
  .drop-hint {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    text-align: center;
    padding: 12px;
  }
  @media (max-width: 800px) {
    .grid-2x2 {
      grid-template-columns: 1fr;
      grid-template-rows: auto;
    }
  }
</style>
