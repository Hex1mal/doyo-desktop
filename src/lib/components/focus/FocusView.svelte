<script lang="ts">
  import { focusStore, formatFocusDuration } from '$lib/stores/focus.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import { timeBlockCreate } from '$lib/api/client';
  import type { Node, PomodoroPhase } from '$lib/types/node';
  import {
    calculateParetoScore,
    flowtimeBreakSuggestion,
    localDateKey,
  } from '$lib/utils/productivity';
  import MatrixView from './MatrixView.svelte';
  import GTDView from './GTDView.svelte';

  let didLoad = false;
  let mode = $state<'pomodoro' | 'stopwatch'>('pomodoro');
  let selectedTaskId = $state('');
  let note = $state('');
  let cycle = $state(1);
  let methodTab = $state<'focus' | 'timebox' | 'matrix' | 'frog' | 'flowtime' | 'gtd' | 'pareto'>(
    'focus',
  );
  let timeboxStart = $state(new Date().toISOString().slice(0, 16));
  let timeboxMinutes = $state(60);
  let paretoImpact = $state(80);
  let paretoEffort = $state(20);
  let flowtimeBreakMinutes = $state(0);
  let flowtimeBreakVisible = $state(false);
  let didStartTick = false;

  $effect(() => {
    if (didLoad) return;
    didLoad = true;
    focusStore.load();
  });

  $effect(() => {
    if (didStartTick) return;
    didStartTick = true;
    const interval = setInterval(() => focusStore.tick(), 500);
    return () => clearInterval(interval);
  });

  $effect(() => {
    const pendingTaskId = focusStore.pendingTaskId;
    if (!pendingTaskId) return;
    selectedTaskId = pendingTaskId;
    methodTab = 'focus';
    mode = 'pomodoro';
    focusStore.clearPendingTaskFocus();
  });

  function activeLabel() {
    const active = focusStore.active;
    if (!active) return 'Ready';
    if (active.method === 'stopwatch') return `Stopwatch - ${active.state}`;
    if (active.method === 'flowtime') return `Flowtime - ${active.state}`;
    if (active.pomodoroPhase === 'short_break') return `Short break - ${active.state}`;
    if (active.pomodoroPhase === 'long_break') return `Long break - ${active.state}`;
    return `Focus - ${active.state}`;
  }

  function visibleSeconds() {
    const active = focusStore.active;
    if (!active) return 0;
    return active.method === 'pomodoro' ? focusStore.remainingSeconds : focusStore.elapsedSeconds;
  }

  function taskOptions() {
    return [...nodeStore.nodes.values()]
      .filter((node) => node.nodeType === 'Task' && !node.deletedAt)
      .sort((a, b) => a.title.localeCompare(b.title));
  }

  function updateNumber(
    key: 'focusMinutes' | 'shortBreakMinutes' | 'longBreakMinutes',
    value: string,
  ) {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      uiStore.setFocusPrefs({ [key]: Math.max(0.02, parsed) });
    }
  }

  function updateInterval(value: string) {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      uiStore.setFocusPrefs({ longBreakInterval: Math.max(1, Math.round(parsed)) });
    }
  }

  function minutesToSeconds(value: number) {
    return Math.max(1, Math.round(value * 60));
  }

  function breakPhase(): PomodoroPhase {
    return cycle % Math.max(1, uiStore.focusPrefs.longBreakInterval) === 0
      ? 'long_break'
      : 'short_break';
  }

  function startPomodoro(phase: PomodoroPhase) {
    const prefs = uiStore.focusPrefs;
    const minutes =
      phase === 'focus'
        ? prefs.focusMinutes
        : phase === 'long_break'
          ? prefs.longBreakMinutes
          : prefs.shortBreakMinutes;
    focusStore.startPomodoro({
      taskId: selectedTaskId || null,
      plannedSeconds: minutesToSeconds(minutes),
      pomodoroPhase: phase,
      pomodoroCycle: cycle,
      note,
    });
  }

  function stopActive(completed: boolean) {
    focusStore.stop(completed, note);
  }

  function selectedTask() {
    return selectedTaskId ? nodeStore.get(selectedTaskId) : null;
  }

  function customOf(task: Node | null | undefined) {
    return ((task?.properties.custom as Record<string, unknown> | undefined) ?? {}) as Record<
      string,
      unknown
    >;
  }

  async function saveTaskMethod(patch: Record<string, unknown>) {
    if (!selectedTaskId) {
      toast.error('Choose a task first');
      return;
    }
    await nodeStore.setTaskCustom(selectedTaskId, patch);
  }

  async function createTimebox() {
    const task = selectedTask();
    const start = new Date(timeboxStart);
    if (!task || Number.isNaN(start.getTime())) {
      toast.error('Choose a task and valid start time');
      return;
    }
    const end = new Date(start.getTime() + Math.max(5, timeboxMinutes) * 60_000);
    try {
      await timeBlockCreate({
        taskId: task.id,
        title: task.title || 'Timebox',
        startTime: start.toISOString(),
        endTime: end.toISOString(),
        allDay: false,
        notes: note,
      });
      await saveTaskMethod({
        plannedMinutes: Math.max(5, timeboxMinutes),
        timeboxingLastPlannedAt: start.toISOString(),
      });
      toast.success('Timebox created');
    } catch (e) {
      toast.error(`Timebox failed: ${String(e)}`);
    }
  }

  function startFlowtime() {
    if (!selectedTaskId) {
      toast.error('Choose a task first');
      return;
    }
    saveTaskMethod({ flowtimeEnabled: true, flowtimeLastStartedAt: new Date().toISOString() });
    flowtimeBreakVisible = false;
    focusStore.startFlowtime(selectedTaskId, note);
  }

  async function stopFlowtime() {
    const seconds = focusStore.elapsedSeconds;
    const saved = await focusStore.stop(true, note);
    if (saved) {
      flowtimeBreakMinutes = flowtimeBreakSuggestion(seconds);
      flowtimeBreakVisible = true;
    }
  }

  function startFlowtimeBreak(minutes = flowtimeBreakMinutes) {
    flowtimeBreakVisible = false;
    focusStore.startPomodoro({
      taskId: null,
      plannedSeconds: Math.max(1, Math.round(minutes * 60)),
      pomodoroPhase: 'short_break',
      pomodoroCycle: cycle,
      note: 'Flowtime break',
    });
  }

  function setFrog(role: 'primary' | 'secondary') {
    if (!selectedTaskId) {
      toast.error('Choose a task first');
      return;
    }
    const today = localDateKey();
    const tasks = taskOptions();
    const secondary = tasks.filter((task) => {
      const custom = customOf(task);
      return (
        custom.frogDate === today && custom.frogRole === 'secondary' && task.id !== selectedTaskId
      );
    });
    if (role === 'secondary' && secondary.length >= 2) {
      toast.error('Only two secondary frogs are allowed per day');
      return;
    }
    if (role === 'primary') {
      for (const task of tasks) {
        const custom = customOf(task);
        if (
          task.id !== selectedTaskId &&
          custom.frogDate === today &&
          (custom.frogRole === 'primary' || custom.frog === true)
        ) {
          nodeStore.setTaskCustom(task.id, { frog: false, frogRole: null });
        }
      }
    }
    saveTaskMethod({ frogDate: today, frogRole: role, frog: role === 'primary' });
  }

  function clearFrog() {
    saveTaskMethod({ frog: false, frogRole: null });
  }

  function frogsForToday(role?: 'primary' | 'secondary') {
    const today = localDateKey();
    return taskOptions().filter((task) => {
      const custom = customOf(task);
      const taskRole = custom.frogRole ?? (custom.frog === true ? 'primary' : null);
      return custom.frogDate === today && (!role || taskRole === role);
    });
  }

  function frogHistory() {
    return taskOptions()
      .filter((task) => typeof customOf(task).frogDate === 'string')
      .sort((a, b) => String(customOf(b).frogDate).localeCompare(String(customOf(a).frogDate)))
      .slice(0, 12);
  }

  function completedBeforeNormal(task: Node) {
    if (!task.completedAt) return 'Not completed yet';
    const date = localDateKey(new Date(task.completedAt));
    const earlierNormal = taskOptions().some((candidate) => {
      if (!candidate.completedAt || candidate.id === task.id) return false;
      const custom = customOf(candidate);
      const isFrog = custom.frogDate === date;
      return (
        !isFrog &&
        localDateKey(new Date(candidate.completedAt)) === date &&
        candidate.completedAt < task.completedAt!
      );
    });
    return earlierNormal ? 'Completed after another normal task' : 'Completed before normal tasks';
  }

  function paretoCandidates() {
    const tasks = taskOptions();
    return tasks
      .map((task) => {
        const custom = customOf(task);
        const impact = Number(custom.paretoImpact ?? 0);
        const effort = Number(custom.paretoEffort ?? 1);
        const override = custom.paretoOverride === true;
        return { task, impact, effort, override, score: calculateParetoScore(impact, effort) };
      })
      .filter((item) => item.override || item.impact > 0)
      .sort(
        (a, b) =>
          Number(b.override) - Number(a.override) || b.score - a.score || b.impact - a.impact,
      )
      .slice(0, Math.max(1, Math.ceil(tasks.length * 0.2)));
  }

  function paretoExplanation(item: ReturnType<typeof paretoCandidates>[number]) {
    if (item.override) return 'Manually selected as high-impact work.';
    return `Estimated high impact (${item.impact}) relative to effort (${item.effort}). This is a prioritization estimate, not an exact formula.`;
  }
