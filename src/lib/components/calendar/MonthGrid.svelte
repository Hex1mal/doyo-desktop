<script lang="ts">
  import { calendarStore } from '$lib/stores/calendar.svelte';
  import type { Node, TimeBlock } from '$lib/types/node';
  import { blocksByDay, localDayKey, monthGrid, tasksByDay } from '$lib/utils/calendar';
  import { onMount } from 'svelte';
  import CalendarItem from './CalendarItem.svelte';

  let {
    currentDate,
    firstDayOfWeek,
    showCompleted,
    tasks,
    blocks,
  }: {
    currentDate: Date;
    firstDayOfWeek: number;
    showCompleted: boolean;
    tasks: Node[];
    blocks: TimeBlock[];
  } = $props();

  let days = $derived(monthGrid(currentDate, firstDayOfWeek));
  let taskMap = $derived(tasksByDay(tasks, showCompleted));
  let blockMap = $derived(blocksByDay(blocks));
  let weekdays = $derived(
    days.slice(0, 7).map((day) => day.toLocaleDateString(undefined, { weekday: 'short' })),
  );
  let today = $state(new Date());
  let todayKey = $derived(localDayKey(today));

  onMount(() => {
    const timer = setInterval(() => {
      today = new Date();
    }, 60_000);
    return () => clearInterval(timer);
  });

  function dropOnDay(event: DragEvent) {
    event.preventDefault();
    const raw = event.dataTransfer?.getData('application/doyo-calendar');
    if (!raw) return;
    const payload = JSON.parse(raw) as { type: 'task' | 'block'; id: string };
    const target = event.currentTarget as HTMLElement;
    calendarStore.applyDrop(payload, target);
  }
</script>

<div class="month-grid">
  {#each weekdays as weekday}
    <div class="weekday">{weekday}</div>
  {/each}
  {#each days as day}
    <section
      class="day-cell"
      class:outside={day.getMonth() !== currentDate.getMonth()}
      class:today={localDayKey(day) === todayKey}
      role="group"
      aria-label={day.toLocaleDateString()}
      data-calendar-drop="day"
      data-calendar-day={localDayKey(day)}
      ondragover={(e) => e.preventDefault()}
      ondrop={dropOnDay}
    >
      <div class="day-head">
        <span>{day.getDate()}</span>
        {#if localDayKey(day) === todayKey}<small>Today</small>{/if}
        <button title="New time block" onclick={() => calendarStore.createBlock(day, 9)}>+</button>
      </div>
      <div class="day-items">
        {#each taskMap.get(localDayKey(day)) ?? [] as task (task.id)}
          <CalendarItem {task} compact />
        {/each}
        {#each blockMap.get(localDayKey(day)) ?? [] as block (block.id)}
          <CalendarItem {block} compact />
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  .month-grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    grid-auto-rows: minmax(94px, 1fr);
    min-width: 0;
    height: 100%;
    overflow: auto;
  }
  .weekday {
    position: sticky;
    top: 0;
    z-index: 1;
    min-height: 30px;
    padding: 8px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    border-right: 1px solid var(--border);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: 800;
  }
  .day-cell {
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    padding: 6px;
    overflow: hidden;
    background: var(--bg-app);
    min-width: 0;
  }
  .day-cell.outside {
    opacity: 0.55;
    background: var(--bg-panel);
  }
  .day-cell.today {
    box-shadow: inset 0 0 0 2px var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, var(--bg-app));
  }
  .day-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-size: var(--text-xs);
    font-weight: 800;
    margin-bottom: 5px;
    gap: 6px;
  }
  .day-head small {
    margin-right: auto;
    border: 1px solid var(--accent);
    border-radius: 999px;
    padding: 1px 5px;
    color: var(--accent);
    font-size: 9px;
    line-height: 1.4;
  }
  .day-head button {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--text-tertiary);
    cursor: pointer;
    width: 20px;
    height: 20px;
  }
  .day-items {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
</style>
