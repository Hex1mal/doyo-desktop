import {
  habitArchive,
  habitCreate,
  habitDelete,
  habitList,
  habitLogDelete,
  habitLogList,
  habitLogUpsert,
  habitSummary,
  habitUpdate,
} from '$lib/api/client';
import { toast } from '$lib/stores/toast.svelte';
import type {
  CreateHabitInput,
  Habit,
  HabitLog,
  HabitLogStatus,
  HabitSummary,
  UpdateHabitInput,
} from '$lib/types/node';
import { localDateKey } from '$lib/utils/productivity';
export { localDateKey } from '$lib/utils/productivity';

const state = $state({
  habits: [] as Habit[],
  logs: [] as HabitLog[],
  summary: {
    activeCount: 0,
    completedToday: 0,
    completionRate: 0,
    bestStreak: 0,
  } as HabitSummary,
  showArchived: false,
  isLoading: false,
  error: '',
});

function addDays(date: Date, days: number) {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
}

function defaultRange() {
  const today = new Date();
  return {
    from: localDateKey(addDays(today, -27)),
    to: localDateKey(today),
  };
}

export const habitStore = {
  get habits() {
    return state.habits;
  },
  get logs() {
    return state.logs;
  },
  get summary() {
    return state.summary;
  },
  get showArchived() {
    return state.showArchived;
  },
  get isLoading() {
    return state.isLoading;
  },
  get error() {
    return state.error;
  },

  async load() {
    state.isLoading = true;
    state.error = '';
    const range = defaultRange();
    try {
      const [habits, logs, summary] = await Promise.all([
        habitList(state.showArchived),
        habitLogList(range.from, range.to),
        habitSummary(range.from, range.to),
      ]);
      state.habits = habits;
      state.logs = logs;
      state.summary = summary;
      return true;
    } catch (e) {
      state.error = String(e);
      toast.error(`Habits failed to load: ${String(e)}`);
      return false;
    } finally {
      state.isLoading = false;
    }
  },

  setShowArchived(value: boolean) {
    state.showArchived = value;
    this.load();
  },

  logFor(habitId: string, date = localDateKey()) {
    return state.logs.find((log) => log.habitId === habitId && log.logDate === date) ?? null;
  },

  async create(input: CreateHabitInput) {
    try {
      const habit = await habitCreate(input);
      state.habits = [...state.habits, habit].sort((a, b) => a.position - b.position);
      await this.load();
      toast.success('Habit created');
      return habit;
    } catch (e) {
      toast.error(`Habit create failed: ${String(e)}`);
      return null;
    }
  },

  async update(id: string, input: UpdateHabitInput) {
    try {
      const updated = await habitUpdate(id, input);
      state.habits = state.habits.map((habit) => (habit.id === id ? updated : habit));
      return updated;
    } catch (e) {
      toast.error(`Habit update failed: ${String(e)}`);
      return null;
    }
  },

  async archive(id: string, archived: boolean) {
    try {
      await habitArchive(id, archived);
      await this.load();
      toast.info(archived ? 'Habit archived' : 'Habit restored');
      return true;
    } catch (e) {
      toast.error(`Habit archive failed: ${String(e)}`);
      return false;
    }
  },

  async delete(id: string) {
    if (!window.confirm('Delete this habit and its logs permanently?')) return false;
    try {
      await habitDelete(id);
      await this.load();
      toast.info('Habit deleted');
      return true;
    } catch (e) {
      toast.error(`Habit delete failed: ${String(e)}`);
      return false;
    }
  },

  async setLog(habitId: string, status: HabitLogStatus, date = localDateKey(), value = 1, note = '') {
    try {
      const log = await habitLogUpsert({ habitId, logDate: date, status, value, note });
      state.logs = [
        ...state.logs.filter((existing) => !(existing.habitId === habitId && existing.logDate === date)),
        log,
      ];
      const range = defaultRange();
      state.summary = await habitSummary(range.from, range.to);
      return log;
    } catch (e) {
      toast.error(`Habit log failed: ${String(e)}`);
      return null;
    }
  },

  async clearLog(habitId: string, date = localDateKey()) {
    try {
      await habitLogDelete(habitId, date);
      state.logs = state.logs.filter((log) => !(log.habitId === habitId && log.logDate === date));
      return true;
    } catch (e) {
      toast.error(`Habit log clear failed: ${String(e)}`);
      return false;
    }
  },
};
