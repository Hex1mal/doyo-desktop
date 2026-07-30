<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { commandPaletteStore } from '$lib/stores/command-palette.svelte';
  import { savedFilterStore } from '$lib/stores/saved-filters.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import SidebarNode from './SidebarNode.svelte';

  let roots = $derived(nodeStore.getRoots());
  let overdue = $derived(nodeStore.getOverdueCount());
  let todayCount = $derived(nodeStore.getTodayNodes().length);
  let inboxCount = $derived(nodeStore.getInboxNodes().length);
  let upcomingCount = $derived(nodeStore.getUpcomingNodes().length);
  let completedCount = $derived(nodeStore.getCompletedNodes().length);
  let trashCount = $derived(nodeStore.trashNodes.length);
  let favorites = $derived(nodeStore.getFavorites());
  let didLoadSavedFilters = false;

  $effect(() => {
    if (didLoadSavedFilters) return;
    didLoadSavedFilters = true;
    savedFilterStore.load();
  });

  let editingFilterId = $state<string | null>(null);
  let editingFilterName = $state('');

  function openView(
    module: 'today' | 'inbox' | 'upcoming' | 'workspaces' | 'search',
    viewMode:
      'today' | 'inbox' | 'upcoming' | 'tree' | 'search' | 'completed' | 'trash' | 'tag' | 'filter',
  ) {
    uiStore.setActiveModule(module);
    nodeStore.setViewMode(viewMode);
    if (viewMode !== 'tree') {
      nodeStore.select(null);
      nodeStore.setFocusRoot(null);
    }
  }

  function startRenameFilter(id: string, name: string) {
    editingFilterId = id;
    editingFilterName = name;
  }

  function commitRenameFilter() {
    if (editingFilterId && editingFilterName.trim()) {
      savedFilterStore.update(editingFilterId, { name: editingFilterName.trim() });
    }
    editingFilterId = null;
    editingFilterName = '';
  }

  function openTagManager() {
    uiStore.setActiveModule('workspaces');
    nodeStore.setViewMode('tag');
    nodeStore.select(null);
    nodeStore.setFocusRoot(null);
  }

  function openFavorites() {
    nodeStore.openFavoritesView();
  }

  function focusOnEdit(node: HTMLInputElement) {
    node.focus();
  }
</script>

