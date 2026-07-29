import { countdownStore } from '$lib/stores/countdowns.svelte';
import { habitStore } from '$lib/stores/habits.svelte';
import { toast } from '$lib/stores/toast.svelte';
import { dueCountdownReminders, dueHabitReminders, type DueReminder } from '$lib/utils/reminders';
import { settingsGet } from '$lib/api/client';

const SENT_KEY = 'doyo.sentReminderKeys.v1';
const LEGACY_SENT_KEY = 'todoapp.sentReminderKeys.v1';

const state = $state({
  started: false,
  intervalId: 0,
  sent: new Set<string>(),
  lastError: '',
});

function loadSent() {
  try {
    const raw =
      window.localStorage.getItem(SENT_KEY) ?? window.localStorage.getItem(LEGACY_SENT_KEY);
    if (!window.localStorage.getItem(SENT_KEY) && raw) window.localStorage.setItem(SENT_KEY, raw);
    state.sent = new Set(raw ? (JSON.parse(raw) as string[]) : []);
  } catch {
    state.sent = new Set();
  }
}

function persistSent() {
  window.localStorage.setItem(SENT_KEY, JSON.stringify([...state.sent].slice(-500)));
}

async function send(reminder: DueReminder) {
  try {
    const { isPermissionGranted, requestPermission, sendNotification } =
      await import('@tauri-apps/plugin-notification');
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
    if (granted) {
      sendNotification({ title: reminder.title, body: reminder.body });
    } else {
      toast.info(reminder.title);
    }
    state.sent.add(reminder.key);
    persistSent();
  } catch (e) {
    state.lastError = String(e);
    toast.info(reminder.title);
    state.sent.add(reminder.key);
    persistSent();
  }
}

async function checkDue() {
  await Promise.all([habitStore.load(), countdownStore.load()]);
  const prefs = await settingsGet<{ habits?: boolean; countdowns?: boolean }>(
    'notification.preferences.v1',
  ).catch(() => null);
  const reminders = [
    ...(prefs?.habits === false
      ? []
      : dueHabitReminders(habitStore.habits, new Date(), state.sent)),
    ...(prefs?.countdowns === false
      ? []
      : dueCountdownReminders(countdownStore.countdowns, new Date(), state.sent)),
  ];
  for (const reminder of reminders) {
    await send(reminder);
  }
}

export const reminderStore = {
  get started() {
    return state.started;
  },
  get lastError() {
    return state.lastError;
  },

  start() {
    if (state.started || typeof window === 'undefined') return;
    state.started = true;
    loadSent();
    checkDue();
    state.intervalId = window.setInterval(checkDue, 60_000);
  },

  stop() {
    if (state.intervalId) window.clearInterval(state.intervalId);
    state.intervalId = 0;
    state.started = false;
  },
};
