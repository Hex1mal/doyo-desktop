import { invoke } from '@tauri-apps/api/core';
import type {
  CreateTimeBlockInput,
  Countdown,
  CreateCountdownInput,
  CreateHabitInput,
  CreateSavedFilterInput,
  FocusSession,
  FocusSummary,
  Habit,
  HabitLog,
  HabitSummary,
  Node,
  SavedFilter,
  SearchFilters,
  SearchResult,
  StartFocusInput,
  Tag,
  TimeBlock,
  UpdateCountdownInput,
  UpdateHabitInput,
  UpdateNodeInput,
  UpdateSavedFilterInput,
  UpdateTimeBlockInput,
  UpsertHabitLogInput,
  StopFocusInput,
} from '$lib/types/node';

type RawRecord = Record<string, unknown>;

function normalizeNodeType(raw: unknown): Node['nodeType'] {
  const type = String(raw ?? 'Task').toLowerCase();
  if (type === 'workspace') return 'Workspace';
  if (type === 'group') return 'Group';
  if (type === 'task') return 'Task';
  if (type === 'note') return 'Note';
  if (type === 'attachment') return 'Attachment';
  if (type === 'comment') return 'Comment';
  return 'Task';
}

function value<T>(raw: RawRecord, camel: string, snake: string, fallback: T): T {
  return (raw[camel] ?? raw[snake] ?? fallback) as T;
}

function normalizeProperties(raw: unknown): Node['properties'] {
  const props = raw && typeof raw === 'object' ? (raw as RawRecord) : {};
  return {
    dueDate: value(props, 'dueDate', 'due_date', null),
    startDate: value(props, 'startDate', 'start_date', null),
    priority: value(props, 'priority', 'priority', undefined),
    reminders: value(props, 'reminders', 'reminders', undefined),
    recurrence: value(props, 'recurrence', 'recurrence', null),
    estimatedDurationMinutes: value(
      props,
      'estimatedDurationMinutes',
      'estimated_duration_minutes',
      undefined,
    ),
    custom: value(props, 'custom', 'custom', undefined),
    icon: value(props, 'icon', 'icon', undefined),
    color: value(props, 'color', 'color', undefined),
    pinned: value(props, 'pinned', 'pinned', undefined),
    favorite: value(props, 'favorite', 'favorite', undefined),
  };
}

function denormalizeProperties(raw: Partial<Node['properties']>): Record<string, unknown> {
  const props = raw as RawRecord;
  const normalized: Record<string, unknown> = {};
  for (const [key, fieldValue] of Object.entries(props)) {
    if (key === 'dueDate') normalized.due_date = fieldValue;
    else if (key === 'startDate') normalized.start_date = fieldValue;
    else if (key === 'estimatedDurationMinutes') normalized.estimated_duration_minutes = fieldValue;
    else normalized[key] = fieldValue;
  }
  return normalized;
}

function denormalizeUpdateNodeInput(changes: UpdateNodeInput): Record<string, unknown> {
  const normalized: Record<string, unknown> = { ...changes };
  if (changes.nodeType !== undefined) {
    normalized.node_type = changes.nodeType;
    delete normalized.nodeType;
  }
  if (changes.isCollapsed !== undefined) {
    normalized.is_collapsed = changes.isCollapsed;
    delete normalized.isCollapsed;
  }
  if (changes.properties !== undefined) {
    normalized.properties = denormalizeProperties(changes.properties);
  }
  return normalized;
}

function normalizeNode(raw: unknown): Node {
  const node = raw as RawRecord;
  return {
    ...(node as unknown as Node),
    id: value(node, 'id', 'id', ''),
    parentId: value(node, 'parentId', 'parent_id', null),
    position: value(node, 'position', 'position', 0),
    nodeType: normalizeNodeType(value(node, 'nodeType', 'node_type', 'Task')),
    title: value(node, 'title', 'title', ''),
    body: value(node, 'body', 'body', ''),
    properties: normalizeProperties(value(node, 'properties', 'properties', {})),
    isCollapsed: value(node, 'isCollapsed', 'is_collapsed', false),
    isCompleted: value(node, 'isCompleted', 'is_completed', false),
    completedAt: value(node, 'completedAt', 'completed_at', null),
    deletedAt: value(node, 'deletedAt', 'deleted_at', null),
    version: value(node, 'version', 'version', 0),
    createdAt: value(node, 'createdAt', 'created_at', ''),
    updatedAt: value(node, 'updatedAt', 'updated_at', ''),
  };
}

