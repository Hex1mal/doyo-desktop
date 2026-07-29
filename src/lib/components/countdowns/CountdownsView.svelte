<script lang="ts">
  import { countdownDelta, countdownStore } from '$lib/stores/countdowns.svelte';
  import type { CountdownMode } from '$lib/types/node';

  let didLoad = false;
  let title = $state('');
  let mode = $state<CountdownMode>('countdown');
  let targetDate = $state(new Date(Date.now() + 86_400_000).toISOString().slice(0, 16));
  let color = $state('#10B981');

  $effect(() => {
    if (didLoad) return;
    didLoad = true;
    countdownStore.load();
  });

  function localInputToIso(value: string) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? new Date().toISOString() : date.toISOString();
  }

  function createCountdown() {
    countdownStore.create({
      title,
      mode,
      targetDate: localInputToIso(targetDate),
      color,
    });
    title = '';
  }

  function isoToLocalInput(value: string | null) {
    if (!value) return '';
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return '';
    const offsetMs = date.getTimezoneOffset() * 60_000;
    return new Date(date.getTime() - offsetMs).toISOString().slice(0, 16);
  }

  function moveCountdown(id: string, direction: -1 | 1) {
    const ids = countdownStore.countdowns.map((countdown) => countdown.id);
    const index = ids.indexOf(id);
    const nextIndex = index + direction;
    if (index < 0 || nextIndex < 0 || nextIndex >= ids.length) return;
    [ids[index], ids[nextIndex]] = [ids[nextIndex], ids[index]];
    countdownStore.reorder(ids);
  }
</script>

<section class="countdowns-view" aria-label="Countdowns">
  <div class="view-header">
    <div>
      <h2>Countdowns</h2>
      <p>Count down to dates or count up from milestones. Records are local and durable.</p>
    </div>
    <label class="inline">
      <input
        type="checkbox"
        checked={countdownStore.showArchived}
        onchange={(event) =>
          countdownStore.setShowArchived((event.target as HTMLInputElement).checked)}
      />
      Show archived
    </label>
  </div>

  <form
    class="countdown-form"
    onsubmit={(event) => {
      event.preventDefault();
      createCountdown();
    }}
  >
    <input placeholder="Countdown title" bind:value={title} required />
    <select bind:value={mode}>
      <option value="countdown">Countdown</option>
      <option value="countup">Count up</option>
    </select>
    <input type="datetime-local" bind:value={targetDate} />
    <input type="color" bind:value={color} />
    <button class="primary" type="submit">Create</button>
  </form>

  {#if countdownStore.isLoading}
    <div class="empty-state small"><p>Loading countdowns...</p></div>
  {:else if countdownStore.countdowns.length === 0}
    <div class="empty-state small"><p>No countdowns yet</p></div>
  {:else}
    <div class="countdown-grid">
      {#each countdownStore.countdowns as countdown (countdown.id)}
        {@const delta = countdownDelta(countdown.targetDate, countdown.mode)}
        <article class:archived={countdown.archived}>
          <div class="accent" style={`background: ${countdown.color ?? '#64748B'}`}></div>
          <div>
            <input
              value={countdown.title}
              aria-label="Countdown title"
              onchange={(event) =>
                countdownStore.update(countdown.id, {
                  title: (event.target as HTMLInputElement).value,
                })}
            />
            <div class="edit-grid">
              <select
                value={countdown.mode}
                onchange={(event) =>
                  countdownStore.update(countdown.id, {
                    mode: (event.target as HTMLSelectElement).value as CountdownMode,
                  })}
              >
                <option value="countdown">Countdown</option>
                <option value="countup">Count up</option>
              </select>
              <input
                aria-label="Target date"
                type="datetime-local"
                value={isoToLocalInput(countdown.targetDate)}
                onchange={(event) =>
                  countdownStore.update(countdown.id, {
                    targetDate: localInputToIso((event.target as HTMLInputElement).value),
                  })}
              />
              <input
                aria-label="Reminder"
                type="datetime-local"
                value={isoToLocalInput(countdown.reminderAt)}
                onchange={(event) =>
                  countdownStore.update(countdown.id, {
                    reminderAt: (event.target as HTMLInputElement).value
                      ? localInputToIso((event.target as HTMLInputElement).value)
                      : null,
                  })}
              />
              <select
                value={countdown.recurrence ?? ''}
                onchange={(event) =>
                  countdownStore.update(countdown.id, {
                    recurrence: (event.target as HTMLSelectElement).value || null,
                  })}
              >
                <option value="">No repeat</option>
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
                <option value="monthly">Monthly</option>
                <option value="yearly">Yearly</option>
              </select>
              <input
                aria-label="Color"
                type="color"
                value={countdown.color ?? '#10B981'}
                onchange={(event) =>
                  countdownStore.update(countdown.id, {
                    color: (event.target as HTMLInputElement).value,
                  })}
              />
            </div>
          </div>
          <strong>{delta.days}</strong>
          <span>{delta.label}</span>
          <div class="actions">
            <button onclick={() => moveCountdown(countdown.id, -1)}>Up</button>
            <button onclick={() => moveCountdown(countdown.id, 1)}>Down</button>
            <button onclick={() => countdownStore.archive(countdown.id, !countdown.archived)}>
              {countdown.archived ? 'Restore' : 'Archive'}
            </button>
            <button class="danger" onclick={() => countdownStore.delete(countdown.id)}
              >Delete</button
            >
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .countdowns-view {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px;
    background: var(--bg-app);
  }
  .view-header,
  .countdown-form,
  .actions,
  .inline {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .view-header {
    justify-content: space-between;
  }
  .view-header p,
  article span {
    color: var(--text-tertiary);
  }
  .countdown-form {
    flex-wrap: wrap;
    margin: 12px 0;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
  }
  input,
  select,
  button {
    min-height: 32px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    padding: 0 8px;
  }
  button {
    cursor: pointer;
  }
  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: white;
  }
  .danger {
    color: var(--danger);
  }
  .countdown-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 10px;
  }
  .edit-grid {
    display: grid;
    grid-template-columns:
      minmax(110px, 0.7fr) minmax(160px, 1fr) minmax(160px, 1fr) minmax(110px, 0.7fr)
      42px;
    gap: 6px;
    min-width: 0;
    margin-top: 6px;
  }
  article {
    display: grid;
    gap: 8px;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-panel);
    padding: 10px;
  }
  article.archived {
    opacity: 0.65;
  }
  .accent {
    height: 4px;
    border-radius: 4px;
  }
  article strong {
    font-size: 34px;
  }
  @media (max-width: 900px) {
    .edit-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
