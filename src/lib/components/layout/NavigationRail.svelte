<script lang="ts">
  import { uiStore, type ActiveModule } from '$lib/stores/ui.svelte';
  import { nodeStore } from '$lib/stores/nodes.svelte';

  type RailItem = {
    id: ActiveModule;
    label: string;
    icon: string;
    viewMode?: 'tree' | 'today' | 'inbox' | 'upcoming' | 'search';
  };

  // Grouped rather than one flat run of twelve. The rail is icon-only, so
  // proximity is the only structure available to say which destinations belong
  // together; without it every module reads as equally likely.
  const topGroups: { name: string; items: RailItem[] }[] = [
    {
      name: 'Focus for today',
      items: [
        { id: 'today', label: 'Today', icon: '◎', viewMode: 'today' },
        { id: 'inbox', label: 'Inbox', icon: '□', viewMode: 'inbox' },
        { id: 'upcoming', label: 'Upcoming', icon: '7', viewMode: 'upcoming' },
      ],
    },
    {
      name: 'Organize',
      items: [{ id: 'workspaces', label: 'Workspaces', icon: 'W', viewMode: 'tree' }],
    },
    {
      name: 'Plan',
      items: [
        { id: 'calendar', label: 'Calendar', icon: 'C' },
        { id: 'kanban', label: 'Kanban', icon: 'K' },
        { id: 'timeline', label: 'Timeline', icon: 'T' },
      ],
    },
    {
      name: 'Practice',
      items: [
        { id: 'productivity', label: 'Productivity Methods', icon: 'P' },
        { id: 'habits', label: 'Habits', icon: 'H' },
        { id: 'countdowns', label: 'Countdowns', icon: 'D' },
      ],
    },
    {
      name: 'Review',
      items: [
        { id: 'statistics', label: 'Statistics', icon: 'S' },
        { id: 'search', label: 'Search', icon: '⌕', viewMode: 'search' },
      ],
    },
  ];

  const bottomItems: RailItem[] = [{ id: 'settings', label: 'Settings', icon: '⚙' }];

  // The rail scrolls when the window is short or the app is zoomed in. Its
  // scrollbar is hidden to keep the 52px column clean, so without this the only
  // hint that more modules exist is a cut-off icon edge. Fade the ends that have
  // content beyond them, and only those ends.
  let railTopEl: HTMLElement | undefined = $state();
  let hasContentAbove = $state(false);
  let hasContentBelow = $state(false);

  function measureOverflow() {
    const el = railTopEl;
    if (!el) return;
    const maxScroll = el.scrollHeight - el.clientHeight;
    // Sub-pixel layout leaves a fraction of a pixel of "scroll" on rails that
    // fit; 1px of slack keeps the fade off in that case.
    hasContentAbove = maxScroll > 1 && el.scrollTop > 1;
    hasContentBelow = maxScroll > 1 && el.scrollTop < maxScroll - 1;
  }

  $effect(() => {
    const el = railTopEl;
    if (!el) return;
    measureOverflow();
    // Height changes come from window resizes and from zoom, which does not
    // fire a resize event, so observe the element itself.
    const observer = new ResizeObserver(measureOverflow);
    observer.observe(el);
    return () => observer.disconnect();
  });

  function activate(item: RailItem) {
    uiStore.setActiveModule(item.id);
    if (item.viewMode) {
      nodeStore.setViewMode(item.viewMode);
      if (item.viewMode !== 'tree') {
        nodeStore.select(null);
        nodeStore.setFocusRoot(null);
      }
    }
  }
</script>

<nav class="rail" aria-label="Primary modules">
  <div
    class="rail-top"
    class:fade-top={hasContentAbove}
    class:fade-bottom={hasContentBelow}
    bind:this={railTopEl}
    onscroll={measureOverflow}
  >
    {#each topGroups as group (group.name)}
      <div class="rail-group" role="group" aria-label={group.name}>
        {#each group.items as item (item.id)}
          <button
            class="rail-button"
            class:active={uiStore.activeModule === item.id}
            title={item.label}
            aria-label={item.label}
            aria-current={uiStore.activeModule === item.id ? 'page' : undefined}
            onclick={() => activate(item)}
          >
            <span>{item.icon}</span>
          </button>
        {/each}
      </div>
    {/each}
  </div>

  <div class="rail-group bottom">
    {#each bottomItems as item}
      <button
        class="rail-button"
        class:active={uiStore.activeModule === item.id}
        title={item.label}
        aria-label={item.label}
        aria-current={uiStore.activeModule === item.id ? 'page' : undefined}
        onclick={() => activate(item)}
      >
        <span>{item.icon}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .rail {
    width: 52px;
    flex: 0 0 52px;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 8px 6px;
    overflow: hidden;
  }

  .rail-top {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }
  .rail-top::-webkit-scrollbar {
    display: none;
  }

  /* Fade the scrollable ends so a clipped icon reads as "there is more here"
     rather than as a rendering glitch. Applied as a mask so it works over the
     sidebar background in either theme without hard-coding a colour. */
  .rail-top.fade-top {
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, #000 24px);
    mask-image: linear-gradient(to bottom, transparent 0, #000 24px);
  }
  .rail-top.fade-bottom {
    -webkit-mask-image: linear-gradient(to top, transparent 0, #000 24px);
    mask-image: linear-gradient(to top, transparent 0, #000 24px);
  }
  .rail-top.fade-top.fade-bottom {
    -webkit-mask-image: linear-gradient(
      to bottom,
      transparent 0,
      #000 24px,
      #000 calc(100% - 24px),
      transparent 100%
    );
    mask-image: linear-gradient(
      to bottom,
      transparent 0,
      #000 24px,
      #000 calc(100% - 24px),
      transparent 100%
    );
  }

  /* Keyboard focus scrolls the button into view; leave room so it does not land
     flush against a faded edge. */
  .rail-top .rail-button {
    scroll-margin: 28px 0;
  }

  .rail-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  /* Whitespace carries the grouping; a rule is only needed where the gap alone
     could read as an accident. */
  .rail-top .rail-group + .rail-group {
    padding-top: 14px;
    border-top: 1px solid var(--border);
    width: 24px;
  }

  .rail-button {
    width: 36px;
    height: 36px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 800;
    line-height: 1;
  }

  .rail-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .rail-button.active {
    background: var(--accent);
    color: white;
  }

  .rail-button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* Twelve modules do not fit a 600px-tall window at the default rhythm. Tighten
     the spacing before falling back to scrolling: reaching every module without
     scrolling beats preserving the exact gaps. Grouping is preserved, and the
     targets stay at 32px. */
  @media (max-height: 760px) {
    .rail-top {
      gap: 8px;
    }
    .rail-top .rail-group + .rail-group {
      padding-top: 8px;
    }
    .rail-button {
      width: 32px;
      height: 32px;
    }
  }
</style>
