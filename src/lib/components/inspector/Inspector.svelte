<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { uiStore, type CompletionPolicy } from '$lib/stores/ui.svelte';
  import * as api from '$lib/api/client';
  import { formatDurationInput } from '$lib/utils/scheduling';

  const completionPolicies: CompletionPolicy[] = ['individual', 'ask', 'cascade'];

  let node = $derived(nodeStore.getSelected());
  let childCount = $derived(node ? nodeStore.getChildren(node.id).length : 0);
  let kindLabel = $derived(node ? nodeStore.getKindLabel(node) : '');
  let isTaskLike = $derived(node?.nodeType === 'Task');
  let assignedTags = $derived(node ? nodeStore.getTagObjects(node.id) : []);
  let tagsValue = $state('');

  $effect(() => {
    if (node) {
      tagsValue = '';
    }
  });

  async function updateTitle(title: string) {
    const n = nodeStore.getSelected();
    if (n && title !== n.title) {
      const updated = await api.nodeUpdate(n.id, { title });
      nodeStore.upsert(updated);
    }
  }

  async function setPriority(p: number) {
    const n = nodeStore.getSelected();
    if (n) {
      await nodeStore.setPriority(n.id, p);
    }
  }

  async function updateBody(body: string) {
    const n = nodeStore.getSelected();
    if (n) {
      await nodeStore.updateBody(n.id, body);
    }
  }

  async function addTag() {
    const n = nodeStore.getSelected();
    const clean = tagsValue.trim();
    if (n && clean) {
      await nodeStore.setTags(n.id, [...nodeStore.getTags(n.id), clean]);
      tagsValue = '';
    }
  }

  async function removeTag(name: string) {
    const n = nodeStore.getSelected();
    if (n) {
      await nodeStore.setTags(
        n.id,
        nodeStore.getTags(n.id).filter((tag) => tag.toLowerCase() !== name.toLowerCase()),
      );
    }
  }

  function repeatLabel(pattern?: string) {
    if (!pattern) return 'No repeat';
    return pattern.charAt(0).toUpperCase() + pattern.slice(1);
  }

  function reminderLabel(offset?: number | null) {
    if (offset === undefined || offset === null) return 'No reminder';
    if (offset === 0) return 'At due time';
    return `${Math.abs(offset)} minutes before`;
  }
</script>

