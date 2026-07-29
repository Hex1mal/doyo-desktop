import {
  backupCreate,
  backupList,
  backupRestore,
  settingsDelete,
  settingsGet,
  settingsList,
  settingsSet,
} from '$lib/api/client';
import { toast } from '$lib/stores/toast.svelte';

export type SettingsPanel =
  | 'general'
  | 'features'
  | 'smartViews'
  | 'notifications'
  | 'dateTime'
  | 'appearance'
  | 'dataBackup'
  | 'importExport'
  | 'keyboard'
  | 'privacy'
  | 'advanced'
  | 'about';

const state = $state({
  values: new Map<string, unknown>(),
  backups: [] as string[],
  isLoading: false,
  error: '',
});

export const settingsStore = {
  get values() {
    return state.values;
  },
  get backups() {
    return state.backups;
  },
  get isLoading() {
    return state.isLoading;
  },
  get error() {
    return state.error;
  },

  async load() {
    state.isLoading = true;
    state.error = '';
    try {
      const [settings, backups] = await Promise.all([settingsList(null), backupList()]);
      state.values = new Map(settings);
      state.backups = backups;
      return true;
    } catch (e) {
      state.error = String(e);
      toast.error(`Settings failed to load: ${String(e)}`);
      return false;
    } finally {
      state.isLoading = false;
    }
  },

  async get<T>(key: string, fallback: T): Promise<T> {
    try {
      const value = await settingsGet<T>(key);
      return value ?? fallback;
    } catch {
      return fallback;
    }
  },

  async set(key: string, value: unknown) {
    try {
      await settingsSet(key, value);
      state.values = new Map(state.values).set(key, value);
      return true;
    } catch (e) {
      toast.error(`Setting save failed: ${String(e)}`);
      return false;
    }
  },

  async delete(key: string) {
    try {
      await settingsDelete(key);
      const next = new Map(state.values);
      next.delete(key);
      state.values = next;
      return true;
    } catch (e) {
      toast.error(`Setting delete failed: ${String(e)}`);
      return false;
    }
  },

  async createBackup() {
    try {
      const path = await backupCreate();
      state.backups = await backupList();
      toast.success('Backup created');
      return path;
    } catch (e) {
      toast.error(`Backup failed: ${String(e)}`);
      return null;
    }
  },

  async restoreBackup(name: string) {
    if (!window.confirm('Restore this backup? The app should be restarted after restore.'))
      return false;
    try {
      const prefs = await this.get('backup.preferences.v1', {
        createSafetyBackupBeforeRestore: true,
      });
      if (prefs.createSafetyBackupBeforeRestore) {
        await backupCreate();
      }
      await backupRestore(name);
      state.backups = await backupList();
      toast.info('Backup restored. Restart the app to reload the restored database.');
      return true;
    } catch (e) {
      toast.error(`Restore failed: ${String(e)}`);
      return false;
    }
  },
};
