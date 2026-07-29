import {
  savedFilterCreate,
  savedFilterDelete,
  savedFilterList,
  savedFilterUpdate,
} from '$lib/api/client';
import { nodeStore } from '$lib/stores/nodes.svelte';
import { toast } from '$lib/stores/toast.svelte';
import type { SavedFilter } from '$lib/types/node';
import { normalizeFilterDefinition } from '$lib/utils/productivity';
import type { ProjectionFilters } from '$lib/utils/task-projection';

const state = $state({
  filters: [] as SavedFilter[],
  selectedId: null as string | null,
  isLoading: false,
  error: '',
});

export const savedFilterStore = {
  get filters() {
    return state.filters;
  },
  get selectedId() {
    return state.selectedId;
  },
  get selected() {
    return state.filters.find((filter) => filter.id === state.selectedId) ?? null;
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
      state.filters = await savedFilterList();
      if (state.selectedId && !state.filters.some((filter) => filter.id === state.selectedId)) {
        state.selectedId = null;
      }
      return true;
    } catch (e) {
      state.error = String(e);
      toast.error(`Saved filters failed to load: ${String(e)}`);
      return false;
    } finally {
      state.isLoading = false;
    }
  },

  select(id: string | null) {
    state.selectedId = id;
    const filter = this.selected;
    if (filter) {
      nodeStore.setFilterDraft(filter.definition as ProjectionFilters);
    }
  },

  async saveFromDraft(name: string) {
    const cleanName = name.trim();
    if (!cleanName) {
      toast.error('Filter name is required');
      return null;
    }
    try {
      const created = await savedFilterCreate({
        name: cleanName,
        definition: normalizeFilterDefinition(
          nodeStore.filterDraft as unknown as Record<string, unknown>,
        ),
      });
      state.filters = [...state.filters, created].sort((a, b) => a.position - b.position);
      state.selectedId = created.id;
      toast.success('Filter saved');
      return created;
    } catch (e) {
      toast.error(`Filter save failed: ${String(e)}`);
      return null;
    }
  },

  async update(id: string, input: { name?: string; definition?: ProjectionFilters }) {
    try {
      const updated = await savedFilterUpdate(id, {
        name: input.name,
        definition: input.definition
          ? normalizeFilterDefinition(input.definition as unknown as Record<string, unknown>)
          : undefined,
      });
      state.filters = state.filters.map((filter) => (filter.id === id ? updated : filter));
      return updated;
    } catch (e) {
      toast.error(`Filter update failed: ${String(e)}`);
      return null;
    }
  },

  async delete(id: string) {
    if (!window.confirm('Delete this saved filter?')) return false;
    try {
      await savedFilterDelete(id);
      state.filters = state.filters.filter((filter) => filter.id !== id);
      if (state.selectedId === id) state.selectedId = null;
      toast.info('Filter deleted');
      return true;
    } catch (e) {
      toast.error(`Filter delete failed: ${String(e)}`);
      return false;
    }
  },
};
