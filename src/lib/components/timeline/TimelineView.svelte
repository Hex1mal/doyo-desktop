<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore, type TimelineZoom } from '$lib/stores/ui.svelte';
  import { addDays } from '$lib/utils/calendar';
  import { projectTasks } from '$lib/utils/task-projection';
  import { taskTimelineRange, timelineVisibleRange } from '$lib/utils/timeline';
  import TimelineItem from './TimelineItem.svelte';

  let prefs = $derived(uiStore.timelinePrefs);
  let currentDate = $derived(new Date(prefs.currentDate));
  let range = $derived(timelineVisibleRange(currentDate, prefs.zoom));
  let dayWidth = $derived(prefs.zoom === 'day' ? 120 : prefs.zoom === 'week' ? 72 : 44);
  let tasks = $derived.by(() => {
    const active = projectTasks([...nodeStore.nodes.values()], {
      mode: 'active',
      sort: 'due',
      tagAssignments: nodeStore.tagAssignments,
    }).filter((item) => taskTimelineRange(item.node));
    if (!prefs.showCompleted) return active;
    return [
      ...active,
      ...projectTasks([...nodeStore.nodes.values()], {
        mode: 'completed',
        sort: 'due',
        tagAssignments: nodeStore.tagAssignments,
      }).filter((item) => taskTimelineRange(item.node)),
    ];
  });

  function setZoom(value: string) {
    uiStore.setTimelinePrefs({ zoom: value as TimelineZoom });
  }

  function shift(days: number) {
    uiStore.setTimelinePrefs({ currentDate: addDays(currentDate, days).toISOString() });
  }
</script>

<section class="timeline-view">
  <header class="timeline-toolbar">
    <div>
      <h2>Timeline</h2>
      <p>Tasks are shown by existing start and due dates. Drag bars to move; drag handles to resize.</p>
    </div>
    <button onclick={() => uiStore.setTimelinePrefs({ currentDate: new Date().toISOString() })}>Today</button>
    <button onclick={() => shift(prefs.zoom === 'month' ? -30 : prefs.zoom === 'week' ? -7 : -1)}>Previous</button>
    <button onclick={() => shift(prefs.zoom === 'month' ? 30 : prefs.zoom === 'week' ? 7 : 1)}>Next</button>
    <label>
      Zoom
      <select value={prefs.zoom} onchange={(e) => setZoom((e.target as HTMLSelectElement).value)}>
        <option value="day">Day</option>
        <option value="week">Week</option>
        <option value="month">Month</option>
      </select>
    </label>
    <label class="check-row">
      <input
        type="checkbox"
        checked={prefs.showCompleted}
        onchange={(e) => uiStore.setTimelinePrefs({ showCompleted: (e.target as HTMLInputElement).checked })}
      />
      Completed
    </label>
  </header>

  <div class="timeline-scroll">
    <div class="timeline-scale" style={`width: ${range.days.length * dayWidth}px`}>
      {#each range.days as day}
        <div class="day-cell" style={`width: ${dayWidth}px`}>
          <strong>{day.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}</strong>
          <span>{day.toLocaleDateString(undefined, { weekday: 'short' })}</span>
        </div>
      {/each}
    </div>
    <div class="timeline-rows" style={`width: ${range.days.length * dayWidth}px`}>
      {#if tasks.length === 0}
        <div class="empty">No scheduled tasks in this range</div>
      {/if}
      {#each tasks as item (item.node.id)}
        <div class="row" style={`--timeline-day-width: ${dayWidth}px`}>
          <TimelineItem {item} days={range.days} {dayWidth} />
        </div>
      {/each}
    </div>
  </div>
</section>

<style>
  .timeline-view {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .timeline-toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .timeline-toolbar h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  .timeline-toolbar p {
    margin: 3px 0 0;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .timeline-toolbar button,
  .timeline-toolbar select {
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-input);
    color: var(--text-secondary);
    height: 30px;
  }
  .timeline-toolbar label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }
  .check-row {
    margin-left: auto;
  }
  .timeline-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .timeline-scale {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    min-height: 44px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
  }
  .day-cell {
    flex: 0 0 auto;
    display: grid;
    align-content: center;
    gap: 2px;
    padding: 6px;
    border-right: 1px solid var(--border);
  }
  .day-cell strong,
  .day-cell span {
    font-size: var(--text-xs);
  }
  .day-cell span {
    color: var(--text-tertiary);
  }
  .timeline-rows {
    min-height: 100%;
  }
  .row {
    position: relative;
    height: 74px;
    border-bottom: 1px solid var(--border);
    background-image: linear-gradient(to right, var(--border) 1px, transparent 1px);
    background-size: var(--timeline-day-width, 72px) 100%;
  }
  .empty {
    padding: 18px;
    color: var(--text-tertiary);
  }
</style>
