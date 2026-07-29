<script lang="ts">
  import { habitStore, localDateKey } from '$lib/stores/habits.svelte';
  import type { HabitFrequency } from '$lib/types/node';

  const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

  let didLoad = false;
  let title = $state('');
  let frequency = $state<HabitFrequency>('daily');
  let days = $state<number[]>([]);
  let goal = $state(1);
  let goalUnit = $state('count');
  let color = $state('#2563EB');
  let reminderTime = $state('');
  let today = $state(new Date());

  $effect(() => {
    if (didLoad) return;
    didLoad = true;
    habitStore.load();
  });

  function toggleDay(dayIndex: number) {
    if (days.includes(dayIndex)) {
      days = days.filter((d) => d !== dayIndex);
    } else {
      days = [...days, dayIndex].sort((a, b) => a - b);
    }
  }

  function createHabit() {
    habitStore.create({
      title,
      frequency,
      days: frequency === 'weekly' ? days : [],
      goal,
      goalUnit,
      color,
      startDate: localDateKey(),
      reminderTime: reminderTime || null,
    });
    title = '';
    days = [];
  }

  function toggleHabitDay(habitId: string, currentDays: number[], dayIndex: number) {
    const next = currentDays.includes(dayIndex)
      ? currentDays.filter((d) => d !== dayIndex)
      : [...currentDays, dayIndex].sort((a, b) => a - b);
    habitStore.update(habitId, { days: next });
  }

  function currentStreak(habitId: string): number {
    const logs = habitStore.logs
      .filter((log) => log.habitId === habitId && log.status === 'completed')
      .map((log) => log.logDate)
      .sort()
      .reverse();
    let streak = 0;
    let expected = localDateKey();
    for (const date of logs) {
      if (date === expected) {
        streak++;
        const d = new Date(expected + 'T00:00:00');
        d.setDate(d.getDate() - 1);
        expected = localDateKey(d);
      } else if (date < expected) {
        break;
      }
    }
    return streak;
  }

  function statusClass(habitId: string) {
    return habitStore.logFor(habitId)?.status ?? 'open';
  }

  function addDays(date: Date, days: number) {
    const next = new Date(date);
    next.setDate(next.getDate() + days);
    return next;
  }

  function trackerDays() {
    return Array.from({ length: 7 }, (_, index) => addDays(today, index - 6));
  }
</script>