function normalizeSearchResult(raw: unknown): SearchResult {
  const result = raw as RawRecord;
  return {
    node: normalizeNode(result.node),
    snippet: value(result, 'snippet', 'snippet', ''),
    breadcrumb: value(result, 'breadcrumb', 'breadcrumb', []),
    rank: value(result, 'rank', 'rank', 0),
  };
}

function normalizeTag(raw: unknown): Tag {
  const tag = raw as RawRecord;
  return {
    id: value(tag, 'id', 'id', ''),
    name: value(tag, 'name', 'name', ''),
    color: value(tag, 'color', 'color', null),
    createdAt: value(tag, 'createdAt', 'created_at', ''),
  };
}

function normalizeTimeBlock(raw: unknown): TimeBlock {
  const block = raw as RawRecord;
  return {
    id: value(block, 'id', 'id', ''),
    taskId: value(block, 'taskId', 'task_id', null),
    title: value(block, 'title', 'title', ''),
    startTime: value(block, 'startTime', 'start_time', ''),
    endTime: value(block, 'endTime', 'end_time', ''),
    allDay: value(block, 'allDay', 'all_day', false),
    notes: value(block, 'notes', 'notes', ''),
    createdAt: value(block, 'createdAt', 'created_at', ''),
    updatedAt: value(block, 'updatedAt', 'updated_at', ''),
  };
}

function normalizeFocusSession(raw: unknown): FocusSession {
  const session = raw as RawRecord;
  return {
    id: value(session, 'id', 'id', ''),
    taskId: value(session, 'taskId', 'task_id', null),
    taskTitle: value(session, 'taskTitle', 'task_title', ''),
    method: value(session, 'method', 'method', 'stopwatch'),
    state: value(session, 'state', 'state', 'stopped'),
    pomodoroPhase: value(session, 'pomodoroPhase', 'pomodoro_phase', null),
    pomodoroCycle: value(session, 'pomodoroCycle', 'pomodoro_cycle', 1),
    plannedSeconds: value(session, 'plannedSeconds', 'planned_seconds', 0),
    accumulatedSeconds: value(session, 'accumulatedSeconds', 'accumulated_seconds', 0),
    elapsedSeconds: value(session, 'elapsedSeconds', 'elapsed_seconds', 0),
    durationSeconds: value(session, 'durationSeconds', 'duration_seconds', 0),
    interruptions: value(session, 'interruptions', 'interruptions', 0),
    note: value(session, 'note', 'note', ''),
    startedAt: value(session, 'startedAt', 'started_at', ''),
    lastStartedAt: value(session, 'lastStartedAt', 'last_started_at', null),
    endedAt: value(session, 'endedAt', 'ended_at', null),
    createdAt: value(session, 'createdAt', 'created_at', ''),
    updatedAt: value(session, 'updatedAt', 'updated_at', ''),
  };
}

function normalizeFocusSummary(raw: unknown): FocusSummary {
  const summary = raw as RawRecord;
  return {
    todaySeconds: value(summary, 'todaySeconds', 'today_seconds', 0),
    totalSeconds: value(summary, 'totalSeconds', 'total_seconds', 0),
    pomodoroCount: value(summary, 'pomodoroCount', 'pomodoro_count', 0),
    stopwatchSeconds: value(summary, 'stopwatchSeconds', 'stopwatch_seconds', 0),
    flowtimeSeconds: value(summary, 'flowtimeSeconds', 'flowtime_seconds', 0),
  };
}

function normalizeSavedFilter(raw: unknown): SavedFilter {
  const filter = raw as RawRecord;
  return {
    id: value(filter, 'id', 'id', ''),
    name: value(filter, 'name', 'name', ''),
    definition: value(filter, 'definition', 'definition', {}),
    position: value(filter, 'position', 'position', 0),
    createdAt: value(filter, 'createdAt', 'created_at', ''),
    updatedAt: value(filter, 'updatedAt', 'updated_at', ''),
  };
}

function normalizeHabit(raw: unknown): Habit {
  const habit = raw as RawRecord;
  return {
    id: value(habit, 'id', 'id', ''),
    title: value(habit, 'title', 'title', ''),
    icon: value(habit, 'icon', 'icon', ''),
    color: value(habit, 'color', 'color', null),
    frequency: value(habit, 'frequency', 'frequency', 'daily'),
    days: value(habit, 'days', 'days', []),
    goal: value(habit, 'goal', 'goal', 1),
    goalUnit: value(habit, 'goalUnit', 'goal_unit', 'count'),
    startDate: value(habit, 'startDate', 'start_date', ''),
    reminderTime: value(habit, 'reminderTime', 'reminder_time', null),
    archived: value(habit, 'archived', 'archived', false),
    position: value(habit, 'position', 'position', 0),
    createdAt: value(habit, 'createdAt', 'created_at', ''),
    updatedAt: value(habit, 'updatedAt', 'updated_at', ''),
  };
}

