<script lang="ts">
  import { focusStore, formatFocusDuration } from '$lib/stores/focus.svelte';
  import { habitStore } from '$lib/stores/habits.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import type { Node } from '$lib/types/node';
  import { type StatsRange } from '$lib/utils/productivity';
  import { focusStatistics, habitStatistics, taskStatistics } from '$lib/utils/statistics';

  type StatsTab = 'overview' | 'tasks' | 'focus' | 'habits';

  let didLoad = false;
  let tab = $state<StatsTab>('overview');
  let range = $state<StatsRange>('week');
  let now = $state(new Date());

  $effect(() => {
    if (didLoad) return;
    didLoad = true;
    focusStore.load();
    habitStore.load();
  });

  let activeNodes = $derived([...nodeStore.nodes.values()].filter((node) => !node.deletedAt));
  let taskStats = $derived(taskStatistics(activeNodes, range, now));
  let focusStats = $derived(focusStatistics(focusStore.history, range, now));
  let habitStats = $derived(habitStatistics(habitStore.logs, range, now));

  function maxValue(buckets: Array<{ value: number }>) {
    return Math.max(1, ...buckets.map((bucket) => bucket.value));
  }

  function pathOf(node: Node) {
    return [...nodeStore.getAncestors(node.id), node];
  }

  function workspaceTitle(node: Node) {
    return pathOf(node).find((item) => item.nodeType === 'Workspace')?.title || 'No workspace';
  }

  function groupTitle(node: Node) {
    const groups = pathOf(node).filter((item) => item.nodeType === 'Group');
    return groups.at(-1)?.title || 'No group';
  }

  function addCount(map: Map<string, number>, key: string, amount = 1) {
    map.set(key, (map.get(key) ?? 0) + amount);
  }

  function completedTaskNodes() {
    return activeNodes.filter((node) => node.nodeType === 'Task' && node.isCompleted);
  }

  function breakdownBy(labeler: (node: Node) => string) {
    const map = new Map<string, number>();
    for (const task of completedTaskNodes()) addCount(map, labeler(task));
    return [...map.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  }

  function tagBreakdown() {
    const map = new Map<string, number>();
    for (const task of completedTaskNodes()) {
      const tags = nodeStore.tagAssignments.get(task.id) ?? [];
      if (tags.length === 0) addCount(map, 'No tag');
      for (const tag of tags) addCount(map, tag.name);
    }
    return [...map.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  }

  function focusByTask() {
    const map = new Map<string, number>();
    for (const session of focusStore.history) {
      addCount(map, session.taskTitle || 'No linked task', session.durationSeconds);
    }
    return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, 8);
  }
</script>

<section class="statistics-view" aria-label="Statistics">
  <div class="view-header">
    <div>
      <h2>Statistics</h2>
      <p>Real persisted task, focus, and habit data. Each task record is counted once.</p>
    </div>
    <select bind:value={range} aria-label="Range" onchange={() => (now = new Date())}>
      <option value="day">Day</option>
      <option value="week">Week</option>
      <option value="month">Month</option>
    </select>
  </div>

  <div class="tabs" aria-label="Statistics tabs">
    <button class:active={tab === 'overview'} onclick={() => (tab = 'overview')}>Overview</button>
    <button class:active={tab === 'tasks'} onclick={() => (tab = 'tasks')}>Tasks</button>
    <button class:active={tab === 'focus'} onclick={() => (tab = 'focus')}>Focus</button>
    <button class:active={tab === 'habits'} onclick={() => (tab = 'habits')}>Habits</button>
  </div>

  {#if tab === 'overview'}
    <div class="stats-grid">
      <article><span>Open tasks</span><strong>{taskStats.openTasks}</strong></article>
      <article>
        <span>Completed in range</span><strong>{taskStats.completedInRange}</strong>
      </article>
      <article><span>Completion rate</span><strong>{taskStats.completionRate}%</strong></article>
      <article><span>Overdue</span><strong>{taskStats.overdue}</strong></article>
      <article>
        <span>Focus in range</span><strong>{formatFocusDuration(focusStats.actualSeconds)}</strong>
      </article>
      <article>
        <span>Total focus</span><strong
          >{formatFocusDuration(focusStore.summary.totalSeconds)}</strong
        >
      </article>
      <article><span>Pomodoros</span><strong>{focusStats.pomodoroCount}</strong></article>
      <article><span>Active habits</span><strong>{habitStore.summary.activeCount}</strong></article>
      <article>
        <span>Best habit streak</span><strong>{habitStore.summary.bestStreak}</strong>
      </article>
    </div>
    {#if taskStats.totalTasks === 0 && focusStats.sessionCount === 0 && habitStats.logCount === 0}
      <div class="empty-state small">
        <p>
          No statistics yet. Complete tasks, focus sessions, or habit logs to populate this view.
        </p>
      </div>
    {/if}
  {:else if tab === 'tasks'}
    <div class="stats-grid">
      <article><span>Total task records</span><strong>{taskStats.totalTasks}</strong></article>
      <article><span>Created in range</span><strong>{taskStats.createdInRange}</strong></article>
      <article><span>Completed all-time</span><strong>{taskStats.completedAll}</strong></article>
      <article>
        <span>Completed in range</span><strong>{taskStats.completedInRange}</strong>
      </article>
      <article><span>Overdue</span><strong>{taskStats.overdue}</strong></article>
      <article><span>Completion rate</span><strong>{taskStats.completionRate}%</strong></article>
    </div>
    <div class="trend-grid">
      {@render Trend('Completed Tasks', taskStats.completedTrend)}
      {@render Trend('Created Tasks', taskStats.createdTrend)}
      {@render Breakdown('Completion by Workspace', breakdownBy(workspaceTitle))}
      {@render Breakdown('Completion by Group/Subgroup', breakdownBy(groupTitle))}
      {@render Breakdown(
        'Completion by Priority',
        breakdownBy((task) =>
          task.properties.priority ? `P${task.properties.priority}` : 'No priority',
        ),
      )}
      {@render Breakdown('Completion by Tag', tagBreakdown())}
    </div>
    {#if taskStats.totalTasks === 0}
      <div class="empty-state small"><p>No task records available for task statistics.</p></div>
    {/if}
  {:else if tab === 'focus'}
    <div class="stats-grid">
      <article><span>Sessions</span><strong>{focusStats.sessionCount}</strong></article>
      <article>
        <span>Actual focus</span><strong>{formatFocusDuration(focusStats.actualSeconds)}</strong>
      </article>
      <article>
        <span>Planned time</span><strong>{formatFocusDuration(focusStats.plannedSeconds)}</strong>
      </article>
      <article>
        <span>Pomodoro</span><strong>{formatFocusDuration(focusStats.pomodoroSeconds)}</strong>
      </article>
      <article>
        <span>Stopwatch</span><strong>{formatFocusDuration(focusStats.stopwatchSeconds)}</strong>
      </article>
      <article>
        <span>Flowtime</span><strong>{formatFocusDuration(focusStats.flowtimeSeconds)}</strong>
      </article>
    </div>
    <div class="trend-grid">
      {@render Trend('Focus Minutes', focusStats.focusTrend)}
      {@render Breakdown(
        'Focus by Task',
        focusByTask().map(([title, seconds]) => [title, Math.round(seconds / 60)]),
        'min',
      )}
    </div>
    {#if focusStats.sessionCount === 0}
      <div class="empty-state small"><p>No focus sessions in this range.</p></div>
    {/if}
  {:else if tab === 'habits'}
    <div class="stats-grid">
      <article><span>Habit logs</span><strong>{habitStats.logCount}</strong></article>
      <article><span>Completed</span><strong>{habitStats.completed}</strong></article>
      <article><span>Partial</span><strong>{habitStats.partial}</strong></article>
      <article><span>Skipped</span><strong>{habitStats.skipped}</strong></article>
      <article><span>Completion rate</span><strong>{habitStats.completionRate}%</strong></article>
      <article><span>Best streak</span><strong>{habitStore.summary.bestStreak}</strong></article>
    </div>
    <div class="trend-grid">
      {@render Trend('Habit Completion', habitStats.habitTrend)}
    </div>
    {#if habitStats.logCount === 0}
      <div class="empty-state small"><p>No habit logs in this range.</p></div>
    {/if}
  {/if}
</section>

{#snippet Trend(title: string, buckets: Array<{ key: string; value: number }>)}
  <section>
    <h3>{title}</h3>
    {#if buckets.every((bucket) => bucket.value === 0)}
      <p class="muted">No data in this range</p>
    {:else}
      <div class="bars">
        {#each buckets as bucket (bucket.key)}
          <div class="bar-wrap" title={`${bucket.key}: ${bucket.value}`}>
            <span style={`height: ${Math.max(4, (bucket.value / maxValue(buckets)) * 72)}px`}
            ></span>
            <small>{bucket.key.slice(5)}</small>
          </div>
        {/each}
      </div>
    {/if}
  </section>
{/snippet}

{#snippet Breakdown(title: string, rows: Array<[string, number]>, suffix = '')}
  <section>
    <h3>{title}</h3>
    {#if rows.length === 0}
      <p class="muted">No data in this range</p>
    {:else}
      <div class="breakdown">
        {#each rows as [label, count]}
          <div><span>{label}</span><strong>{count}{suffix}</strong></div>
        {/each}
      </div>
    {/if}
  </section>
{/snippet}

<style>
  .statistics-view {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px;
    background: var(--bg-app);
  }
  .view-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 12px;
  }
  .tabs button,
  select {
    min-height: 32px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    padding: 0 10px;
  }
  .tabs button.active {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .view-header p,
  article span,
  .muted {
    color: var(--text-tertiary);
  }
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 10px;
    margin-top: 12px;
  }
  .trend-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 10px;
    margin-top: 12px;
  }
  .trend-grid section {
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
    padding: 12px;
    min-width: 0;
  }
  .trend-grid h3 {
    margin: 0 0 10px;
    font-size: var(--text-sm);
  }
  .bars {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(18px, 1fr);
    align-items: end;
    gap: 5px;
    min-height: 102px;
    overflow-x: auto;
  }
  .bar-wrap {
    display: grid;
    align-items: end;
    justify-items: center;
    gap: 4px;
    min-width: 18px;
  }
  .bar-wrap span {
    width: 100%;
    max-width: 18px;
    border-radius: 4px 4px 0 0;
    background: var(--accent);
  }
  .bar-wrap small {
    color: var(--text-tertiary);
    font-size: 9px;
    writing-mode: vertical-rl;
  }
  article,
  .breakdown > div {
    display: grid;
    gap: 6px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
    padding: 12px;
  }
  article strong {
    font-size: 24px;
  }
  .breakdown {
    display: grid;
    gap: 6px;
  }
  .breakdown > div {
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
  }
</style>
