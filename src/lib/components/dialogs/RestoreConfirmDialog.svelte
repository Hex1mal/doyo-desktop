<script lang="ts">
  import { restoreConfirm } from '$lib/stores/restore-confirm.svelte';
  import { overlayLayer } from '$lib/stores/overlay.svelte';

  overlayLayer('restore-confirm', () => restoreConfirm.isOpen);

  let dialogEl: HTMLElement | undefined = $state();
  let cancelEl: HTMLButtonElement | undefined = $state();
  let returnFocusTo: HTMLElement | null = null;

  const backupName = $derived(restoreConfirm.backupName);

  $effect(() => {
    if (!restoreConfirm.isOpen) return;
    // Remember what opened the dialog so focus can go back there on close.
    returnFocusTo = document.activeElement as HTMLElement | null;
    // Cancel takes initial focus: this replaces a destructive action, so the
    // safe choice is the one a stray Enter or Space lands on.
    queueMicrotask(() => cancelEl?.focus());

    return () => {
      const target = returnFocusTo;
      returnFocusTo = null;
      queueMicrotask(() => target?.focus());
    };
  });

  function cancel() {
    restoreConfirm.cancel();
  }

  function accept() {
    restoreConfirm.accept();
  }

  function focusable(): HTMLElement[] {
    if (!dialogEl) return [];
    return [...dialogEl.querySelectorAll<HTMLElement>('button:not(:disabled)')];
  }

  function handleKeydown(event: KeyboardEvent) {
    // The app binds its shortcuts on window, and they do not know a modal is
    // open: Tab would indent the selected node, Delete would delete it, Enter
    // would create a sibling. Stop every key here so a confirmation dialog
    // cannot mutate the tree behind it.
    event.stopPropagation();

    if (event.key === 'Escape') {
      event.preventDefault();
      cancel();
      return;
    }
    if (event.key !== 'Tab') return;

    // Keep Tab inside the dialog; a modal that lets focus escape into the app
    // behind it is unusable with a keyboard.
    const items = focusable();
    if (items.length === 0) return;
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (event.shiftKey && (active === first || !dialogEl?.contains(active))) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (active === last || !dialogEl?.contains(active))) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

{#if restoreConfirm.isOpen}
  <div class="overlay" role="presentation" onclick={cancel} onkeydown={handleKeydown}>
    <div
      bind:this={dialogEl}
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="restore-confirm-title"
      aria-describedby="restore-confirm-body"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={handleKeydown}
    >
      <h2 id="restore-confirm-title">Restore this backup?</h2>

      <div id="restore-confirm-body" class="body">
        <p class="lead">Doyo will replace your current data with this backup.</p>

        <div class="target">
          <span class="target-label">Selected backup</span>
          <code class="target-name">{backupName}</code>
        </div>

        <p class="reassure">
          Your current database is snapshotted first. If this is not the backup you wanted, restore
          the <code>doyo-pre-restore-*.db</code> snapshot named in the confirmation message to get back
          to where you are now.
        </p>
      </div>

      <footer>
        <button bind:this={cancelEl} type="button" onclick={cancel}>Cancel</button>
        <button type="button" class="danger" onclick={accept}>Restore backup</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1500;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.45);
    padding: var(--space-6);
  }
  .dialog {
    width: min(460px, 100%);
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    background: var(--bg-modal);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.35);
    padding: var(--space-5);
  }
  .dialog:focus {
    outline: none;
  }
  h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--text-lg);
    color: var(--text-primary);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  p {
    margin: 0;
    font-size: var(--text-sm);
    line-height: 1.5;
    color: var(--text-secondary);
  }
  .lead {
    color: var(--text-primary);
  }
  .target {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
  }
  .target-label {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .target-name {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text-primary);
    word-break: break-all;
  }
  .reassure {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .reassure code {
    font-family: var(--font-mono);
    word-break: break-all;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-5);
  }
  button {
    padding: 7px 14px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  button:hover {
    background: var(--bg-hover);
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  button.danger {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
    font-weight: 500;
  }
  button.danger:hover {
    /* No dedicated hover token for danger; darken the same hue instead of
       falling back to the neutral hover background. */
    filter: brightness(0.92);
  }
  button.danger:focus-visible {
    /* Not --danger: a red ring on a red button is invisible. The accent reads
       clearly against both the red fill and the modal background. */
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
