<script lang="ts">
  import { uiStore, type CalendarView } from '$lib/stores/ui.svelte';

  let { currentDate }: { currentDate: Date } = $props();

  function shift(delta: number) {
    const view = uiStore.calendarPrefs.view;
    const next = new Date(currentDate);
    if (view === 'month') next.setMonth(next.getMonth() + delta);
    else if (view === 'week') next.setDate(next.getDate() + delta * 7);
    else next.setDate(next.getDate() + delta);
    uiStore.setCalendarPrefs({ currentDate: next.toISOString() });
  }

  function setView(view: CalendarView) {
    uiStore.setCalendarPrefs({ view });
  }
</script>

<div class="calendar-toolbar">
  <div class="nav-buttons">
    <button onclick={() => uiStore.setCalendarPrefs({ currentDate: new Date().toISOString() })}>
      Today
    </button>
    <button aria-label="Previous" onclick={() => shift(-1)}>‹</button>
    <button aria-label="Next" onclick={() => shift(1)}>›</button>
  </div>

  <strong>
    {currentDate.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })}
  </strong>

  <div class="view-buttons">
    {#each ['month', 'week', 'day', 'agenda'] as view}
      <button
        class:active={uiStore.calendarPrefs.view === view}
        onclick={() => setView(view as CalendarView)}
      >
        {view[0].toUpperCase() + view.slice(1)}
      </button>
    {/each}
    <label>
      First day
      <select
        value={uiStore.calendarPrefs.firstDayOfWeek}
        onchange={(e) =>
          uiStore.setCalendarPrefs({
            firstDayOfWeek: Number((e.target as HTMLSelectElement).value),
          })}
      >
        <option value="0">Sun</option>
        <option value="1">Mon</option>
        <option value="6">Sat</option>
      </select>
    </label>
    <label>
      <input
        type="checkbox"
        checked={uiStore.calendarPrefs.showCompleted}
        onchange={(e) =>
          uiStore.setCalendarPrefs({
            showCompleted: (e.target as HTMLInputElement).checked,
          })}
      />
      Completed
    </label>
  </div>
</div>

<style>
  .calendar-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
    flex-wrap: wrap;
  }
  .nav-buttons,
  .view-buttons {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  button,
  select {
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 5px 8px;
    font-size: var(--text-xs);
    font-weight: 700;
  }
  button:hover,
  button.active {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--accent);
  }
  label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: 700;
  }
</style>
