<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import { groupProjection, type GroupMode, type SortMode } from '$lib/utils/task-projection';
  import CalendarView from '$lib/components/calendar/CalendarView.svelte';
  import CountdownsView from '$lib/components/countdowns/CountdownsView.svelte';
  import FocusView from '$lib/components/focus/FocusView.svelte';
  import HabitsView from '$lib/components/habits/HabitsView.svelte';
  import KanbanView from '$lib/components/kanban/KanbanView.svelte';
  import SettingsView from '$lib/components/settings/SettingsView.svelte';
  import StatisticsView from '$lib/components/statistics/StatisticsView.svelte';
  import TimelineView from '$lib/components/timeline/TimelineView.svelte';
  import { savedFilterStore } from '$lib/stores/saved-filters.svelte';
  import TreeNode from './TreeNode.svelte';

  let list = $derived(nodeStore.getFlatVisibleList());
  let todayList = $derived(nodeStore.getTodayNodes());
  let inboxList = $derived(nodeStore.getInboxNodes());
  let upcomingList = $derived(nodeStore.getUpcomingNodes());
  let completedList = $derived(nodeStore.getCompletedNodes());
  let listPrefs = $derived(uiStore.getListPrefs(nodeStore.viewMode));
  let listSort = $derived(listPrefs.sort as SortMode);
  let listGroup = $derived(listPrefs.group as GroupMode);
  let upcomingItems = $derived(nodeStore.getTaskProjection('upcoming', listSort));
  let completedItems = $derived(nodeStore.getTaskProjection('completed', listSort));
  let completedSearch = $state('');
  let completedSearchItems = $derived.by(() => {
    const query = completedSearch.trim().toLowerCase();
    if (!query) return completedItems;
    return completedItems.filter((item) => {
      return (
        item.node.title.toLowerCase().includes(query) ||
        item.node.body.toLowerCase().includes(query) ||
        item.path.some((node) => node.title.toLowerCase().includes(query)) ||
        item.tags.some((tag) => tag.name.toLowerCase().includes(query))
      );
    });
  });
  let tagItems = $derived(nodeStore.getTaskProjection('tag', listSort));
  let filteredItems = $derived(nodeStore.getFilteredProjection(listSort));
  let completedGroups = $derived(
    groupProjection(completedSearchItems, listGroup === 'none' ? 'completionPeriod' : listGroup),
  );
  let upcomingGroups = $derived(
    groupProjection(upcomingItems, listGroup === 'none' ? 'due' : listGroup),
  );
  let tagGroups = $derived(groupProjection(tagItems, listGroup));
  let filterGroups = $derived(groupProjection(filteredItems, listGroup));
  let selected = $derived(nodeStore.getSelected());
  let activeRoot = $derived(nodeStore.getFocusedRoot());
  let renderedList = $derived.by(() => {
    if (!activeRoot) return list;
    return [
      { node: activeRoot, depth: 0 },
      ...list.map((item) => ({ node: item.node, depth: item.depth + 1 })),
    ];
  });
  let contextNode = $derived(selected ?? activeRoot);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let collapsedSections = $state(new Set<string>());
  let trashDestinations = $state(new Map<string, string>());
  let saveFilterName = $state('');
  let newTagName = $state('');
  let newTagColor = $state('#64748B');
  let didLoadSavedFilters = false;

  $effect(() => {
    if (didLoadSavedFilters) return;
    didLoadSavedFilters = true;
    savedFilterStore.load();
  });

  function handleSearchInput(value: string) {
    nodeStore.setSearchQuery(value);
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => nodeStore.runSearch(value), 180);
  }

  function openSearchResult(id: string) {
    const ancestors = nodeStore.getAncestors(id);
    nodeStore.setFocusRoot(ancestors[0]?.id ?? id);
    nodeStore.setViewMode('tree');
    nodeStore.expandAncestors(id);
    nodeStore.select(id);
  }

  function plainSnippet(snippet: string) {
    return snippet.replace(/<\/?mark>/g, '');
  }

  function contextTitle() {
    if (!activeRoot) return 'All Workspaces';
    return activeRoot.title || 'Untitled';
  }

  function contextType() {
    if (!activeRoot) return 'Overview';
    return nodeStore.getKindLabel(activeRoot);
  }

  function createActionTitle() {
    if (!contextNode) return 'Create a workspace from the sidebar or empty state';
    return 'Right-click or press Shift+F10 for contextual actions';
  }

  function pathTitle(path: Array<{ title: string }>) {
    return path.map((node) => node.title || 'Untitled').join(' › ');
  }

  function toggleSection(key: string) {
    const next = new Set(collapsedSections);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    collapsedSections = next;
  }

  function updateSort(value: string) {
    uiStore.setListPrefs(nodeStore.viewMode, { sort: value as SortMode });
  }

  function updateGroup(value: string) {
    uiStore.setListPrefs(nodeStore.viewMode, { group: value as GroupMode });
  }

  function updateDensity(value: string) {
    uiStore.setListPrefs(nodeStore.viewMode, {
      density: value === 'compact' ? 'compact' : 'comfortable',
    });
  }

  function validRestoreTargets(node: { nodeType: string; id: string }) {
    return [...nodeStore.nodes.values()]
      .filter((target) => {
        if (target.id === node.id || target.deletedAt) return false;
        if (node.nodeType === 'Workspace') return false;
        if (node.nodeType === 'Group')
          return target.nodeType === 'Workspace' || target.nodeType === 'Group';
        if (node.nodeType === 'Task') {
          return (
            target.nodeType === 'Workspace' ||
            target.nodeType === 'Group' ||
            target.nodeType === 'Task'
          );
        }
        return false;
      })
      .sort((a, b) => nodeStore.getPath(a.id).localeCompare(nodeStore.getPath(b.id)));
  }

  function setTrashDestination(nodeId: string, destinationId: string) {
    const next = new Map(trashDestinations);
    if (destinationId) next.set(nodeId, destinationId);
    else next.delete(nodeId);
    trashDestinations = next;
  }

  function restoreTrashNode(nodeId: string) {
    nodeStore.restoreFromTrash(nodeId, trashDestinations.get(nodeId) ?? null);
  }

  function saveCurrentFilter() {
    savedFilterStore.saveFromDraft(saveFilterName);
    saveFilterName = '';
  }
