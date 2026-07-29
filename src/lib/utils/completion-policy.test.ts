import { describe, expect, it } from 'vitest';
import {
  completionCascadeMessage,
  shouldCascadeWithoutPrompt,
  shouldInspectDescendants,
} from './completion-policy';

describe('completion policy helpers', () => {
  it('uses individual as a non-recursive policy', () => {
    expect(shouldInspectDescendants('individual', true)).toBe(false);
    expect(shouldCascadeWithoutPrompt('individual', true)).toBe(false);
  });

  it('asks only when completing in ask mode', () => {
    expect(shouldInspectDescendants('ask', true)).toBe(true);
    expect(shouldInspectDescendants('ask', false)).toBe(false);
  });

  it('cascades only when completing in cascade mode', () => {
    expect(shouldCascadeWithoutPrompt('cascade', true)).toBe(true);
    expect(shouldCascadeWithoutPrompt('cascade', false)).toBe(false);
  });

  it('states the number of affected descendants', () => {
    expect(completionCascadeMessage(5)).toContain('5 incomplete descendant tasks');
    expect(completionCascadeMessage(1)).toContain('1 incomplete descendant task');
  });
});
