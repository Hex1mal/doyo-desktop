import { recoveryCandidates, recoveryRestore, startupReport } from '$lib/api/client';
import { applyRestoreOutcome } from '$lib/stores/restore';
import { toast } from '$lib/stores/toast.svelte';
import type { RecoveryCandidate, StartupReport } from '$lib/types/node';

const state = $state({
  report: null as StartupReport | null,
  isRestoring: false,
  /** Set when the user chooses to continue past the recovery screen. */
  dismissed: false,
});

export const startupStore = {
  get report() {
    return state.report;
  },
  get isRestoring() {
    return state.isRestoring;
  },
  /** True while the recovery screen should block the app. */
  get needsAttention() {
    return !state.dismissed && !!state.report && state.report.status !== 'ok';
  },
  get candidates(): RecoveryCandidate[] {
    return state.report?.recoveryCandidates ?? [];
  },

  async load() {
    try {
      state.report = await startupReport();
    } catch {
      // A backend that cannot answer is not itself a reason to block the app.
      state.report = null;
    }
  },

  async refreshCandidates() {
    if (!state.report) return;
    try {
      state.report = { ...state.report, recoveryCandidates: await recoveryCandidates() };
    } catch (e) {
      toast.error(`Could not list recovery files: ${String(e)}`);
    }
  },

  async restore(candidate: RecoveryCandidate) {
    if (state.isRestoring) return;
    state.isRestoring = true;
    try {
      const outcome = await recoveryRestore(candidate.name, candidate.source);
      await applyRestoreOutcome(outcome);
    } catch (e) {
      toast.error(`Restore failed: ${String(e)}`);
    } finally {
      state.isRestoring = false;
    }
  },

  /** Continue with the empty database, leaving the quarantined copy untouched. */
  dismiss() {
    state.dismissed = true;
  },
};
