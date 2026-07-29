import type { Node } from '../types/node';
import { addDays, localDayKey, startOfLocalDay } from './calendar';

export type TimelineZoom = 'day' | 'week' | 'month';

export interface TimelineRange {
  start: Date;
  end: Date;
  days: Date[];
}

export function timelineVisibleRange(currentDate: Date, zoom: TimelineZoom): TimelineRange {
  const start = startOfLocalDay(currentDate);
  const length = zoom === 'day' ? 7 : zoom === 'week' ? 28 : 90;
  const days = Array.from({ length }, (_, index) => addDays(start, index));
  return { start, end: addDays(start, length), days };
}

export function taskTimelineRange(node: Node) {
  const due = node.properties.dueDate ? new Date(node.properties.dueDate) : null;
  const start = node.properties.startDate ? new Date(node.properties.startDate) : due;
  if (!due || !start || Number.isNaN(due.getTime()) || Number.isNaN(start.getTime())) return null;
  const startDay = startOfLocalDay(start);
  const endDay = startOfLocalDay(due);
  if (endDay < startDay) return null;
  return { start: startDay, end: endDay };
}

export function validateTimelineRange(start: Date, end: Date) {
  return !Number.isNaN(start.getTime()) && !Number.isNaN(end.getTime()) && end >= start;
}

export function moveTimelineRange(node: Node, dayDelta: number) {
  const range = taskTimelineRange(node);
  if (!range) return null;
  const nextStart = addDays(range.start, dayDelta);
  const nextEnd = addDays(range.end, dayDelta);
  return {
    startDate: node.properties.startDate ? nextStart.toISOString() : null,
    dueDate: nextEnd.toISOString(),
  };
}

export function resizeTimelineStart(node: Node, dayDelta: number) {
  const range = taskTimelineRange(node);
  if (!range) return null;
  const nextStart = addDays(range.start, dayDelta);
  if (!validateTimelineRange(nextStart, range.end)) return null;
  return {
    startDate: nextStart.toISOString(),
    dueDate: range.end.toISOString(),
  };
}

export function resizeTimelineEnd(node: Node, dayDelta: number) {
  const range = taskTimelineRange(node);
  if (!range) return null;
  const nextEnd = addDays(range.end, dayDelta);
  if (!validateTimelineRange(range.start, nextEnd)) return null;
  return {
    startDate: node.properties.startDate ? range.start.toISOString() : null,
    dueDate: nextEnd.toISOString(),
  };
}

export function timelineDayIndex(day: Date, days: Date[]) {
  const key = localDayKey(day);
  return days.findIndex((candidate) => localDayKey(candidate) === key);
}
