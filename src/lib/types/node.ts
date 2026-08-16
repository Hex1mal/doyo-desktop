export type NodeType = 'Workspace' | 'Group' | 'Task' | 'Note' | 'Attachment' | 'Comment';

export interface NodeProperties {
  dueDate?: string | null;
  startDate?: string | null;
  priority?: number;
  reminders?: ReminderConfig[];
  recurrence?: RecurrenceConfig | null;
  estimatedDurationMinutes?: number;
  custom?: Record<string, unknown>;
  icon?: string;
  color?: string;
  pinned?: boolean;
  favorite?: boolean;
}

export interface ReminderConfig {
  time?: string | null;
  offsetMinutes?: number | null;
  type: string;
}

export interface RecurrenceConfig {
  pattern: string;
  interval: number;
  days?: number[];
}

export interface Node {
  id: string;
  parentId: string | null;
  position: number;
  nodeType: NodeType;
  title: string;
  body: string;
  properties: NodeProperties;
  isCollapsed: boolean;
  isCompleted: boolean;
  completedAt: string | null;
  deletedAt: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface NodeViewModel {
  id: string;
  parentId: string | null;
  nodeType: NodeType;
  title: string;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
  isCompleted: boolean;
  priority?: number;
  dueDate?: string;
  tags: string[];
  childCount: number;
  completedChildCount: number;
}

export interface CreateNodeInput {
  parentId?: string | null;
  nodeType: NodeType;
  title: string;
  body?: string;
  properties?: Partial<NodeProperties>;
  position?: number;
}

export interface UpdateNodeInput {
  title?: string;
  body?: string;
  nodeType?: NodeType;
  isCollapsed?: boolean;
  properties?: Partial<NodeProperties>;
}

export interface SearchFilters {
  nodeTypes?: NodeType[];
  tags?: string[];
  priority?: number;
  dueBefore?: string;
  dueAfter?: string;
  isCompleted?: boolean;
}

export interface SearchResult {
  node: Node;
  snippet: string;
  breadcrumb: string[];
  rank: number;
}

export interface Tag {
  id: string;
  name: string;
  color: string | null;
  createdAt: string;
}

export interface TimeBlock {
  id: string;
  taskId: string | null;
  title: string;
  startTime: string;
  endTime: string;
  allDay: boolean;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateTimeBlockInput {
  taskId?: string | null;
  title: string;
  startTime: string;
  endTime: string;
  allDay: boolean;
  notes?: string;
}

export interface UpdateTimeBlockInput {
  taskId?: string | null;
  title?: string;
  startTime?: string;
  endTime?: string;
  allDay?: boolean;
  notes?: string;
}

export type FocusMethod = 'pomodoro' | 'stopwatch' | 'flowtime';
export type FocusState = 'running' | 'paused' | 'completed' | 'stopped';
export type PomodoroPhase = 'focus' | 'short_break' | 'long_break';

export interface FocusSession {
  id: string;
  taskId: string | null;
  taskTitle: string;
  method: FocusMethod;
  state: FocusState;
  pomodoroPhase: PomodoroPhase | null;
  pomodoroCycle: number;
  plannedSeconds: number;
  accumulatedSeconds: number;
  elapsedSeconds: number;
  durationSeconds: number;
  interruptions: number;
  note: string;
  startedAt: string;
  lastStartedAt: string | null;
  endedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface StartFocusInput {
  method: FocusMethod;
  taskId?: string | null;
  plannedSeconds?: number;
  pomodoroPhase?: PomodoroPhase | null;
  pomodoroCycle?: number;
  note?: string;
}

export interface StopFocusInput {
  completed: boolean;
  note?: string | null;
}

export interface FocusSummary {
  todaySeconds: number;
  totalSeconds: number;
  pomodoroCount: number;
  stopwatchSeconds: number;
  flowtimeSeconds: number;
}

export interface SavedFilter {
  id: string;
  name: string;
  definition: Record<string, unknown>;
  position: number;
  createdAt: string;
  updatedAt: string;
}

export interface CreateSavedFilterInput {
  name: string;
  definition: Record<string, unknown>;
}

export interface UpdateSavedFilterInput {
  name?: string;
  definition?: Record<string, unknown>;
  position?: number;
}

export type HabitFrequency = 'daily' | 'weekly';
export type HabitLogStatus = 'completed' | 'skipped' | 'partial';

export interface Habit {
  id: string;
  title: string;
  icon: string;
  color: string | null;
  frequency: HabitFrequency;
  days: number[];
  goal: number;
  goalUnit: string;
  startDate: string;
  reminderTime: string | null;
  archived: boolean;
  position: number;
  createdAt: string;
  updatedAt: string;
}

export interface HabitLog {
  id: string;
  habitId: string;
  logDate: string;
  status: HabitLogStatus;
  value: number;
  note: string;
  createdAt: string;
  updatedAt: string;
}

export interface HabitSummary {
  activeCount: number;
  completedToday: number;
  completionRate: number;
  bestStreak: number;
}

export interface CreateHabitInput {
  title: string;
  icon?: string;
  color?: string | null;
  frequency: HabitFrequency;
  days?: number[];
  goal?: number;
  goalUnit?: string;
  startDate: string;
  reminderTime?: string | null;
}

export interface UpdateHabitInput {
  title?: string;
  icon?: string;
  color?: string | null;
  frequency?: HabitFrequency;
  days?: number[];
  goal?: number;
  goalUnit?: string;
  startDate?: string;
  reminderTime?: string | null;
  archived?: boolean;
  position?: number;
}

export interface UpsertHabitLogInput {
  habitId: string;
  logDate: string;
  status: HabitLogStatus;
  value?: number;
  note?: string;
}

export type CountdownMode = 'countdown' | 'countup';

export interface Countdown {
  id: string;
  title: string;
  targetDate: string;
  mode: CountdownMode;
  icon: string;
  color: string | null;
  recurrence: string | null;
  reminderAt: string | null;
  archived: boolean;
  position: number;
  createdAt: string;
  updatedAt: string;
}

export interface CreateCountdownInput {
  title: string;
  targetDate: string;
  mode: CountdownMode;
  icon?: string;
  color?: string | null;
  recurrence?: string | null;
  reminderAt?: string | null;
}

export interface UpdateCountdownInput {
  title?: string;
  targetDate?: string;
  mode?: CountdownMode;
  icon?: string;
  color?: string | null;
  recurrence?: string | null;
  reminderAt?: string | null;
  archived?: boolean;
  position?: number;
}

export type StartupStatus = 'ok' | 'recovered' | 'ephemeral';

export interface RecoveryCandidate {
  name: string;
  source: 'backup' | 'migrationBackup';
  schemaVersion: number;
  sizeBytes: number;
  modifiedAt: string | null;
}

export interface StartupReport {
  status: StartupStatus;
  summary: string | null;
  detail: string | null;
  quarantinedPath: string | null;
  recoveryCandidates: RecoveryCandidate[];
}

export interface RestoreOutcome {
  /** Restoring this snapshot undoes the restore that produced it. */
  snapshotName: string | null;
  /** True when the restored database is already live and no restart is needed. */
  activated: boolean;
  activationError: string | null;
}
