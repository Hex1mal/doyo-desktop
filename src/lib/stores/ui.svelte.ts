import { settingsGet, settingsSet } from '$lib/api/client';
import { clampZoom, nextZoom } from '$lib/utils/zoom';
import { getCurrentWebview } from '@tauri-apps/api/webview';

export type ActiveModule =
  | 'today'
  | 'inbox'
  | 'upcoming'
  | 'workspaces'
  | 'calendar'
  | 'kanban'
  | 'timeline'
  | 'productivity'
  | 'habits'
  | 'countdowns'
  | 'statistics'
  | 'search'
  | 'settings';

export type CompletionPolicy = 'individual' | 'ask' | 'cascade';
export type ListSort =
  'manual' | 'title' | 'created' | 'updated' | 'due' | 'priority' | 'completed';
export type ListGroup =
  'none' | 'workspace' | 'group' | 'due' | 'priority' | 'tag' | 'completionPeriod';
export type ListDensity = 'compact' | 'comfortable';

export type ListPrefs = {
  sort: ListSort;
  group: ListGroup;
  density: ListDensity;
};
export type CalendarView = 'month' | 'week' | 'day' | 'agenda';
export type CalendarPrefs = {
  view: CalendarView;
  currentDate: string;
  firstDayOfWeek: number;
  showCompleted: boolean;
};
export type KanbanMode = 'status' | 'priority' | 'tag' | 'workspace' | 'group';
export type KanbanPrefs = {
  mode: KanbanMode;
  showCompleted: boolean;
  statusColumns: string[];
};
export type TimelineZoom = 'day' | 'week' | 'month';
export type TimelinePrefs = {
  zoom: TimelineZoom;
  currentDate: string;
  showCompleted: boolean;
};
export type FocusPrefs = {
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  longBreakInterval: number;
};

type UiPrefs = {
  activeModule: ActiveModule;
  sidebarVisible: boolean;
  inspectorVisible: boolean;
  sidebarWidth: number;
  inspectorWidth: number;
  completionPolicy: CompletionPolicy;
  listPrefs: Record<string, ListPrefs>;
  calendarPrefs: CalendarPrefs;
  kanbanPrefs: KanbanPrefs;
  timelinePrefs: TimelinePrefs;
  focusPrefs: FocusPrefs;
  zoomLevel: number;
};

const PREF_KEY = 'doyo.uiPrefs.v1';
const LEGACY_PREF_KEY = 'todoapp.uiPrefs.v1';
const SETTINGS_PREF_KEY = 'ui.preferences.v1';
const THEME_SETTINGS_KEY = 'ui.theme';
const DEFAULT_PREFS: UiPrefs = {
  activeModule: 'workspaces',
  sidebarVisible: true,
  inspectorVisible: true,
  sidebarWidth: 280,
  inspectorWidth: 320,
  completionPolicy: 'individual',
  listPrefs: {},
  calendarPrefs: {
    view: 'month',
    currentDate: new Date().toISOString(),
    firstDayOfWeek: 1,
    showCompleted: false,
  },
  kanbanPrefs: {
    mode: 'status',
    showCompleted: false,
    statusColumns: ['Inbox', 'Next', 'Doing', 'Waiting', 'Done'],
  },
  timelinePrefs: {
    zoom: 'week',
    currentDate: new Date().toISOString(),
    showCompleted: false,
  },
  focusPrefs: {
    focusMinutes: 25,
    shortBreakMinutes: 5,
    longBreakMinutes: 15,
    longBreakInterval: 4,
  },
  zoomLevel: 1,
};

