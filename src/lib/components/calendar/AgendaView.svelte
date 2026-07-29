<script lang="ts">
  import type { Node, TimeBlock } from '$lib/types/node';
  import { addDays, blocksByDay, localDayKey, startOfLocalDay, tasksByDay } from '$lib/utils/calendar';
  import CalendarItem from './CalendarItem.svelte';

  let {
    currentDate,
    showCompleted,
    tasks,
    blocks,
  }: {
    currentDate: Date;
    showCompleted: boolean;
    tasks: Node[];
    blocks: TimeBlock[];
  } = $props();

  let days = $derived(Array.from({ length: 30 }, (_, index) => addDays(startOfLocalDay(currentDate), index)));
  let taskMap = $derived(tasksByDay(tasks, showCompleted));
  let blockMap = $derived(blocksByDay(blocks));
</script>

<div class="agenda">
  {#each days as day}
    {@const key = localDayKey(day)}
    {@const dayTasks = taskMap.get(key) ?? []}
    {@const dayBlocks = blockMap.get(key) ?? []}
    {#if dayTasks.length || dayBlocks.length}
      <section class="agenda-day">
        <h3>
          {day.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' })}
        </h3>
        <div class="agenda-items">
          {#each dayTasks as task (task.id)}
            <CalendarItem {task} />
          {/each}
          {#each dayBlocks as block (block.id)}
            <CalendarItem {block} />
          {/each}
        </div>
      </section>
    {/if}
  {/each}
</div>

<style>
  .agenda {
    overflow-y: auto;
    padding: 12px;
    display: grid;
    gap: 12px;
  }
  .agenda-day {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-panel);
    padding: 10px;
  }
  h3 {
    margin: 0 0 8px;
    font-size: var(--text-sm);
  }
  .agenda-items {
    display: grid;
    gap: 6px;
  }
</style>
