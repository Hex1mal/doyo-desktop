import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: vi.fn(() => ({ setZoom: vi.fn() })),
}));

describe('restore confirmation gate', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it('reports the selected backup while a request is pending', async () => {
    const { restoreConfirm } = await import('./restore-confirm.svelte');

    expect(restoreConfirm.isOpen).toBe(false);
    expect(restoreConfirm.backupName).toBe(null);

    const answer = restoreConfirm.request('doyo-backup-20260819-101500.db');
    expect(restoreConfirm.isOpen).toBe(true);
    expect(restoreConfirm.backupName).toBe('doyo-backup-20260819-101500.db');

    restoreConfirm.cancel();
    await expect(answer).resolves.toBe(false);
    expect(restoreConfirm.isOpen).toBe(false);
    expect(restoreConfirm.backupName).toBe(null);
  });

  it('resolves true only when the user accepts', async () => {
    const { restoreConfirm } = await import('./restore-confirm.svelte');

    const accepted = restoreConfirm.request('a.db');
    restoreConfirm.accept();
    await expect(accepted).resolves.toBe(true);

    const cancelled = restoreConfirm.request('b.db');
    restoreConfirm.cancel();
    await expect(cancelled).resolves.toBe(false);
  });

  it('settles a superseded request instead of leaving it hanging', async () => {
    const { restoreConfirm } = await import('./restore-confirm.svelte');

    const first = restoreConfirm.request('first.db');
    const second = restoreConfirm.request('second.db');

    await expect(first).resolves.toBe(false);
    expect(restoreConfirm.backupName).toBe('second.db');

    restoreConfirm.accept();
    await expect(second).resolves.toBe(true);
  });

  it('is inert once settled, so a stray accept cannot restore anything', async () => {
    const { restoreConfirm } = await import('./restore-confirm.svelte');

    const answer = restoreConfirm.request('c.db');
    restoreConfirm.cancel();
    await expect(answer).resolves.toBe(false);

    // Accepting with nothing pending must not reopen or re-resolve.
    restoreConfirm.accept();
    expect(restoreConfirm.isOpen).toBe(false);
    expect(restoreConfirm.backupName).toBe(null);
  });
});

describe('restoreBackup is gated on the confirmation', () => {
  const client = {
    backupCreate: vi.fn(async () => 'doyo-backup-new.db'),
    backupList: vi.fn(async () => ['one.db']),
    backupRestore: vi.fn(async () => ({
      activated: true,
      snapshotName: 'doyo-pre-restore-20260819.db',
      activationError: null,
    })),
    settingsGet: vi.fn(async () => null),
    settingsSet: vi.fn(async () => undefined),
    settingsList: vi.fn(async () => []),
    settingsDelete: vi.fn(async () => undefined),
  };

  beforeEach(() => {
    vi.resetModules();
    for (const fn of Object.values(client)) fn.mockClear();
    vi.doMock('$lib/api/client', () => client);
    vi.doMock('$lib/stores/restore', () => ({ applyRestoreOutcome: vi.fn(async () => undefined) }));
  });

  it('does not touch the database when the user cancels', async () => {
    const { restoreConfirm } = await import('./restore-confirm.svelte');
    const { settingsStore } = await import('./settings.svelte');

    const result = settingsStore.restoreBackup('one.db');
    restoreConfirm.cancel();

    await expect(result).resolves.toBe(false);
    expect(client.backupRestore).not.toHaveBeenCalled();
    expect(client.backupCreate).not.toHaveBeenCalled();
  });

  it('runs the safety backup and restore only after the user confirms', async () => {
    const { restoreConfirm } = await import('./restore-confirm.svelte');
    const { settingsStore } = await import('./settings.svelte');

    const result = settingsStore.restoreBackup('one.db');
    // Nothing may happen while the dialog is still open.
    expect(client.backupRestore).not.toHaveBeenCalled();

    restoreConfirm.accept();

    await expect(result).resolves.toBe(true);
    expect(client.backupRestore).toHaveBeenCalledWith('one.db');
  });
});
