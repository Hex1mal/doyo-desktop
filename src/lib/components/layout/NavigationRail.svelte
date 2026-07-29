<script lang="ts">
  import { uiStore, type ActiveModule } from '$lib/stores/ui.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';

  type RailItem = {
    id: ActiveModule;
    label: string;
    icon: string;
    viewMode?: 'tree' | 'today' | 'inbox' | 'upcoming' | 'search';
  };

  const topItems: RailItem[] = [
    { id: 'today', label: 'Today', icon: '◎', viewMode: 'today' },
    { id: 'inbox', label: 'Inbox', icon: '□', viewMode: 'inbox' },
    { id: 'upcoming', label: 'Upcoming', icon: '7', viewMode: 'upcoming' },
    { id: 'workspaces', label: 'Workspaces', icon: 'W', viewMode: 'tree' },
    { id: 'calendar', label: 'Calendar', icon: 'C' },
    { id: 'kanban', label: 'Kanban', icon: 'K' },
    { id: 'timeline', label: 'Timeline', icon: 'T' },
    { id: 'productivity', label: 'Productivity Methods', icon: 'P' },
    { id: 'habits', label: 'Habits', icon: 'H' },
    { id: 'countdowns', label: 'Countdowns', icon: 'D' },
    { id: 'statistics', label: 'Statistics', icon: 'S' },
    { id: 'search', label: 'Search', icon: '⌕', viewMode: 'search' },
  ];

  const bottomItems: RailItem[] = [{ id: 'settings', label: 'Settings', icon: '⚙' }];

  function activate(item: RailItem) {
    uiStore.setActiveModule(item.id);
    if (item.viewMode) {
      nodeStore.setViewMode(item.viewMode);
      if (item.viewMode !== 'tree') {
        nodeStore.select(null);
        nodeStore.setFocusRoot(null);
      }
    }
  }
</script>

<nav class="rail" aria-label="Primary modules">
  <div class="rail-group">
    {#each topItems as item}
      <button
        class="rail-button"
        class:active={uiStore.activeModule === item.id}
        title={item.label}
        aria-label={item.label}
        aria-current={uiStore.activeModule === item.id ? 'page' : undefined}
        onclick={() => activate(item)}
      >
        <span>{item.icon}</span>
      </button>
    {/each}
  </div>

  <div class="rail-group bottom">
    {#each bottomItems as item}
      <button
        class="rail-button"
        class:active={uiStore.activeModule === item.id}
        title={item.label}
        aria-label={item.label}
        aria-current={uiStore.activeModule === item.id ? 'page' : undefined}
        onclick={() => activate(item)}
      >
        <span>{item.icon}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .rail {
    width: 52px;
    flex: 0 0 52px;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 8px 6px;
    overflow: hidden;
  }

  .rail-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .rail-button {
    width: 36px;
    height: 36px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 800;
    line-height: 1;
  }

  .rail-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .rail-button.active {
    background: var(--accent);
    color: white;
  }

  .rail-button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