function getInitialTheme(): 'light' | 'dark' {
  if (typeof window !== 'undefined') {
    const stored = localStorage.getItem('theme');
    if (stored === 'dark' || stored === 'light') return stored;
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) return 'dark';
  }
  return 'light';
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function getInitialPrefs(): UiPrefs {
  if (typeof window === 'undefined') return DEFAULT_PREFS;
  try {
    const raw = localStorage.getItem(PREF_KEY) ?? localStorage.getItem(LEGACY_PREF_KEY);
    if (!localStorage.getItem(PREF_KEY) && raw) localStorage.setItem(PREF_KEY, raw);
    if (!raw) return DEFAULT_PREFS;
    const parsed = JSON.parse(raw) as Partial<UiPrefs>;
    return {
      activeModule: parsed.activeModule ?? DEFAULT_PREFS.activeModule,
      sidebarVisible: parsed.sidebarVisible ?? DEFAULT_PREFS.sidebarVisible,
      inspectorVisible: parsed.inspectorVisible ?? DEFAULT_PREFS.inspectorVisible,
      sidebarWidth: clamp(parsed.sidebarWidth ?? DEFAULT_PREFS.sidebarWidth, 220, 520),
      inspectorWidth: clamp(parsed.inspectorWidth ?? DEFAULT_PREFS.inspectorWidth, 260, 560),
      completionPolicy:
        parsed.completionPolicy === 'ask' || parsed.completionPolicy === 'cascade'
          ? parsed.completionPolicy
          : DEFAULT_PREFS.completionPolicy,
      listPrefs: parsed.listPrefs ?? DEFAULT_PREFS.listPrefs,
      calendarPrefs: {
        ...DEFAULT_PREFS.calendarPrefs,
        ...(parsed.calendarPrefs ?? {}),
        // Which month you were last looking at is not a preference. Restoring it
        // reopens Doyo on an arbitrary past date, so the calendar always starts
        // on today; view mode and week start below stay persisted.
        currentDate: DEFAULT_PREFS.calendarPrefs.currentDate,
      },
      kanbanPrefs: {
        ...DEFAULT_PREFS.kanbanPrefs,
        ...(parsed.kanbanPrefs ?? {}),
      },
      timelinePrefs: {
        ...DEFAULT_PREFS.timelinePrefs,
        ...(parsed.timelinePrefs ?? {}),
        // Same reasoning as the calendar: start where the user is, not where
        // they left off in some earlier session.
        currentDate: DEFAULT_PREFS.timelinePrefs.currentDate,
      },
      focusPrefs: {
        ...DEFAULT_PREFS.focusPrefs,
        ...(parsed.focusPrefs ?? {}),
      },
      zoomLevel: clampZoom(parsed.zoomLevel ?? DEFAULT_PREFS.zoomLevel),
    };
  } catch {
    return DEFAULT_PREFS;
  }
}

const initialPrefs = getInitialPrefs();

const state = $state({
  theme: getInitialTheme() as 'light' | 'dark',
  activeModule: initialPrefs.activeModule,
  sidebarVisible: initialPrefs.sidebarVisible,
  inspectorVisible: initialPrefs.inspectorVisible,
  sidebarWidth: initialPrefs.sidebarWidth,
  inspectorWidth: initialPrefs.inspectorWidth,
  completionPolicy: initialPrefs.completionPolicy,
  listPrefs: initialPrefs.listPrefs,
  calendarPrefs: initialPrefs.calendarPrefs,
  kanbanPrefs: initialPrefs.kanbanPrefs,
  timelinePrefs: initialPrefs.timelinePrefs,
  focusPrefs: initialPrefs.focusPrefs,
  zoomLevel: initialPrefs.zoomLevel,
  zoomFeedback: '',
  moveDialogNodeId: null as string | null,
  configDialogNodeId: null as string | null,
  focusMode: false,
  dueDatePromptOpen: false,
  quickOpenOpen: false,
});

function persistPrefs() {
  if (typeof window === 'undefined') return;
  const prefs: UiPrefs = {
    activeModule: state.activeModule,
    sidebarVisible: state.sidebarVisible,
    inspectorVisible: state.inspectorVisible,
    sidebarWidth: state.sidebarWidth,
    inspectorWidth: state.inspectorWidth,
    completionPolicy: state.completionPolicy,
    listPrefs: state.listPrefs,
    calendarPrefs: state.calendarPrefs,
    kanbanPrefs: state.kanbanPrefs,
    timelinePrefs: state.timelinePrefs,
    focusPrefs: state.focusPrefs,
    zoomLevel: state.zoomLevel,
  };
  localStorage.setItem(PREF_KEY, JSON.stringify(prefs));
  settingsSet(SETTINGS_PREF_KEY, prefs).catch(() => {
    /* localStorage remains the fallback when the backend is not ready. */
  });
}

let zoomFeedbackTimer: ReturnType<typeof setTimeout> | undefined;

function showZoomFeedback() {
  state.zoomFeedback = `Zoom: ${Math.round(state.zoomLevel * 100)}%`;
  clearTimeout(zoomFeedbackTimer);
  zoomFeedbackTimer = setTimeout(() => {
    state.zoomFeedback = '';
  }, 900);
}

async function applyZoom(level: number, feedback = false) {
  state.zoomLevel = clampZoom(level);
  if (typeof document !== 'undefined') {
    document.documentElement.style.setProperty('--app-zoom', String(state.zoomLevel));
  }
  try {
    await getCurrentWebview().setZoom(state.zoomLevel);
  } catch {
    if (typeof document !== 'undefined') {
      document.body.style.zoom = String(state.zoomLevel);
    }
  }
  if (feedback) showZoomFeedback();
}

