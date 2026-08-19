<script lang="ts">
  import { commandPaletteStore } from '$lib/stores/command-palette.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import * as api from '$lib/api/client';
  import { overlayLayer } from '$lib/stores/overlay.svelte';

  overlayLayer('command-palette', () => commandPaletteStore.isOpen);

  interface CommandItem {
    id: string;
    name: string;
    category: string;
    shortcut?: string;
    execute: () => void;
  }

  let commands: CommandItem[] = [
    {
      id: 'node.create',
      name: 'Create Task',
      category: 'Nodes',
      shortcut: 'Ctrl+N',
      execute: async () => {
        await nodeStore.createTask();
        commandPaletteStore.close();
      },
    },
    {
      id: 'node.create-group',
      name: 'Create Group/Subgroup',
      category: 'Nodes',
      execute: async () => {
        await nodeStore.createGroup();
        commandPaletteStore.close();
      },
    },
    {
      id: 'node.create-workspace',
      name: 'Create Workspace',
      category: 'Nodes',
      execute: async () => {
        await nodeStore.createWorkspace('My Workspace');
        commandPaletteStore.close();
      },
    },
    {
      id: 'node.create-subtask',
      name: 'Create Subtask',
      category: 'Nodes',
      execute: async () => {
        await nodeStore.createSubtask();
        commandPaletteStore.close();
      },
    },
    {
      id: 'node.duplicate',
      name: 'Duplicate Node',
      category: 'Nodes',
      execute: async () => {
        const id = nodeStore.selectedId;
        if (id) {
          const dup = await api.nodeDuplicate(id);
          nodeStore.upsert(dup);
          nodeStore.select(dup.id);
        }
        commandPaletteStore.close();
      },
    },
    {
      id: 'node.toggle-complete',
      name: 'Toggle Complete',
      category: 'Nodes',
      shortcut: 'Ctrl+Enter',
      execute: async () => {
        const id = nodeStore.selectedId;
        if (id) {
          await nodeStore.toggleComplete(id);
        }
        commandPaletteStore.close();
      },
    },
    {
      id: 'node.delete',
      name: 'Delete Selected',
      category: 'Nodes',
      shortcut: 'Delete',
      execute: async () => {
        await nodeStore.deleteSelected();
        commandPaletteStore.close();
      },
    },
    {
      id: 'theme.toggle',
      name: 'Toggle Theme',
      category: 'Appearance',
      shortcut: 'Ctrl+Shift+D',
      execute: () => {
        uiStore.toggleTheme();
        commandPaletteStore.close();
      },
    },
    {
      id: 'theme.dark',
      name: 'Switch to Dark Mode',
      category: 'Appearance',
      execute: () => {
        uiStore.setTheme('dark');
        commandPaletteStore.close();
      },
    },
    {
      id: 'theme.light',
      name: 'Switch to Light Mode',
      category: 'Appearance',
      execute: () => {
        uiStore.setTheme('light');
        commandPaletteStore.close();
      },
    },
    {
      id: 'sidebar.toggle',
      name: 'Toggle Sidebar',
      category: 'View',
      shortcut: 'Ctrl+\\',
      execute: () => {
        uiStore.toggleSidebar();
        commandPaletteStore.close();
      },
    },
    {
      id: 'inspector.toggle',
      name: 'Toggle Inspector',
      category: 'View',
      shortcut: 'Ctrl+I',
      execute: () => {
        uiStore.toggleInspector();
        commandPaletteStore.close();
      },
    },
    {
      id: 'focus.toggle',
      name: 'Toggle Focus Mode',
      category: 'View',
      execute: () => {
        uiStore.toggleFocusMode();
        commandPaletteStore.close();
      },
    },
    {
      id: 'view.today',
      name: 'View: Today',
      category: 'View',
      execute: () => {
        nodeStore.setViewMode('today');
        commandPaletteStore.close();
      },
    },
    {
      id: 'view.inbox',
      name: 'View: Inbox',
      category: 'View',
      execute: () => {
        nodeStore.setViewMode('inbox');
        commandPaletteStore.close();
      },
    },
    {
      id: 'view.tree',
      name: 'View: Tree',
      category: 'View',
      execute: () => {
        nodeStore.setViewMode('tree');
        commandPaletteStore.close();
      },
    },
    {
      id: 'export.json',
      name: 'Export as JSON',
      category: 'Export',
      execute: async () => {
        try {
          const json = await api.exportJson();
          await navigator.clipboard.writeText(json);
        } catch {
          // Keep the command palette quiet; Settings provides the full export workflow.
        }
        commandPaletteStore.close();
      },
    },
    {
      id: 'undo',
      name: 'Undo',
      category: 'Edit',
      shortcut: 'Ctrl+Z',
      execute: async () => {
        await nodeStore.undo();
        commandPaletteStore.close();
      },
    },
    {
      id: 'redo',
      name: 'Redo',
      category: 'Edit',
      shortcut: 'Ctrl+Shift+Z',
      execute: async () => {
        await nodeStore.redo();
        commandPaletteStore.close();
      },
    },
    {
      id: 'priority.p1',
      name: 'Set Priority → P1 Critical',
      category: 'Priority',
      shortcut: 'Ctrl+1',
      execute: async () => {
        const id = nodeStore.selectedId;
        if (id) {
          await nodeStore.setPriority(id, 1);
        }
        commandPaletteStore.close();
      },
    },
    {
      id: 'priority.p2',
      name: 'Set Priority → P2 High',
      category: 'Priority',
      shortcut: 'Ctrl+2',
      execute: async () => {
        const id = nodeStore.selectedId;
        if (id) {
          await nodeStore.setPriority(id, 2);
        }
        commandPaletteStore.close();
      },
    },
    {
      id: 'priority.p3',
      name: 'Set Priority → P3 Medium',
      category: 'Priority',
      shortcut: 'Ctrl+3',
      execute: async () => {
        const id = nodeStore.selectedId;
        if (id) {
          await nodeStore.setPriority(id, 3);
        }
        commandPaletteStore.close();
      },
    },
    {
      id: 'priority.p4',
      name: 'Set Priority → P4 Low',
      category: 'Priority',
      shortcut: 'Ctrl+4',
      execute: async () => {
        const id = nodeStore.selectedId;
        if (id) {
          await nodeStore.setPriority(id, 4);
        }
        commandPaletteStore.close();
      },
    },
  ];
  let inputEl: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (commandPaletteStore.isOpen) {
      queueMicrotask(() => inputEl?.focus());
    }
  });

  let filtered = $derived(
    commandPaletteStore.query
      ? commands.filter((c) =>
          c.name.toLowerCase().includes(commandPaletteStore.query.toLowerCase()),
        )
      : commands,
  );

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      commandPaletteStore.moveDown(filtered.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      commandPaletteStore.moveUp();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filtered[commandPaletteStore.selectedIndex]) {
        filtered[commandPaletteStore.selectedIndex].execute();
      }
    } else if (e.key === 'Escape') {
      commandPaletteStore.close();
    }
  }

  let grouped = $derived.by(() => {
    const groups: Record<string, CommandItem[]> = {};
    for (const cmd of filtered) {
      if (!groups[cmd.category]) groups[cmd.category] = [];
      groups[cmd.category].push(cmd);
    }
    return groups;
  });