</script>

<section class="focus-view" aria-label="Productivity Methods">
  <div class="focus-toolbar">
    <div>
      <h2>Productivity Methods</h2>
    </div>
    <div class="tabs" aria-label="Productivity method">
      <button class:active={methodTab === 'focus'} onclick={() => (methodTab = 'focus')}
        >Focus</button
      >
      <button class:active={methodTab === 'timebox'} onclick={() => (methodTab = 'timebox')}
        >Timebox</button
      >
      <button class:active={methodTab === 'matrix'} onclick={() => (methodTab = 'matrix')}
        >Matrix</button
      >
      <button class:active={methodTab === 'frog'} onclick={() => (methodTab = 'frog')}>Frog</button>
      <button class:active={methodTab === 'flowtime'} onclick={() => (methodTab = 'flowtime')}
        >Flowtime</button
      >
      <button class:active={methodTab === 'gtd'} onclick={() => (methodTab = 'gtd')}>GTD</button>
      <button class:active={methodTab === 'pareto'} onclick={() => (methodTab = 'pareto')}
        >Pareto</button
      >
    </div>
  </div>

  <div class="focus-grid">
    <section class="timer-panel">
      {#if methodTab === 'focus'}
        <div class="tabs inline-tabs" aria-label="Timer type">
          <button class:active={mode === 'pomodoro'} onclick={() => (mode = 'pomodoro')}
            >Pomodoro</button
          >
          <button class:active={mode === 'stopwatch'} onclick={() => (mode = 'stopwatch')}
            >Stopwatch</button
          >
        </div>
      {/if}

      <div class="timer-face">
        <span>{activeLabel()}</span>
        <strong>{formatFocusDuration(visibleSeconds())}</strong>
        <small>{focusStore.error || focusStore.active?.taskTitle || 'No linked task'}</small>
      </div>

      {#if methodTab === 'focus'}
        <!-- Durations are set once and then rarely touched, so they sit behind a
             disclosure instead of five spinners competing with the timer and the
             start button every time this page opens. -->
        <details class="timer-settings">
          <summary>Timer settings</summary>
          <div class="settings-grid">
            <label
              >Focus <input
                type="number"
                min="0.02"
                step="0.1"
                value={uiStore.focusPrefs.focusMinutes}
                onchange={(event) =>
                  updateNumber('focusMinutes', (event.target as HTMLInputElement).value)}
              /></label
            >
            <label
              >Short break <input
                type="number"
                min="0.02"
                step="0.1"
                value={uiStore.focusPrefs.shortBreakMinutes}
                onchange={(event) =>
                  updateNumber('shortBreakMinutes', (event.target as HTMLInputElement).value)}
              /></label
            >
            <label
              >Long break <input
                type="number"
                min="0.02"
                step="0.1"
                value={uiStore.focusPrefs.longBreakMinutes}
                onchange={(event) =>
                  updateNumber('longBreakMinutes', (event.target as HTMLInputElement).value)}
              /></label
            >
            <label
              >Long interval <input
                type="number"
                min="1"
                step="1"
                value={uiStore.focusPrefs.longBreakInterval}
                onchange={(event) => updateInterval((event.target as HTMLInputElement).value)}
              /></label
            >
            <label>Cycle <input type="number" min="1" step="1" bind:value={cycle} /></label>
          </div>
        </details>
      {/if}

      <label>
        Linked Task/Subtask
        <select bind:value={selectedTaskId} disabled={Boolean(focusStore.active)}>
          <option value="">No linked task</option>
          {#each taskOptions() as task (task.id)}
            <option value={task.id}
              >{task.title || 'Untitled'} - {nodeStore.getKindLabel(task)}</option
            >
          {/each}
        </select>
      </label>

      <label>
        Note
        <textarea rows="3" bind:value={note}></textarea>
      </label>

      {#if focusStore.error}
        <p class="error">{focusStore.error}</p>
      {/if}

      {#if methodTab === 'focus'}
        <!-- Starting and running are different moments. Showing all seven
             controls at once meant most of the row was permanently dimmed and
             the one button that mattered had to be hunted for. Nothing was
             removed: each action still appears whenever it can be used. -->
        {#if focusStore.active}
          <div class="actions">
            {#if focusStore.active.state === 'running'}
              <button class="primary" onclick={() => focusStore.pause()}>Pause</button>
            {:else if focusStore.active.state === 'paused'}
              <button class="primary" onclick={() => focusStore.resume()}>Resume</button>
            {/if}
            {#if focusStore.active.method === 'pomodoro'}
              <button onclick={() => stopActive(true)}>Complete Period</button>
            {/if}
            <button onclick={() => stopActive(focusStore.active?.method === 'stopwatch')}
              >Stop</button
            >
          </div>
        {:else}
          <div class="actions">
            {#if mode === 'pomodoro'}
              <button class="primary" onclick={() => startPomodoro('focus')}>Start Focus</button>
              <button onclick={() => startPomodoro(breakPhase())}>Start Break</button>
            {:else}
              <button
                class="primary"
                onclick={() => focusStore.startStopwatch(selectedTaskId || null, note)}
                >Start Stopwatch</button
              >
            {/if}
          </div>
        {/if}
      {:else if methodTab === 'timebox'}
        <div class="method-panel">
          <label>Start <input type="datetime-local" bind:value={timeboxStart} /></label>
          <label>Minutes <input type="number" min="5" step="5" bind:value={timeboxMinutes} /></label
          >
          <button class="primary" onclick={createTimebox}>Create Calendar Timebox</button>
        </div>
      {:else if methodTab === 'matrix'}
        <MatrixView />
      {:else if methodTab === 'frog'}
        <div class="method-panel">
          <button class="primary" onclick={() => setFrog('primary')}>Set Primary Frog</button>
          <button onclick={() => setFrog('secondary')}>Set Secondary Frog</button>
          <button onclick={clearFrog}>Clear Frog</button>
          <span>Primary: {frogsForToday('primary')[0]?.title ?? 'None'}</span>
          <span>Secondary: {frogsForToday('secondary').length}/2</span>
        </div>
        <div class="method-list">
          <strong>Daily history</strong>
          {#each frogHistory() as task (task.id)}
            <button class="method-row" onclick={() => nodeStore.select(task.id)}>
              <span>{String(customOf(task).frogDate)} · {task.title || 'Untitled'}</span>
              <small>{completedBeforeNormal(task)}</small>
            </button>
          {:else}
            <p class="muted">No frog history yet</p>
          {/each}
        </div>
      {:else if methodTab === 'flowtime'}
        <div class="method-panel">
          <button class="primary" disabled={Boolean(focusStore.active)} onclick={startFlowtime}
            >Start Flowtime</button
          >
          <button
            disabled={!focusStore.active || focusStore.active.state !== 'running'}
            onclick={() => focusStore.pause()}>Pause</button
          >
          <button
            disabled={!focusStore.active || focusStore.active.state !== 'paused'}
            onclick={() => focusStore.resume()}>Resume</button
          >
          <button
            disabled={!focusStore.active || focusStore.active.method !== 'flowtime'}
            onclick={stopFlowtime}>Stop and Save</button
          >
          {#if flowtimeBreakVisible}
            <label
              >Suggested break <input
                type="number"
                min="1"
                step="1"
                bind:value={flowtimeBreakMinutes}
              /></label
            >
            <button onclick={() => startFlowtimeBreak()}>Accept Break</button>
            <button onclick={() => startFlowtimeBreak(flowtimeBreakMinutes)}>Modify Break</button>
            <button onclick={() => (flowtimeBreakVisible = false)}>Skip Break</button>
          {/if}
        </div>
      {:else if methodTab === 'gtd'}
        <GTDView />
      {:else if methodTab === 'pareto'}
        <div class="method-panel">
          <label>Impact <input type="range" min="0" max="100" bind:value={paretoImpact} /></label>
          <label>Effort <input type="range" min="1" max="100" bind:value={paretoEffort} /></label>
          <button
            class="primary"
            onclick={() =>
              saveTaskMethod({
                paretoImpact,
                paretoEffort,
                paretoScore: calculateParetoScore(paretoImpact, paretoEffort),
              })}>Save Pareto Score</button
          >
          <button
            onclick={() =>
              saveTaskMethod({ paretoOverride: !customOf(selectedTask()).paretoOverride })}
          >
            {customOf(selectedTask()).paretoOverride ? 'Remove Manual Override' : 'Manual Override'}
          </button>
        </div>
        <div class="method-list">
          <strong>High-impact shortlist</strong>
          {#each paretoCandidates() as item (item.task.id)}
            <button class="method-row" onclick={() => nodeStore.select(item.task.id)}>
              <span>{item.task.title || 'Untitled'} · score {item.score}</span>
              <small>{paretoExplanation(item)}</small>
            </button>
          {:else}
            <p class="muted">Set impact and effort on tasks to build a shortlist.</p>
          {/each}
        </div>
      {/if}
    </section>

    <aside class="summary-panel" aria-label="Focus summary">
      <div>
        <span>Today</span><strong>{formatFocusDuration(focusStore.summary.todaySeconds)}</strong>
      </div>
      <div>
        <span>Total</span><strong>{formatFocusDuration(focusStore.summary.totalSeconds)}</strong>
      </div>
      <div><span>Pomodoros</span><strong>{focusStore.summary.pomodoroCount}</strong></div>
      <div>
        <span>Stopwatch</span><strong
          >{formatFocusDuration(focusStore.summary.stopwatchSeconds)}</strong
        >
      </div>
      <div>
        <span>Flowtime</span><strong
          >{formatFocusDuration(focusStore.summary.flowtimeSeconds)}</strong
        >
      </div>
    </aside>
  </div>

  <section class="history-panel" aria-label="Focus session history">
    <h3>Session History</h3>
    {#if focusStore.history.length === 0}
      <p class="muted">No completed focus sessions yet</p>
    {:else}
      {#each focusStore.history as session (session.id)}
        <article>
          <div>
            <strong
              >{session.method === 'pomodoro'
                ? 'Pomodoro'
                : session.method === 'flowtime'
                  ? 'Flowtime'
                  : 'Stopwatch'}</strong
            >
            <span>{session.taskTitle || 'No linked task'}</span>
          </div>
          <span>{formatFocusDuration(session.durationSeconds)}</span>
          <span
            >{session.plannedSeconds
              ? `${formatFocusDuration(session.plannedSeconds)} planned`
              : 'Unplanned'}</span
          >
          <span>{session.interruptions} interruptions</span>
        </article>
      {/each}
    {/if}
  </section>
</section>

<style>
  .focus-view {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--bg-app);
    color: var(--text-primary);
  }
  .focus-toolbar,
  .focus-grid,
  .history-panel {
    padding: 14px 18px;
  }
  .focus-toolbar {
    display: flex;
    align-items: center;
    gap: 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .timer-face small,
  .summary-panel span,
  .history-panel span,
  .muted {
    color: var(--text-tertiary);
  }
  .tabs,
  .method-panel,
  .actions,
  .summary-panel > div,
  .history-panel article {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tabs {
    margin-left: auto;
    flex-wrap: wrap;
  }
  .inline-tabs {
    margin-left: 0;
  }
  button,
  input,
  select,
  textarea {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
  }
  button {
    min-height: 32px;
    padding: 0 10px;
    cursor: pointer;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
  button.active,
  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .focus-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 14px;
    min-width: 0;
  }
  .timer-panel,
  .summary-panel,
  .history-panel {
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
  }
  .timer-panel {
    display: grid;
    gap: 14px;
    padding: 14px;
    min-width: 0;
  }
  .timer-face {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 22px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-input);
  }
  .timer-face strong {
    font-size: 44px;
    line-height: 1;
  }
  .timer-settings {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
  }
  .timer-settings summary {
    cursor: pointer;
    font-size: var(--text-xs);
    color: var(--text-secondary);
    list-style: none;
  }
  .timer-settings summary::-webkit-details-marker {
    display: none;
  }
  .timer-settings summary::before {
    content: '▸ ';
    color: var(--text-tertiary);
  }
  .timer-settings[open] summary::before {
    content: '▾ ';
  }
  .timer-settings summary:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 4px;
  }
  .timer-settings[open] .settings-grid {
    margin-top: 10px;
  }
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 10px;
    min-width: 0;
  }
  label {
    display: grid;
    gap: 5px;
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }
  input,
  select {
    min-height: 32px;
    min-width: 0;
    padding: 0 8px;
  }
  textarea {
    min-height: 76px;
    padding: 8px;
    resize: vertical;
  }
  .summary-panel {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 12px;
    min-width: 0;
  }
  .actions {
    flex-wrap: wrap;
  }
  .method-panel {
    flex-wrap: wrap;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-input);
    padding: 10px;
  }
  .method-list {
    display: grid;
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-input);
    padding: 10px;
  }
  .method-row {
    display: grid;
    gap: 3px;
    min-height: 42px;
    text-align: left;
  }
  .method-row small {
    color: var(--text-tertiary);
  }
  .summary-panel > div,
  .history-panel article {
    justify-content: space-between;
    min-height: 40px;
    border-bottom: 1px solid var(--border);
  }
  .history-panel {
    margin: 0 18px 18px;
  }
  .history-panel h3 {
    margin-top: 0;
  }
  .error {
    color: var(--danger);
  }
  @media (max-width: 900px) {
    .focus-grid,
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