function applyTheme(theme: 'light' | 'dark') {
  state.theme = theme;
  localStorage.setItem('theme', theme);
  settingsSet(THEME_SETTINGS_KEY, theme).catch(() => {
    /* localStorage remains the fallback when the backend is not ready. */
  });
  document.documentElement.setAttribute('data-theme', theme);
  document.documentElement.style.colorScheme = theme;
}

function applyPrefs(prefs: Partial<UiPrefs>) {
  state.activeModule = prefs.activeModule ?? state.activeModule;
  state.sidebarVisible = prefs.sidebarVisible ?? state.sidebarVisible;
  state.inspectorVisible = prefs.inspectorVisible ?? state.inspectorVisible;
  state.sidebarWidth = clamp(prefs.sidebarWidth ?? state.sidebarWidth, 220, 520);
  state.inspectorWidth = clamp(prefs.inspectorWidth ?? state.inspectorWidth, 260, 560);
  if (
    prefs.completionPolicy === 'individual' ||
    prefs.completionPolicy === 'ask' ||
    prefs.completionPolicy === 'cascade'
  ) {
    state.completionPolicy = prefs.completionPolicy;
  }
  state.listPrefs = prefs.listPrefs ?? state.listPrefs;
  state.calendarPrefs = { ...state.calendarPrefs, ...(prefs.calendarPrefs ?? {}) };
  state.kanbanPrefs = { ...state.kanbanPrefs, ...(prefs.kanbanPrefs ?? {}) };
  state.timelinePrefs = { ...state.timelinePrefs, ...(prefs.timelinePrefs ?? {}) };
  state.focusPrefs = { ...state.focusPrefs, ...(prefs.focusPrefs ?? {}) };
  state.zoomLevel = clampZoom(prefs.zoomLevel ?? state.zoomLevel);
}

