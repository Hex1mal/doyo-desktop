import type { Node, TimeBlock } from '../types/node';

export type CalendarViewMode = 'month' | 'week' | 'day' | 'agenda';

export function startOfLocalDay(date: Date) {
  const copy = new Date(date);
  copy.setHours(0, 0, 0, 0);
  return copy;
}

export function addDays(date: Date, days: number) {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + days);
  return copy;
}

export function addMinutes(date: Date, minutes: number) {
  return new Date(date.getTime() + minutes * 60000);
}

export function localDayKey(date: Date | string) {
  const d = typeof date === 'string' ? new Date(date) : date;
  if (Number.isNaN(d.getTime())) return '';
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
    d.getDate(),
  ).padStart(2, '0')}`;
}

export function parseLocalDayKey(dayKey: string) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dayKey);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const date = new Date(year, month - 1, day);
  if (
    Number.isNaN(date.getTime()) ||
    date.getFullYear() !== year ||
    date.getMonth() !== month - 1 ||
    date.getDate() !== day
  ) {
    return null;
  }
  return date;
}

export function weekStart(date: Date, firstDayOfWeek: number) {
  const start = startOfLocalDay(date);
  const delta = (start.getDay() - firstDayOfWeek + 7) % 7;
  start.setDate(start.getDate() - delta);
  return start;
}

export function monthGrid(date: Date, firstDayOfWeek: number) {
  const first = new Date(date.getFullYear(), date.getMonth(), 1);
  const start = weekStart(first, firstDayOfWeek);
  return Array.from({ length: 42 }, (_, index) => addDays(start, index));
}

export function visibleRange(view: CalendarViewMode, currentDate: Date, firstDayOfWeek: number) {
  if (view === 'month') {
    const days = monthGrid(currentDate, firstDayOfWeek);
    return { start: days[0], end: addDays(days[41], 1) };
  }
  if (view === 'week') {
    const start = weekStart(currentDate, firstDayOfWeek);
    return { start, end: addDays(start, 7) };
  }
  if (view === 'agenda') {
    const start = startOfLocalDay(currentDate);
    return { start, end: addDays(start, 30) };
  }
  const start = startOfLocalDay(currentDate);
  return { start, end: addDays(start, 1) };
}

export function isAllDayTask(node: Node) {
  const custom = node.properties.custom;
  if (custom && typeof custom === 'object' && !Array.isArray(custom) && custom.calendarAllDay) {
    return true;
  }
  if (!node.properties.dueDate) return true;
  const due = new Date(node.properties.dueDate);
  return due.getHours() === 0 && due.getMinutes() === 0;
}

export function itemDurationMinutes(node: Node | TimeBlock) {
  if ('startTime' in node) {
    const minutes = Math.round(
      (new Date(node.endTime).getTime() - new Date(node.startTime).getTime()) / 60000,
    );
    return Math.max(15, minutes);
  }
  return node.properties.estimatedDurationMinutes ?? 60;
}

export function dateWithTime(day: Date, hour: number, minute = 0) {
  const date = startOfLocalDay(day);
  date.setHours(hour, minute, 0, 0);
  return date;
}

export function hourFromPointerY(clientY: number, slotElement: HTMLElement) {
  const hour = Number(slotElement.dataset.calendarHour);
  if (!Number.isFinite(hour)) return null;
  const rect = slotElement.getBoundingClientRect();
  const ratio = Math.min(0.999, Math.max(0, (clientY - rect.top) / Math.max(1, rect.height)));
  const minute = Math.round((ratio * 60) / 15) * 15;
  if (minute >= 60) return { hour: hour + 1, minute: 0 };
  return { hour, minute };
}

/**
 * Describe where a dragged task lands: the new due date, plus the one custom key
 * the calendar owns. Deliberately not a whole `custom` object - callers send this
 * straight through as a patch, and returning a merged snapshot here would put
 * every other view's metadata back on the wire.
 */
export function moveTaskDate(node: Node, targetDay: Date, hour?: number, minute = 0) {
  if (hour === undefined) {
    const date = startOfLocalDay(targetDay);
    return {
      dueDate: date.toISOString(),
      custom: { calendarAllDay: true },
    };
  }
  const date = dateWithTime(targetDay, hour, minute);
  return {
    dueDate: date.toISOString(),
    custom: { calendarAllDay: false },
  };
}

export function validateTimeRange(start: Date, end: Date) {
  return !Number.isNaN(start.getTime()) && !Number.isNaN(end.getTime()) && end > start;
}

export function tasksByDay(nodes: Node[], includeCompleted: boolean) {
  const result = new Map<string, Node[]>();
  for (const node of nodes) {
    if (node.deletedAt || node.nodeType !== 'Task' || !node.properties.dueDate) continue;
    if (node.isCompleted && !includeCompleted) continue;
    const key = localDayKey(node.properties.dueDate);
    const list = result.get(key) ?? [];
    list.push(node);
    result.set(key, list);
  }
  for (const list of result.values()) {
    list.sort((a, b) => {
      const due = (a.properties.dueDate ?? '').localeCompare(b.properties.dueDate ?? '');
      if (due) return due;
      return (a.properties.priority ?? 4) - (b.properties.priority ?? 4);
    });
  }
  return result;
}

export function blocksByDay(blocks: TimeBlock[]) {
  const result = new Map<string, TimeBlock[]>();
  for (const block of blocks) {
    const key = localDayKey(block.startTime);
    const list = result.get(key) ?? [];
    list.push(block);
    result.set(key, list);
  }
  for (const list of result.values()) {
    list.sort((a, b) => a.startTime.localeCompare(b.startTime));
  }
  return result;
}