<div class="inspector">
  {#if node}
    <div class="inspector-header">
      <div>
        <h3>{kindLabel}</h3>
        <p>{childCount} child{childCount === 1 ? '' : 'ren'}</p>
      </div>
      <div class="header-actions">
        <button
          class="favorite-btn"
          class:active={node.properties.favorite}
          title={node.properties.favorite ? 'Remove favorite' : 'Add favorite'}
          onclick={() => nodeStore.setFavorite(node.id, !node.properties.favorite)}
        >
          {node.properties.favorite ? '★' : '☆'}
        </button>
        <button
          class="collapse-btn"
          title="Collapse inspector"
          aria-label="Collapse inspector"
          onclick={() => uiStore.setInspectorVisible(false)}
        >
          Hide
        </button>
      </div>
    </div>

    <div class="section">
      <label class="label" for="inspector-title">Title</label>
      <input
        id="inspector-title"
        class="field-input"
        value={node.title}
        onchange={(e) => updateTitle((e.target as HTMLInputElement).value)}
      />
    </div>

    <div class="section">
      <div class="label">Type</div>
      <div class="type-pill">{kindLabel}</div>
    </div>

    <div class="section">
      <div class="label">Create Inside</div>
      <div class="quick-actions">
        {#if node.nodeType === 'Workspace'}
          <button onclick={() => nodeStore.createGroupUnder(node.id)}>Group</button>
          <button onclick={() => nodeStore.createTaskUnder(node.id)}>Task</button>
        {:else if node.nodeType === 'Group'}
          <button onclick={() => nodeStore.createSubgroupUnder(node.id)}>Subgroup</button>
          <button onclick={() => nodeStore.createTaskUnder(node.id)}>Task</button>
        {:else if node.nodeType === 'Task'}
          <button onclick={() => nodeStore.createSubtaskUnder(node.id)}>Subtask</button>
        {/if}
      </div>
    </div>

    {#if isTaskLike}
      <div class="section">
        <div class="label">Completion</div>
        <button class="completion-btn" onclick={() => nodeStore.toggleComplete(node.id)}>
          {node.isCompleted ? 'Mark incomplete' : 'Mark complete'}
        </button>
      </div>

      <div class="section">
        <div class="label">Completion Policy</div>
        <div class="policy-row" role="group" aria-label="Completion policy">
          {#each completionPolicies as policy}
            <button
              class:active={uiStore.completionPolicy === policy}
              onclick={() => uiStore.setCompletionPolicy(policy)}
            >
              {policy[0].toUpperCase() + policy.slice(1)}
            </button>
          {/each}
        </div>
      </div>

      <div class="section">
        <div class="label">Priority</div>
        <div class="priority-row">
          {#each [1, 2, 3, 4] as p}
            <button
              class="priority-btn p{p}"
              class:active={node.properties.priority === p}
              onclick={() => setPriority(p)}
            >
              P{p}
            </button>
          {/each}
        </div>
      </div>

      <div class="section">
        <div class="label">Schedule</div>
        <div class="schedule-summary">
          {#if node.properties.dueDate}
            <span>{new Date(node.properties.dueDate).toLocaleString()}</span>
          {:else}
            <span class="no-date">No due date</span>
          {/if}
          <span>{reminderLabel(node.properties.reminders?.[0]?.offsetMinutes)}</span>
          <span>{repeatLabel(node.properties.recurrence?.pattern)}</span>
          <span>
            {node.properties.estimatedDurationMinutes
              ? formatDurationInput(node.properties.estimatedDurationMinutes)
              : 'No estimate'}
          </span>
          <button class="set-btn" onclick={() => uiStore.openDueDatePrompt()}>Schedule...</button>
        </div>
      </div>

      <div class="section">
        <label class="label" for="inspector-tags">Tags</label>
        <div class="tag-editor">
          <div class="tag-chips">
            {#each assignedTags as tag (tag.id)}
              <button
                class="tag-chip"
                style={tag.color ? `--tag-color: ${tag.color}` : ''}
                title="Remove tag"
                onclick={() => removeTag(tag.name)}
              >
                {tag.name} <span aria-hidden="true">×</span>
              </button>
            {/each}
          </div>
          <input
            id="inspector-tags"
            class="field-input"
            bind:value={tagsValue}
            list="known-tags"
            placeholder="Add tag and press Enter"
            onkeydown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addTag();
              }
            }}
          />
          <datalist id="known-tags">
            {#each nodeStore.tags as tag (tag.id)}
              <option value={tag.name}></option>
            {/each}
          </datalist>
        </div>
      </div>

      <div class="section">
        <label class="label" for="inspector-body">Description</label>
        <textarea
          id="inspector-body"
          class="body-editor"
          value={node.body}
          placeholder="Add details..."
          onchange={(e) => updateBody((e.target as HTMLTextAreaElement).value)}
        ></textarea>
      </div>
    {/if}

    <div class="section">
      <div class="label">Info</div>
      <div class="meta-row">
        <span class="meta-label">Created</span>
        <span class="meta-value">{new Date(node.createdAt).toLocaleString()}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">Updated</span>
        <span class="meta-value">{new Date(node.updatedAt).toLocaleString()}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">ID</span>
        <span class="meta-value mono">{node.id.slice(0, 8)}...</span>
      </div>
    </div>
  {:else}
    <div class="empty">
      <p>Select a node to view details</p>
      <p class="hint">Press <kbd>Space</kbd> or click a node</p>
    </div>
  {/if}
</div>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    padding: 0;
  }
  .inspector-header {
    padding: var(--space-4);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .inspector-header h3 {
    font-size: var(--text-base);
    font-weight: 600;
  }
  .inspector-header p {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    margin-top: 2px;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .favorite-btn,
  .collapse-btn {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-tertiary);
    cursor: pointer;
    height: 30px;
  }
  .favorite-btn {
    width: 30px;
    font-size: var(--text-base);
  }
  .collapse-btn {
    padding: 0 9px;
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .favorite-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .collapse-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--accent);
  }
  .favorite-btn.active {
    color: var(--warning);
    border-color: rgba(245, 158, 11, 0.4);
    background: rgba(245, 158, 11, 0.1);
  }
  .section {
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border);
  }
  .label {
    display: block;
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0;
    margin-bottom: var(--space-2);
    font-weight: 600;
  }
  .field-input {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: var(--text-sm);
  }
  .tag-editor {
    display: grid;
    gap: 8px;
  }
  .tag-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag-chip {
    border: 1px solid color-mix(in srgb, var(--tag-color, var(--accent)) 30%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--tag-color, var(--accent)) 14%, transparent);
    color: var(--tag-color, var(--accent));
    cursor: pointer;
    font-size: var(--text-xs);
    padding: 4px 8px;
  }
  .type-pill {
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .priority-row {
    display: flex;
    gap: var(--space-1);
  }
  .policy-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-1);
  }
  .policy-row button {
    border: 1px solid var(--border);
    border-radius: 5px;
    min-height: 30px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .policy-row button:hover,
  .policy-row button.active {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--accent);
  }
  .quick-actions {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
  }
  .quick-actions button {
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 5px 9px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: var(--text-xs);
    font-weight: 650;
  }
  .quick-actions button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--accent);
  }
  .completion-btn {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 7px 10px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: var(--text-sm);
    font-weight: 650;
    text-align: left;
  }
  .completion-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--accent);
  }
  .priority-btn {
    flex: 1;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: transparent;
    cursor: pointer;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text-secondary);
  }
  .priority-btn:hover {
    background: var(--bg-hover);
  }
  .priority-btn.p1.active {
    background: var(--priority-p1);
    color: white;
    border-color: var(--priority-p1);
  }
  .priority-btn.p2.active {
    background: var(--priority-p2);
    color: white;
    border-color: var(--priority-p2);
  }
  .priority-btn.p3.active {
    background: var(--priority-p3);
    color: black;
    border-color: var(--priority-p3);
  }
  .priority-btn.p4.active {
    background: var(--text-tertiary);
    color: white;
  }
  .schedule-summary {
    display: grid;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
  .no-date {
    color: var(--text-tertiary);
  }
  .set-btn {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 10px;
    font-size: var(--text-xs);
    background: var(--bg-input);
    cursor: pointer;
    color: var(--text-secondary);
    font-weight: 700;
  }
  .set-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--accent);
  }
  .body-editor {
    width: 100%;
    min-height: 150px;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--bg-input);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    resize: vertical;
    line-height: 1.6;
  }
  .meta-row {
    display: flex;
    justify-content: space-between;
    font-size: var(--text-xs);
    padding: var(--space-1) 0;
  }
  .meta-label {
    color: var(--text-tertiary);
  }
  .meta-value {
    color: var(--text-secondary);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: var(--space-8);
    color: var(--text-tertiary);
  }
  .hint {
    font-size: var(--text-sm);
    margin-top: var(--space-2);
  }
  kbd {
    padding: 1px 5px;
    background: var(--bg-active);
    border-radius: 3px;
    font-size: var(--text-xs);
  }
</style>
