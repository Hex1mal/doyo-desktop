<script lang="ts">
  import type { Node } from '$lib/types/node';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';
  import { overlayLayer } from '$lib/stores/overlay.svelte';

  const colorOptions = ['#6366F1', '#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6'];
  const viewOptions = ['list', 'kanban', 'timeline', 'calendar'];

  const node = $derived(
    uiStore.configDialogNodeId ? (nodeStore.get(uiStore.configDialogNodeId) ?? null) : null,
  );

  overlayLayer(
    'node-config-dialog',
    () => node !== null && (node.nodeType === 'Workspace' || node.nodeType === 'Group'),
  );
  let title = $state('');
  let icon = $state('');
  let color = $state('');
  let defaultView = $state('list');
  let parentId = $state('');
  let error = $state('');
  let previousNodeId = $state<string | null>(null);
  let titleInputEl: HTMLInputElement | undefined = $state();

  function canParentGroup(source: Node, destination: Node, blocked: Set<string>) {
    if (source.id === destination.id || blocked.has(destination.id)) return false;
    return destination.nodeType === 'Workspace' || destination.nodeType === 'Group';
  }

  const parentOptions = $derived.by(() => {
    if (!node || node.nodeType !== 'Group') return [];
    const blocked = new Set(nodeStore.getDescendants(node.id).map((descendant) => descendant.id));
    return [...nodeStore.nodes.values()]
      .filter((candidate) => canParentGroup(node, candidate, blocked))
      .map((candidate) => ({
        id: candidate.id,
        label: nodeStore.getKindLabel(candidate),
        path: nodeStore.getPath(candidate.id),
      }))
      .sort((a, b) => a.path.localeCompare(b.path));
  });

  $effect(() => {
    if (!node || previousNodeId === node.id) return;
    previousNodeId = node.id;
    title = node.title;
    icon = node.properties.icon ?? '';
    color = node.properties.color ?? '';
    defaultView =
      typeof node.properties.custom?.defaultView === 'string'
        ? node.properties.custom.defaultView
        : 'list';
    parentId = node.parentId ?? '';
    error = '';
    queueMicrotask(() => {
      titleInputEl?.focus();
      titleInputEl?.select();
    });
  });

  function close() {
    error = '';
    previousNodeId = null;
    uiStore.closeConfigDialog();
  }

  async function save() {
    if (!node) return;
    const cleanTitle = title.trim();
    if (!cleanTitle) {
      error = 'Title is required.';
      return;
    }
    const result = await nodeStore.configureNode(node.id, {
      title: cleanTitle,
      icon: icon.trim(),
      color,
      defaultView,
      parentId: node.nodeType === 'Group' ? parentId : undefined,
    });
    if (result.ok) close();
    else error = result.error ?? 'Unable to save configuration.';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'Enter' && !(e.target instanceof HTMLTextAreaElement)) {
      e.preventDefault();
      save();
    }
  }
</script>

{#if node && (node.nodeType === 'Workspace' || node.nodeType === 'Group')}
  <div class="overlay" role="presentation" onclick={close} onkeydown={handleKeydown}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="config-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
    >
      <header>
        <div>
          <h2 id="config-title">Configure {nodeStore.getKindLabel(node)}</h2>
          <p>{nodeStore.getPath(node.id)}</p>
        </div>
        <button class="icon-btn" aria-label="Close configuration dialog" onclick={close}>×</button>
      </header>

      <div class="content">
        <form
          class="fields"
          onsubmit={(e) => {
            e.preventDefault();
            save();
          }}
        >
          <label>
            <span>Title</span>
            <input bind:this={titleInputEl} bind:value={title} required />
          </label>
          <label>
            <span>Icon</span>
            <input bind:value={icon} maxlength="8" placeholder="W, book, #" />
          </label>
          <label>
            <span>Default View</span>
            <select bind:value={defaultView}>
              {#each viewOptions as view}
                <option value={view}>{view[0].toUpperCase() + view.slice(1)}</option>
              {/each}
            </select>
          </label>
          {#if node.nodeType === 'Group'}
            <label>
              <span>Parent</span>
              <select bind:value={parentId}>
                {#each parentOptions as option}
                  <option value={option.id}>{option.label}: {option.path}</option>
                {/each}
              </select>
            </label>
          {/if}
          <div>
            <span class="field-label">Color</span>
            <div class="swatches" aria-label="Color">
              {#each colorOptions as option}
                <button
                  type="button"
                  class="swatch"
                  class:active={color === option}
                  style={`background: ${option}`}
                  aria-label={`Use color ${option}`}
                  onclick={() => (color = option)}
                ></button>
              {/each}
              <button type="button" class="clear" onclick={() => (color = '')}>Clear</button>
            </div>
          </div>
        </form>

        <aside class="preview">
          <div class="preview-title">Preview</div>
          <div class="preview-row" style={`border-left-color: ${color || 'var(--accent)'}`}>
            <span class="preview-icon" style={`background: ${color || 'var(--accent)'}`}>
              {icon || title.slice(0, 1).toUpperCase() || 'N'}
            </span>
            <div>
              <strong>{title || 'Untitled'}</strong>
              <span>{nodeStore.getKindLabel(node)} · {defaultView}</span>
            </div>
          </div>
        </aside>
      </div>

      {#if error}<div class="error">{error}</div>{/if}

      <footer>
        <button class="secondary" type="button" onclick={close}>Cancel</button>
        <button class="primary" type="button" onclick={save}>Save</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1210;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.35);
    padding: 24px;
  }
  .dialog {
    width: min(760px, 100%);
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
    justify-content: flex-end;
    border-top: 1px solid var(--border);
    border-bottom: none;
  }
  h2 {
    font-size: var(--text-lg);
    margin: 0 0 4px;
  }
  p {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    margin: 0;
  }
  .content {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
    gap: 16px;
    padding: 16px;
    overflow-y: auto;
  }
  .fields {
    display: grid;
    gap: 12px;
  }
  label,
  .fields > div {
    display: grid;
    gap: 6px;
  }
  label span,
  .field-label,
  .preview-title {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: 800;
    text-transform: uppercase;
  }
  input,
  select {
    width: 100%;
    padding: 9px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
  }
  .swatches {
    display: flex;
    align-items: center;
    gap: 7px;
    flex-wrap: wrap;
  }
  .swatch {
    width: 26px;
    height: 26px;
    border: 2px solid transparent;
    border-radius: 50%;
    cursor: pointer;
  }
  .swatch.active {
    border-color: var(--text-primary);
  }
  .clear,
  .icon-btn,
  .primary,
  .secondary {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .clear {
    padding: 5px 9px;
  }
  .icon-btn {
    width: 30px;
    height: 30px;
  }
  .preview {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    background: var(--bg-panel);
    min-width: 0;
  }
  .preview-row {
    margin-top: 10px;
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 10px;
    border-left: 4px solid var(--accent);
    border-radius: 6px;
    background: var(--bg-hover);
  }
  .preview-icon {
    width: 30px;
    height: 30px;
    border-radius: 7px;
    color: white;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    flex-shrink: 0;
  }
  .preview-row strong,
  .preview-row span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preview-row span {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    margin-top: 3px;
  }
  .error {
    margin: 0 16px 12px;
    padding: 10px 12px;
    border-radius: 6px;
    background: rgba(239, 68, 68, 0.12);
    color: var(--danger);
  }
  .primary,
  .secondary {
    padding: 8px 14px;
  }
  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  @media (max-width: 720px) {
    .content {
      grid-template-columns: 1fr;
    }
  }
</style>
