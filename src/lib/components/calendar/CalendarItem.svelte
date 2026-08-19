<script lang="ts">
  import { calendarStore } from '$lib/stores/calendar.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import type { Node, TimeBlock } from '$lib/types/node';
  import { itemDurationMinutes } from '$lib/utils/calendar';
  import { overlayLayer } from '$lib/stores/overlay.svelte';

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

  // Time blocks carry five actions. Rendering them inline put Link/Unlink/-30/
  // +30/Delete on every block in every view, which left a month cell mostly
  // buttons and pushed the block's own title and time to the margins. They live
  // in a menu now: title and time first, actions on request.
  let menuOpen = $state(false);

  overlayLayer('calendar-block-menu', () => menuOpen, 'menu');
  let menuX = $state(0);
  let menuY = $state(0);
  let menuEl: HTMLElement | undefined = $state();
  let menuTriggerEl: HTMLButtonElement | undefined = $state();

  const MENU_WIDTH = 200;
  const MENU_HEIGHT = 196;

  function openMenu(event: MouseEvent) {
    event.stopPropagation();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    // Fixed positioning, because month day cells clip their overflow.
    menuX = Math.max(8, Math.min(rect.right - MENU_WIDTH, window.innerWidth - MENU_WIDTH - 8));
    menuY =
      rect.bottom + MENU_HEIGHT > window.innerHeight - 8
        ? Math.max(8, rect.top - MENU_HEIGHT - 4)
        : rect.bottom + 4;
    menuOpen = true;
    queueMicrotask(() => menuEl?.querySelector('button')?.focus());
  }

  function closeMenu(refocus = true) {
    if (!menuOpen) return;
    menuOpen = false;
    if (refocus) queueMicrotask(() => menuTriggerEl?.focus());
  }

  function runAction(action: () => void) {
    closeMenu();
    action();
  }

  function menuKeydown(event: KeyboardEvent) {
    // Global shortcuts are bound on window and would otherwise act on the tree
    // while this menu has focus.
    event.stopPropagation();

    if (event.key === 'Escape') {
      event.preventDefault();
      closeMenu();
      return;
    }
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp' && event.key !== 'Tab') return;
    const items = [...(menuEl?.querySelectorAll<HTMLButtonElement>('button') ?? [])];
    if (items.length === 0) return;
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    const step = event.key === 'ArrowUp' || (event.key === 'Tab' && event.shiftKey) ? -1 : 1;
    event.preventDefault();
    items[(current + step + items.length) % items.length].focus();
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
    <strong class="block-title">{block.title || 'Planning block'}</strong>
    <span class="time">
      {new Date(block.startTime).toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
      })}{#if !compact}
        -
        {new Date(block.endTime).toLocaleTimeString([], {
          hour: '2-digit',
          minute: '2-digit',
        })}{/if}
    </span>
    {#if !compact}
      <span class="duration">{itemDurationMinutes(block)}m</span>
    {/if}
    <button
      bind:this={menuTriggerEl}
      type="button"
      class="more"
      class:open={menuOpen}
      title="Time block actions"
      aria-label="Actions for {block.title || 'Planning block'}"
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      onclick={openMenu}>⋯</button
    >
    {#if !compact}
      <button
        class="resize-handle"
        class:resizing
        title="Drag to resize"
        aria-label="Drag to resize time block"
        onpointerdown={beginResize}>↕</button
      >
    {/if}
  </div>

  {#if menuOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="menu-backdrop"
      onclick={() => closeMenu()}
      oncontextmenu={(e) => {
        e.preventDefault();
        closeMenu();
      }}
    ></div>
    <div
      bind:this={menuEl}
      class="block-menu"
      role="menu"
      aria-label="Time block actions"
      tabindex="-1"
      style="left: {menuX}px; top: {menuY}px;"
      onkeydown={menuKeydown}
    >
      <button
        type="button"
        role="menuitem"
        onclick={() => runAction(() => calendarStore.linkSelectedTask(block.id))}
        >Link selected task</button
      >
      <button
        type="button"
        role="menuitem"
        onclick={() => runAction(() => calendarStore.unlinkBlock(block.id))}>Unlink task</button
      >
      <div class="menu-separator"></div>
      <button
        type="button"
        role="menuitem"
        onclick={() => runAction(() => calendarStore.resizeBlock(block.id, -30))}
        >Shorten by 30 minutes</button
      >
      <button
        type="button"
        role="menuitem"
        onclick={() => runAction(() => calendarStore.resizeBlock(block.id, 30))}
        >Extend by 30 minutes</button
      >
      <div class="menu-separator"></div>
      <button
        type="button"
        role="menuitem"
        class="destructive"
        onclick={() => runAction(() => calendarStore.deleteBlock(block.id))}>Delete block</button
      >
    </div>
  {/if}
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
  }
  /* Non-compact blocks keep the drag handle pinned right; leave room for it so
     the menu trigger does not sit underneath it. */
  .calendar-item.block:not(.compact) .more {
    margin-right: 24px;
  }
  /* The block's own title is the thing being scheduled, so it gets the flexible
     space and truncates rather than pushing the time out of the cell. */
  .block-title {
    min-width: 0;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .duration {
    color: var(--text-tertiary);
    font-size: 10px;
    flex-shrink: 0;
  }
  .item-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1 1 auto;
  }
  /* Reserve enough room for a few characters of the title before the metadata
     beside it is allowed to take the rest of a narrow month cell. */
  .calendar-item.compact .item-title,
  .calendar-item.compact .block-title {
    flex-basis: 50%;
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
  /* Month cells are narrow. What is scheduled matters more than exactly when, so
     let the supporting metadata give way before the title does. */
  .calendar-item.compact .time,
  .calendar-item.compact .priority,
  .calendar-item.compact .tag {
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    /* Clip rather than wrap: a wrapped time doubles the row height and takes the
       space back from the title it was supposed to yield to. */
    white-space: nowrap;
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
  /* Sits quietly until the block is hovered or focused, so a month cell reads as
     its blocks rather than as a grid of buttons. Kept in the DOM at all times so
     it stays reachable by keyboard, where :focus-visible reveals it. */
  .more {
    margin-left: auto;
    flex-shrink: 0;
    width: 20px;
    height: 18px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--text-tertiary);
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    opacity: 0;
    transition: opacity 100ms ease;
  }
  .calendar-item.block:hover .more,
  .more:focus-visible,
  .more.open {
    opacity: 1;
    border-color: var(--border);
    background: var(--bg-panel);
  }
  .more:hover {
    color: var(--text-primary);
  }
  .more:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  /* In a month cell the 25px this reserves is worth more to the title than to a
     control that is invisible until the block is hovered, so take it out of the
     flow there and let it sit over the right edge when it appears. */
  .calendar-item.compact .more {
    position: absolute;
    right: 3px;
    top: 50%;
    transform: translateY(-50%);
    margin-left: 0;
  }

  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 900;
  }
  .block-menu {
    position: fixed;
    z-index: 901;
    width: 200px;
    background: var(--bg-modal);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
    padding: 4px;
    display: flex;
    flex-direction: column;
  }
  .block-menu button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 10px;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--text-primary);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .block-menu button:hover {
    background: var(--bg-hover);
  }
  .block-menu button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .block-menu button.destructive {
    color: var(--danger);
  }
  .menu-separator {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
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
