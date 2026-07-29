<script lang="ts">
  import { uiStore } from '$lib/stores/ui.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { parseNaturalDate, formatDue } from '$lib/utils/date';
  import { toast } from '$lib/stores/toast.svelte';

  let value = $state('');
  let inputEl: HTMLInputElement | undefined = $state();

  $effect(() => {
    inputEl?.focus();
  });

  async function submit() {
    const id = nodeStore.selectedId;
    if (!id) {
      uiStore.closeDueDatePrompt();
      return;
    }
    if (!value.trim()) {
      await nodeStore.setDueDate(id, null);
      uiStore.closeDueDatePrompt();
      return;
    }
    const iso = parseNaturalDate(value);
    if (!iso) {
      toast.error('Could not parse date. Try: tomorrow, next friday, 2026-08-01');
      return;
    }
    await nodeStore.setDueDate(id, iso);
    uiStore.closeDueDatePrompt();
  }

  function quickSet(text: string) {
    const iso = parseNaturalDate(text);
    if (!iso) return;
    value = text;
    const id = nodeStore.selectedId;
    if (!id) return;
    nodeStore.setDueDate(id, iso);
    uiStore.closeDueDatePrompt();
  }

  let selected = $derived(nodeStore.getSelected());
  let currentDue = $derived(selected?.properties.dueDate || null);
</script>

<div
  class="overlay"
  role="presentation"
  tabindex="-1"
  onclick={() => uiStore.closeDueDatePrompt()}
  onkeydown={(e) => {
    if (e.key === 'Escape') uiStore.closeDueDatePrompt();
  }}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Set due date"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <h3>Set due date</h3>

    {#if currentDue}
      <div class="current-due">
        Current: <strong>{formatDue(currentDue)}</strong>
        <button class="clear-link" onclick={() => {
          if (nodeStore.selectedId) nodeStore.setDueDate(nodeStore.selectedId, null);
          uiStore.closeDueDatePrompt();
        }}>Clear</button>
      </div>
    {/if}

    <div class="presets">
      <button onclick={() => quickSet('today')}>Today</button>
      <button onclick={() => quickSet('tomorrow')}>Tomorrow</button>
      <button onclick={() => quickSet('in 2 days')}>+2 days</button>
      <button onclick={() => quickSet('in 3 days')}>+3 days</button>
      <button onclick={() => quickSet('next monday')}>Next Mon</button>
      <button onclick={() => quickSet('next friday')}>Next Fri</button>
      <button onclick={() => quickSet('in 1 week')}>+1 week</button>
      <button onclick={() => quickSet('in 2 weeks')}>+2 weeks</button>
      <button onclick={() => quickSet('in 1 month')}>+1 mo</button>
    </div>

    <p class="hint">Or type: tomorrow · next monday · in 3 days · 2026-08-01 · empty to clear</p>
    <input
      bind:this={inputEl}
      bind:value
      placeholder="e.g. tomorrow"
      onkeydown={(e) => {
        if (e.key === 'Enter') { e.preventDefault(); submit(); }
        if (e.key === 'Escape') uiStore.closeDueDatePrompt();
      }}
    />
    <div class="actions">
      <button class="secondary" onclick={() => uiStore.closeDueDatePrompt()}>Cancel</button>
      <button class="primary" onclick={submit}>Set</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0; z-index: 1100;
    background: rgba(0,0,0,0.4);
    display: flex; align-items: flex-start; justify-content: center;
    padding-top: 18vh;
  }
  .dialog {
    width: 420px;
    background: var(--bg-modal);
    border-radius: 10px;
    padding: 16px;
    box-shadow: 0 12px 40px rgba(0,0,0,0.25);
  }
  h3 { margin: 0 0 6px; font-size: var(--text-lg); }
  .current-due {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: 10px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .clear-link {
    border: none; background: none;
    color: var(--danger);
    font-size: var(--text-xs);
    cursor: pointer;
    text-decoration: underline;
  }
  .clear-link:hover { opacity: 0.8; }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 12px;
  }
  .presets button {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 5px;
    font-size: var(--text-xs);
    background: var(--bg-input);
    color: var(--text-primary);
    cursor: pointer;
    white-space: nowrap;
  }
  .presets button:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .hint { font-size: var(--text-xs); color: var(--text-tertiary); margin-bottom: 12px; }
  input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: var(--text-base);
    outline: none;
  }
  input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-subtle); }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
  button {
    border: none; border-radius: 6px; padding: 8px 14px; cursor: pointer; font-size: var(--text-sm);
  }
  .primary { background: var(--accent); color: white; }
  .primary:hover { background: var(--accent-hover); }
  .secondary { background: var(--bg-hover); color: var(--text-primary); }
</style>