</script>

{#if commandPaletteStore.isOpen}
  <div
    class="overlay"
    role="presentation"
    tabindex="-1"
    onclick={() => commandPaletteStore.close()}
    onkeydown={(e) => {
      if (e.key === 'Escape') commandPaletteStore.close();
    }}
  >
    <div
      class="palette"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <input
        bind:this={inputEl}
        placeholder="Type a command..."
        value={commandPaletteStore.query}
        oninput={(e) => commandPaletteStore.setQuery((e.target as HTMLInputElement).value)}
        onkeydown={handleKeydown}
      />
      <div class="results">
        {#each Object.entries(grouped) as [category, items]}
          <div class="category-label">{category}</div>
          {#each items as cmd, i}
            <button
              class="command-item"
              class:selected={cmd.id === filtered[commandPaletteStore.selectedIndex]?.id}
              onclick={() => cmd.execute()}
            >
              <span class="cmd-name">{cmd.name}</span>
              {#if cmd.shortcut}
                <kbd>{cmd.shortcut}</kbd>
              {/if}
            </button>
          {/each}
        {/each}

        {#if filtered.length === 0}
          <div class="no-results">No matching commands</div>
        {/if}
      </div>
      <div class="footer">
        <span>↑↓ navigate</span>
        <span>↵ execute</span>
        <span>Esc close</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 15vh;
  }
  .palette {
    width: 560px;
    max-height: 420px;
    background: var(--bg-modal);
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  input {
    width: 100%;
    padding: var(--space-4);
    border: none;
    border-bottom: 1px solid var(--border);
    font-size: var(--text-lg);
    background: transparent;
    color: var(--text-primary);
    outline: none;
  }
  .results {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2);
  }
  .category-label {
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
  }
  .command-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    font-size: var(--text-sm);
    font-family: inherit;
    text-align: left;
  }
  .command-item:hover {
    background: var(--bg-hover);
  }
  .command-item.selected {
    background: var(--bg-active);
  }
  .cmd-name {
    flex: 1;
  }
  kbd {
    font-size: var(--text-xs);
    padding: 1px 5px;
    background: var(--bg-hover);
    border-radius: 4px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }
  .no-results {
    padding: var(--space-8);
    text-align: center;
    color: var(--text-tertiary);
  }
  .footer {
    display: flex;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    border-top: 1px solid var(--border);
    justify-content: center;
  }
</style>
