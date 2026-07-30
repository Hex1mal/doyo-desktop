<script lang="ts">
  import type { Node, TimeBlock } from '$lib/types/node';
  import {
    addDays,
    blocksByDay,
    localDayKey,
    startOfLocalDay,
    tasksByDay,
  } from '$lib/utils/calendar';
  import { onMount } from 'svelte';
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

  let days = $derived(
    Array.from({ length: 30 }, (_, index) => addDays(startOfLocalDay(currentDate), index)),
  );
  let taskMap = $derived(tasksByDay(tasks, showCompleted));
  let blockMap = $derived(blocksByDay(blocks));
  let today = $state(new Date());
  let todayKey = $derived(localDayKey(today));

  onMount(() => {
    const timer = setInterval(() => {
      today = new Date();
    }, 60_000);
    return () => clearInterval(timer);
  });
</script>

<div class="agenda">
  {#each days as day}
    {@const key = localDayKey(day)}
    {@const dayTasks = taskMap.get(key) ?? []}
    {@const dayBlocks = blockMap.get(key) ?? []}
    {#if dayTasks.length || dayBlocks.length}
      <section class="agenda-day" class:today={key === todayKey}>
        <h3>
          {day.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' })}
          {#if key === todayKey}<span>Today</span>{/if}
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
  .agenda-day.today {
    border-color: var(--accent);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  h3 {
    margin: 0 0 8px;
    font-size: var(--text-sm);
  }
  h3 span {
    margin-left: 8px;
    color: var(--accent);
    font-size: 10px;
    text-transform: uppercase;
  }
  .agenda-items {
    display: grid;
    gap: 6px;
  }
</style>
