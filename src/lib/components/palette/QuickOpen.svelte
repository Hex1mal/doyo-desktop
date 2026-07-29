<script lang="ts">
  import { uiStore } from '$lib/stores/ui.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import * as api from '$lib/api/client';
  import type { Node } from '$lib/types/node';

  let query = $state('');
  let results = $state<Node[]>([]);
  let selectedIdx = $state(0);
  let isLoading = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (uiStore.quickOpenOpen) {
      queueMicrotask(() => inputEl?.focus());
    }
  });

  async function doSearch(q: string) {
    if (!q.trim()) {
      results = [];
      return;
    }
    isLoading = true;
    try {
      results = await api.quickFind(q);
      selectedIdx = 0;
    } catch {
      results = [];
    } finally {
      isLoading = false;
    }
  }

  function onInput(v: string) {
    query = v;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => doSearch(v), 150);
  }

  function selectResult(idx: number) {
    const node = results[idx];
    if (!node) return;
    nodeStore.setViewMode('tree');
    nodeStore.expandAncestors(node.id);
    nodeStore.select(node.id);
    uiStore.closeQuickOpen();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIdx = Math.min(results.length - 1, selectedIdx + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIdx = Math.max(0, selectedIdx - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      selectResult(selectedIdx);
    } else if (e.key === 'Escape') {
      uiStore.closeQuickOpen();
    }
  }
</script>

{#if uiStore.quickOpenOpen}
  <div
    class="overlay"
    role="presentation"
    tabindex="-1"
    onclick={() => uiStore.closeQuickOpen()}
    onkeydown={(e) => {
      if (e.key === 'Escape') uiStore.closeQuickOpen();
    }}
  >
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Quick open"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <input
        bind:this={inputEl}
        placeholder="Search nodes by title..."
        value={query}
        oninput={(e) => onInput((e.target as HTMLInputElement).value)}
        onkeydown={handleKeydown}
      />
      {#if isLoading}
        <div class="status">Searching…</div>
      {:else if results.length > 0}
        <div class="results">
          {#each results as node, i}
            <button
              class="result-item"
              class:selected={i === selectedIdx}
              onclick={() => selectResult(i)}
            >
              <span class="type-tag t{node.nodeType}">{node.nodeType}</span>
              <span class="title">{node.title || '(untitled)'}</span>
              {#if node.parentId}
                <span class="breadcrumb">
                  {nodeStore
                    .getAncestors(node.id)
                    .map((a) => a.title)
                    .join(' › ')}
                </span>
              {/if}
            </button>
          {/each}
        </div>
      {:else if query.trim() && !isLoading}
        <div class="status no-results">No matching nodes</div>
      {:else}
        <div class="status hint">Start typing to search nodes</div>
      {/if}
      <div class="footer">
        <span>↑↓ navigate</span>
        <span>↵ open</span>
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
  .dialog {
    width: 520px;
    max-height: 400px;
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
  .result-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
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
  .result-item:hover {
    background: var(--bg-hover);
  }
  .result-item.selected {
    background: var(--bg-active);
  }
  .type-tag {
    font-size: var(--text-xs);
    padding: 1px 6px;
    border-radius: 3px;
    font-weight: 600;
    flex-shrink: 0;
  }
  .tTask {
    background: var(--priority-p4);
    color: white;
  }
  .tGroup {
    background: var(--accent-subtle);
    color: var(--accent);
  }
  .tNote {
    background: #e8f5e9;
    color: #2e7d32;
  }
  .tWorkspace {
    background: #e3f2fd;
    color: #1565c0;
  }
  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .breadcrumb {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 180px;
    flex-shrink: 0;
  }
  .status {
    padding: var(--space-6);
    text-align: center;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }
  .hint {
    color: var(--text-tertiary);
  }
  .no-results {
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
