<script lang="ts">
  import { uiStore } from '$lib/stores/ui.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import { addDays, localDayKey, monthGrid, parseLocalDayKey } from '$lib/utils/calendar';
  import { onMount } from 'svelte';
  import {
    dateTimeIso,
    formatDurationInput,
    normalizeTypedTime,
    parseDurationMinutes,
    quickDateKey,
    recurrenceChoice,
    recurrenceFromChoice,
    reminderFromChoice,
  } from '$lib/utils/scheduling';
  import { overlayLayer } from '$lib/stores/overlay.svelte';

  // Mounted only while open, so the layer lasts the component's lifetime.
  overlayLayer('due-date-prompt', () => true);

  const timePresets = ['09:00', '12:00', '14:00', '18:00'];
  const durationPresets = ['15m', '30m', '45m', '1h', '1h 30m'];

  let selected = $derived(nodeStore.getSelected());
  let currentDate = $state(new Date());
  let selectedDay = $state('');
  let timeValue = $state('');
  let reminderChoice = $state('none');
  let repeatChoice = $state('none');
  let durationValue = $state('');
  let error = $state('');
  let initializedFor = $state<string | null>(null);
  let dialogEl: HTMLDivElement | undefined = $state();
  let today = $state(new Date());

  let days = $derived(monthGrid(currentDate, uiStore.calendarPrefs.firstDayOfWeek));
  let todayKey = $derived(localDayKey(today));
  let weekdays = $derived(
    days.slice(0, 7).map((day) => day.toLocaleDateString(undefined, { weekday: 'short' })),
  );

  onMount(() => {
    const timer = setInterval(() => {
      today = new Date();
    }, 60_000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    if (!selected || initializedFor === selected.id) return;
    initializedFor = selected.id;
    const due = selected.properties.dueDate ? new Date(selected.properties.dueDate) : null;
    selectedDay = due && !Number.isNaN(due.getTime()) ? localDayKey(due) : '';
    timeValue =
      due && !Number.isNaN(due.getTime()) && (due.getHours() || due.getMinutes())
        ? `${String(due.getHours()).padStart(2, '0')}:${String(due.getMinutes()).padStart(2, '0')}`
        : '';
    currentDate = due && !Number.isNaN(due.getTime()) ? due : new Date();
    reminderChoice = selected.properties.reminders?.[0]
      ? String(selected.properties.reminders[0].offsetMinutes ?? 'at-time')
      : 'none';
    repeatChoice = recurrenceChoice(selected.properties.recurrence);
    durationValue = formatDurationInput(selected.properties.estimatedDurationMinutes);
    queueMicrotask(() => dialogEl?.focus());
  });

  function shiftMonth(delta: number) {
    currentDate = new Date(currentDate.getFullYear(), currentDate.getMonth() + delta, 1);
  }

  function selectQuick(action: string) {
    selectedDay = quickDateKey(action);
    if (!selectedDay) timeValue = '';
    const parsed = selectedDay ? parseLocalDayKey(selectedDay) : null;
    if (parsed) currentDate = parsed;
  }

  function handleGridKeydown(event: KeyboardEvent) {
    if (!selectedDay) selectedDay = todayKey;
    const current = parseLocalDayKey(selectedDay);
    if (!current) return;
    let next: Date | null = null;
    if (event.key === 'ArrowLeft') next = addDays(current, -1);
    if (event.key === 'ArrowRight') next = addDays(current, 1);
    if (event.key === 'ArrowUp') next = addDays(current, -7);
    if (event.key === 'ArrowDown') next = addDays(current, 7);
    if (!next) return;
    event.preventDefault();
    selectedDay = localDayKey(next);
    currentDate = next;
  }

  async function save() {
    error = '';
    if (!selected) return;
    const normalizedTime = normalizeTypedTime(timeValue);
    if (normalizedTime === null) {
      error = 'Enter a valid 24-hour time such as 09:00, 14:30, or 930.';
      return;
    }
    if (normalizedTime && !selectedDay) {
      error = 'A time requires a date.';
      return;
    }
    const dueDate = selectedDay ? dateTimeIso(selectedDay, normalizedTime || '') : null;
    if (selectedDay && !dueDate) {
      error = 'Choose a valid date.';
      return;
    }
    const parsedDuration =
      durationValue.trim() === '' ? undefined : parseDurationMinutes(durationValue.trim());
    if (parsedDuration === null) {
      error = 'Enter a valid duration such as 30m, 1h, 1h 30m, or 90m.';
      return;
    }
    const duration = parsedDuration;
    let reminders;
    try {
      reminders = reminderFromChoice(reminderChoice, dueDate);
    } catch (e) {
      error = String(e instanceof Error ? e.message : e);
      return;
    }
    const saved = await nodeStore.saveScheduling(selected.id, {
      dueDate,
      reminders,
      recurrence: recurrenceFromChoice(repeatChoice),
      estimatedDurationMinutes: duration,
    });
    if (saved) {
      toast.success('Schedule saved');
      uiStore.closeDueDatePrompt();
    }
  }
</script>

<div
  class="overlay"
  role="presentation"
  tabindex="-1"
  onclick={() => uiStore.closeDueDatePrompt()}
  onkeydown={(event) => {
    if (event.key === 'Escape') uiStore.closeDueDatePrompt();
  }}
>
  <div
    bind:this={dialogEl}
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Schedule task"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => {
      event.stopPropagation();
      if (event.key === 'Enter' && event.ctrlKey) save();
      if (event.key === 'Escape') uiStore.closeDueDatePrompt();
    }}
  >
    <header>
      <button aria-label="Previous month" onclick={() => shiftMonth(-1)}>‹</button>
      <strong
        >{currentDate.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })}</strong
      >
      <button aria-label="Next month" onclick={() => shiftMonth(1)}>›</button>
    </header>

    <div
      class="calendar"
      role="grid"
      tabindex="0"
      aria-label="Choose date"
      onkeydown={handleGridKeydown}
    >
      {#each weekdays as weekday}
        <div class="weekday">{weekday}</div>
      {/each}
      {#each days as day}
        {@const key = localDayKey(day)}
        <button
          class="day"
          class:outside={day.getMonth() !== currentDate.getMonth()}
          class:selected={selectedDay === key}
          class:today={todayKey === key}
          aria-label={day.toLocaleDateString()}
          aria-current={todayKey === key ? 'date' : undefined}
          onclick={() => (selectedDay = key)}
        >
          <span>{day.getDate()}</span>
          {#if todayKey === key}<small>Today</small>{/if}
        </button>
      {/each}
    </div>

    <div class="quick-actions">
      <button onclick={() => selectQuick('today')}>Today</button>
      <button onclick={() => selectQuick('tomorrow')}>Tomorrow</button>
      <button onclick={() => selectQuick('3days')}>3 Days Later</button>
      <button onclick={() => selectQuick('sunday')}>This Sunday</button>
      <button onclick={() => selectQuick('none')}>No Date</button>
    </div>

    <div class="settings">
      <label>
        <span>Time</span>
        <input
          value={timeValue}
          placeholder="No time"
          oninput={(event) => (timeValue = (event.target as HTMLInputElement).value)}
        />
      </label>
      <div class="preset-row">
        {#each timePresets as preset}
          <button onclick={() => (timeValue = preset)}>{preset}</button>
        {/each}
        <button onclick={() => (timeValue = '')}>Clear time</button>
      </div>
      <label>
        <span>Reminder</span>
        <select bind:value={reminderChoice}>
          <option value="none">No reminder</option>
          <option value="at-time">At due time</option>
          <option value="-10">10 minutes before</option>
          <option value="-30">30 minutes before</option>
          <option value="-60">1 hour before</option>
        </select>
      </label>
      <label>
        <span>Repeat</span>
        <select bind:value={repeatChoice}>
          <option value="none">No repeat</option>
          <option value="daily">Daily</option>
          <option value="weekly">Weekly</option>
          <option value="monthly">Monthly</option>
        </select>
      </label>
      <label>
        <span>Duration</span>
        <input
          value={durationValue}
          placeholder="No estimate"
          oninput={(event) => (durationValue = (event.target as HTMLInputElement).value)}
        />
      </label>
      <div class="preset-row">
        {#each durationPresets as preset}
          <button onclick={() => (durationValue = preset)}>{preset}</button>
        {/each}
        <button onclick={() => (durationValue = '')}>Clear duration</button>
      </div>
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <footer>
      <button class="secondary" onclick={() => uiStore.closeDueDatePrompt()}>Cancel</button>
      <button class="primary" onclick={save}>Done</button>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1100;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.46);
    padding: 24px;
  }
  .dialog {
    width: min(560px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    background: var(--bg-modal);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 16px;
    box-shadow: 0 18px 54px rgba(0, 0, 0, 0.32);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  header {
    margin-bottom: 12px;
  }
  header strong {
    font-size: var(--text-lg);
  }
  .calendar {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 4px;
  }
  .weekday {
    text-align: center;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: 800;
    padding: 6px 0;
  }
  .day {
    min-height: 48px;
    border: 1px solid transparent;
    border-radius: 7px;
    background: transparent;
    color: var(--text-primary);
    display: grid;
    place-items: center;
    gap: 1px;
    cursor: pointer;
  }
  .day:hover,
  .day:focus-visible {
    border-color: var(--accent);
    outline: none;
  }
  .day.outside {
    color: var(--text-tertiary);
    opacity: 0.65;
  }
  .day.selected {
    background: var(--accent);
    color: white;
    font-weight: 800;
  }
  .day.today {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .day.today:not(.selected) {
    background: var(--accent-subtle);
  }
  .day small {
    font-size: 9px;
    font-weight: 800;
  }
  .quick-actions,
  .preset-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .quick-actions {
    margin: 14px 0;
  }
  .settings {
    display: grid;
    gap: 10px;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    padding: 12px 0;
  }
  label {
    display: grid;
    grid-template-columns: 100px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    color: var(--text-secondary);
  }
  input,
  select {
    min-height: 34px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    padding: 0 10px;
  }
  input:focus,
  select:focus {
    border-color: var(--accent);
    outline: 3px solid var(--accent-subtle);
  }
  button {
    border: 1px solid var(--border);
    border-radius: 6px;
    min-height: 32px;
    padding: 0 12px;
    background: var(--bg-input);
    color: var(--text-primary);
    cursor: pointer;
  }
  button:hover,
  button:focus-visible {
    border-color: var(--accent);
    outline: none;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 800;
  }
  .secondary {
    background: var(--bg-hover);
  }
  footer {
    justify-content: flex-end;
    margin-top: 14px;
  }
  .error {
    margin: 10px 0 0;
    color: var(--danger);
    font-size: var(--text-sm);
    font-weight: 700;
  }
</style>
