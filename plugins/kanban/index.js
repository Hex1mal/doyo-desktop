// Kanban Board Plugin for Doyo
// Reference implementation proving the plugin API surface

__app.log.info('Kanban Board plugin loaded');

__app.ui.registerCommand({
  id: 'kanban.open',
  name: 'Kanban: Open Board',
  category: 'Kanban',
  shortcut: 'Ctrl+Shift+K',
  execute: () => {
    __app.ui.registerView({
      id: 'kanban-view',
      name: 'Kanban Board',
      icon: '',
      render: (container) => {
        renderKanbanBoard(container);
        return () => {
          container.innerHTML = '';
        };
      },
    });
  },
});

async function renderKanbanBoard(container) {
  container.innerHTML = `
    <div class="kanban-board" style="display:flex; gap:16px; padding:16px; overflow-x:auto; height:100%;">
      <div id="col-todo" class="kanban-column" style="flex:1; min-width:250px; background:var(--bg-panel); border-radius:8px; padding:12px;">
        <h3 style="margin:0 0 12px; font-size:14px; color:var(--text-secondary);">To Do</h3>
        <div class="kanban-cards" data-col="todo" style="min-height:100px;"></div>
      </div>
      <div id="col-in-progress" class="kanban-column" style="flex:1; min-width:250px; background:var(--bg-panel); border-radius:8px; padding:12px;">
        <h3 style="margin:0 0 12px; font-size:14px; color:var(--text-secondary);">In Progress</h3>
        <div class="kanban-cards" data-col="in-progress" style="min-height:100px;"></div>
      </div>
      <div id="col-done" class="kanban-column" style="flex:1; min-width:250px; background:var(--bg-panel); border-radius:8px; padding:12px;">
        <h3 style="margin:0 0 12px; font-size:14px; color:var(--text-secondary);">Done</h3>
        <div class="kanban-cards" data-col="done" style="min-height:100px;"></div>
      </div>
    </div>
  `;

  try {
    const tasks = await __app.nodes.query({ types: ['Task'], isCompleted: false });
    for (const task of tasks) {
      const status = task.properties?.custom?.status || 'todo';
      const col = container.querySelector(`[data-col="${status}"]`);
      if (col) {
        col.appendChild(createCard(task));
      }
    }
  } catch (err) {
    __app.log.error('Failed to load tasks for Kanban:', err);
  }
}

function createCard(task) {
  const card = document.createElement('div');
  card.className = 'kanban-card';
  card.style.cssText = 'padding:12px; background:var(--bg-input); border-radius:6px; margin-bottom:8px; cursor:pointer; font-size:13px; border:1px solid var(--border);';
  card.textContent = task.title || 'Untitled';

  card.addEventListener('click', () => {
    __app.ui.navigateToNode(task.id);
  });

  card.draggable = true;
  card.addEventListener('dragstart', (e) => {
    e.dataTransfer.setData('text/plain', task.id);
  });

  return card;
}

// Drag and drop between columns
if (typeof document !== 'undefined') {
  document.addEventListener('dragover', (e) => {
    const col = e.target.closest('.kanban-column, .kanban-cards');
    if (col) {
      e.preventDefault();
    }
  });

  document.addEventListener('drop', async (e) => {
    e.preventDefault();
    const col = e.target.closest('.kanban-column, .kanban-cards');
    if (!col) return;
    const targetCol = col.querySelector('.kanban-cards') || col;
    const nodeId = e.dataTransfer.getData('text/plain');
    if (!nodeId) return;
    const newStatus = targetCol.dataset.col;

    try {
      const node = await __app.nodes.get(nodeId);
      const newProps = {
        ...node.properties,
        custom: { ...(node.properties.custom || {}), status: newStatus },
      };
      await __app.nodes.update(nodeId, { properties: newProps });
      targetCol.appendChild(e.target.closest('.kanban-card') || createCard(node));
    } catch (err) {
      __app.log.error('Kanban drop failed:', err);
    }
  });
}
