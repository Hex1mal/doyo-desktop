import type { CompletionPolicy } from '$lib/stores/ui.svelte';

export function completionCascadeMessage(count: number) {
  return `This task has ${count} incomplete descendant ${count === 1 ? 'task' : 'tasks'}. Complete them too?`;
}

export function shouldInspectDescendants(policy: CompletionPolicy, completing: boolean) {
  return completing && policy === 'ask';
}

export function shouldCascadeWithoutPrompt(policy: CompletionPolicy, completing: boolean) {
  return completing && policy === 'cascade';
}
