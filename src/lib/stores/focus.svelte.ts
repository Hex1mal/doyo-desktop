import {
  focusGetActive,
  focusList,
  focusPause,
  focusResume,
  focusStart,
  focusStop,
  focusSummary,
  settingsGet,
} from '$lib/api/client';
import { toast } from '$lib/stores/toast.svelte';
import type { FocusSession, FocusSummary, PomodoroPhase, StartFocusInput } from '$lib/types/node';
import { formatFocusDuration } from '$lib/utils/focus';

const state = $state({
  active: null as FocusSession | null,
  history: [] as FocusSession[],
  summary: {
    todaySeconds: 0,
    totalSeconds: 0,
    pomodoroCount: 0,
    stopwatchSeconds: 0,
    flowtimeSeconds: 0,
  } as FocusSummary,
  now: Date.now(),
  isLoading: false,
  error: '',
  finishingId: null as string | null,
  pendingTaskId: null as string | null,
});

function activeElapsed(session: FocusSession | null, now = state.now) {
  if (!session) return 0;
  if (session.state !== 'running' || !session.lastStartedAt) return session.elapsedSeconds;
  const since = Math.max(0, Math.floor((now - new Date(session.lastStartedAt).getTime()) / 1000));
  return session.accumulatedSeconds + since;
}

function phaseTitle(phase: PomodoroPhase | null) {
  if (phase === 'short_break') return 'Short break';
  if (phase === 'long_break') return 'Long break';
  return 'Focus';
}

async function notify(title: string, body: string) {
  try {
    const prefs = await settingsGet<{ pomodoro?: boolean }>('notification.preferences.v1');
    if (prefs?.pomodoro === false) return;
    const { isPermissionGranted, requestPermission, sendNotification } =
      await import('@tauri-apps/plugin-notification');
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
    if (granted) sendNotification({ title, body });
  } catch {
    // Notification support can be unavailable in dev or on some Linux sessions.
  }
}

async function refreshLists() {
  state.history = await focusList(80);
  state.summary = await focusSummary();
}

async function completeExpiredPomodoro() {
  const session = state.active;
  if (
    !session ||
    session.method !== 'pomodoro' ||
    session.state !== 'running' ||
    session.plannedSeconds <= 0 ||
    state.finishingId === session.id
  ) {
    return;
  }
  const remaining = session.plannedSeconds - activeElapsed(session);
  if (remaining > 0) return;
  state.finishingId = session.id;
  try {
    const completed = await focusStop(session.id, { completed: true, note: session.note });
    state.active = null;
    await refreshLists();
    await notify('Pomodoro period ended', `${phaseTitle(completed.pomodoroPhase)} is complete.`);
    toast.success(`${phaseTitle(completed.pomodoroPhase)} completed`);
  } catch (e) {
    toast.error(`Focus timer could not complete: ${String(e)}`);
  } finally {
    state.finishingId = null;
  }
}

export const focusStore = {
  get active() {
    return state.active;
  },
  get history() {
    return state.history;
  },
  get summary() {
    return state.summary;
  },
  get isLoading() {
    return state.isLoading;
  },
  get error() {
    return state.error;
  },
  get pendingTaskId() {
    return state.pendingTaskId;
  },
  get elapsedSeconds() {
    return activeElapsed(state.active);
  },
  get remainingSeconds() {
    if (!state.active || state.active.plannedSeconds <= 0) return 0;
    return Math.max(0, state.active.plannedSeconds - activeElapsed(state.active));
  },

  tick() {
    state.now = Date.now();
    completeExpiredPomodoro();
  },

  requestTaskFocus(taskId: string) {
    state.pendingTaskId = taskId;
  },

  clearPendingTaskFocus() {
    state.pendingTaskId = null;
  },

  async load() {
    state.isLoading = true;
    state.error = '';
    try {
      state.active = await focusGetActive();
      await refreshLists();
      await completeExpiredPomodoro();
    } catch (e) {
      state.error = String(e);
      toast.error(`Focus data failed to load: ${String(e)}`);
    } finally {
      state.isLoading = false;
    }
  },

  async start(input: StartFocusInput) {
    try {
      state.active = await focusStart(input);
      await refreshLists();
      return true;
    } catch (e) {
      toast.error(`Focus timer could not start: ${String(e)}`);
      return false;
    }
  },

  startPomodoro(input: Omit<StartFocusInput, 'method'>) {
    return this.start({ ...input, method: 'pomodoro' });
  },

  startStopwatch(taskId: string | null, note = '') {
    return this.start({
      method: 'stopwatch',
      taskId,
      plannedSeconds: 0,
      pomodoroPhase: null,
      note,
    });
  },

  startFlowtime(taskId: string | null, note = '') {
    return this.start({ method: 'flowtime', taskId, plannedSeconds: 0, pomodoroPhase: null, note });
  },

  async pause() {
    if (!state.active) return false;
    try {
      state.active = await focusPause(state.active.id);
      await refreshLists();
      return true;
    } catch (e) {
      toast.error(`Focus timer could not pause: ${String(e)}`);
      return false;
    }
  },

  async resume() {
    if (!state.active) return false;
    try {
      state.active = await focusResume(state.active.id);
      return true;
    } catch (e) {
      toast.error(`Focus timer could not resume: ${String(e)}`);
      return false;
    }
  },

  async stop(completed: boolean, note?: string) {
    if (!state.active) return false;
    try {
      const stopped = await focusStop(state.active.id, {
        completed,
        note: note ?? state.active.note,
      });
      state.active = null;
      await refreshLists();
      if (stopped.method === 'pomodoro' && completed) {
        await notify('Pomodoro period ended', `${phaseTitle(stopped.pomodoroPhase)} is complete.`);
      }
      return true;
    } catch (e) {
      toast.error(`Focus timer could not stop: ${String(e)}`);
      return false;
    }
  },
};

export { formatFocusDuration };