function normalizeHabitLog(raw: unknown): HabitLog {
  const log = raw as RawRecord;
  return {
    id: value(log, 'id', 'id', ''),
    habitId: value(log, 'habitId', 'habit_id', ''),
    logDate: value(log, 'logDate', 'log_date', ''),
    status: value(log, 'status', 'status', 'completed'),
    value: value(log, 'value', 'value', 1),
    note: value(log, 'note', 'note', ''),
    createdAt: value(log, 'createdAt', 'created_at', ''),
    updatedAt: value(log, 'updatedAt', 'updated_at', ''),
  };
}

function normalizeHabitSummary(raw: unknown): HabitSummary {
  const summary = raw as RawRecord;
  return {
    activeCount: value(summary, 'activeCount', 'active_count', 0),
    completedToday: value(summary, 'completedToday', 'completed_today', 0),
    completionRate: value(summary, 'completionRate', 'completion_rate', 0),
    bestStreak: value(summary, 'bestStreak', 'best_streak', 0),
  };
}

function normalizeCountdown(raw: unknown): Countdown {
  const countdown = raw as RawRecord;
  return {
    id: value(countdown, 'id', 'id', ''),
    title: value(countdown, 'title', 'title', ''),
    targetDate: value(countdown, 'targetDate', 'target_date', ''),
    mode: value(countdown, 'mode', 'mode', 'countdown'),
    icon: value(countdown, 'icon', 'icon', ''),
    color: value(countdown, 'color', 'color', null),
    recurrence: value(countdown, 'recurrence', 'recurrence', null),
    reminderAt: value(countdown, 'reminderAt', 'reminder_at', null),
    archived: value(countdown, 'archived', 'archived', false),
    position: value(countdown, 'position', 'position', 0),
    createdAt: value(countdown, 'createdAt', 'created_at', ''),
    updatedAt: value(countdown, 'updatedAt', 'updated_at', ''),
  };
}

export async function nodeGet(id: string): Promise<Node> {
  return normalizeNode(await invoke('node_get', { id }));
}

export async function nodeCreate(
  parentId: string | null,
  nodeType: string,
  title: string,
  body?: string,
): Promise<Node> {
  return normalizeNode(
    await invoke('node_create', { parentId, nodeType, title, body: body || '' }),
  );
}

export async function nodeUpdate(id: string, changes: UpdateNodeInput): Promise<Node> {
  let normalizedChanges = changes;
  if (changes.properties !== undefined) {
    const current = await nodeGet(id);
    normalizedChanges = {
      ...changes,
      properties: {
        ...current.properties,
        ...changes.properties,
      },
    };
  }
  return normalizeNode(await invoke('node_update', { id, changes: denormalizeUpdateNodeInput(normalizedChanges) }));
}

export async function nodeDelete(id: string, permanent: boolean = false): Promise<void> {
  return invoke('node_delete', { id, permanent });
}

export async function trashGetNodes(): Promise<Node[]> {
  const nodes = await invoke<unknown[]>('trash_get_nodes');
  return nodes.map(normalizeNode);
}

export async function trashRestore(
  id: string,
  destinationParentId: string | null = null,
): Promise<Node> {
  return normalizeNode(await invoke('trash_restore', { id, destinationParentId }));
}

export async function trashEmpty(): Promise<number> {
  return invoke('trash_empty');
}

export async function nodeDuplicate(id: string): Promise<Node> {
  return normalizeNode(await invoke('node_duplicate', { id }));
}

export async function nodeMove(
  id: string,
  newParentId: string | null,
  position: number,
): Promise<void> {
  return invoke('node_move', { id, newParentId, position });
}

export async function nodeReorder(parentId: string, childIds: string[]): Promise<void> {
  return invoke('node_reorder', { parentId, childIds });
}

export async function treeGetChildren(parentId: string | null): Promise<Node[]> {
  const nodes = await invoke<unknown[]>('tree_get_children', { parentId });
  return nodes.map(normalizeNode);
}

export async function treeGetAncestors(id: string): Promise<Node[]> {
  const nodes = await invoke<unknown[]>('tree_get_ancestors', { id });
  return nodes.map(normalizeNode);
}

export async function treeGetFull(rootId: string | null = null): Promise<Node[]> {
  const nodes = await invoke<unknown[]>('tree_get_full', { rootId });
  return nodes.map(normalizeNode);
}