<div class="sidebar-inner">
  <div class="brand-row">
    <div class="brand">Doyo</div>
    <div class="theme-switch" aria-label="Theme">
      <button
        class:active={uiStore.theme === 'light'}
        title="Light mode"
        onclick={() => uiStore.setTheme('light')}
      >
        Light
      </button>
      <button
        class:active={uiStore.theme === 'dark'}
        title="Dark mode"
        onclick={() => uiStore.setTheme('dark')}
      >
        Dark
      </button>
    </div>
  </div>

  <nav class="nav">
    <button
      class="nav-item"
      title="Today"
      aria-label="Today"
      class:active={nodeStore.viewMode === 'today' && uiStore.activeModule === 'today'}
      onclick={() => openView('today', 'today')}
    >
      <span class="icon">◎</span>
      <span>Today</span>
      {#if todayCount > 0}<span class="badge">{todayCount}</span>{/if}
      {#if overdue > 0}<span class="badge danger">{overdue}</span>{/if}
    </button>
    <button
      class="nav-item"
      title="Inbox"
      aria-label="Inbox"
      class:active={nodeStore.viewMode === 'inbox' && uiStore.activeModule === 'inbox'}
      onclick={() => openView('inbox', 'inbox')}
    >
      <span class="icon">↓</span>
      <span>Inbox</span>
      {#if inboxCount > 0}<span class="badge">{inboxCount}</span>{/if}
    </button>
    <button
      class="nav-item"
      title="Next 7 Days"
      aria-label="Next 7 Days"
      class:active={nodeStore.viewMode === 'upcoming' && uiStore.activeModule === 'upcoming'}
      onclick={() => openView('upcoming', 'upcoming')}
    >
      <span class="icon">7</span>
      <span>Next 7 Days</span>
      {#if upcomingCount > 0}<span class="badge">{upcomingCount}</span>{/if}
    </button>
    <button
      class="nav-item"
      title="Completed"
      aria-label="Completed"
      class:active={nodeStore.viewMode === 'completed'}
      onclick={() => {
        uiStore.setActiveModule('workspaces');
        nodeStore.setViewMode('completed');
        nodeStore.select(null);
        nodeStore.setFocusRoot(null);
      }}
    >
      <span class="icon">✓</span>
      <span>Completed</span>
      {#if completedCount > 0}<span class="badge">{completedCount}</span>{/if}
    </button>
    <button
      class="nav-item"
      title="Trash"
      aria-label="Trash"
      class:active={nodeStore.viewMode === 'trash'}
      onclick={() => {
        uiStore.setActiveModule('workspaces');
        nodeStore.setViewMode('trash');
        nodeStore.select(null);
        nodeStore.setFocusRoot(null);
        nodeStore.loadTrash();
      }}
    >
      <span class="icon">⌫</span>
      <span>Trash</span>
      {#if trashCount > 0}<span class="badge">{trashCount}</span>{/if}
    </button>
    <button
      class="nav-item"
      title="Search"
      aria-label="Search"
      onclick={() => uiStore.openQuickOpen()}
    >
      <span class="icon">⌕</span>
      <span>Search</span>
      <kbd>Ctrl+P</kbd>
    </button>
    <button
      class="nav-item"
      title="Filters"
      aria-label="Filters"
      class:active={nodeStore.viewMode === 'filter'}
      onclick={() => {
        uiStore.setActiveModule('workspaces');
        nodeStore.setViewMode('filter');
        nodeStore.select(null);
        nodeStore.setFocusRoot(null);
      }}
    >
      <span class="icon">▦</span>
      <span>Filters</span>
    </button>
    <button
      class="nav-item"
      title="Commands"
      aria-label="Commands"
      onclick={() => commandPaletteStore.open()}
    >
      <span class="icon">⌘</span>
      <span>Commands</span>
      <kbd>Ctrl+K</kbd>
    </button>
    <button
      class="nav-item"
      title="Favorites"
      aria-label="Favorites"
      class:active={nodeStore.viewMode === 'favorites'}
      onclick={openFavorites}
    >
      <span class="icon">★</span>
      <span>Favorites</span>
      {#if favorites.length > 0}<span class="badge">{favorites.length}</span>{/if}
    </button>
  </nav>

  <details class="compact-panel">
    <summary>
      <span>Tags</span>
      <span class="summary-count">{nodeStore.tags.length}</span>
      <button class="add" title="Manage tags" onclick={openTagManager}>⚙</button>
      <button class="add" title="Refresh tags" onclick={() => nodeStore.loadTags()}>↻</button>
    </summary>
    <div class="tags">
      {#each nodeStore.tags as tag (tag.id)}
        <div
          class="tag-row"
          class:active={nodeStore.selectedTagId === tag.id && nodeStore.viewMode === 'tag'}
        >
          <button
            class="tag-select"
            onclick={() => {
              uiStore.setActiveModule('workspaces');
              nodeStore.setSelectedTag(tag.id);
              nodeStore.select(null);
              nodeStore.setFocusRoot(null);
            }}
          >
            <span class="tag-dot" style={tag.color ? `background: ${tag.color}` : ''}></span>
            <span>{tag.name}</span>
          </button>
          <button
            class="row-action danger"
            title="Delete tag"
            onclick={(e) => {
              e.stopPropagation();
              nodeStore.deleteTag(tag.id);
            }}>×</button
          >
        </div>
      {/each}
      {#if nodeStore.tags.length === 0}
        <div class="empty compact">No tags yet</div>
      {/if}
    </div>
  </details>

  <details class="compact-panel">
    <summary>
      <span>Saved Filters</span>
      <span class="summary-count">{savedFilterStore.filters.length}</span>
      <button class="add" title="Refresh filters" onclick={() => savedFilterStore.load()}>↻</button>
    </summary>
    <div class="filters">
      {#each savedFilterStore.filters as filter (filter.id)}
        <div
          class="filter-row"
          class:active={savedFilterStore.selectedId === filter.id &&
            nodeStore.viewMode === 'filter'}
        >
          {#if editingFilterId === filter.id}
            <form
              class="filter-rename-form"
              onsubmit={(e) => {
                e.preventDefault();
                commitRenameFilter();
              }}
            >
              <input
                class="filter-rename-input"
                bind:value={editingFilterName}
                onblur={commitRenameFilter}
                onkeydown={(e) => {
                  if (e.key === 'Escape') {
                    editingFilterId = null;
                    editingFilterName = '';
                  }
                }}
                use:focusOnEdit
              />
            </form>
          {:else}
            <button
              class="filter-select"
              onclick={() => {
                savedFilterStore.select(filter.id);
                openView('workspaces', 'filter');
              }}
            >
              <span>{filter.name}</span>
            </button>
            <button
              class="row-action"
              title="Rename filter"
              onclick={(e) => {
                e.stopPropagation();
                startRenameFilter(filter.id, filter.name);
              }}>✎</button
            >
            <button
              class="row-action danger"
              title="Delete filter"
              onclick={(e) => {
                e.stopPropagation();
                savedFilterStore.delete(filter.id);
              }}>×</button
            >
          {/if}
        </div>
      {/each}
      {#if savedFilterStore.filters.length === 0}
        <div class="empty compact">No saved filters</div>
      {/if}
    </div>
  </details>

  <div class="section-label">
    <span>Workspaces</span>
    <button class="add" title="New workspace" onclick={() => nodeStore.createWorkspace()}>+</button>
  </div>

  <div class="tree">
    {#each roots as ws (ws.id)}
      <SidebarNode node={ws} />
    {/each}

    {#if roots.length === 0}
      <div class="empty">
        <p>No workspaces yet</p>
        <button class="cta" onclick={() => nodeStore.createWorkspace()}>Create one</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .sidebar-inner {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .brand-row {
    padding: 12px 12px 10px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .brand {
    font-weight: 700;
    font-size: var(--text-lg);
    letter-spacing: 0;
    min-width: 0;
  }
  .theme-switch {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    flex-shrink: 0;
  }
  .theme-switch button {
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    font-size: 10px;
    line-height: 1;
    padding: 5px 7px;
  }
  .theme-switch button:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }
  .theme-switch button.active {
    color: var(--text-primary);
    background: var(--bg-active);
    font-weight: 700;
  }
  .nav {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 4px;
  }
  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    width: 100%;
    min-width: 0;
    height: 32px;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: 6px;
    font-size: var(--text-sm);
    color: var(--text-primary);
    text-align: center;
  }
  .nav-item > span:not(.icon):not(.badge) {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .nav-item:hover {
    background: var(--bg-hover);
  }
  .nav-item.active {
    background: var(--bg-active);
    color: var(--accent);
    font-weight: 600;
  }
  .icon {
    width: 18px;
    text-align: center;
    opacity: 0.7;
  }
  .badge {
    position: absolute;
    right: 2px;
    top: 2px;
    margin-left: 0;
    font-size: 10px;
    background: var(--bg-active);
    padding: 1px 6px;
    border-radius: 8px;
    color: var(--text-secondary);
  }
  .badge.danger {
    background: rgba(239, 68, 68, 0.15);
    color: var(--danger);
  }
  kbd {
    display: none;
    margin-left: auto;
    font-size: 10px;
    padding: 1px 4px;
    background: var(--bg-active);
    border-radius: 3px;
    color: var(--text-tertiary);
  }
  .compact-panel {
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .compact-panel summary {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-tertiary);
    font-weight: 700;
    cursor: pointer;
    list-style: none;
  }
  .compact-panel summary::-webkit-details-marker {
    display: none;
  }
  .compact-panel summary::before {
    content: '▸';
    font-size: 10px;
  }
  .compact-panel[open] summary::before {
    content: '▾';
  }
  .summary-count {
    margin-left: auto;
    padding: 1px 6px;
    border-radius: 8px;
    background: var(--bg-active);
    color: var(--text-tertiary);
    font-size: 10px;
  }
  .section-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-tertiary);
    font-weight: 600;
  }
  .add {
    border: none;
    background: var(--bg-active);
    width: 20px;
    height: 20px;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-secondary);
    line-height: 1;
  }
  .add:hover {
    background: var(--accent);
    color: white;
  }
  .tags {
    max-height: 160px;
    overflow-y: auto;
    padding: 0 8px 8px;
  }
  .filters {
    max-height: 140px;
    overflow-y: auto;
    padding: 0 8px 8px;
  }
  .tag-row {
    display: flex;
    align-items: center;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 6px;
    font-size: var(--text-sm);
    text-align: left;
  }
  .tag-row.active {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .tag-select {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    border-radius: 6px;
    padding: 6px 8px;
    font-size: var(--text-sm);
    text-align: left;
  }
  .tag-select:hover {
    background: var(--bg-hover);
  }
  .filter-row {
    display: flex;
    align-items: center;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 6px;
    font-size: var(--text-sm);
    text-align: left;
  }
  .filter-row.active {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .filter-select {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    border-radius: 6px;
    padding: 6px 8px;
    font-size: var(--text-sm);
    text-align: left;
  }
  .filter-select:hover {
    background: var(--bg-hover);
  }
  .row-action {
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    width: 20px;
    height: 20px;
    border-radius: 3px;
    font-size: 11px;
    padding: 0;
    flex-shrink: 0;
  }
  .row-action:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .row-action.danger:hover {
    background: rgba(239, 68, 68, 0.15);
    color: var(--danger);
  }
  .filter-rename-form {
    flex: 1;
    padding: 2px 6px;
  }
  .filter-rename-input {
    width: 100%;
    min-height: 24px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--text-primary);
    padding: 2px 6px;
    font-size: var(--text-sm);
    outline: none;
  }
  .tag-row:hover,
  .tag-row.active,
  .filter-row:hover,
  .filter-row.active {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .tag-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--accent);
    flex-shrink: 0;
  }
  .tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 6px 12px;
  }
  .empty {
    padding: 16px;
    text-align: center;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }
  .empty.compact {
    padding: 8px;
    text-align: left;
  }
  .cta {
    margin-top: 8px;
    border: none;
    background: var(--accent);
    color: white;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: var(--text-sm);
  }
</style>
