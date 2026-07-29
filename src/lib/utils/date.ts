/** Simple natural language date parser for due dates */
export function parseNaturalDate(input: string): string | null {
  const s = input.trim().toLowerCase();
  if (!s) return null;

  const now = new Date();
  const at = (d: Date, h = 9, m = 0) => {
    d.setHours(h, m, 0, 0);
    return d.toISOString();
  };

  if (s === 'today' || s === 'tod') return at(new Date(now));
  if (s === 'tomorrow' || s === 'tmr' || s === 'tom') {
    const d = new Date(now);
    d.setDate(d.getDate() + 1);
    return at(d);
  }
  if (s === 'yesterday') {
    const d = new Date(now);
    d.setDate(d.getDate() - 1);
    return at(d);
  }

  const inMatch = s.match(/^in\s+(\d+)\s*(d|day|days|w|week|weeks|m|month|months)?$/);
  if (inMatch) {
    const n = Number(inMatch[1]);
    const unit = inMatch[2] || 'd';
    const d = new Date(now);
    if (unit.startsWith('w')) d.setDate(d.getDate() + n * 7);
    else if (unit.startsWith('m')) d.setMonth(d.getMonth() + n);
    else d.setDate(d.getDate() + n);
    return at(d);
  }

  const nextDow = s.match(/^next\s+(mon|tue|wed|thu|fri|sat|sun|monday|tuesday|wednesday|thursday|friday|saturday|sunday)$/);
  if (nextDow) {
    const map: Record<string, number> = {
      sun: 0, sunday: 0, mon: 1, monday: 1, tue: 2, tuesday: 2, wed: 3, wednesday: 3,
      thu: 4, thursday: 4, fri: 5, friday: 5, sat: 6, saturday: 6,
    };
    const target = map[nextDow[1]];
    const d = new Date(now);
    const delta = (target + 7 - d.getDay()) % 7 || 7;
    d.setDate(d.getDate() + delta);
    return at(d);
  }

  // ISO or YYYY-MM-DD
  if (/^\d{4}-\d{2}-\d{2}/.test(s)) {
    const d = new Date(s);
    if (!isNaN(d.getTime())) return d.toISOString();
  }

  // MM/DD or DD.MM
  const slash = s.match(/^(\d{1,2})[\/\-.](\d{1,2})(?:[\/\-.](\d{2,4}))?$/);
  if (slash) {
    const a = Number(slash[1]);
    const b = Number(slash[2]);
    const y = slash[3] ? Number(slash[3].length === 2 ? '20' + slash[3] : slash[3]) : now.getFullYear();
    // Prefer ISO-ish: if first > 12 treat as DD/MM else MM/DD for US, but default DD/MM for Linux power users
    let month: number, day: number;
    if (a > 12) {
      day = a; month = b;
    } else if (b > 12) {
      month = a; day = b;
    } else {
      day = a; month = b; // DD/MM default
    }
    const d = new Date(y, month - 1, day, 9, 0, 0);
    if (!isNaN(d.getTime())) return d.toISOString();
  }

  return null;
}

export function formatDue(iso?: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (isNaN(d.getTime())) return '';
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const target = new Date(d);
  target.setHours(0, 0, 0, 0);
  const diff = Math.round((target.getTime() - today.getTime()) / 86400000);
  if (diff === 0) return 'Today';
  if (diff === 1) return 'Tomorrow';
  if (diff === -1) return 'Yesterday';
  if (diff < 0) return `${Math.abs(diff)}d overdue`;
  if (diff < 7) return d.toLocaleDateString(undefined, { weekday: 'short' });
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function isOverdue(iso?: string | null): boolean {
  if (!iso) return false;
  const d = new Date(iso);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  return d < today;
}
