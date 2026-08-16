import type { RestoreOutcome } from '$lib/types/node';
import { toast } from '$lib/stores/toast.svelte';

/**
 * Bring the UI in line with a database that was just restored.
 *
 * The backend repoints its connection during the restore, so the data is
 * already live. Every store in the app was hydrated from the previous database
 * though, so the view is reloaded rather than each store being refreshed
 * individually: enumerating them invites one being forgotten and left showing
 * rows that no longer exist.
 */
export async function applyRestoreOutcome(outcome: RestoreOutcome): Promise<void> {
  if (!outcome.activated) {
    toast.error(
      outcome.activationError
        ? `Restored, but could not load it: ${outcome.activationError}. Restart Doyo.`
        : 'Restored, but could not load it. Restart Doyo.',
    );
    return;
  }

  toast.success(
    outcome.snapshotName
      ? `Backup restored. Undo by restoring ${outcome.snapshotName}.`
      : 'Backup restored.',
  );

  // Let the toast paint before the reload replaces the view.
  await new Promise((resolve) => setTimeout(resolve, 600));
  window.location.reload();
}
