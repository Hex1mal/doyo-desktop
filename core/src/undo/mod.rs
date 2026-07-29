use crate::node::model::Node;

#[derive(Debug, Clone)]
pub enum UndoOp {
    /// Undo create = delete this node; redo = re-insert snapshot
    Create(Node),
    Update(String, Node),
    Delete(String, Node),
    Move(String, Option<String>, f64),
}

#[derive(Debug, Clone)]
pub struct UndoAction {
    pub op: UndoOp,
    pub description: String,
}

pub struct UndoStack {
    pub(crate) undo_stack: Vec<UndoAction>,
    pub(crate) redo_stack: Vec<UndoAction>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push_create(&mut self, node: Node) {
        let title = node.title.clone();
        self.undo_stack.push(UndoAction {
            op: UndoOp::Create(node),
            description: format!("Created \"{}\"", title),
        });
        self.redo_stack.clear();
    }

    pub fn push_update(&mut self, id: String, prev: Node) {
        self.undo_stack.push(UndoAction {
            op: UndoOp::Update(id.clone(), prev),
            description: format!("Updated node {}", id),
        });
        self.redo_stack.clear();
    }

    pub fn push_delete(&mut self, id: String, node: Node) {
        self.undo_stack.push(UndoAction {
            op: UndoOp::Delete(id.clone(), node),
            description: format!("Deleted node {}", id),
        });
        self.redo_stack.clear();
    }

    pub fn push_move(&mut self, id: String, prev_parent: Option<String>, prev_pos: f64) {
        self.undo_stack.push(UndoAction {
            op: UndoOp::Move(id.clone(), prev_parent, prev_pos),
            description: format!("Moved node {}", id),
        });
        self.redo_stack.clear();
    }

    pub fn pop_undo(&mut self) -> Option<UndoAction> {
        self.undo_stack.pop()
    }

    pub fn push_redo(&mut self, action: UndoAction) {
        self.redo_stack.push(action);
    }

    pub fn pop_redo(&mut self) -> Option<UndoAction> {
        self.redo_stack.pop()
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }
}
