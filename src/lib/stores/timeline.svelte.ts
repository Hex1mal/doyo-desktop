import { nodeSetDueDate, nodeUpdate } from '$lib/api/client';
import { nodeStore } from '$lib/stores/nodes.svelte';
import { toast } from '$lib/stores/toast.svelte';
import type { Node } from '$lib/types/node';
import { moveTimelineRange, resizeTimelineEnd, resizeTimelineStart, validateTimelineRange } from '$lib/utils/timeline';

async function persistRange(node: Node, range: { startDate: string | null; dueDate: string }) {
  const before = node;
  try {
    if (range.startDate) {
      const updated = await nodeUpdate(node.id, { properties: { startDate: range.startDate } });
      nodeStore.upsert(updated);
    } else if (node.properties.startDate) {
      const updated = await nodeUpdate(node.id, { properties: { startDate: null } });
      nodeStore.upsert(updated);
    }
    nodeStore.upsert(await nodeSetDueDate(node.id, range.dueDate));
    return true;
  } catch (e) {
    nodeStore.upsert(before);
    toast.error(`Timeline update failed: ${String(e)}`);
    return false;
  }
}

export const timelineStore = {
  async moveTask(node: Node, dayDelta: number) {
    const range = moveTimelineRange(node, dayDelta);
    if (!range || !range.dueDate) {
      toast.error('Task needs a valid due date before it can move on the timeline');
      return false;
    }
    return persistRange(node, { startDate: range.startDate, dueDate: range.dueDate });
  },

  async resizeStart(node: Node, dayDelta: number) {
    const range = resizeTimelineStart(node, dayDelta);
    if (!range || !range.dueDate || !range.startDate) {
      toast.error('Timeline start cannot move after the due date');
      return false;
    }
    return persistRange(node, { startDate: range.startDate, dueDate: range.dueDate });
  },

  async resizeEnd(node: Node, dayDelta: number) {
    const range = resizeTimelineEnd(node, dayDelta);
    if (!range || !range.dueDate) {
      toast.error('Timeline end cannot move before the start date');
      return false;
    }
    const startDate = range.startDate ?? node.properties.startDate ?? null;
    const start = startDate ? new Date(startDate) : new Date(range.dueDate);
    const end = new Date(range.dueDate);
    if (!validateTimelineRange(start, end)) {
      toast.error('Timeline range is invalid');
      return false;
    }
    return persistRange(node, { startDate, dueDate: range.dueDate });
  },
};
