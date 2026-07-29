<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';

  let crumbs = $derived.by(() => {
    const sel = nodeStore.getSelected();
    if (!sel) {
      if (nodeStore.viewMode === 'today') return [{ id: null, title: 'Today' }];
      if (nodeStore.viewMode === 'inbox') return [{ id: null, title: 'Inbox' }];
      if (nodeStore.viewMode === 'upcoming') return [{ id: null, title: 'Next 7 Days' }];
      if (nodeStore.viewMode === 'completed') return [{ id: null, title: 'Completed' }];
      const moduleTitles: Record<string, string> = {
        calendar: 'Calendar',
        productivity: 'Productivity Methods',
        habits: 'Habits',
        countdowns: 'Countdowns',
        statistics: 'Statistics',
        search: 'Search',
        settings: 'Settings',
      };
      if (moduleTitles[uiStore.activeModule]) {
        return [{ id: null, title: moduleTitles[uiStore.activeModule] }];
      }
      return [{ id: null, title: 'All nodes' }];
    }
    const ancestors = nodeStore.getAncestors(sel.id);
    return [
      ...ancestors.map((a) => ({ id: a.id, title: a.title || 'Untitled' })),
      { id: sel.id, title: sel.title || 'Untitled' },
    ];
  });
</script>

<nav class="breadcrumb" aria-label="Breadcrumb">
  {#each crumbs as c, i}
    {#if i > 0}<span class="sep">›</span>{/if}
    <button
      class="crumb"
      class:current={i === crumbs.length - 1}
      onclick={() => {
        if (c.id) {
          nodeStore.setViewMode('tree');
          nodeStore.expandAncestors(c.id);
          nodeStore.select(c.id);
        }
      }}
    >
      {c.title}
    </button>
  {/each}
</nav>

<style>
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 16px;
    font-size: var(--text-sm);
    overflow-x: auto;
    white-space: nowrap;
  }
  .sep {
    color: var(--text-tertiary);
  }
  .crumb {
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: inherit;
  }
  .crumb:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .crumb.current {
    color: var(--text-primary);
    font-weight: 600;
    cursor: default;
  }
</style>