export async function nodeSetDueDate(id: string, dueDate: string | null): Promise<Node> {
  return normalizeNode(await invoke('node_set_due_date', { id, dueDate }));
}

export async function nodeSetPriority(id: string, priority: number): Promise<Node> {
  return normalizeNode(await invoke('node_set_priority', { id, priority }));
}

export async function nodeToggleComplete(id: string): Promise<Node> {
  return normalizeNode(await invoke('node_toggle_complete', { id }));
}

export async function nodeSetCompletion(
  id: string,
  completed: boolean,
  cascade: boolean,
): Promise<Node> {
  return normalizeNode(await invoke('node_set_completion', { id, completed, cascade }));
}

export async function nodeIncompleteDescendantCount(id: string): Promise<number> {
  return invoke('node_incomplete_descendant_count', { id });
}

export async function searchQuery(
  query: string,
  filters: SearchFilters = {},
): Promise<SearchResult[]> {
  const results = await invoke<unknown[]>('search_query', { query, filters });
  return results.map(normalizeSearchResult);
}

export async function quickFind(query: string): Promise<Node[]> {
  const nodes = await invoke<unknown[]>('quick_find', { query });
  return nodes.map(normalizeNode);
}

export async function getTodayTasks(): Promise<Node[]> {
  const nodes = await invoke<unknown[]>('get_today_tasks');
  return nodes.map(normalizeNode);
}

export async function getNodeCount(): Promise<number> {
  return invoke('get_node_count');
}

export async function undo(): Promise<string> {
  return invoke('undo');
}

export async function redo(): Promise<string> {
  return invoke('redo');
}

export async function exportJson(rootId: string | null = null): Promise<string> {
  return invoke('export_json', { rootId });
}

export async function exportMarkdown(rootId: string | null, outputDir: string): Promise<void> {
  return invoke('export_markdown', { rootId, outputDir });
}

export async function importJson(json: string, parentId: string | null = null): Promise<string[]> {
  return invoke('import_json', { json, parentId });
}

export async function tagList(): Promise<Tag[]> {
  const tags = await invoke<unknown[]>('tag_list');
  return tags.map(normalizeTag);
}

export async function tagCreate(name: string, color: string | null = null): Promise<Tag> {
  return normalizeTag(await invoke('tag_create', { name, color }));
}

export async function tagRename(
  id: string,
  name: string,
  color: string | null = null,
): Promise<Tag> {
  return normalizeTag(await invoke('tag_rename', { id, name, color }));
}

export async function tagDelete(id: string): Promise<void> {
  return invoke('tag_delete', { id });
}

export async function tagAssign(nodeId: string, tagId: string): Promise<void> {
  return invoke('tag_assign', { nodeId, tagId });
}

export async function tagRemove(nodeId: string, tagId: string): Promise<void> {
  return invoke('tag_remove', { nodeId, tagId });
}

export async function tagGetForNode(nodeId: string): Promise<Tag[]> {
  const tags = await invoke<unknown[]>('tag_get_for_node', { nodeId });
  return tags.map(normalizeTag);
}

export async function tagQueryTasks(tagId: string): Promise<Node[]> {
  const nodes = await invoke<unknown[]>('tag_query_tasks', { tagId });
  return nodes.map(normalizeNode);
}

export async function tagSyncLegacy(): Promise<number> {
  return invoke('tag_sync_legacy');
}

export async function timeBlockList(start: string, end: string): Promise<TimeBlock[]> {
  const blocks = await invoke<unknown[]>('time_block_list', { start, end });
  return blocks.map(normalizeTimeBlock);
}

export async function timeBlockCreate(input: CreateTimeBlockInput): Promise<TimeBlock> {
  return normalizeTimeBlock(await invoke('time_block_create', { input }));
}

export async function timeBlockUpdate(
  id: string,
  input: UpdateTimeBlockInput,
): Promise<TimeBlock> {
  return normalizeTimeBlock(await invoke('time_block_update', { id, input }));
}

export async function timeBlockDelete(id: string): Promise<void> {
  return invoke('time_block_delete', { id });
}

export async function focusStart(input: StartFocusInput): Promise<FocusSession> {
  return normalizeFocusSession(await invoke('focus_start', { input }));
}

export async function focusGetActive(): Promise<FocusSession | null> {
  const session = await invoke<unknown | null>('focus_get_active');
  return session ? normalizeFocusSession(session) : null;
}

export async function focusPause(id: string): Promise<FocusSession> {
  return normalizeFocusSession(await invoke('focus_pause', { id }));
}

