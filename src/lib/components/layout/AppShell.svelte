<script lang="ts">
  import Sidebar from '$lib/components/sidebar/Sidebar.svelte';
  import TreeView from '$lib/components/tree/TreeView.svelte';
  import Inspector from '$lib/components/inspector/Inspector.svelte';
  import CommandPalette from '$lib/components/palette/CommandPalette.svelte';
  import QuickOpen from '$lib/components/palette/QuickOpen.svelte';
  import DueDatePrompt from '$lib/components/shared/DueDatePrompt.svelte';
  import ToastHost from '$lib/components/shared/ToastHost.svelte';
  import StatusBar from '$lib/components/layout/StatusBar.svelte';
  import Breadcrumb from '$lib/components/layout/Breadcrumb.svelte';
  import NavigationRail from '$lib/components/layout/NavigationRail.svelte';
  import MoveDialog from '$lib/components/dialogs/MoveDialog.svelte';
  import NodeConfigDialog from '$lib/components/dialogs/NodeConfigDialog.svelte';
  import { uiStore, initTheme } from '$lib/stores/ui.svelte';
  import { handleGlobalKeydown } from '$lib/keyboard/handler';
  import { nodeStore } from '$lib/stores/nodes.svelte';
  import { reminderStore } from '$lib/stores/reminders.svelte';
  import StartupRecovery from '$lib/components/layout/StartupRecovery.svelte';
  import { startupStore } from '$lib/stores/startup.svelte';

  let didBootstrap = false;

  $effect(() => {
    if (didBootstrap) return;
    didBootstrap = true;
    initTheme();
    const loadWithRetry = async (attempt = 0) => {
      if (attempt === 0) {
        await uiStore.loadPersistedSettings();
        // Surfaced before anything else: if the database could not be opened,
        // the user needs to know before they start typing into an empty one.
        await startupStore.load();
      }
      const loaded = await nodeStore.load();
      reminderStore.start();
      if (!loaded && attempt < 8) {
        window.setTimeout(() => loadWithRetry(attempt + 1), 150);
      }
    };
    loadWithRetry();
  });

  function beginResize(which: 'sidebar' | 'inspector', event: MouseEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = which === 'sidebar' ? uiStore.sidebarWidth : uiStore.inspectorWidth;

    document.body.classList.add('is-resizing');

    const handleMove = (moveEvent: MouseEvent) => {
      const delta = moveEvent.clientX - startX;
      if (which === 'sidebar') {
        uiStore.setSidebarWidth(startWidth + delta);
      } else {
        uiStore.setInspectorWidth(startWidth - delta);
      }
    };

    const handleUp = () => {
      document.body.classList.remove('is-resizing');
      window.removeEventListener('mousemove', handleMove);
      window.removeEventListener('mouseup', handleUp);
    };

    window.addEventListener('mousemove', handleMove);
    window.addEventListener('mouseup', handleUp);
  }

  function handleViewportResize() {
    uiStore.clampPanelSizes(window.innerWidth);
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} onresize={handleViewportResize} />

<div class="root">
  <div class="app-shell">
    {#if !uiStore.focusMode}
      <NavigationRail />
    {/if}

    {#if !uiStore.focusMode && uiStore.sidebarVisible}
      <aside class="sidebar" style={`width: ${uiStore.sidebarWidth}px`}>
        <Sidebar />
      </aside>
      <button
        class="split-handle sidebar-handle"
        aria-label="Resize sidebar"
        title="Drag to resize. Double-click to reset."
        onmousedown={(e) => beginResize('sidebar', e)}
        ondblclick={() => uiStore.resetSidebarWidth()}
      ></button>
    {/if}

    <main class="main-area">
      <header class="main-header">
        <Breadcrumb />
      </header>
      <div class="main-content">
        <TreeView />
      </div>
    </main>

    {#if !uiStore.focusMode && uiStore.inspectorVisible}
      <button
        class="split-handle inspector-handle"
        aria-label="Resize inspector"
        title="Drag to resize. Double-click to reset."
        onmousedown={(e) => beginResize('inspector', e)}
        ondblclick={() => uiStore.resetInspectorWidth()}
      ></button>
      <aside class="inspector" style={`width: ${uiStore.inspectorWidth}px`}>
        <Inspector />
      </aside>
    {:else if !uiStore.focusMode}
      <button
        class="inspector-restore"
        aria-label="Open inspector"
        title="Open inspector"
        onclick={() => uiStore.setInspectorVisible(true)}
      >
        Inspector
      </button>
    {/if}
  </div>
  <StatusBar />
</div>

<CommandPalette />
<QuickOpen />
<MoveDialog />
<NodeConfigDialog />
{#if uiStore.dueDatePromptOpen}
  <DueDatePrompt />
{/if}
<ToastHost />
<StartupRecovery />
{#if uiStore.zoomFeedback}
  <div class="zoom-feedback" role="status">{uiStore.zoomFeedback}</div>
{/if}

<style>
  .root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
  .app-shell {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow: hidden;
    position: relative;
  }
  .sidebar {
    min-width: 200px;
    max-width: 520px;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .main-area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-app);
  }
  .main-header {
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .main-content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .inspector {
    min-width: 260px;
    max-width: 560px;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .split-handle {
    width: 12px;
    flex: 0 0 12px;
    border: none;
    padding: 0;
    background: transparent;
    cursor: col-resize;
    position: relative;
    z-index: 5;
  }
  .split-handle::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 5px;
    width: 1px;
    background: var(--border);
  }
  .split-handle:hover,
  .split-handle:focus-visible {
    background: var(--accent-subtle);
    outline: none;
  }
  .split-handle:hover::after,
  .split-handle:focus-visible::after {
    background: var(--accent);
  }
  .inspector-restore {
    position: absolute;
    right: 10px;
    top: 58px;
    z-index: 20;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-panel);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 7px 10px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .inspector-restore:hover,
  .inspector-restore:focus-visible {
    color: var(--text-primary);
    border-color: var(--accent);
    outline: none;
  }
  .zoom-feedback {
    position: fixed;
    left: 50%;
    bottom: 34px;
    transform: translateX(-50%);
    z-index: 1300;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg-modal);
    color: var(--text-primary);
    padding: 7px 14px;
    font-size: var(--text-sm);
    font-weight: 800;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.18);
  }
</style>
