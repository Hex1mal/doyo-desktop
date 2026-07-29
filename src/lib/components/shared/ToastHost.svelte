<script lang="ts">
  import { toast } from '$lib/stores/toast.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each toast.items as t (t.id)}
    <div class="toast {t.type}" role="status">
      {t.message}
      <button class="x" onclick={() => toast.dismiss(t.id)}>×</button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    bottom: 36px;
    right: 16px;
    z-index: 2000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    min-width: 220px;
    max-width: 360px;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--bg-modal);
    border: 1px solid var(--border);
    box-shadow: 0 8px 24px rgba(0,0,0,0.18);
    font-size: var(--text-sm);
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
  }
  .toast.error { border-color: var(--danger); }
  .toast.success { border-color: var(--success); }
  .x {
    margin-left: auto;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-tertiary);
    font-size: 16px;
  }
</style>