</script>

<div class="tree-view">
  {#if nodeStore.isLoading}
    <div class="empty-state"><p>Loading workspace…</p></div>
  {:else if uiStore.activeModule === 'calendar'}
    <CalendarView />
  {:else if uiStore.activeModule === 'kanban'}
    <KanbanView />
  {:else if uiStore.activeModule === 'timeline'}
    <TimelineView />
  {:else if uiStore.activeModule === 'productivity'}
    <FocusView />
  {:else if uiStore.activeModule === 'habits'}
    <HabitsView />
  {:else if uiStore.activeModule === 'countdowns'}
    <CountdownsView />
  {:else if uiStore.activeModule === 'statistics'}
    <StatisticsView />
  {:else if uiStore.activeModule === 'settings'}
    <SettingsView />
  {:else if nodeStore.viewMode === 'today'}
    <div class="view-panel">
      <h2 class="view-title">Today</h2>
      {#if todayList.length === 0}
        <div class="empty-state small">
          <p>Nothing due today</p>
          <button class="primary" onclick={() => nodeStore.createWorkspace('My Workspace')}
            >Create workspace</button
          >
        </div>
      {:else}
        <div class="flat-list">
          {#each todayList as node (node.id)}
            <TreeNode
              {node}
              depth={0}
              isSelected={nodeStore.selectedId === node.id}
              isEditing={nodeStore.editingId === node.id}
              flat
            />
          {/each}
        </div>
      {/if}
    </div>
  {:else if nodeStore.viewMode === 'upcoming'}
    <div class="view-panel">
      <div class="view-heading">
        <div>
          <h2 class="view-title">Next 7 Days</h2>
          <p class="view-hint">
            Today plus the next six local calendar days. Overdue tasks are separate.
          </p>
        </div>
      </div>
      <div class="list-controls">
        <label
          >Sort <select
            value={listPrefs.sort}
            onchange={(e) => updateSort((e.target as HTMLSelectElement).value)}
            ><option value="manual">Manual</option><option value="title">Title</option><option
              value="created">Created</option
            ><option value="updated">Updated</option><option value="due">Due date</option><option
              value="priority">Priority</option
            ></select
          ></label
        >
        <label
          >Group <select
            value={listPrefs.group}
            onchange={(e) => updateGroup((e.target as HTMLSelectElement).value)}
            ><option value="none">None</option><option value="workspace">Workspace</option><option
              value="group">Group/Subgroup</option
            ><option value="due">Due date</option><option value="priority">Priority</option><option
              value="tag">Tag</option
            ></select
          ></label
        >
        <label
          >Density <select
            value={listPrefs.density}
            onchange={(e) => updateDensity((e.target as HTMLSelectElement).value)}
            ><option value="comfortable">Comfortable</option><option value="compact">Compact</option
            ></select
          ></label
        >
      </div>
      {#if upcomingItems.length === 0}
        <div class="empty-state small">
          <p>No upcoming tasks</p>
        </div>
      {:else}
        <div class="flat-list density-{listPrefs.density}">
          {#each upcomingGroups as group (group.key)}
            <button class="section-row" onclick={() => toggleSection(`upcoming:${group.key}`)}>
              <span>{collapsedSections.has(`upcoming:${group.key}`) ? '▶' : '▼'}</span>
              <strong>{group.title}</strong>
              <span>{group.items.length}</span>
            </button>
            {#if !collapsedSections.has(`upcoming:${group.key}`)}
              {#each group.items as item (item.node.id)}
                <TreeNode
                  node={item.node}
                  depth={0}
                  isSelected={nodeStore.selectedId === item.node.id}
                  isEditing={nodeStore.editingId === item.node.id}
                  flat
                />
                <div class="projection-meta">{pathTitle(item.path.slice(0, -1)) || 'No path'}</div>
              {/each}
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {:else if nodeStore.viewMode === 'completed'}
    <div class="view-panel">
      <div class="view-heading">
        <h2 class="view-title">Completed</h2>
      </div>
      <div class="list-controls">
        <label>Search <input placeholder="Completed search" bind:value={completedSearch} /></label>
        <label
          >Sort <select
            value={listPrefs.sort}
            onchange={(e) => updateSort((e.target as HTMLSelectElement).value)}
            ><option value="completed">Completion date</option><option value="title">Title</option
            ><option value="created">Created</option><option value="updated">Updated</option><option
              value="due">Due date</option
            ><option value="priority">Priority</option></select
          ></label
        >
        <label
          >Group <select
            value={listPrefs.group}
            onchange={(e) => updateGroup((e.target as HTMLSelectElement).value)}
            ><option value="none">Completion period</option><option value="workspace"
              >Workspace</option
            ><option value="group">Group/Subgroup</option><option value="due">Due date</option
            ><option value="priority">Priority</option><option value="tag">Tag</option></select
          ></label
        >
        <label
          >Density <select
            value={listPrefs.density}
            onchange={(e) => updateDensity((e.target as HTMLSelectElement).value)}
            ><option value="comfortable">Comfortable</option><option value="compact">Compact</option
            ></select
          ></label
        >
      </div>
      {#if completedSearchItems.length === 0}
        <div class="empty-state small">
          <p>
            {completedSearch.trim()
              ? 'No completed tasks match this search'
              : 'No completed tasks yet'}
          </p>
        </div>
      {:else}
        <div class="flat-list density-{listPrefs.density}">
          {#each completedGroups as group (group.key)}
            <button class="section-row" onclick={() => toggleSection(`completed:${group.key}`)}>
              <span>{collapsedSections.has(`completed:${group.key}`) ? '▶' : '▼'}</span>
              <strong>{group.title}</strong>
              <span>{group.items.length}</span>
            </button>
            {#if !collapsedSections.has(`completed:${group.key}`)}
              {#each group.items as item (item.node.id)}
                <TreeNode
                  node={item.node}
                  depth={0}
                  isSelected={nodeStore.selectedId === item.node.id}
                  isEditing={nodeStore.editingId === item.node.id}
                  flat
                />
                <div class="projection-meta">
                  <span>{pathTitle(item.path.slice(0, -1)) || 'No path'}</span>
                  {#if item.node.completedAt}
                    <span>{new Date(item.node.completedAt).toLocaleString()}</span>
                  {/if}
                </div>
              {/each}
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {:else if nodeStore.viewMode === 'trash'}
    <div class="view-panel">
      <div class="view-heading">
        <h2 class="view-title">Trash</h2>
        {#if nodeStore.trashNodes.length > 0}
          <button class="danger-action" onclick={() => nodeStore.emptyTrash()}>Empty Trash</button>
        {/if}
      </div>
      {#if nodeStore.trashNodes.length === 0}
        <div class="empty-state small">
          <p>Trash is empty</p>
        </div>
      {:else}
        <div class="trash-list">
          {#each nodeStore.trashNodes as node (node.id)}
            <div class="trash-row">
              <div class="trash-main">
                <strong>{node.title || 'Untitled'}</strong>
                <span>{nodeStore.getKindLabel(node)}</span>
                {#if node.deletedAt}<span>Deleted {new Date(node.deletedAt).toLocaleString()}</span
                  >{/if}
                <span
                  >{node.parentId
                    ? `Original parent: ${node.parentId.slice(0, 8)}...`
                    : 'Root item'}</span
                >
              </div>
              <div class="trash-actions">
                {#if validRestoreTargets(node).length > 0}
                  <select
                    aria-label="Restore destination"
                    value={trashDestinations.get(node.id) ?? ''}
                    onchange={(e) =>
                      setTrashDestination(node.id, (e.target as HTMLSelectElement).value)}
                  >
                    <option value="">Original parent</option>
                    {#each validRestoreTargets(node) as target (target.id)}
                      <option value={target.id}>{nodeStore.getPath(target.id)}</option>
                    {/each}
                  </select>
                {/if}
                <button onclick={() => restoreTrashNode(node.id)}>Restore</button>
                <button class="danger-action" onclick={() => nodeStore.permanentlyDelete(node.id)}>
                  Delete permanently
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else if nodeStore.viewMode === 'inbox'}
    <div class="view-panel">
      <h2 class="view-title">Inbox</h2>
      {#if inboxList.length === 0}
        <div class="empty-state small">
          <p>Inbox zero</p>
          <button class="primary" onclick={() => nodeStore.createWorkspace('Inbox')}
            >Create Inbox workspace</button
          >
        </div>
      {:else}
        <div class="flat-list">
          {#each inboxList as node (node.id)}
            <TreeNode
              {node}
              depth={0}
              isSelected={nodeStore.selectedId === node.id}
              isEditing={nodeStore.editingId === node.id}
              flat
            />
          {/each}
        </div>
      {/if}
    </div>
  {:else if nodeStore.viewMode === 'tag'}
    <div class="view-panel">
      <div class="view-heading">
        <h2 class="view-title">
          Tag: {nodeStore.tags.find((tag) => tag.id === nodeStore.selectedTagId)?.name ?? 'Unknown'}
        </h2>
      </div>
      <div class="tag-manager">
        <input placeholder="New tag" bind:value={newTagName} />
        <input type="color" bind:value={newTagColor} />
        <button
          onclick={async () => {
            await nodeStore.createTag(newTagName, newTagColor);
            newTagName = '';
          }}>Create Tag</button
        >
        {#each nodeStore.tags as tag (tag.id)}
          <div class="tag-edit">
            <span style={`background: ${tag.color ?? '#64748B'}`}></span>
            <input
              value={tag.name}
              onchange={(event) =>
                nodeStore.renameTag(tag.id, (event.target as HTMLInputElement).value, tag.color)}
            />
            <input
              type="color"
              value={tag.color ?? '#64748B'}
              onchange={(event) =>
                nodeStore.renameTag(tag.id, tag.name, (event.target as HTMLInputElement).value)}
            />
            <button onclick={() => nodeStore.setSelectedTag(tag.id)}>Open</button>
            <button class="danger-action" onclick={() => nodeStore.deleteTag(tag.id)}>Delete</button
            >
          </div>
        {/each}
      </div>
      <div class="list-controls">
        <label
          >Sort <select
            value={listPrefs.sort}
            onchange={(e) => updateSort((e.target as HTMLSelectElement).value)}
            ><option value="manual">Manual</option><option value="title">Title</option><option
              value="created">Created</option
            ><option value="updated">Updated</option><option value="due">Due date</option><option
              value="priority">Priority</option
            ></select
          ></label
        >
        <label
          >Group <select
            value={listPrefs.group}
            onchange={(e) => updateGroup((e.target as HTMLSelectElement).value)}
            ><option value="none">None</option><option value="workspace">Workspace</option><option
              value="group">Group/Subgroup</option
            ><option value="due">Due date</option><option value="priority">Priority</option></select
          ></label
        >
        <label
          >Density <select
            value={listPrefs.density}
            onchange={(e) => updateDensity((e.target as HTMLSelectElement).value)}
            ><option value="comfortable">Comfortable</option><option value="compact">Compact</option
            ></select
          ></label
        >
      </div>
      {#if tagItems.length === 0}
        <div class="empty-state small"><p>No tasks with this tag</p></div>
      {:else}
        <div class="flat-list density-{listPrefs.density}">
          {#each tagGroups as group (group.key)}
            <button class="section-row" onclick={() => toggleSection(`tag:${group.key}`)}>
              <span>{collapsedSections.has(`tag:${group.key}`) ? '▶' : '▼'}</span>
              <strong>{group.title}</strong>
              <span>{group.items.length}</span>
            </button>
            {#if !collapsedSections.has(`tag:${group.key}`)}
              {#each group.items as item (item.node.id)}
                <TreeNode
                  node={item.node}
                  depth={0}
                  isSelected={nodeStore.selectedId === item.node.id}
                  isEditing={nodeStore.editingId === item.node.id}
                  flat
                />
                <div class="projection-meta">{pathTitle(item.path.slice(0, -1)) || 'No path'}</div>
              {/each}
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {:else if nodeStore.viewMode === 'filter'}
    <div class="view-panel">
      <div class="view-heading">
        <h2 class="view-title">Filter Preview</h2>
      </div>
      <div class="filter-builder">
        <label
          >Text <input
            value={nodeStore.filterDraft.text ?? ''}
            oninput={(e) =>
              nodeStore.setFilterDraft({
                ...nodeStore.filterDraft,
                text: (e.target as HTMLInputElement).value,
              })}
          /></label
        >
        <label
          >Priority <select
            value={nodeStore.filterDraft.priority ?? ''}
            onchange={(e) =>
              nodeStore.setFilterDraft({
                ...nodeStore.filterDraft,
                priority: Number((e.target as HTMLSelectElement).value) || null,
              })}
            ><option value="">Any</option><option value="1">P1</option><option value="2">P2</option
            ><option value="3">P3</option><option value="4">P4</option></select
          ></label
        >
        <label
          >Tag <select
            value={nodeStore.filterDraft.tagIds?.[0] ?? ''}
            onchange={(e) => {
              const value = (e.target as HTMLSelectElement).value;
              nodeStore.setFilterDraft({ ...nodeStore.filterDraft, tagIds: value ? [value] : [] });
            }}
            ><option value="">Any</option>{#each nodeStore.tags as tag (tag.id)}<option
                value={tag.id}>{tag.name}</option
              >{/each}</select
          ></label
        >
        <label
          >Due <select
            value={nodeStore.filterDraft.dueState ?? ''}
            onchange={(e) =>
              nodeStore.setFilterDraft({
                ...nodeStore.filterDraft,
                dueState: ((e.target as HTMLSelectElement).value || undefined) as never,
              })}
            ><option value="">Any</option><option value="none">No due date</option><option
              value="due">Has due date</option
            ><option value="overdue">Overdue</option><option value="today">Today</option><option
              value="upcoming">Next 7 Days</option
            ></select
          ></label
        >
        <label>Save <input placeholder="Filter name" bind:value={saveFilterName} /></label>
        <button onclick={saveCurrentFilter}>Save Filter</button>
        <label
          >Saved <select
            value={savedFilterStore.selectedId ?? ''}
            onchange={(e) => savedFilterStore.select((e.target as HTMLSelectElement).value || null)}
            ><option value="">Choose saved filter</option
            >{#each savedFilterStore.filters as filter (filter.id)}<option value={filter.id}
                >{filter.name}</option
              >{/each}</select
          ></label
        >
        {#if savedFilterStore.selectedId}
          <button
            onclick={() =>
              savedFilterStore.update(savedFilterStore.selectedId as string, {
                definition: nodeStore.filterDraft,
              })}>Update Saved</button
          >
          <button
            class="danger-action"
            onclick={() => savedFilterStore.delete(savedFilterStore.selectedId as string)}
            >Delete Saved</button
          >
        {/if}
      </div>
      <div class="list-controls">
        <label
          >Sort <select
            value={listPrefs.sort}
            onchange={(e) => updateSort((e.target as HTMLSelectElement).value)}
            ><option value="manual">Manual</option><option value="title">Title</option><option
              value="created">Created</option
            ><option value="updated">Updated</option><option value="due">Due date</option><option
              value="priority">Priority</option
            ></select
          ></label
        >
        <label
          >Group <select
            value={listPrefs.group}
            onchange={(e) => updateGroup((e.target as HTMLSelectElement).value)}
            ><option value="none">None</option><option value="workspace">Workspace</option><option
              value="group">Group/Subgroup</option
            ><option value="due">Due date</option><option value="priority">Priority</option><option
              value="tag">Tag</option
            ></select
          ></label
        >
        <label
          >Density <select
            value={listPrefs.density}
            onchange={(e) => updateDensity((e.target as HTMLSelectElement).value)}
            ><option value="comfortable">Comfortable</option><option value="compact">Compact</option
            ></select
          ></label
        >
      </div>
      {#if filteredItems.length === 0}
        <div class="empty-state small"><p>No tasks match this filter</p></div>
      {:else}
        <div class="flat-list density-{listPrefs.density}">
          {#each filterGroups as group (group.key)}
            <button class="section-row" onclick={() => toggleSection(`filter:${group.key}`)}>
              <span>{collapsedSections.has(`filter:${group.key}`) ? '▶' : '▼'}</span>
              <strong>{group.title}</strong>
              <span>{group.items.length}</span>
            </button>
            {#if !collapsedSections.has(`filter:${group.key}`)}
              {#each group.items as item (item.node.id)}
                <TreeNode
                  node={item.node}
                  depth={0}
                  isSelected={nodeStore.selectedId === item.node.id}
                  isEditing={nodeStore.editingId === item.node.id}
                  flat
                />
                <div class="projection-meta">{pathTitle(item.path.slice(0, -1)) || 'No path'}</div>
              {/each}
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {:else if nodeStore.viewMode === 'search'}
    <div class="view-panel">
      <h2 class="view-title">Search</h2>
      <input
        class="search-input"
        placeholder="Search tasks, groups, notes…"
        value={nodeStore.searchQuery}
        oninput={(e) => handleSearchInput((e.target as HTMLInputElement).value)}
      />
      {#if nodeStore.isSearchLoading}
        <div class="view-hint">Searching...</div>
      {:else if nodeStore.searchResults.length > 0}
        <div class="search-results">
          {#each nodeStore.searchResults as result (result.node.id)}
            <button class="search-result" onclick={() => openSearchResult(result.node.id)}>
              <span class="result-title">{result.node.title || 'Untitled'}</span>
              {#if result.breadcrumb.length > 0}
                <span class="result-path">{result.breadcrumb.join(' › ')}</span>
              {/if}
              {#if result.snippet}
                <span class="result-snippet">{plainSnippet(result.snippet)}</span>
              {/if}
            </button>
          {/each}
        </div>
      {:else if nodeStore.searchQuery.trim()}
        <div class="empty-state small">
          <p>No matching nodes</p>
        </div>
      {:else}
        <div class="view-hint">Type to search full text over titles, bodies, and tags.</div>
      {/if}
    </div>
  {:else if list.length === 0 && !activeRoot}
    <div class="empty-state">
      <div class="empty-icon">🌱</div>
      <h2>Start your workspace</h2>
      <p>Infinite nesting. Keyboard-first. 100% offline.</p>
      <div class="cta-row">
        <button class="primary" onclick={() => nodeStore.createWorkspace('Personal')}>
          Create workspace
        </button>
        <button class="secondary" onclick={() => nodeStore.createWorkspace('My Workspace')}>
          New workspace
        </button>
      </div>
      <div class="quick-start">
        <div><kbd>Enter</kbd> new sibling</div>
        <div><kbd>Shift+Enter</kbd> new child</div>
        <div><kbd>Tab</kbd> indent · <kbd>Shift+Tab</kbd> outdent</div>
        <div><kbd>Ctrl+D</kbd> due date · <kbd>Ctrl+1–4</kbd> priority</div>
        <div><kbd>Ctrl+K</kbd> command palette · <kbd>Ctrl+P</kbd> jump</div>
        <div><kbd>↑↓</kbd> navigate · <kbd>←→</kbd> collapse/expand</div>
      </div>
    </div>
  {:else}
    <div class="scope-header">
      <div class="scope-title">
        <span class="scope-type">{contextType()}</span>
        <strong>{contextTitle()}</strong>
      </div>
      {#if activeRoot}
        <button
          class="scope-clear"
          onclick={() => {
            nodeStore.setFocusRoot(null);
            nodeStore.select(null);
          }}
        >
          Show all
        </button>
      {/if}
    </div>

    <!-- TOOLBAR -->
    <div class="toolbar">
      <div class="toolbar-context">
        <span>{createActionTitle()}</span>
      </div>
      <div class="toolbar-left">
        {#if !contextNode}
          <button class="tb-btn primary" onclick={() => nodeStore.createWorkspace('My Workspace')}>
            + Workspace
          </button>
        {:else}
          <span class="toolbar-hint">Context menu: right-click or Shift+F10</span>
        {/if}
      </div>

      <div class="toolbar-right">
        {#if selected}
          <button
            class="tb-btn icon"
            title={selected.properties.favorite ? 'Remove favorite' : 'Add favorite'}
            onclick={() => nodeStore.setFavorite(selected.id, !selected.properties.favorite)}
          >
            {selected.properties.favorite ? '★' : '☆'}
          </button>
          {#if selected.nodeType === 'Task' || selected.nodeType === 'Group'}
            <button
              class="tb-btn icon"
              title="Set due date (Ctrl+D)"
              onclick={() => uiStore.openDueDatePrompt()}
            >
              ⏰
            </button>
          {/if}
          {#if selected.nodeType === 'Task'}
            <button
              class="tb-btn"
              title="Toggle complete (Ctrl+Enter)"
              onclick={() => nodeStore.toggleComplete(selected.id)}
            >
              {selected.isCompleted ? '↩ Reopen' : '✓ Done'}
            </button>
          {/if}
        {/if}
      </div>
    </div>

    <!-- TREE LIST -->
    <div class="tree-scroll" role="tree" aria-label="Nodes">
      {#if list.length === 0 && activeRoot}
        <div class="empty-inside">
          <p>No items inside {activeRoot.title || 'this node'}.</p>
          <div class="empty-actions">
            {#if activeRoot.nodeType === 'Workspace'}
              <button class="primary" onclick={() => nodeStore.createGroupUnder(activeRoot.id)}
                >Create Group</button
              >
              <button class="secondary" onclick={() => nodeStore.createTaskUnder(activeRoot.id)}
                >Create Task</button
              >
            {:else if activeRoot.nodeType === 'Group'}
              <button class="primary" onclick={() => nodeStore.createSubgroupUnder(activeRoot.id)}
                >Create Subgroup</button
              >
              <button class="secondary" onclick={() => nodeStore.createTaskUnder(activeRoot.id)}
                >Create Task</button
              >
            {:else if activeRoot.nodeType === 'Task'}
              <button class="primary" onclick={() => nodeStore.createSubtaskUnder(activeRoot.id)}
                >Create Subtask</button
              >
            {/if}
          </div>
        </div>
      {:else}
        {#each renderedList as { node, depth } (node.id)}
          <TreeNode
            {node}
            {depth}
            isSelected={nodeStore.selectedId === node.id}
            isEditing={nodeStore.editingId === node.id}
          />
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tree-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }
  .tree-scroll,
  .flat-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0 24px;
  }
  .scope-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
    flex-shrink: 0;
  }
  .scope-title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .scope-title strong {
    font-size: var(--text-base);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .scope-type {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    text-transform: uppercase;
    font-weight: 700;
  }
  .scope-clear {
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 5px;
    padding: 4px 9px;
    font-size: var(--text-xs);
  }
  .scope-clear:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .view-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding-right: 8px;
  }
  .list-controls,
  .filter-builder,
  .tag-manager {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .list-controls label,
  .filter-builder label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .list-controls select,
  .list-controls input,
  .filter-builder select,
  .filter-builder input,
  .filter-builder button,
  .tag-manager input,
  .tag-manager button {
    min-height: 28px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: var(--text-xs);
    padding: 3px 7px;
  }
  .tag-edit {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }
  .tag-edit span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: 0 0 10px;
  }
  .section-row {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    width: 100%;
    border: none;
    border-top: 1px solid var(--border);
    background: var(--bg-panel);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 7px 12px;
    text-align: left;
    font-size: var(--text-xs);
  }
  .section-row:hover {
    background: var(--bg-hover);
  }
  .projection-meta {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 0 14px 6px 38px;
    color: var(--text-tertiary);
    font-size: 11px;
    min-width: 0;
  }
  .density-compact :global(.tree-node) {
    height: 28px;
  }
  .trash-list {
    display: grid;
    gap: 8px;
    padding: 12px;
    overflow-y: auto;
  }
  .trash-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-panel);
    padding: 10px;
  }
  .trash-main {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .trash-main span {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .trash-actions {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    justify-content: flex-end;
    gap: 6px;
  }
  .trash-actions button,
  .trash-actions select,
  .danger-action {
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 5px 8px;
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .danger-action {
    color: var(--danger);
    border-color: rgba(239, 68, 68, 0.35);
  }

  /* TOOLBAR */
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
    flex-shrink: 0;
    gap: 12px;
    flex-wrap: wrap;
  }
  .toolbar-context {
    flex-basis: 100%;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
  }
  .toolbar-left,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .toolbar-hint {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .tb-btn {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 12px;
    font-size: var(--text-sm);
    background: var(--bg-input);
    color: var(--text-primary);
    cursor: pointer;
    white-space: nowrap;
    font-weight: 500;
    transition: background 0.1s;
  }
  .tb-btn:hover {
    background: var(--bg-hover);
  }
  .tb-btn.primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .tb-btn.primary:hover {
    background: var(--accent-hover);
  }
  .tb-btn.icon {
    padding: 5px 8px;
    font-size: var(--text-base);
  }

  .empty-inside {
    margin: 24px;
    border: 1px dashed var(--border);
    border-radius: 6px;
    padding: 24px;
    text-align: center;
    color: var(--text-secondary);
  }
  .empty-inside p {
    margin-bottom: 12px;
  }
  .empty-actions {
    display: flex;
    justify-content: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  /* VIEW PANELS */
  .view-panel {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px 32px;
  }
  .module-placeholder {
    max-width: 680px;
  }
  .module-kicker {
    color: var(--accent);
    font-size: var(--text-xs);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 8px;
  }
  .view-title {
    font-size: var(--text-xl);
    margin-bottom: 16px;
    font-weight: 600;
  }
  .view-hint {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
    margin-top: 12px;
    padding: 0 4px;
  }
  .search-results {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 12px;
  }
  .search-result {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(140px, 1fr) minmax(0, 220px);
    gap: 4px 12px;
    text-align: left;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-panel);
    color: var(--text-primary);
    cursor: pointer;
  }
  .search-result:hover {
    background: var(--bg-hover);
    border-color: var(--accent);
  }
  .result-title {
    font-size: var(--text-sm);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .result-path {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
  }
  .result-snippet {
    grid-column: 1 / -1;
    color: var(--text-secondary);
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .search-input {
    width: 100%;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: var(--text-base);
    background: var(--bg-input);
    color: var(--text-primary);
    outline: none;
    margin-bottom: 4px;
  }
  .search-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-subtle);
  }

  /* EMPTY STATE */
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 48px 24px;
    color: var(--text-secondary);
  }
  .empty-state.small {
    padding: 32px;
  }
  .empty-icon {
    font-size: 48px;
    margin-bottom: 12px;
  }
  h2 {
    color: var(--text-primary);
    margin-bottom: 8px;
    font-size: var(--text-xl);
  }
  p {
    margin-bottom: 8px;
    color: var(--text-secondary);
  }
  .cta-row {
    display: flex;
    gap: 10px;
    margin: 20px 0 28px;
  }
  button {
    border: none;
    border-radius: 8px;
    padding: 10px 16px;
    cursor: pointer;
    font-size: var(--text-sm);
  }
  .primary {
    background: var(--accent);
    color: white;
  }
  .primary:hover {
    background: var(--accent-hover);
  }
  .secondary {
    background: var(--bg-active);
    color: var(--text-primary);
  }
  .quick-start {
    text-align: left;
    font-size: var(--text-sm);
    color: var(--text-tertiary);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 1px 5px;
    background: var(--bg-active);
    border-radius: 4px;
    color: var(--text-secondary);
  }
</style>