export async function focusResume(id: string): Promise<FocusSession> {
  return normalizeFocusSession(await invoke('focus_resume', { id }));
}

export async function focusStop(id: string, input: StopFocusInput): Promise<FocusSession> {
  return normalizeFocusSession(await invoke('focus_stop', { id, input }));
}

export async function focusList(limit: number = 50): Promise<FocusSession[]> {
  const sessions = await invoke<unknown[]>('focus_list', { limit });
  return sessions.map(normalizeFocusSession);
}

export async function focusSummary(): Promise<FocusSummary> {
  return normalizeFocusSummary(await invoke('focus_summary'));
}

export async function savedFilterList(): Promise<SavedFilter[]> {
  const filters = await invoke<unknown[]>('saved_filter_list');
  return filters.map(normalizeSavedFilter);
}

export async function savedFilterCreate(input: CreateSavedFilterInput): Promise<SavedFilter> {
  return normalizeSavedFilter(await invoke('saved_filter_create', { input }));
}

export async function savedFilterUpdate(
  id: string,
  input: UpdateSavedFilterInput,
): Promise<SavedFilter> {
  return normalizeSavedFilter(await invoke('saved_filter_update', { id, input }));
}

export async function savedFilterDelete(id: string): Promise<void> {
  return invoke('saved_filter_delete', { id });
}

export async function habitList(includeArchived = false): Promise<Habit[]> {
  const habits = await invoke<unknown[]>('habit_list', { includeArchived });
  return habits.map(normalizeHabit);
}

export async function habitCreate(input: CreateHabitInput): Promise<Habit> {
  return normalizeHabit(await invoke('habit_create', { input }));
}

export async function habitUpdate(id: string, input: UpdateHabitInput): Promise<Habit> {
  return normalizeHabit(await invoke('habit_update', { id, input }));
}

export async function habitArchive(id: string, archived: boolean): Promise<Habit> {
  return normalizeHabit(await invoke('habit_archive', { id, archived }));
}

export async function habitDelete(id: string): Promise<void> {
  return invoke('habit_delete', { id });
}

export async function habitLogUpsert(input: UpsertHabitLogInput): Promise<HabitLog> {
  return normalizeHabitLog(await invoke('habit_log_upsert', { input }));
}

export async function habitLogDelete(habitId: string, logDate: string): Promise<void> {
  return invoke('habit_log_delete', { habitId, logDate });
}

export async function habitLogList(from: string, to: string): Promise<HabitLog[]> {
  const logs = await invoke<unknown[]>('habit_log_list', { from, to });
  return logs.map(normalizeHabitLog);
}

export async function habitSummary(from: string, to: string): Promise<HabitSummary> {
  return normalizeHabitSummary(await invoke('habit_summary', { from, to }));
}

export async function countdownList(includeArchived = false): Promise<Countdown[]> {
  const countdowns = await invoke<unknown[]>('countdown_list', { includeArchived });
  return countdowns.map(normalizeCountdown);
}

export async function countdownCreate(input: CreateCountdownInput): Promise<Countdown> {
  return normalizeCountdown(await invoke('countdown_create', { input }));
}

export async function countdownUpdate(id: string, input: UpdateCountdownInput): Promise<Countdown> {
  return normalizeCountdown(await invoke('countdown_update', { id, input }));
}

export async function countdownArchive(id: string, archived: boolean): Promise<Countdown> {
  return normalizeCountdown(await invoke('countdown_archive', { id, archived }));
}

export async function countdownDelete(id: string): Promise<void> {
  return invoke('countdown_delete', { id });
}

export async function countdownReorder(ids: string[]): Promise<Countdown[]> {
  const countdowns = await invoke<unknown[]>('countdown_reorder', { ids });
  return countdowns.map(normalizeCountdown);
}

export async function settingsGet<T = unknown>(key: string): Promise<T | null> {
  return invoke<T | null>('settings_get', { key });
}

export async function settingsSet(key: string, settingValue: unknown): Promise<void> {
  return invoke('settings_set', { key, value: settingValue });
}

export async function settingsDelete(key: string): Promise<void> {
  return invoke('settings_delete', { key });
}

export async function settingsList(prefix: string | null = null): Promise<Array<[string, unknown]>> {
  return invoke<Array<[string, unknown]>>('settings_list', { prefix });
}

export async function backupCreate(): Promise<string> {
  return invoke('backup_create');
}

export async function backupList(): Promise<string[]> {
  return invoke('backup_list');
}

export async function backupRestore(backupName: string): Promise<void> {
  return invoke('backup_restore', { backupName });
}
