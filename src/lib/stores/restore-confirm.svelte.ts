/**
 * Confirmation gate for restoring a backup.
 *
 * Restoring replaces the live database, so it needs an explicit confirmation.
 * The settings store owns the restore sequence (safety backup, validation,
 * activation), and it is easier to keep that sequence in one place than to
 * split it across a component, so the confirmation is exposed as a promise the
 * store can await. `RestoreConfirmDialog` renders whatever request is pending.
 */

type PendingRestore = {
  backupName: string;
  resolve: (confirmed: boolean) => void;
};

const state = $state<{ pending: PendingRestore | null }>({ pending: null });

function settle(confirmed: boolean) {
  const pending = state.pending;
  if (!pending) return;
  state.pending = null;
  pending.resolve(confirmed);
}

export const restoreConfirm = {
  get isOpen() {
    return state.pending !== null;
  },
  get backupName() {
    return state.pending?.backupName ?? null;
  },

  /** Resolves true when the user confirms, false when they cancel or dismiss. */
  request(backupName: string): Promise<boolean> {
    // A second request can only mean the first is stale; decline it rather than
    // leaving its caller waiting on a promise nothing will ever resolve.
    settle(false);
    return new Promise<boolean>((resolve) => {
      state.pending = { backupName, resolve };
    });
  },

  accept() {
    settle(true);
  },

  cancel() {
    settle(false);
  },
};