export const uiStore = {
  get theme() {
    return state.theme;
  },
  get activeModule() {
    return state.activeModule;
  },
  get sidebarVisible() {
    return state.sidebarVisible;
  },
  get inspectorVisible() {
    return state.inspectorVisible;
  },
  get sidebarWidth() {
    return state.sidebarWidth;
  },
  get inspectorWidth() {
    return state.inspectorWidth;
  },
  get completionPolicy() {
    return state.completionPolicy;
  },
  get listPrefs() {
    return state.listPrefs;
  },
  get calendarPrefs() {
    return state.calendarPrefs;
  },
  get kanbanPrefs() {
    return state.kanbanPrefs;
  },
  get timelinePrefs() {
    return state.timelinePrefs;
  },
  get focusPrefs() {
    return state.focusPrefs;
  },
  get zoomLevel() {
    return state.zoomLevel;
  },
  get zoomFeedback() {
    return state.zoomFeedback;
  },
  get moveDialogNodeId() {
    return state.moveDialogNodeId;
  },
  get configDialogNodeId() {
    return state.configDialogNodeId;
  },
  get focusMode() {
    return state.focusMode;
  },
  get dueDatePromptOpen() {
    return state.dueDatePromptOpen;
  },
  get quickOpenOpen() {
    return state.quickOpenOpen;
  },

  async loadPersistedSettings() {
    try {
      const [prefs, theme] = await Promise.all([
        settingsGet<Partial<UiPrefs>>(SETTINGS_PREF_KEY),
        settingsGet<'light' | 'dark'>(THEME_SETTINGS_KEY),
      ]);
      if (prefs && typeof prefs === 'object') {
        applyPrefs(prefs);
        persistPrefs();
      }
      if (theme === 'light' || theme === 'dark') {
        applyTheme(theme);
      }
      await applyZoom(state.zoomLevel);
      return true;
    } catch {
      return false;
    }
  },

  setTheme(theme: 'light' | 'dark') {
    applyTheme(theme);
  },

  toggleTheme() {
    const current = document.documentElement.getAttribute('data-theme') ?? state.theme;
    applyTheme(current === 'light' ? 'dark' : 'light');
  },

  setActiveModule(module: ActiveModule) {
    state.activeModule = module;
    persistPrefs();
  },

  toggleSidebar() {
    state.sidebarVisible = !state.sidebarVisible;
    persistPrefs();
  },

  toggleInspector() {
    state.inspectorVisible = !state.inspectorVisible;
    persistPrefs();
  },

  setSidebarVisible(visible: boolean) {
    state.sidebarVisible = visible;
    persistPrefs();
  },

  setInspectorVisible(visible: boolean) {
    state.inspectorVisible = visible;
    persistPrefs();
  },

  setSidebarWidth(width: number) {
    state.sidebarWidth = clamp(width, 220, 520);
    persistPrefs();
  },

  setInspectorWidth(width: number) {
    state.inspectorWidth = clamp(width, 260, 560);
    persistPrefs();
  },

  resetSidebarWidth() {
    state.sidebarWidth = DEFAULT_PREFS.sidebarWidth;
    persistPrefs();
  },

  resetInspectorWidth() {
    state.inspectorWidth = DEFAULT_PREFS.inspectorWidth;
    persistPrefs();
  },

  clampPanelSizes(viewportWidth: number) {
    const mainMinimum = 360;
    const railAndHandles = state.focusMode ? 24 : 84;
    let changed = false;

    if (state.sidebarVisible && state.inspectorVisible) {
      const sidebarMax = Math.max(
        220,
        Math.min(520, viewportWidth - state.inspectorWidth - mainMinimum - railAndHandles),
      );
      const nextSidebar = clamp(state.sidebarWidth, 220, sidebarMax);
      if (nextSidebar !== state.sidebarWidth) {
        state.sidebarWidth = nextSidebar;
        changed = true;
      }

      const inspectorMax = Math.max(
        260,
        Math.min(560, viewportWidth - state.sidebarWidth - mainMinimum - railAndHandles),
      );
      const nextInspector = clamp(state.inspectorWidth, 260, inspectorMax);
      if (nextInspector !== state.inspectorWidth) {
        state.inspectorWidth = nextInspector;
        changed = true;
      }
    } else if (state.sidebarVisible) {
      const sidebarMax = Math.max(220, Math.min(520, viewportWidth - mainMinimum - railAndHandles));
      const nextSidebar = clamp(state.sidebarWidth, 220, sidebarMax);
      if (nextSidebar !== state.sidebarWidth) {
        state.sidebarWidth = nextSidebar;
        changed = true;
      }
    } else if (state.inspectorVisible) {
      const inspectorMax = Math.max(
        260,
        Math.min(560, viewportWidth - mainMinimum - railAndHandles),
      );
      const nextInspector = clamp(state.inspectorWidth, 260, inspectorMax);
      if (nextInspector !== state.inspectorWidth) {
        state.inspectorWidth = nextInspector;
        changed = true;
      }
    }

    if (changed) persistPrefs();
  },

  setCompletionPolicy(policy: CompletionPolicy) {
    state.completionPolicy = policy;
    persistPrefs();
  },

  getListPrefs(key: string): ListPrefs {
    return state.listPrefs[key] ?? { sort: 'manual', group: 'none', density: 'comfortable' };
  },

  setListPrefs(key: string, prefs: Partial<ListPrefs>) {
    state.listPrefs = {
      ...state.listPrefs,
      [key]: {
        ...this.getListPrefs(key),
        ...prefs,
      },
    };
    persistPrefs();
  },

  setCalendarPrefs(prefs: Partial<CalendarPrefs>) {
    state.calendarPrefs = {
      ...state.calendarPrefs,
      ...prefs,
    };
    persistPrefs();
  },

  setKanbanPrefs(prefs: Partial<KanbanPrefs>) {
    state.kanbanPrefs = {
      ...state.kanbanPrefs,
      ...prefs,
    };
    persistPrefs();
  },

  setTimelinePrefs(prefs: Partial<TimelinePrefs>) {
    state.timelinePrefs = {
      ...state.timelinePrefs,
      ...prefs,
    };
    persistPrefs();
  },

  setFocusPrefs(prefs: Partial<FocusPrefs>) {
    state.focusPrefs = {
      ...state.focusPrefs,
      ...prefs,
    };
    persistPrefs();
  },

  setZoomLevel(level: number, feedback = true) {
    applyZoom(level, feedback).catch(() => {});
    persistPrefs();
  },

  zoomIn() {
    this.setZoomLevel(nextZoom(state.zoomLevel, 'in'));
  },

  zoomOut() {
    this.setZoomLevel(nextZoom(state.zoomLevel, 'out'));
  },

  resetZoom() {
    this.setZoomLevel(1);
  },

  openMoveDialog(id: string) {
    state.moveDialogNodeId = id;
  },

  closeMoveDialog() {
    state.moveDialogNodeId = null;
  },

  openConfigDialog(id: string) {
    state.configDialogNodeId = id;
  },

  closeConfigDialog() {
    state.configDialogNodeId = null;
  },

  toggleFocusMode() {
    state.focusMode = !state.focusMode;
  },

  exitFocusMode() {
    state.focusMode = false;
  },

  openDueDatePrompt() {
    state.dueDatePromptOpen = true;
  },

  closeDueDatePrompt() {
    state.dueDatePromptOpen = false;
  },

  openQuickOpen() {
    state.quickOpenOpen = true;
  },

  closeQuickOpen() {
    state.quickOpenOpen = false;
  },
};

export function initTheme() {
  applyTheme(state.theme);
  applyZoom(state.zoomLevel).catch(() => {});
}
