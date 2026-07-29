import type { FocusSession, HabitLog, Node } from '$lib/types/node';
import { buildDailyBuckets, isWithinRange, type StatsRange } from '$lib/utils/productivity';

export function uniqueActiveTaskRecords(nodes: Node[]) {
  const unique = new Map<string, Node>();
  for (const node of nodes) {
    if (node.nodeType === 'Task' && !node.deletedAt) unique.set(node.id, node);
  }
  return [...unique.values()];
}

export function taskStatistics(nodes: Node[], range: StatsRange, now = new Date()) {
  const tasks = uniqueActiveTaskRecords(nodes);
  const completedAll = tasks.filter((task) => task.isCompleted);
  const completedInRange = completedAll.filter((task) => isWithinRange(task.completedAt, range, now));
  const createdInRange = tasks.filter((task) => isWithinRange(task.createdAt, range, now));
  const overdue = tasks.filter((task) => {
    if (task.isCompleted || !task.properties.dueDate) return false;
    const due = new Date(task.properties.dueDate);
    return !Number.isNaN(due.getTime()) && due < now;
  });
  return {
    totalTasks: tasks.length,
    openTasks: tasks.filter((task) => !task.isCompleted).length,
    completedAll: completedAll.length,
    completedInRange: completedInRange.length,
    createdInRange: createdInRange.length,
    overdue: overdue.length,
    completionRate: tasks.length === 0 ? 0 : Math.round((completedAll.length / tasks.length) * 100),
    completedTrend: buildDailyBuckets(range, completedInRange.map((task) => ({ date: task.completedAt })), now),
    createdTrend: buildDailyBuckets(range, createdInRange.map((task) => ({ date: task.createdAt })), now),
  };
}

export function focusStatistics(sessions: FocusSession[], range: StatsRange, now = new Date()) {
  const inRange = sessions.filter((session) => isWithinRange(session.startedAt, range, now));
  const secondsByMethod = { pomodoro: 0, stopwatch: 0, flowtime: 0 };
  let plannedSeconds = 0;
  let actualSeconds = 0;
  for (const session of inRange) {
    secondsByMethod[session.method] += session.durationSeconds;
    plannedSeconds += session.plannedSeconds;
    actualSeconds += session.durationSeconds;
  }
  return {
    sessionCount: inRange.length,
    plannedSeconds,
    actualSeconds,
    pomodoroSeconds: secondsByMethod.pomodoro,
    stopwatchSeconds: secondsByMethod.stopwatch,
    flowtimeSeconds: secondsByMethod.flowtime,
    pomodoroCount: inRange.filter((session) => session.method === 'pomodoro').length,
    focusTrend: buildDailyBuckets(
      range,
      inRange.map((session) => ({ date: session.startedAt, amount: Math.round(session.durationSeconds / 60) })),
      now,
    ),
  };
}

export function habitStatistics(logs: HabitLog[], range: StatsRange, now = new Date()) {
  const inRange = logs.filter((log) => isWithinRange(`${log.logDate}T00:00:00`, range, now));
  const completed = inRange.filter((log) => log.status === 'completed');
  const partial = inRange.filter((log) => log.status === 'partial');
  const skipped = inRange.filter((log) => log.status === 'skipped');
  return {
    logCount: inRange.length,
    completed: completed.length,
    partial: partial.length,
    skipped: skipped.length,
    completionRate: inRange.length === 0 ? 0 : Math.round((completed.length / inRange.length) * 100),
    habitTrend: buildDailyBuckets(
      range,
      inRange.map((log) => ({ date: `${log.logDate}T00:00:00`, amount: log.status === 'completed' ? 1 : 0 })),
      now,
    ),
  };
}

