<script lang="ts">
  import type { Node } from '$lib/types/node';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import { overlayLayer } from '$lib/stores/overlay.svelte';

  const node = $derived(
    uiStore.moveDialogNodeId ? (nodeStore.get(uiStore.moveDialogNodeId) ?? null) : null,
  );

  overlayLayer('move-dialog', () => node !== null);
  let error = $state('');
  let selectedIndex = $state(0);
  let previousNodeId = $state<string | null>(null);
  let dialogEl: HTMLElement | undefined = $state();

  function canMoveTo(source: Node, destination: Node, blocked: Set<string>) {
    if (source.id === destination.id || blocked.has(destination.id)) return false;
    if (source.nodeType === 'Workspace') return false;
    if (source.nodeType === 'Group') {
      return destination.nodeType === 'Workspace' || destination.nodeType === 'Group';
    }
    if (source.nodeType === 'Task') {
      return (
        destination.nodeType === 'Workspace' ||
        destination.nodeType === 'Group' ||
        destination.nodeType === 'Task'
      );
    }
    return false;
  }

  const destinations = $derived.by(() => {
    if (!node) return [];
    const blocked = new Set(nodeStore.getDescendants(node.id).map((descendant) => descendant.id));
    return [...nodeStore.nodes.values()]
      .filter((candidate) => canMoveTo(node, candidate, blocked))
      .map((candidate) => ({
        node: candidate,
        path: nodeStore.getPath(candidate.id),
        label: nodeStore.getKindLabel(candidate),
      }))
      .sort((a, b) => a.path.localeCompare(b.path));
  });

  $effect(() => {
    if (node && previousNodeId !== node.id) {
      previousNodeId = node.id;
      selectedIndex = 0;
      error = '';
      queueMicrotask(() => dialogEl?.focus());
    }
    if (selectedIndex >= destinations.length) selectedIndex = Math.max(0, destinations.length - 1);
  });

  function close() {
    error = '';
    previousNodeId = null;
    uiStore.closeMoveDialog();
  }

  async function confirm(destinationId = destinations[selectedIndex]?.node.id) {
    if (!node || !destinationId) return;
    error = '';
    const result = await nodeStore.moveTo(node.id, destinationId);
    if (result.ok) close();
    else error = result.error ?? 'Backend rejected this move. The hierarchy was left unchanged.';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      confirm();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(destinations.length - 1, selectedIndex + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(0, selectedIndex - 1);
    }
  }
</script>

{#if node}
  <div class="overlay" role="presentation" onclick={close} onkeydown={handleKeydown}>
    <div
      bind:this={dialogEl}
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="move-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
    >
      <header>
        <div>
          <h2 id="move-title">Move {nodeStore.getKindLabel(node)}</h2>
          <p>{nodeStore.getPath(node.id)}</p>
        </div>
        <button class="icon-btn" aria-label="Close move dialog" onclick={close}>×</button>
      </header>

      {#if node.nodeType === 'Workspace'}
        <div class="empty">Workspaces cannot be moved under another node.</div>
      {:else if destinations.length === 0}
        <div class="empty">No valid destinations are available.</div>
      {:else}
        <div class="destination-list" role="listbox" aria-label="Move destination">
          {#each destinations as destination, index (destination.node.id)}
            <button
              class="destination"
              class:selected={index === selectedIndex}
              role="option"
              aria-selected={index === selectedIndex}
              onclick={() => {
                selectedIndex = index;
                confirm(destination.node.id);
              }}
              onmouseenter={() => (selectedIndex = index)}
            >
              <span class="kind">{destination.label}</span>
              <span class="path">{destination.path}</span>
            </button>
          {/each}
        </div>
      {/if}

      {#if error}<div class="error">{error}</div>{/if}

      <footer>
        <button class="secondary" onclick={close}>Cancel</button>
        <button class="primary" disabled={destinations.length === 0} onclick={() => confirm()}>
          Move
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1200;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.35);
    padding: 24px;
  }
  .dialog {
    width: min(620px, 100%);
    max-height: min(720px, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
    background: var(--bg-modal);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
  }
  footer {
    border-top: 1px solid var(--border);
    border-bottom: none;
    justify-content: flex-end;
  }
  h2 {
    font-size: var(--text-lg);
    margin: 0 0 4px;
  }
  p {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
  .icon-btn {
    width: 30px;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .destination-list {
    overflow-y: auto;
    padding: 8px;
    min-height: 220px;
  }
  .destination {
    width: 100%;
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    text-align: left;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--text-primary);
    padding: 9px 10px;
    cursor: pointer;
  }
  .destination:hover,
  .destination.selected {
    background: var(--bg-active);
    border-color: var(--accent);
  }
  .kind {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: 800;
    text-transform: uppercase;
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty,
  .error {
    margin: 16px;
    padding: 12px;
    border-radius: 6px;
    background: var(--bg-hover);
    color: var(--text-secondary);
  }
  .error {
    background: rgba(239, 68, 68, 0.12);
    color: var(--danger);
  }
  .primary,
  .secondary {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 14px;
    cursor: pointer;
  }
  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .primary:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .secondary {
    background: var(--bg-input);
    color: var(--text-secondary);
  }
</style>
