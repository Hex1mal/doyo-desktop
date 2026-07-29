import {
  nodeSetDueDate,
  nodeUpdate,
  timeBlockCreate,
  timeBlockDelete,
  timeBlockList,
  timeBlockUpdate,
} from '$lib/api/client';
import { nodeStore } from '$lib/stores/nodes.svelte';
import { toast } from '$lib/stores/toast.svelte';
import type { Node, TimeBlock, UpdateTimeBlockInput } from '$lib/types/node';
import {
  addMinutes,
  dateWithTime,
  hourFromPointerY,
  moveTaskDate,
  parseLocalDayKey,
  validateTimeRange,
} from '$lib/utils/calendar';

export type CalendarDragPayload = {
  type: 'task' | 'block';
  id: string;
};

const state = $state({
  blocks: [] as TimeBlock[],
  isLoading: false,
  error: null as string | null,
  dragPayload: null as CalendarDragPayload | null,
});

function mergedCustom(node: Node, patch: Record<string, unknown>) {
  const existing =
    node.properties.custom && typeof node.properties.custom === 'object'
      ? node.properties.custom
      : {};
  return { ...existing, ...patch };
}

export const calendarStore = {
  get blocks() {
    return state.blocks;
  },
  get isLoading() {
    return state.isLoading;
  },
  get error() {
    return state.error;
  },
  get dragPayload() {
    return state.dragPayload;
  },

  beginDrag(payload: CalendarDragPayload) {
    state.dragPayload = payload;
    document.body.classList.add('calendar-is-dragging');
  },

  clearDrag() {
    state.dragPayload = null;
    document.body.classList.remove('calendar-is-dragging');
  },

  async load(start: Date, end: Date) {
    state.isLoading = true;
    state.error = null;
    try {
      state.blocks = await timeBlockList(start.toISOString(), end.toISOString());
    } catch (e) {
      state.error = String(e);
      toast.error('Failed to load calendar blocks');
    } finally {
      state.isLoading = false;
    }
  },

  async moveTaskToDay(node: Node, day: Date) {
    const before = node;
    const moved = moveTaskDate(node, day);
    try {
      const updated = await nodeUpdate(node.id, {
        properties: { custom: mergedCustom(node, moved.custom) },
      });
      nodeStore.upsert(await nodeSetDueDate(updated.id, moved.dueDate));
    } catch (e) {
      nodeStore.upsert(before);
      toast.error(`Could not move task: ${String(e)}`);
    }
  },

  async moveTaskToSlot(node: Node, day: Date, hour: number, minute = 0) {
    const before = node;
    const moved = moveTaskDate(node, day, hour, minute);
    try {
      const updated = await nodeUpdate(node.id, {
        properties: {
          estimatedDurationMinutes: node.properties.estimatedDurationMinutes ?? 60,
          custom: mergedCustom(node, moved.custom),
        },
      });
      nodeStore.upsert(await nodeSetDueDate(updated.id, moved.dueDate));
    } catch (e) {
      nodeStore.upsert(before);
      toast.error(`Could not schedule task: ${String(e)}`);
    }
  },

  async createBlock(day: Date, hour = 9, taskId: string | null = null) {
    const start = dateWithTime(day, hour);
    const end = addMinutes(start, 60);
    try {
      const block = await timeBlockCreate({
        taskId,
        title: taskId ? 'Linked task block' : 'Planning block',
        startTime: start.toISOString(),
        endTime: end.toISOString(),
        allDay: false,
        notes: '',
      });
      state.blocks = [...state.blocks, block];
      return block;
    } catch (e) {
      toast.error(`Could not create block: ${String(e)}`);
      return null;
    }
  },

  async updateBlock(id: string, input: UpdateTimeBlockInput) {
    const previous = state.blocks.find((block) => block.id === id);
    try {
      const updated = await timeBlockUpdate(id, input);
      state.blocks = state.blocks.map((block) => (block.id === id ? updated : block));
      return updated;
    } catch (e) {
      if (previous) {
        state.blocks = state.blocks.map((block) => (block.id === id ? previous : block));
      }
      toast.error(`Could not update block: ${String(e)}`);
      return null;
    }
  },

  async moveBlock(id: string, day: Date, hour: number) {
    const block = state.blocks.find((item) => item.id === id);
    if (!block) return null;
    const duration = new Date(block.endTime).getTime() - new Date(block.startTime).getTime();
    const start = dateWithTime(day, hour);
    const end = new Date(start.getTime() + duration);
    if (!validateTimeRange(start, end)) {
      toast.error('Invalid block range');
      return null;
    }
    return this.updateBlock(id, {
      startTime: start.toISOString(),
      endTime: end.toISOString(),
      allDay: false,
    });
  },

  async moveBlockToTime(id: string, day: Date, hour: number, minute = 0) {
    const block = state.blocks.find((item) => item.id === id);
    if (!block) return null;
    const duration = new Date(block.endTime).getTime() - new Date(block.startTime).getTime();
    const start = dateWithTime(day, hour, minute);
    const end = new Date(start.getTime() + duration);
    if (!validateTimeRange(start, end)) {
      toast.error('Invalid block range');
      return null;
    }
    return this.updateBlock(id, {
      startTime: start.toISOString(),
      endTime: end.toISOString(),
      allDay: false,
    });
  },

  async applyDrop(payload: CalendarDragPayload, dropElement: HTMLElement, clientY?: number) {
    const dayKey = dropElement.dataset.calendarDay;
    if (!dayKey) {
      toast.error('Calendar drop target is missing a date');
      return null;
    }
    const day = parseLocalDayKey(dayKey);
    if (!day) {
      toast.error('Calendar drop target has an invalid date');
      return null;
    }

    const dropType = dropElement.dataset.calendarDrop;
    if (dropType === 'slot') {
      const slotTime =
        clientY === undefined
          ? {
              hour: Number(dropElement.dataset.calendarHour),
              minute: 0,
            }
          : hourFromPointerY(clientY, dropElement);
      if (!slotTime || !Number.isFinite(slotTime.hour) || slotTime.hour < 0 || slotTime.hour > 23) {
        toast.error('Calendar drop target has an invalid time');
        return null;
      }
      if (payload.type === 'task') {
        const task = nodeStore.get(payload.id);
        return task ? this.moveTaskToSlot(task, day, slotTime.hour, slotTime.minute) : null;
      }
      return this.moveBlockToTime(payload.id, day, slotTime.hour, slotTime.minute);
    }

    if (dropType === 'day') {
      if (payload.type === 'task') {
        const task = nodeStore.get(payload.id);
        return task ? this.moveTaskToDay(task, day) : null;
      }
      return this.moveBlock(payload.id, day, 9);
    }

    toast.error('Unsupported calendar drop target');
    return null;
  },

  async finishPointerDrop(payload: CalendarDragPayload, clientX: number, clientY: number) {
    const target = document.elementFromPoint(clientX, clientY);
    const dropElement =
      target instanceof HTMLElement ? target.closest<HTMLElement>('[data-calendar-drop]') : null;
    if (!dropElement) {
      toast.error('Drop on a calendar day or time slot');
      return null;
    }
    return this.applyDrop(payload, dropElement, clientY);
  },

  async resizeBlock(id: string, deltaMinutes: number) {
    const block = state.blocks.find((item) => item.id === id);
    if (!block) return null;
    const start = new Date(block.startTime);
    const end = addMinutes(new Date(block.endTime), deltaMinutes);
    if (!validateTimeRange(start, end)) {
      toast.error('Block duration cannot be negative');
      return null;
    }
    return this.updateBlock(id, { endTime: end.toISOString() });
  },

  async linkSelectedTask(id: string) {
    const selected = nodeStore.getSelected();
    if (!selected || selected.nodeType !== 'Task') {
      toast.error('Select a task before linking');
      return null;
    }
    return this.updateBlock(id, { taskId: selected.id });
  },

  async unlinkBlock(id: string) {
    return this.updateBlock(id, { taskId: null });
  },

  async deleteBlock(id: string) {
    try {
      await timeBlockDelete(id);
      state.blocks = state.blocks.filter((block) => block.id !== id);
    } catch (e) {
      toast.error(`Could not delete block: ${String(e)}`);
    }
  },
};
