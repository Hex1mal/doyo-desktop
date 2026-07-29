<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import { nodeCreate } from '$lib/api/client';
  import type { Node } from '$lib/types/node';

  const GTD_STAGES = [
    { key: 'inbox', title: 'Capture', desc: 'Quick inbox items', icon: '↓' },
    { key: 'clarify', title: 'Clarify', desc: 'Decide outcome and next step', icon: '?' },
    { key: 'next', title: 'Next Action', desc: 'Do next', icon: '→' },
    { key: 'project', title: 'Project', desc: 'Move into a Workspace Group/Subgroup', icon: 'P' },
    { key: 'waiting', title: 'Waiting For', desc: 'Delegated or blocked', icon: '⏳' },
    { key: 'someday', title: 'Someday / Maybe', desc: 'Future consideration', icon: '💭' },
    { key: 'reference', title: 'Reference', desc: 'Reference material', icon: '📚' },
    { key: 'review', title: 'Review', desc: 'Weekly review queue', icon: 'R' },
    { key: 'engage', title: 'Engage', desc: 'Ready to work now', icon: '▶' },
  ];

  let dragTaskId = $state<string | null>(null);
  let hoverStage = $state<string | null>(null);
  let quickCaptureTitle = $state('');
  let reviewMode = $state(false);
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

  function gtdKey(node: Node): string {
    const custom = (node.properties.custom as Record<string, unknown> | undefined) ?? {};
    const state = custom.gtdState;
    if (typeof state === 'string' && GTD_STAGES.some((s) => s.key === state)) return state;
    return 'inbox'; // default: unprocessed goes to inbox
  }

  function tasksFor(key: string): Node[] {
    return allTasks.filter((t) => gtdKey(t) === key);
  }

  function handleDragStart(e: DragEvent, taskId: string) {
    dragTaskId = taskId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', taskId);
    }
  }

  function handleDragOver(e: DragEvent, stage: string) {
    e.preventDefault();
    hoverStage = stage;
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  }

  function handleDragLeave() {
    hoverStage = null;
  }

  async function handleDrop(e: DragEvent, gtdState: string) {
    e.preventDefault();
    hoverStage = null;
    if (!dragTaskId) return;
    const taskId = dragTaskId;
    dragTaskId = null;
    await moveTaskToStage(taskId, gtdState);
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
      const stage =
        target instanceof HTMLElement ? target.closest<HTMLElement>('[data-gtd-stage]') : null;
      hoverStage = stage?.dataset.gtdStage ?? null;
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
    const stage =
      target instanceof HTMLElement ? target.closest<HTMLElement>('[data-gtd-stage]') : null;
    const gtdState = stage?.dataset.gtdStage;
    hoverStage = null;
    if (!gtdState) return;
    await moveTaskToStage(active.taskId, gtdState);
  }

  function cancelPointerDrag() {
    pointerDrag = null;
    pointerDragging = false;
    hoverStage = null;
    document.body.classList.remove('calendar-is-dragging');
  }

  async function moveTaskToStage(taskId: string, gtdState: string) {
    await nodeStore.setTaskCustom(taskId, { gtdState });
    const stage = GTD_STAGES.find((s) => s.key === gtdState);
    toast.info(`Task moved to ${stage?.title ?? gtdState}`);
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
    return `${diff}d`;
  }

  async function quickCapture() {
    const title = quickCaptureTitle.trim();
    if (!title) return;
    // Create a root-level task (parent = null) for quick capture
    const roots = nodeStore.getRoots();
    const parentId = roots.length > 0 ? roots[0].id : null;
    try {
      const created = await nodeCreate(parentId, 'Task', title);
      nodeStore.upsert(created);
      if (parentId) nodeStore.expand(parentId);
      await nodeStore.setTaskCustom(created.id, { gtdState: 'inbox' });
      quickCaptureTitle = '';
      toast.success('Captured to GTD Inbox');
    } catch (e) {
      toast.error(`Capture failed: ${String(e)}`);
    }
  }

  let inboxCount = $derived(tasksFor('inbox').length);
  let nextCount = $derived(tasksFor('next').length);
  let totalClassified = $derived(allTasks.length);

  function unclassified(): Node[] {
    return allTasks.filter((t) => {
      const custom = (t.properties.custom as Record<string, unknown> | undefined) ?? {};
      const state = custom.gtdState;
      return typeof state !== 'string' || !GTD_STAGES.some((s) => s.key === state);
    });
  }

  let newTasks = $derived(unclassified());
</script>

