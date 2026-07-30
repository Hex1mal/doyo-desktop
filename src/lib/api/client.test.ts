import { describe, expect, it } from 'vitest';
import { denormalizeProperties, normalizeProperties } from './client';

describe('api property normalization', () => {
  it('round-trips nested reminder offset naming', () => {
    const payload = denormalizeProperties({
      dueDate: '2026-07-31T01:30:00.000Z',
      reminders: [{ time: null, offsetMinutes: -30, type: 'relative' }],
      estimatedDurationMinutes: 90,
    });

    expect(payload).toMatchObject({
      due_date: '2026-07-31T01:30:00.000Z',
      estimated_duration_minutes: 90,
      reminders: [{ time: null, offset_minutes: -30, type: 'relative' }],
    });

    expect(normalizeProperties(payload)).toMatchObject({
      dueDate: '2026-07-31T01:30:00.000Z',
      estimatedDurationMinutes: 90,
      reminders: [{ time: null, offsetMinutes: -30, type: 'relative' }],
    });
  });
});
