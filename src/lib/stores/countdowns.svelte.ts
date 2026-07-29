import {
  countdownArchive,
  countdownCreate,
  countdownDelete,
  countdownList,
  countdownReorder,
  countdownUpdate,
} from '$lib/api/client';
import { toast } from '$lib/stores/toast.svelte';
import type { Countdown, CreateCountdownInput, UpdateCountdownInput } from '$lib/types/node';
export { countdownDelta } from '$lib/utils/productivity';

const state = $state({
  countdowns: [] as Countdown[],
  showArchived: false,
  isLoading: false,
  error: '',
});

export const countdownStore = {
  get countdowns() {
    return state.countdowns;
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
    try {
      state.countdowns = await countdownList(state.showArchived);
      return true;
    } catch (e) {
      state.error = String(e);
      toast.error(`Countdowns failed to load: ${String(e)}`);
      return false;
    } finally {
      state.isLoading = false;
    }
  },

  setShowArchived(value: boolean) {
    state.showArchived = value;
    this.load();
  },

  async create(input: CreateCountdownInput) {
    try {
      const countdown = await countdownCreate(input);
      state.countdowns = [...state.countdowns, countdown].sort((a, b) => a.position - b.position);
      toast.success('Countdown created');
      return countdown;
    } catch (e) {
      toast.error(`Countdown create failed: ${String(e)}`);
      return null;
    }
  },

  async update(id: string, input: UpdateCountdownInput) {
    try {
      const updated = await countdownUpdate(id, input);
      state.countdowns = state.countdowns.map((countdown) => (countdown.id === id ? updated : countdown));
      return updated;
    } catch (e) {
      toast.error(`Countdown update failed: ${String(e)}`);
      return null;
    }
  },

  async archive(id: string, archived: boolean) {
    try {
      await countdownArchive(id, archived);
      await this.load();
      toast.info(archived ? 'Countdown archived' : 'Countdown restored');
      return true;
    } catch (e) {
      toast.error(`Countdown archive failed: ${String(e)}`);
      return false;
    }
  },

  async delete(id: string) {
    if (!window.confirm('Delete this countdown permanently?')) return false;
    try {
      await countdownDelete(id);
      await this.load();
      toast.info('Countdown deleted');
      return true;
    } catch (e) {
      toast.error(`Countdown delete failed: ${String(e)}`);
      return false;
    }
  },

  async reorder(ids: string[]) {
    try {
      state.countdowns = await countdownReorder(ids);
      return true;
    } catch (e) {
      toast.error(`Countdown reorder failed: ${String(e)}`);
      return false;
    }
  },
};
