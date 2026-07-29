<script lang="ts">
  import { calendarStore } from '$lib/stores/calendar.svelte';
  import type { Node, TimeBlock } from '$lib/types/node';
  import { addDays, blocksByDay, localDayKey, tasksByDay, weekStart } from '$lib/utils/calendar';
  import CalendarItem from './CalendarItem.svelte';

  let {
    currentDate,
    firstDayOfWeek,
    showCompleted,
    tasks,
    blocks,
    singleDay = false,
  }: {
    currentDate: Date;
    firstDayOfWeek: number;
    showCompleted: boolean;
    tasks: Node[];
    blocks: TimeBlock[];
    singleDay?: boolean;
  } = $props();

  const hours = Array.from({ length: 15 }, (_, index) => index + 7);
  let days = $derived(
    singleDay
      ? [currentDate]
      : Array.from({ length: 7 }, (_, index) => addDays(weekStart(currentDate, firstDayOfWeek), index)),
  );
  let taskMap = $derived(tasksByDay(tasks, showCompleted));
  let blockMap = $derived(blocksByDay(blocks));

  function timedItems(day: Date, hour: number) {
    const key = localDayKey(day);
    const taskItems = (taskMap.get(key) ?? []).filter((task) => {
      const due = task.properties.dueDate ? new Date(task.properties.dueDate) : null;
      return due && due.getHours() === hour;
    });
    const blockItems = (blockMap.get(key) ?? []).filter((block) => new Date(block.startTime).getHours() === hour);
    return { taskItems, blockItems };
  }

  function dropOnSlot(event: DragEvent) {
    event.preventDefault();
    const raw = event.dataTransfer?.getData('application/doyo-calendar');
    if (!raw) return;
    const payload = JSON.parse(raw) as { type: 'task' | 'block'; id: string };
    const target = event.currentTarget as HTMLElement;
    calendarStore.applyDrop(payload, target, event.clientY);
  }
</script>

<div class="week-grid" style={`grid-template-columns: 64px repeat(${days.length}, minmax(160px, 1fr))`}>
  <div class="corner"></div>
  {#each days as day}
    <div class="day-header">
      <strong>{day.toLocaleDateString(undefined, { weekday: 'short' })}</strong>
      <span>{day.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}</span>
    </div>
  {/each}

  {#each hours as hour}
    <div class="hour-label">{String(hour).padStart(2, '0')}:00</div>
    {#each days as day}
      {@const items = timedItems(day, hour)}
      <section
        class="slot"
        role="group"
        aria-label={`${day.toLocaleDateString()} ${String(hour).padStart(2, '0')}:00`}
        data-calendar-drop="slot"
        data-calendar-day={localDayKey(day)}
        data-calendar-hour={hour}
        ondragover={(e) => e.preventDefault()}
        ondrop={dropOnSlot}
      >
        <button class="slot-add" title="New time block" onclick={() => calendarStore.createBlock(day, hour)}>
          +
        </button>
        {#each items.taskItems as task (task.id)}
          <CalendarItem {task} />
        {/each}
        {#each items.blockItems as block (block.id)}
          <CalendarItem {block} />
        {/each}
      </section>
    {/each}
  {/each}
</div>

<style>
  .week-grid {
    display: grid;
    min-width: min(820px, 100%);
    overflow: auto;
    height: 100%;
  }
  .corner,
  .day-header,
  .hour-label,
  .slot {
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .corner,
  .day-header {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--bg-panel);
  }
  .day-header {
    min-height: 42px;
    display: grid;
    align-content: center;
    gap: 2px;
    padding: 6px 8px;
  }
  .day-header span,
  .hour-label {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .hour-label {
    padding: 8px;
    background: var(--bg-panel);
  }
  .slot {
    min-height: 76px;
    padding: 5px;
    display: grid;
    align-content: start;
    gap: 4px;
    position: relative;
    min-width: 0;
  }
  .slot-add {
    position: absolute;
    top: 4px;
    right: 4px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--text-tertiary);
    cursor: pointer;
    width: 18px;
    height: 18px;
    line-height: 1;
  }
</style>
