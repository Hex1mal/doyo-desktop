<script lang="ts">
  import { nodeStore } from '$lib/stores/nodes.svelte';
</script>

<footer class="status-bar">
  <span class="left">{nodeStore.statusMessage}</span>
  <span class="center">
    {#if nodeStore.isLoading}
      Loading…
    {:else}
      {nodeStore.count} nodes
    {/if}
  </span>
  <span class="right">
    <kbd>Ctrl+K</kbd> commands · <kbd>Ctrl+P</kbd> jump · <kbd>Enter</kbd> new
  </span>
</footer>

<style>
  .status-bar {
    height: 26px;
    background: var(--bg-sidebar);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 var(--space-3);
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    flex-shrink: 0;
    user-select: none;
  }
  .left { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .center { flex: 0 0 auto; padding: 0 var(--space-4); }
  .right { flex: 1; text-align: right; }
  kbd {
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 0 3px;
    background: var(--bg-active);
    border-radius: 3px;
  }
</style>