<section class="gtd-view" aria-label="GTD Workflow">
  <div class="gtd-toolbar">
    <div>
      <h2>Getting Things Done — GTD</h2>
      <p>
        Inbox {inboxCount} · Next Actions {nextCount} · Total {totalClassified}
      </p>
    </div>
    <div class="toolbar-actions">
      <button class:active={reviewMode} onclick={() => (reviewMode = !reviewMode)}>
        {reviewMode ? 'Exit Review' : 'Weekly Review'}
      </button>
    </div>
  </div>

  {#if reviewMode}
    <div class="review-panel">
      <h3>Weekly Review</h3>
      <p>Review each stage and decide: keep, move forward, or archive.</p>
      <div class="review-stages">
        {#each GTD_STAGES as stage (stage.key)}
          {@const tasks = tasksFor(stage.key)}
          <div class="review-stage">
            <strong>{stage.title}</strong>
            <span>{tasks.length} tasks</span>
            {#each tasks.slice(0, 5) as task (task.id)}
              <div
                class="review-task"
                role="button"
                tabindex="0"
                onclick={() => {
                  nodeStore.select(task.id);
                  nodeStore.setViewMode('tree');
                }}
                onkeydown={(e) => {
                  if (e.key === 'Enter') {
                    nodeStore.select(task.id);
                    nodeStore.setViewMode('tree');
                  }
                }}
              >
                {task.title || 'Untitled'}
              </div>
            {/each}
            {#if tasks.length > 5}<span class="more">+{tasks.length - 5} more</span>{/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="capture-bar">
    <form
      class="capture-form"
      onsubmit={(e) => {
        e.preventDefault();
        quickCapture();
      }}
    >
      <input
        class="capture-input"
        placeholder="Quick capture — type an idea and press Enter"
        bind:value={quickCaptureTitle}
      />
      <button class="primary" type="submit" disabled={!quickCaptureTitle.trim()}>Capture</button>
    </form>
  </div>

  {#if newTasks.length > 0}
    <div class="new-tasks-bar">
      <strong>New (unprocessed)</strong>
      <span>{newTasks.length} tasks without GTD state</span>
      {#each newTasks as task (task.id)}
        <div
          class="new-task-chip"
          role="button"
          tabindex="0"
          draggable="true"
          ondragstart={(e) => handleDragStart(e, task.id)}
          onpointerdown={(e) => beginPointerDrag(e, task.id)}
          onpointermove={updatePointerDrag}
          onpointerup={endPointerDrag}
          onpointercancel={cancelPointerDrag}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              nodeStore.select(task.id);
              nodeStore.setViewMode('tree');
              e.preventDefault();
            }
          }}
        >
          {task.title || 'Untitled'}
        </div>
      {/each}
    </div>
  {/if}

  <div class="gtd-stages">
    {#each GTD_STAGES as stage (stage.key)}
      {@const tasks = tasksFor(stage.key)}
      <div
        class="gtd-stage"
        class:hovered={hoverStage === stage.key}
        data-gtd-stage={stage.key}
        role="region"
        aria-label={`GTD ${stage.title}`}
        ondragover={(e) => handleDragOver(e, stage.key)}
        ondragleave={handleDragLeave}
        ondrop={(e) => handleDrop(e, stage.key)}
      >
        <div class="stage-header">
          <span class="stage-icon">{stage.icon}</span>
          <strong>{stage.title}</strong>
          <span class="stage-desc">{stage.desc}</span>
          <span class="count">{tasks.length}</span>
        </div>
        <div class="stage-tasks">
          {#each tasks as task (task.id)}
            <div
              class="gtd-task-card"
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
</section>

<style>
  .gtd-view {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--bg-app);
  }
  .gtd-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
    flex-wrap: wrap;
  }
  .gtd-toolbar h2 {
    margin: 0;
  }
  .gtd-toolbar p {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }
  .toolbar-actions {
    display: flex;
    gap: 8px;
  }
  button {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    min-height: 32px;
    padding: 0 10px;
    cursor: pointer;
  }
  button.active,
  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
  .capture-bar {
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .capture-form {
    display: flex;
    gap: 8px;
  }
  .capture-input {
    flex: 1;
    min-height: 36px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    padding: 0 10px;
  }
  .review-panel {
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .review-panel h3 {
    margin: 0 0 4px;
  }
  .review-panel p {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .review-stages {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 8px;
    margin-top: 10px;
  }
  .review-stage {
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-input);
    padding: 8px;
  }
  .review-stage strong {
    font-size: var(--text-xs);
    display: block;
  }
  .review-stage span {
    font-size: 10px;
    color: var(--text-tertiary);
  }
  .review-task {
    font-size: var(--text-xs);
    padding: 3px 0;
    cursor: pointer;
  }
  .review-task:hover {
    color: var(--accent);
  }
  .more {
    font-size: 10px;
    color: var(--text-tertiary);
  }
  .new-tasks-bar {
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-input);
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .new-tasks-bar strong {
    font-size: var(--text-xs);
  }
  .new-tasks-bar span {
    color: var(--text-tertiary);
    font-size: 11px;
  }
  .new-task-chip {
    border: 1px dashed var(--border);
    border-radius: 12px;
    padding: 2px 8px;
    font-size: 11px;
    cursor: grab;
    background: var(--bg-panel);
  }
  .new-task-chip:active {
    cursor: grabbing;
  }
  .gtd-stages {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 10px;
    padding: 14px 18px;
  }
  .gtd-stage {
    border: 2px solid var(--border);
    border-radius: 10px;
    background: var(--bg-panel);
    display: flex;
    flex-direction: column;
    min-height: 140px;
    transition:
      border-color 0.15s,
      background 0.15s;
  }
  .gtd-stage.hovered {
    border-color: var(--accent);
    background: var(--accent-subtle, rgba(37, 99, 235, 0.05));
  }
  .stage-header {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 8px 10px 6px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .stage-icon {
    font-size: 14px;
  }
  .stage-header strong {
    font-size: var(--text-xs);
    white-space: nowrap;
  }
  .stage-desc {
    font-size: 10px;
    color: var(--text-tertiary);
    display: none;
  }
  .stage-header .count {
    margin-left: auto;
    background: var(--bg-active);
    padding: 1px 7px;
    border-radius: 8px;
    font-size: 11px;
  }
  .stage-tasks {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: grid;
    gap: 4px;
    align-content: start;
  }
  .gtd-task-card {
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 5px 7px;
    background: var(--bg-input);
    cursor: grab;
    display: grid;
    gap: 2px;
  }
  .gtd-task-card:hover {
    border-color: var(--accent);
  }
  .gtd-task-card:active {
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
    gap: 4px;
  }
  .priority {
    font-size: 10px;
    padding: 0 4px;
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
    padding: 8px;
  }
  @media (max-width: 800px) {
    .gtd-stages {
      grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    }
    .stage-desc {
      display: inline;
    }
  }
</style>
