export function normalizeFilterDefinition(definition: Record<string, unknown>) {
  return Object.fromEntries(
    Object.entries(definition).filter(([, value]) => {
      if (value === undefined || value === null || value === '') return false;
      if (Array.isArray(value) && value.length === 0) return false;
      return true;
    }),
  );
}

export function calculateParetoScore(impact: number, effort: number) {
  const safeImpact = Math.max(0, Math.min(100, Number.isFinite(impact) ? impact : 0));
  const safeEffort = Math.max(1, Math.min(100, Number.isFinite(effort) ? effort : 1));
  return Math.round((safeImpact / safeEffort) * 100);
}

export function flowtimeBreakSuggestion(durationSeconds: number) {
  const minutes = Math.max(0, Math.round(durationSeconds / 60));
  if (minutes < 25) return 5;
  if (minutes < 50) return 8;
  if (minutes < 90) return 12;
  return 20;
}

export function countdownDelta(
  targetDate: string,
  mode: 'countdown' | 'countup',
  now = new Date(),
) {
  const target = new Date(targetDate);
  if (Number.isNaN(target.getTime())) return { days: 0, label: 'Invalid date' };
  const diff =
    mode === 'countup' ? now.getTime() - target.getTime() : target.getTime() - now.getTime();
  const days = Math.floor(Math.abs(diff) / 86_400_000);
  if (mode === 'countup') return { days, label: `${days} days since` };
  return { days, label: diff < 0 ? `${days} days overdue` : `${days} days left` };
}

export function localDateKey(date = new Date()) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

export type StatsRange = 'day' | 'week' | 'month';

export function rangeStart(range: StatsRange, now = new Date()) {
  const start = new Date(now);
  start.setHours(0, 0, 0, 0);
  if (range === 'day') return start;
  if (range === 'week') {
    start.setDate(start.getDate() - 6);
    return start;
  }
  start.setDate(start.getDate() - 29);
  return start;
}

export function isWithinRange(
  dateValue: string | null | undefined,
  range: StatsRange,
  now = new Date(),
) {
  if (!dateValue) return false;
  const date = new Date(dateValue);
  if (Number.isNaN(date.getTime())) return false;
  const start = rangeStart(range, now);
  const end = new Date(now);
  end.setHours(23, 59, 59, 999);
  return date >= start && date <= end;
}

export function buildDailyBuckets(
  range: StatsRange,
  values: Array<{ date: string | null | undefined; amount?: number }>,
  now = new Date(),
) {
  const start = rangeStart(range, now);
  const end = new Date(now);
  end.setHours(0, 0, 0, 0);
  const days = Math.max(1, Math.round((end.getTime() - start.getTime()) / 86_400_000) + 1);
  const buckets = Array.from({ length: days }, (_, index) => {
    const date = new Date(start);
    date.setDate(start.getDate() + index);
    return { key: localDateKey(date), value: 0 };
  });
  const byKey = new Map(buckets.map((bucket) => [bucket.key, bucket]));
  for (const item of values) {
    if (!item.date) continue;
    const key = localDateKey(new Date(item.date));
    const bucket = byKey.get(key);
    if (bucket) bucket.value += item.amount ?? 1;
  }
  return buckets;
}
