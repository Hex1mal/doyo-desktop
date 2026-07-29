<script lang="ts">
  import { kanbanStore } from '$lib/stores/kanban.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore, type KanbanMode } from '$lib/stores/ui.svelte';
  import { groupKanbanItems, kanbanColumns } from '$lib/utils/kanban';
  import { projectTasks } from '$lib/utils/task-projection';
  import KanbanCard from './KanbanCard.svelte';

  let newStatus = $state('');
  let renamingStatus = $state('');
  let renameValue = $state('');

  let prefs = $derived(uiStore.kanbanPrefs);
  let mode = $derived(prefs.mode);
  let tasks = $derived.by(() => {
    const active = projectTasks([...nodeStore.nodes.values()], {
      mode: 'active',
      sort: 'manual',
      tagAssignments: nodeStore.tagAssignments,
    });
    if (!prefs.showCompleted) return active;
    return [
      ...active,
      ...projectTasks([...nodeStore.nodes.values()], {
        mode: 'completed',
        sort: 'manual',
        tagAssignments: nodeStore.tagAssignments,
      }),
    ];
  });
  let columns = $derived(
    kanbanColumns(mode, tasks, {
      statusColumns: prefs.statusColumns,
      tags: nodeStore.tags,
      nodes: [...nodeStore.nodes.values()],
    }),
  );
  let groups = $derived(groupKanbanItems(columns, mode, tasks));

  function setMode(value: string) {
    uiStore.setKanbanPrefs({ mode: value as KanbanMode });
  }

  function addStatus() {
    const clean = newStatus.trim();
    if (!clean || prefs.statusColumns.includes(clean)) return;
    uiStore.setKanbanPrefs({ statusColumns: [...prefs.statusColumns, clean] });
    newStatus = '';
  }

  async function saveRename(oldStatus: string) {
    const clean = renameValue.trim();
    if (!clean) return;
    const columns = prefs.statusColumns.map((status) => (status === oldStatus ? clean : status));
    const ok = await kanbanStore.renameStatus(oldStatus, clean);
    if (ok) uiStore.setKanbanPrefs({ statusColumns: [...new Set(columns)] });
    renamingStatus = '';
    renameValue = '';
  }
</script>

<section class="kanban-view">
  <header class="kanban-toolbar">
    <div>
      <h2>Kanban</h2>
      <p>Columns are projections of existing tasks. Dragging changes the selected grouping field.</p>
    </div>
    <label>
      Columns
      <select value={mode} onchange={(e) => setMode((e.target as HTMLSelectElement).value)}>
        <option value="status">Status</option>
        <option value="priority">Priority</option>
        <option value="tag">Tag</option>
        <option value="workspace">Workspace</option>
        <option value="group">Group/Subgroup</option>
      </select>
    </label>
    <div class="mode-buttons" aria-label="Kanban grouping mode">
      {#each ['status', 'priority', 'tag', 'workspace', 'group'] as option}
        <button class:active={mode === option} onclick={() => setMode(option)}>
          {option === 'group' ? 'Group' : option[0].toUpperCase() + option.slice(1)}
        </button>
      {/each}
    </div>
    <label class="check-row">
      <input
        type="checkbox"
        checked={prefs.showCompleted}
        onchange={(e) => uiStore.setKanbanPrefs({ showCompleted: (e.target as HTMLInputElement).checked })}
      />
      Completed
    </label>
  </header>

  {#if mode === 'status'}
    <div class="status-editor">
      <input placeholder="New status column" bind:value={newStatus} onkeydown={(e) => e.key === 'Enter' && addStatus()} />
      <button onclick={addStatus}>Add column</button>
    </div>
  {/if}

  <div class="kanban-board">
    {#each groups as group (group.key)}
      <section class="kanban-column" data-kanban-column={group.key}>
        <header>
          {#if mode === 'status' && renamingStatus === group.key}
            <input
              bind:value={renameValue}
              onkeydown={(e) => {
                if (e.key === 'Enter') saveRename(group.key);
                if (e.key === 'Escape') renamingStatus = '';
              }}
            />
          {:else}
            <strong>{group.title}</strong>
          {/if}
          <span>{group.items.length}</span>
          {#if mode === 'status'}
            <button
              title="Rename status"
              onclick={() => {
                renamingStatus = group.key;
                renameValue = group.title;
              }}
            >
              Edit
            </button>
          {/if}
        </header>
        <div class="column-body">
          {#if group.items.length === 0}
            <p class="empty">Drop tasks here</p>
          {/if}
          {#each group.items as item (item.node.id)}
            <KanbanCard {item} {mode} columnKey={group.key} />
          {/each}
        </div>
      </section>
    {/each}
  </div>
</section>

<style>
  .kanban-view {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .kanban-toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .kanban-toolbar h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  .kanban-toolbar p {
    margin: 3px 0 0;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .kanban-toolbar label,
  .status-editor,
  .mode-buttons {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }
  .mode-buttons button {
    height: 28px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .mode-buttons button.active {
    border-color: var(--accent);
    color: var(--text-primary);
    background: var(--accent-subtle);
  }
  .check-row {
    margin-left: auto;
  }
  .status-editor {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .status-editor input,
  .kanban-column header input {
    height: 28px;
    padding: 0 8px;
  }
  .status-editor button,
  .kanban-column header button {
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-secondary);
    border-radius: 5px;
    cursor: pointer;
    height: 28px;
  }
  .kanban-board {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(230px, 280px);
    gap: 10px;
    overflow: auto;
    padding: 12px;
  }
  .kanban-column {
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
    display: flex;
    flex-direction: column;
  }
  .kanban-column > header {
    min-height: 42px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }
  .kanban-column > header span {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .kanban-column > header button {
    margin-left: auto;
    font-size: var(--text-xs);
  }
  .column-body {
    flex: 1;
    min-height: 120px;
    overflow: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .empty {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    margin: 0;
    padding: 8px;
    border: 1px dashed var(--border);
    border-radius: 6px;
    text-align: center;
  }
</style>
