<script lang="ts">
  import { calendarStore } from '$lib/stores/calendar.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import { projectTasks } from '$lib/utils/task-projection';
  import { visibleRange } from '$lib/utils/calendar';
  import AgendaView from './AgendaView.svelte';
  import CalendarToolbar from './CalendarToolbar.svelte';
  import MonthGrid from './MonthGrid.svelte';
  import WeekGrid from './WeekGrid.svelte';

  let currentDate = $derived(new Date(uiStore.calendarPrefs.currentDate));
  let firstDayOfWeek = $derived(uiStore.calendarPrefs.firstDayOfWeek);
  let calendarView = $derived(uiStore.calendarPrefs.view);
  let showCompleted = $derived(uiStore.calendarPrefs.showCompleted);
  let range = $derived(visibleRange(calendarView, currentDate, firstDayOfWeek));
  let activeItems = $derived(
    projectTasks([...nodeStore.nodes.values()], {
      mode: 'active',
      sort: 'due',
      tagAssignments: nodeStore.tagAssignments,
    }),
  );
  let completedItems = $derived(
    showCompleted
      ? projectTasks([...nodeStore.nodes.values()], {
          mode: 'completed',
          sort: 'due',
          tagAssignments: nodeStore.tagAssignments,
        })
      : [],
  );
  let tasks = $derived([...activeItems, ...completedItems].map((item) => item.node));

  $effect(() => {
    calendarStore.load(range.start, range.end);
  });
</script>

<div class="calendar-view">
  <CalendarToolbar {currentDate} />
  <div class="calendar-surface">
    {#if calendarStore.isLoading}
      <div class="empty-state small"><p>Loading calendar...</p></div>
    {:else if calendarView === 'month'}
      <MonthGrid
        {currentDate}
        {firstDayOfWeek}
        {showCompleted}
        {tasks}
        blocks={calendarStore.blocks}
      />
    {:else if calendarView === 'week'}
      <WeekGrid
        {currentDate}
        {firstDayOfWeek}
        {showCompleted}
        {tasks}
        blocks={calendarStore.blocks}
      />
    {:else if calendarView === 'day'}
      <WeekGrid
        {currentDate}
        {firstDayOfWeek}
        {showCompleted}
        {tasks}
        blocks={calendarStore.blocks}
        singleDay
      />
    {:else}
      <AgendaView {currentDate} {showCompleted} {tasks} blocks={calendarStore.blocks} />
    {/if}
  </div>
</div>

<style>
  .calendar-view {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
    overflow: hidden;
  }
  .calendar-surface {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
