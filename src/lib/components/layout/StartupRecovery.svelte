<script lang="ts">
  import { startupStore } from '$lib/stores/startup.svelte';
  import type { RecoveryCandidate } from '$lib/types/node';
  import { overlayLayer } from '$lib/stores/overlay.svelte';

  overlayLayer('startup-recovery', () => startupStore.needsAttention);

  let showDetail = $state(false);
  let dialogEl: HTMLElement | undefined = $state();

  const report = $derived(startupStore.report);
  const candidates = $derived(startupStore.candidates);
  const isEphemeral = $derived(report?.status === 'ephemeral');

  $effect(() => {
    if (startupStore.needsAttention) queueMicrotask(() => dialogEl?.focus());
  });

  function formatSize(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatWhen(iso: string | null) {
    if (!iso) return 'Unknown date';
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return 'Unknown date';
    return date.toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function sourceLabel(candidate: RecoveryCandidate) {
    return candidate.source === 'migrationBackup' ? 'Pre-upgrade copy' : 'Backup';
  }

  function onKeydown(event: KeyboardEvent) {
    // No Escape-to-close: continuing is a real decision, so it needs the button.
    if (event.key === 'Escape') event.stopPropagation();
  }
</script>

{#if startupStore.needsAttention && report}
  <div class="overlay" role="presentation">
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="recovery-title"
      aria-describedby="recovery-summary"
      tabindex="-1"
      bind:this={dialogEl}
      onkeydown={onKeydown}
    >
      <header>
        <div>
          <h2 id="recovery-title">
            {isEphemeral ? 'Doyo cannot save your data' : 'Doyo could not open your database'}
          </h2>
          <p id="recovery-summary">{report.summary}</p>
        </div>
      </header>

      <div class="body">
        {#if report.quarantinedPath}
          <p class="preserved">
            Your previous database was not deleted. It is kept at
            <code>{report.quarantinedPath}</code>
          </p>
        {/if}

        {#if candidates.length > 0}
          <h3>Restore from a saved copy</h3>
          <p class="hint">
            These copies were checked and can be restored right now. The newest is listed first.
          </p>
          <ul class="candidates">
            {#each candidates as candidate (candidate.source + candidate.name)}
              <li>
                <div class="candidate-info">
                  <span class="candidate-when">{formatWhen(candidate.modifiedAt)}</span>
                  <span class="candidate-meta">
                    {sourceLabel(candidate)} · {formatSize(candidate.sizeBytes)}
                  </span>
                </div>
                <button
                  class="primary"
                  disabled={startupStore.isRestoring}
                  onclick={() => startupStore.restore(candidate)}
                >
                  Restore
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty">
            <h3>No saved copies found</h3>
            <p class="hint">
              Doyo could not find a backup to restore. You can continue with an empty database — the
              file above is left untouched, so it can still be recovered by hand.
            </p>
          </div>
        {/if}
      </div>

      <footer>
        <button class="link" onclick={() => (showDetail = !showDetail)} aria-expanded={showDetail}>
          {showDetail ? 'Hide technical details' : 'Technical details'}
        </button>
        <div class="footer-actions">
          <button
            onclick={() => startupStore.refreshCandidates()}
            disabled={startupStore.isRestoring}>Refresh</button
          >
          {#if !isEphemeral}
            <button onclick={() => startupStore.dismiss()} disabled={startupStore.isRestoring}>
              Continue with empty database
            </button>
          {/if}
        </div>
      </footer>

      {#if showDetail && report.detail}
        <pre class="detail">{report.detail}</pre>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1400;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.45);
    padding: 24px;
  }
  .dialog {
    width: min(560px, 100%);
    max-height: min(720px, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
    background: var(--bg-modal);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }
  .dialog:focus {
    outline: none;
  }
  header {
    padding: 18px 20px 14px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0 0 6px;
    font-size: var(--text-lg);
    color: var(--text-primary);
  }
  h3 {
    margin: 0 0 4px;
    font-size: var(--text-base);
    color: var(--text-primary);
  }
  p {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .hint {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .body {
    padding: 16px 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .preserved {
    padding: 10px 12px;
    background: var(--bg-hover);
    border-radius: 6px;
    font-size: var(--text-xs);
    color: var(--text-secondary);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    word-break: break-all;
    color: var(--text-primary);
  }
  .candidates {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }
  .candidates li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
  }
  .candidates li + li {
    border-top: 1px solid var(--border);
  }
  .candidate-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .candidate-when {
    font-size: var(--text-sm);
    color: var(--text-primary);
  }
  .candidate-meta {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .empty {
    padding: 4px 0;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
  }
  .footer-actions {
    display: flex;
    gap: 8px;
  }
  button {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  button.primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  button.link {
    border: none;
    background: none;
    color: var(--text-tertiary);
    padding: 6px 0;
    font-size: var(--text-xs);
  }
  button.link:hover {
    color: var(--text-secondary);
    background: none;
  }
  .detail {
    margin: 0;
    padding: 12px 20px 16px;
    border-top: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 160px;
    overflow-y: auto;
  }
</style>
