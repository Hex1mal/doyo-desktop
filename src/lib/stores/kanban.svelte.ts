import { nodeMove, nodeSetPriority, nodeUpdate, tagAssign, tagRemove } from '$lib/api/client';
import { nodeStore } from '$lib/stores/nodes.svelte';
import { toast } from '$lib/stores/toast.svelte';
import type { KanbanMode } from '$lib/stores/ui.svelte';
import type { Node } from '$lib/types/node';
import { mergeCustomStatus } from '$lib/utils/kanban';

function tagsFor(nodeId: string) {
  return nodeStore.getTagObjects(nodeId);
}

export const kanbanStore = {
  async moveTask(task: Node, mode: KanbanMode, columnKey: string, sourceColumnKey?: string) {
    try {
      if (mode === 'status') {
        const updated = await nodeUpdate(task.id, {
          properties: { custom: mergeCustomStatus(task, columnKey) },
        });
        nodeStore.upsert(updated);
      } else if (mode === 'priority') {
        const priority = Number(columnKey);
        if (!Number.isFinite(priority) || priority < 1 || priority > 4) {
          throw new Error('Invalid priority column');
        }
        nodeStore.upsert(await nodeSetPriority(task.id, priority));
      } else if (mode === 'tag') {
        const current = tagsFor(task.id);
        if (columnKey === 'none') {
          for (const tag of current) {
            if (!tag.id.startsWith('legacy:')) await tagRemove(task.id, tag.id);
          }
          await nodeStore.loadTags();
        } else {
          if (sourceColumnKey && sourceColumnKey !== 'none' && sourceColumnKey !== columnKey) {
            const source = current.find((tag) => tag.id === sourceColumnKey);
            if (source && !source.id.startsWith('legacy:')) await tagRemove(task.id, source.id);
          }
          if (!current.some((tag) => tag.id === columnKey)) await tagAssign(task.id, columnKey);
          await nodeStore.loadTags();
        }
      } else if (mode === 'workspace' || mode === 'group') {
        await nodeMove(task.id, columnKey, 999999);
        await nodeStore.load();
        nodeStore.expand(columnKey);
      }
    } catch (e) {
      toast.error(`Kanban move failed: ${String(e)}`);
      throw e;
    }
  },

  async renameStatus(oldStatus: string, nextStatus: string) {
    const clean = nextStatus.trim();
    if (!clean) {
      toast.error('Status title is required');
      return false;
    }
    try {
      const tasks = [...nodeStore.nodes.values()].filter((node) => {
        const custom = node.properties.custom;
        return (
          node.nodeType === 'Task' &&
          !node.deletedAt &&
          custom &&
          typeof custom === 'object' &&
          !Array.isArray(custom) &&
          custom.status === oldStatus
        );
      });
      for (const task of tasks) {
        nodeStore.upsert(
          await nodeUpdate(task.id, {
            properties: { custom: mergeCustomStatus(task, clean) },
          }),
        );
      }
      return true;
    } catch (e) {
      toast.error(`Status rename failed: ${String(e)}`);
      return false;
    }
  },
};