<section class="habits-view" aria-label="Habits">
  <div class="view-header">
    <div>
      <h2>Habits</h2>
      <p>Daily and weekly trackers stored locally with real habit logs.</p>
    </div>
    <label class="inline">
      <input
        type="checkbox"
        checked={habitStore.showArchived}
        onchange={(event) => habitStore.setShowArchived((event.target as HTMLInputElement).checked)}
      />
      Show archived
    </label>
  </div>

  <div class="summary-strip">
    <div><span>Active</span><strong>{habitStore.summary.activeCount}</strong></div>
    <div><span>Today</span><strong>{habitStore.summary.completedToday}</strong></div>
    <div><span>Rate</span><strong>{Math.round(habitStore.summary.completionRate * 100)}%</strong></div>
    <div><span>Best streak</span><strong>{habitStore.summary.bestStreak}</strong></div>
  </div>

  <form class="habit-form" onsubmit={(event) => { event.preventDefault(); createHabit(); }}>
    <input aria-label="Habit title" placeholder="New habit" bind:value={title} required />
    <select bind:value={frequency} aria-label="Frequency">
      <option value="daily">Daily</option>
      <option value="weekly">Weekly</option>
    </select>
    {#if frequency === 'weekly'}
      <div class="day-selector">
        {#each WEEKDAYS as day, i}
          <button
            type="button"
            class:day-active={days.includes(i)}
            onclick={() => toggleDay(i)}
            title={day}
          >{day}</button>
        {/each}
      </div>
    {/if}
    <input aria-label="Goal" type="number" min="0.1" step="0.1" bind:value={goal} />
    <input aria-label="Goal unit" placeholder="unit" bind:value={goalUnit} />
    <input aria-label="Color" type="color" bind:value={color} />
    <input aria-label="Reminder time" type="time" bind:value={reminderTime} />
    <button class="primary" type="submit">Create</button>
  </form>

  {#if habitStore.isLoading}
    <div class="empty-state small"><p>Loading habits...</p></div>
  {:else if habitStore.habits.length === 0}
    <div class="empty-state small"><p>No habits yet</p></div>
  {:else}
    <div class="habit-list">
      {#each habitStore.habits as habit (habit.id)}
        <article class:archived={habit.archived}>
          <div class="color-dot" style={`background: ${habit.color ?? '#64748B'}`}></div>
          <div class="habit-main">
            <input
              aria-label="Habit title"
              value={habit.title}
              onchange={(event) => habitStore.update(habit.id, { title: (event.target as HTMLInputElement).value })}
            />
            <div class="edit-grid">
              <select value={habit.frequency} onchange={(event) => habitStore.update(habit.id, { frequency: (event.target as HTMLSelectElement).value as HabitFrequency })}>
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
              </select>
              <input aria-label="Goal" type="number" min="0.1" step="0.1" value={habit.goal} onchange={(event) => habitStore.update(habit.id, { goal: Number((event.target as HTMLInputElement).value) || 1 })} />
              <input aria-label="Goal unit" value={habit.goalUnit} onchange={(event) => habitStore.update(habit.id, { goalUnit: (event.target as HTMLInputElement).value })} />
              <input aria-label="Reminder" type="time" value={habit.reminderTime ?? ''} onchange={(event) => habitStore.update(habit.id, { reminderTime: (event.target as HTMLInputElement).value || null })} />
              <input aria-label="Color" type="color" value={habit.color ?? '#2563EB'} onchange={(event) => habitStore.update(habit.id, { color: (event.target as HTMLInputElement).value })} />
            </div>
            {#if habit.frequency === 'weekly'}
              <div class="day-selector inline">
                {#each WEEKDAYS as day, i}
                  <button
                    type="button"
                    class:day-active={habit.days.includes(i)}
                    onclick={() => toggleHabitDay(habit.id, habit.days, i)}
                    title={day}
                  >{day.slice(0, 1)}</button>
                {/each}
              </div>
            {/if}
            <div class="streak-badge" title="Current streak">
              Streak: <strong>{currentStreak(habit.id)}</strong> days
            </div>
          </div>
          <div class="tracker" aria-label="Seven day tracker">
            {#each trackerDays() as date (localDateKey(date))}
              {@const key = localDateKey(date)}
              {@const log = habitStore.logFor(habit.id, key)}
              <button
                class:done={log?.status === 'completed'}
                class:partial={log?.status === 'partial'}
                class:skipped={log?.status === 'skipped'}
                title={`${key}: ${log?.status ?? 'open'}`}
                onclick={() => habitStore.setLog(habit.id, log?.status === 'completed' ? 'skipped' : 'completed', key)}
              >
                {date.getDate()}
              </button>
            {/each}
          </div>
          <div class="log-actions" aria-label="Today log">
            <button class:active={statusClass(habit.id) === 'completed'} onclick={() => habitStore.setLog(habit.id, 'completed')}>
              Done
            </button>
            <button class:active={statusClass(habit.id) === 'partial'} onclick={() => habitStore.setLog(habit.id, 'partial', localDateKey(), habit.goal / 2)}>
              Partial
            </button>
            <button class:active={statusClass(habit.id) === 'skipped'} onclick={() => habitStore.setLog(habit.id, 'skipped', localDateKey(), 0)}>
              Skip
            </button>
            <button onclick={() => habitStore.clearLog(habit.id)}>Clear</button>
          </div>
          <button onclick={() => habitStore.archive(habit.id, !habit.archived)}>
            {habit.archived ? 'Restore' : 'Archive'}
          </button>
          <button class="danger" onclick={() => habitStore.delete(habit.id)}>Delete</button>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .habits-view {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px;
    background: var(--bg-app);
  }
  .view-header,
  .summary-strip,
  .habit-form,
  article,
  .log-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .view-header {
    justify-content: space-between;
  }
  .view-header p {
    color: var(--text-tertiary);
  }
  .summary-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
    margin: 12px 0;
  }
  .summary-strip > div,
  article,
  .habit-form {
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
    padding: 10px;
  }
  .summary-strip > div {
    display: grid;
    gap: 4px;
  }
  .summary-strip span {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .habit-form {
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  input,
  select,
  button {
    min-height: 32px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    padding: 0 8px;
  }
  button {
    cursor: pointer;
  }
  .primary,
  .log-actions .active {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .danger {
    color: var(--danger);
  }
  .habit-list {
    display: grid;
    gap: 8px;
  }
  article {
    min-width: 0;
  }
  article.archived {
    opacity: 0.65;
  }
  .color-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex: 0 0 12px;
  }
  .habit-main {
    display: grid;
    gap: 4px;
    flex: 1;
    min-width: 160px;
  }
  .edit-grid {
    display: grid;
    grid-template-columns: repeat(5, minmax(70px, 1fr));
    gap: 6px;
    min-width: 0;
  }
  .day-selector {
    display: flex;
    gap: 3px;
    flex-wrap: wrap;
  }
  .day-selector.inline {
    margin-top: 4px;
  }
  .day-selector button {
    min-width: 28px;
    min-height: 26px;
    padding: 2px 4px;
    font-size: 10px;
    border-radius: 4px;
  }
  .day-selector button.day-active {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .streak-badge {
    margin-top: 6px;
    font-size: var(--text-xs);
    color: var(--text-secondary);
  }
  .streak-badge strong {
    color: var(--accent);
  }
  .tracker {
    display: grid;
    grid-template-columns: repeat(7, 28px);
    gap: 4px;
  }
  .tracker button {
    width: 28px;
    min-height: 28px;
    padding: 0;
    font-size: 11px;
  }
  .tracker button.done {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .tracker button.partial {
    border-color: #f59e0b;
    background: rgba(245, 158, 11, 0.2);
  }
  .tracker button.skipped {
    border-color: var(--danger);
    color: var(--danger);
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  @media (max-width: 900px) {
    .summary-strip {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    article {
      flex-wrap: wrap;
    }
    .edit-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
