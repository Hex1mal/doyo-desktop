<script lang="ts">
  import * as api from '$lib/api/client';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { settingsStore, type SettingsPanel } from '$lib/stores/settings.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import { uiStore } from '$lib/stores/ui.svelte';

  const panels: Array<{ id: SettingsPanel; title: string }> = [
    { id: 'general', title: 'General' },
    { id: 'features', title: 'Features' },
    { id: 'smartViews', title: 'Smart Views' },
    { id: 'notifications', title: 'Notifications' },
    { id: 'dateTime', title: 'Date and Time' },
    { id: 'appearance', title: 'Appearance' },
    { id: 'dataBackup', title: 'Data and Backup' },
    { id: 'importExport', title: 'Import and Export' },
    { id: 'keyboard', title: 'Keyboard Shortcuts' },
    { id: 'privacy', title: 'Privacy' },
    { id: 'advanced', title: 'Advanced' },
    { id: 'about', title: 'About' },
  ];

  let didLoad = false;
  let activePanel = $state<SettingsPanel>('general');
  let notificationPrefs = $state({ pomodoro: true, habits: true, countdowns: true });
  let datePrefs = $state({ timezone: 'local', hourCycle: '24', dateFormat: 'yyyy-mm-dd' });
  let appearancePrefs = $state({ themeMode: 'light', fontScale: 100, reducedMotion: false, accentColor: '#2563eb' });
  let backupPrefs = $state({ createSafetyBackupBeforeRestore: true });
  let importJsonText = $state('');
  let exportPreview = $state('');
  let markdownOutputDir = $state('');

  $effect(() => {
    if (didLoad) return;
    didLoad = true;
    loadSettings();
  });

  async function loadSettings() {
    await settingsStore.load();
    notificationPrefs = await settingsStore.get('notification.preferences.v1', notificationPrefs);
    datePrefs = await settingsStore.get('datetime.preferences.v1', datePrefs);
    appearancePrefs = await settingsStore.get('appearance.preferences.v1', appearancePrefs);
    backupPrefs = await settingsStore.get('backup.preferences.v1', backupPrefs);
    applyAppearance();
  }

  async function saveSetting(key: string, value: unknown) {
    await settingsStore.set(key, value);
  }

  function applyAppearance() {
    document.documentElement.style.setProperty('--user-font-scale', String(appearancePrefs.fontScale / 100));
    document.documentElement.style.setProperty('--accent', appearancePrefs.accentColor);
    document.documentElement.toggleAttribute('data-reduced-motion', appearancePrefs.reducedMotion);
  }

  async function setNotification(key: keyof typeof notificationPrefs, value: boolean) {
    notificationPrefs = { ...notificationPrefs, [key]: value };
    await saveSetting('notification.preferences.v1', notificationPrefs);
  }

  async function setDatePref(key: keyof typeof datePrefs, value: string) {
    datePrefs = { ...datePrefs, [key]: value };
    await saveSetting('datetime.preferences.v1', datePrefs);
  }

  async function setAppearance(key: keyof typeof appearancePrefs, value: string | number | boolean) {
    appearancePrefs = { ...appearancePrefs, [key]: value };
    if (key === 'themeMode') {
      const mode = String(value);
      if (mode === 'system') {
        uiStore.setTheme(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
      } else {
        uiStore.setTheme(mode as 'light' | 'dark');
      }
    }
    applyAppearance();
    await saveSetting('appearance.preferences.v1', appearancePrefs);
  }

  async function exportToClipboard() {
    try {
      exportPreview = await api.exportJson(null);
      await navigator.clipboard.writeText(exportPreview);
      toast.success('JSON export copied to clipboard');
    } catch (e) {
      toast.error(`Export failed: ${String(e)}`);
    }
  }

  async function importFromText() {
    const text = importJsonText.trim();
    if (!text) {
      toast.error('Paste JSON to import first');
      return;
    }
    if (!window.confirm('Import these nodes into the root? Existing data will be preserved.')) return;
    try {
      const ids = await api.importJson(text, null);
      await nodeStore.load();
      importJsonText = '';
      toast.success(`Imported ${ids.length} nodes`);
    } catch (e) {
      toast.error(`Import failed: ${String(e)}`);
    }
  }

  async function exportMarkdown() {
    const dir = markdownOutputDir.trim();
    if (!dir) {
      toast.error('Enter an output directory path');
      return;
    }
    try {
      await api.exportMarkdown(null, dir);
      toast.success('Markdown export completed');
    } catch (e) {
      toast.error(`Markdown export failed: ${String(e)}`);
    }
  }
</script>

<section class="settings-view" aria-label="Settings Center">
  <aside class="settings-nav" aria-label="Settings sections">
    {#each panels as panel (panel.id)}
      <button class:active={activePanel === panel.id} onclick={() => (activePanel = panel.id)}>
        {panel.title}
      </button>
    {/each}
  </aside>

  <div class="settings-panel">
    <h2>{panels.find((panel) => panel.id === activePanel)?.title}</h2>

    {#if activePanel === 'general'}
      <label>Completion policy
        <select value={uiStore.completionPolicy} onchange={(event) => uiStore.setCompletionPolicy((event.target as HTMLSelectElement).value as never)}>
          <option value="individual">Individual</option>
          <option value="ask">Ask</option>
          <option value="cascade">Cascade</option>
        </select>
      </label>
      <label>Start module
        <select value={uiStore.activeModule} onchange={(event) => uiStore.setActiveModule((event.target as HTMLSelectElement).value as never)}>
          <option value="workspaces">Workspaces</option>
          <option value="today">Today</option>
          <option value="upcoming">Next 7 Days</option>
          <option value="calendar">Calendar</option>
          <option value="kanban">Kanban</option>
          <option value="timeline">Timeline</option>
          <option value="productivity">Productivity</option>
          <option value="habits">Habits</option>
          <option value="countdowns">Countdowns</option>
          <option value="statistics">Statistics</option>
          <option value="settings">Settings</option>
        </select>
      </label>
    {:else if activePanel === 'features'}
      <div class="control-grid">
        <label>Sidebar visible
          <input type="checkbox" checked={uiStore.sidebarVisible} onchange={(event) => uiStore.setSidebarVisible((event.target as HTMLInputElement).checked)} />
        </label>
        <label>Inspector visible
          <input type="checkbox" checked={uiStore.inspectorVisible} onchange={(event) => uiStore.setInspectorVisible((event.target as HTMLInputElement).checked)} />
        </label>
        <label>Focus mode
          <input type="checkbox" checked={uiStore.focusMode} onchange={(event) => (event.target as HTMLInputElement).checked ? uiStore.toggleFocusMode() : uiStore.exitFocusMode()} />
        </label>
      </div>
    {:else if activePanel === 'smartViews'}
      <div class="control-grid">
        <label>Completed sort
          <select value={uiStore.getListPrefs('completed').sort} onchange={(event) => uiStore.setListPrefs('completed', { sort: (event.target as HTMLSelectElement).value as never })}>
            <option value="completed">Completion date</option>
            <option value="title">Title</option>
            <option value="priority">Priority</option>
            <option value="due">Due date</option>
          </select>
        </label>
        <label>Completed grouping
          <select value={uiStore.getListPrefs('completed').group} onchange={(event) => uiStore.setListPrefs('completed', { group: (event.target as HTMLSelectElement).value as never })}>
            <option value="completionPeriod">Completion period</option>
            <option value="workspace">Workspace</option>
            <option value="priority">Priority</option>
            <option value="none">None</option>
          </select>
        </label>
        <label>List density
          <select value={uiStore.getListPrefs('completed').density} onchange={(event) => uiStore.setListPrefs('completed', { density: (event.target as HTMLSelectElement).value as never })}>
            <option value="comfortable">Comfortable</option>
            <option value="compact">Compact</option>
          </select>
        </label>
      </div>
      <label>Calendar completed tasks
        <input
          type="checkbox"
          checked={uiStore.calendarPrefs.showCompleted}
          onchange={(event) => uiStore.setCalendarPrefs({ showCompleted: (event.target as HTMLInputElement).checked })}
        />
      </label>
      <label>Kanban completed tasks
        <input type="checkbox" checked={uiStore.kanbanPrefs.showCompleted} onchange={(event) => uiStore.setKanbanPrefs({ showCompleted: (event.target as HTMLInputElement).checked })} />
      </label>
      <label>Timeline completed tasks
        <input type="checkbox" checked={uiStore.timelinePrefs.showCompleted} onchange={(event) => uiStore.setTimelinePrefs({ showCompleted: (event.target as HTMLInputElement).checked })} />
      </label>
    {:else if activePanel === 'notifications'}
      <label>Pomodoro notifications
        <input type="checkbox" checked={notificationPrefs.pomodoro} onchange={(event) => setNotification('pomodoro', (event.target as HTMLInputElement).checked)} />
      </label>
      <label>Habit reminders
        <input type="checkbox" checked={notificationPrefs.habits} onchange={(event) => setNotification('habits', (event.target as HTMLInputElement).checked)} />
      </label>
      <label>Countdown reminders
        <input type="checkbox" checked={notificationPrefs.countdowns} onchange={(event) => setNotification('countdowns', (event.target as HTMLInputElement).checked)} />
      </label>
    {:else if activePanel === 'dateTime'}
      <div class="control-grid">
        <label>Timezone
          <select value={datePrefs.timezone} onchange={(event) => setDatePref('timezone', (event.target as HTMLSelectElement).value)}>
            <option value="local">Local system timezone</option>
            <option value="UTC">UTC</option>
          </select>
        </label>
        <label>First day of week
          <select value={uiStore.calendarPrefs.firstDayOfWeek} onchange={(event) => uiStore.setCalendarPrefs({ firstDayOfWeek: Number((event.target as HTMLSelectElement).value) })}>
            <option value="0">Sunday</option>
            <option value="1">Monday</option>
            <option value="6">Saturday</option>
          </select>
        </label>
        <label>Time format
          <select value={datePrefs.hourCycle} onchange={(event) => setDatePref('hourCycle', (event.target as HTMLSelectElement).value)}>
            <option value="24">24-hour</option>
            <option value="12">12-hour</option>
          </select>
        </label>
        <label>Date format
          <select value={datePrefs.dateFormat} onchange={(event) => setDatePref('dateFormat', (event.target as HTMLSelectElement).value)}>
            <option value="yyyy-mm-dd">YYYY-MM-DD</option>
            <option value="dd/mm/yyyy">DD/MM/YYYY</option>
            <option value="mm/dd/yyyy">MM/DD/YYYY</option>
          </select>
        </label>
        <label>Default calendar view
          <select value={uiStore.calendarPrefs.view} onchange={(event) => uiStore.setCalendarPrefs({ view: (event.target as HTMLSelectElement).value as never })}>
            <option value="month">Month</option>
            <option value="week">Week</option>
            <option value="day">Day</option>
            <option value="agenda">Agenda</option>
          </select>
        </label>
      </div>
    {:else if activePanel === 'appearance'}
      <div class="control-grid">
        <label>Theme
          <select value={appearancePrefs.themeMode} onchange={(event) => setAppearance('themeMode', (event.target as HTMLSelectElement).value)}>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
            <option value="system">System</option>
          </select>
        </label>
        <label>Font scale
          <input type="number" min="80" max="130" step="5" value={appearancePrefs.fontScale} onchange={(event) => setAppearance('fontScale', Number((event.target as HTMLInputElement).value))} />
        </label>
        <label>Accent color
          <input type="color" value={appearancePrefs.accentColor} onchange={(event) => setAppearance('accentColor', (event.target as HTMLInputElement).value)} />
        </label>
        <label>Reduced motion
          <input type="checkbox" checked={appearancePrefs.reducedMotion} onchange={(event) => setAppearance('reducedMotion', (event.target as HTMLInputElement).checked)} />
        </label>
      </div>
    {:else if activePanel === 'dataBackup'}
      <label>Create safety backup before restore
        <input type="checkbox" checked={backupPrefs.createSafetyBackupBeforeRestore} onchange={(event) => { backupPrefs = { createSafetyBackupBeforeRestore: (event.target as HTMLInputElement).checked }; saveSetting('backup.preferences.v1', backupPrefs); }} />
      </label>
      <div class="backup-actions">
        <button class="primary" onclick={() => settingsStore.createBackup()}>Create Backup</button>
        <button onclick={() => settingsStore.load()}>Refresh</button>
      </div>
      {#if settingsStore.backups.length === 0}
        <p>No backups yet</p>
      {:else}
        {#each settingsStore.backups as backup (backup)}
          <div class="backup-row">
            <span>{backup}</span>
            <button onclick={() => settingsStore.restoreBackup(backup)}>Restore</button>
          </div>
        {/each}
      {/if}
    {:else if activePanel === 'importExport'}
      <div class="backup-actions">
        <button class="primary" onclick={exportToClipboard}>Export JSON to Clipboard</button>
      </div>
      {#if exportPreview}
        <textarea rows="5" readonly value={exportPreview}></textarea>
      {/if}
      <label>Import JSON
        <textarea rows="7" bind:value={importJsonText} placeholder="Paste Doyo JSON export"></textarea>
      </label>
      <button onclick={importFromText}>Import JSON</button>
      <label>Markdown output directory
        <input bind:value={markdownOutputDir} placeholder="~/DoyoExport" />
      </label>
      <button onclick={exportMarkdown}>Export Markdown</button>
    {:else if activePanel === 'keyboard'}
      <div class="shortcut-grid">
        <span>Ctrl+K</span><strong>Command palette</strong>
        <span>Ctrl+P</span><strong>Quick open</strong>
        <span>Shift+F10</span><strong>Context menu</strong>
        <span>Ctrl+Enter</span><strong>Complete task</strong>
      </div>
    {:else if activePanel === 'privacy'}
      <label>Local-only mode
        <input type="checkbox" checked disabled />
      </label>
      <label>Store notification sent markers locally
        <input type="checkbox" checked disabled />
      </label>
    {:else if activePanel === 'advanced'}
      <button onclick={() => uiStore.loadPersistedSettings()}>Reload persisted UI settings</button>
      <button onclick={loadSettings}>Reload all settings panels</button>
    {:else if activePanel === 'about'}
      <p>Doyo local-first desktop task manager. Tauri + SvelteKit + Rust + SQLite.</p>
    {/if}
  </div>
</section>

<style>
  .settings-view {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    background: var(--bg-app);
    overflow: hidden;
  }
  .settings-nav {
    display: grid;
    align-content: start;
    gap: 3px;
    padding: 10px;
    border-right: 1px solid var(--border);
    background: var(--bg-panel);
    overflow: auto;
  }
  .settings-nav button,
  .settings-panel button,
  select,
  input,
  textarea {
    min-height: 32px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    padding: 0 8px;
  }
  textarea {
    min-height: 96px;
    padding: 8px;
    resize: vertical;
  }
  button {
    cursor: pointer;
  }
  .settings-nav button {
    text-align: left;
  }
  .settings-nav button.active,
  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .settings-panel {
    display: grid;
    align-content: start;
    gap: 14px;
    padding: 16px;
    overflow: auto;
  }
  label {
    display: grid;
    gap: 6px;
    max-width: 360px;
    color: var(--text-secondary);
  }
  .backup-actions,
  .backup-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .backup-row {
    justify-content: space-between;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
    padding: 8px;
  }
  .shortcut-grid {
    display: grid;
    grid-template-columns: 100px minmax(0, 1fr);
    gap: 8px;
  }
  @media (max-width: 800px) {
    .settings-view {
      grid-template-columns: 1fr;
    }
    .settings-nav {
      display: flex;
      overflow-x: auto;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
  }
</style>
