use crate::activity::ActivityRepository;
use crate::db::Database;
use crate::error::{Error, Result};
use crate::node::handler;
use crate::node::model::*;
use crate::node::repository::NodeRepository;
use crate::undo::{UndoAction, UndoOp, UndoStack};
use std::sync::Arc;

pub struct NodeService {
    repo: NodeRepository,
    undo_stack: UndoStack,
    activity_repo: ActivityRepository,
}

impl NodeService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            repo: NodeRepository::new(db.clone()),
            undo_stack: UndoStack::new(),
            activity_repo: ActivityRepository::new(db),
        }
    }

    fn validate_parent_rule(&self, node_type: &NodeType, parent_id: Option<&str>) -> Result<()> {
        let parent = match parent_id {
            Some(id) => Some(self.repo.get(id)?),
            None => None,
        };

        match node_type {
            NodeType::Workspace => {
                if parent.is_some() {
                    return Err(Error::Validation(
                        "Workspace cannot be nested inside another node".into(),
                    ));
                }
            }
            NodeType::Group => match parent.as_ref().map(|p| &p.node_type) {
                Some(NodeType::Workspace) | Some(NodeType::Group) => {}
                Some(_) => {
                    return Err(Error::Validation(
                        "Group/subgroup can only be created inside a workspace, group, or subgroup"
                            .into(),
                    ));
                }
                None => {
                    return Err(Error::Validation(
                        "Group must belong to a workspace or another group".into(),
                    ));
                }
            },
            NodeType::Task => match parent.as_ref().map(|p| &p.node_type) {
                Some(NodeType::Workspace) | Some(NodeType::Group) | Some(NodeType::Task) => {}
                Some(_) => {
                    return Err(Error::Validation(
                        "Task/subtask can only be created inside a workspace, group, subgroup, task, or subtask".into(),
                    ));
                }
                None => {
                    return Err(Error::Validation(
                        "Task must belong to a workspace, group, subgroup, task, or subtask".into(),
                    ));
                }
            },
            NodeType::Note | NodeType::Attachment | NodeType::Comment => {}
        }

        Ok(())
    }

    pub fn create(&mut self, input: CreateNodeInput) -> Result<Node> {
        let mut input = input;
        self.validate_parent_rule(&input.node_type, input.parent_id.as_deref())?;
        let h = handler::get_handler(&input.node_type);
        if input.node_type == NodeType::Task && input.properties.priority.is_none() {
            let defaults = h.default_properties();
            input.properties.priority = defaults.priority;
        }
        h.validate_properties(&input.properties)?;

        let node = self.repo.create(&input)?;
        self.activity_repo.log(
            &node.id,
            "created",
            &serde_json::json!({"title": &node.title}),
        )?;
        self.undo_stack.push_create(node.clone());
        Ok(node)
    }

    pub fn get(&self, id: &str) -> Result<Node> {
        self.repo.get(id)
    }

    pub fn update(&mut self, id: &str, changes: UpdateNodeInput) -> Result<Node> {
        let before = self.repo.get(id)?;
        if let Some(ref node_type) = changes.node_type {
            self.validate_parent_rule(node_type, before.parent_id.as_deref())?;
        }
        if let Some(ref props) = changes.properties {
            let target_type = changes.node_type.as_ref().unwrap_or(&before.node_type);
            handler::get_handler(target_type).validate_properties(props)?;
        }
        let node = self.repo.update(id, &changes)?;
        self.activity_repo
            .log(id, "updated", &serde_json::to_value(&changes)?)?;
        self.undo_stack.push_update(id.to_string(), before);
        Ok(node)
    }

    pub fn replace_properties(&mut self, id: &str, properties: NodeProperties) -> Result<Node> {
        let before = self.repo.get(id)?;
        handler::get_handler(&before.node_type).validate_properties(&properties)?;
        let node = self.repo.replace_properties(id, &properties)?;
        self.activity_repo.log(
            id,
            "updated",
            &serde_json::json!({"properties": "replaced"}),
        )?;
        self.undo_stack.push_update(id.to_string(), before);
        Ok(node)
    }

    pub fn delete(&mut self, id: &str, permanent: bool) -> Result<()> {
        let node = if permanent {
            self.repo.get_any(id)?
        } else {
            self.repo.get(id)?
        };
        if !permanent {
            self.activity_repo
                .log(id, "deleted", &serde_json::json!({"title": &node.title}))?;
        }
        if permanent {
            self.repo.hard_delete(id)?;
        } else {
            self.repo.soft_delete(id, true)?;
        }
        self.undo_stack.push_delete(id.to_string(), node);
        Ok(())
    }

    pub fn get_deleted_nodes(&self) -> Result<Vec<Node>> {
        self.repo.get_deleted_nodes()
    }

    pub fn restore(&mut self, id: &str, destination_parent_id: Option<&str>) -> Result<Node> {
        let deleted = self.repo.get_any(id)?;
        if deleted.deleted_at.is_none() {
            return Err(Error::Validation("Node is not in Trash".into()));
        }

        let target_parent = match destination_parent_id {
            Some(parent_id) => Some(parent_id),
            None => deleted.parent_id.as_deref(),
        };

        self.validate_parent_rule(&deleted.node_type, target_parent)?;
        if let Some(parent_id) = target_parent {
            let parent = self.repo.get(parent_id)?;
            if parent.deleted_at.is_some() {
                return Err(Error::Validation(
                    "Choose an active destination before restoring this node".into(),
                ));
            }
        }

        let restored = self.repo.restore_subtree(id, destination_parent_id)?;
        self.activity_repo.log(
            id,
            "restored",
            &serde_json::json!({
                "destinationParentId": destination_parent_id
            }),
        )?;
        Ok(restored)
    }

    pub fn empty_trash(&mut self) -> Result<u32> {
        self.repo.empty_trash()
    }

    pub fn duplicate(&mut self, id: &str) -> Result<Node> {
        let node = self.repo.duplicate(id)?;
        self.activity_repo.log(
            &node.id,
            "created",
            &serde_json::json!({"title": &node.title, "duplicatedFrom": id}),
        )?;
        self.undo_stack.push_create(node.clone());
        Ok(node)
    }

    pub fn move_node(
        &mut self,
        id: &str,
        new_parent_id: Option<&str>,
        position: f64,
    ) -> Result<()> {
        let before = self.repo.get(id)?;
        self.validate_parent_rule(&before.node_type, new_parent_id)?;
        let before_parent_id = before.parent_id.clone();
        let before_position = before.position;
        self.repo.move_node(id, new_parent_id, position)?;
        self.activity_repo.log(
            id,
            "moved",
            &serde_json::json!({
                "fromParentId": before_parent_id,
                "toParentId": new_parent_id,
                "fromPosition": before_position,
                "toPosition": position
            }),
        )?;
        self.undo_stack
            .push_move(id.to_string(), before.parent_id.clone(), before.position);
        Ok(())
    }

    pub fn move_node_ordered(
        &mut self,
        id: &str,
        new_parent_id: Option<&str>,
        target_index: usize,
    ) -> Result<()> {
        let before = self.repo.get(id)?;
        self.validate_parent_rule(&before.node_type, new_parent_id)?;
        let before_parent_id = before.parent_id.clone();
        let before_position = before.position;
        self.repo
            .move_node_ordered(id, new_parent_id, target_index)?;
        self.activity_repo.log(
            id,
            "moved",
            &serde_json::json!({
                "fromParentId": before_parent_id,
                "toParentId": new_parent_id,
                "fromPosition": before_position,
                "targetIndex": target_index
            }),
        )?;
        self.undo_stack
            .push_move(id.to_string(), before.parent_id.clone(), before.position);
        Ok(())
    }

    pub fn reorder_children(&mut self, parent_id: &str, child_ids: &[String]) -> Result<()> {
        self.repo.reorder_children(parent_id, child_ids)?;
        self.activity_repo.log(
            parent_id,
            "reordered",
            &serde_json::json!({"childIds": child_ids}),
        )?;
        Ok(())
    }

    pub fn reorder_root_children(&mut self, child_ids: &[String]) -> Result<()> {
        if child_ids.is_empty() {
            return Ok(());
        }
        for child_id in child_ids {
            let node = self.repo.get(child_id)?;
            if node.parent_id.is_some() || node.node_type != NodeType::Workspace {
                return Err(Error::Validation(
                    "Only root workspaces can be reordered at the root".into(),
                ));
            }
        }
        self.repo.reorder_root_children(child_ids)?;
        self.activity_repo.log(
            child_ids.first().map_or("root", String::as_str),
            "reordered-root",
            &serde_json::json!({"childIds": child_ids}),
        )?;
        Ok(())
    }

    pub fn get_children(&self, parent_id: Option<&str>) -> Result<Vec<Node>> {
        self.repo.get_children(parent_id)
    }

    pub fn get_descendants(&self, id: &str) -> Result<Vec<Node>> {
        self.repo.get_descendants(id, None)
    }

    pub fn get_ancestors(&self, id: &str) -> Result<Vec<Node>> {
        self.repo.get_ancestors(id)
    }

    pub fn get_full_tree(&self, root_id: Option<&str>) -> Result<Vec<Node>> {
        self.repo.get_full_tree(root_id)
    }

    pub fn set_due_date(
        &mut self,
        id: &str,
        due_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Node> {
        self.repo.get(id)?;
        // Patch only due_date; a full-blob rewrite here would drop any property
        // key this build does not model.
        let patch = serde_json::json!({
            "due_date": due_date.map(|d| d.to_rfc3339()),
        });
        let node = self.repo.patch_properties(id, &patch)?;
        self.activity_repo.log(
            id,
            "updated",
            &serde_json::json!({"dueDate": node.properties.due_date}),
        )?;
        Ok(node)
    }

    pub fn set_priority(&mut self, id: &str, priority: i32) -> Result<Node> {
        if Priority::from_i32(priority).is_none() {
            return Err(Error::Validation(format!("Invalid priority: {}", priority)));
        }
        self.repo.get(id)?;
        let node = self
            .repo
            .patch_properties(id, &serde_json::json!({ "priority": priority }))?;
        self.activity_repo
            .log(id, "updated", &serde_json::json!({"priority": priority}))?;
        Ok(node)
    }

    pub fn toggle_complete(&mut self, id: &str) -> Result<Node> {
        let before = self.repo.get(id)?;
        if before.node_type != NodeType::Task {
            return Err(Error::Validation(
                "Only tasks and subtasks can be completed".into(),
            ));
        }
        let conn = self.repo.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let is_completed: i32 = conn.query_row(
            "SELECT is_completed FROM nodes WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )?;

        if is_completed == 0 {
            conn.execute(
                "UPDATE nodes SET is_completed = 1, completed_at = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
                rusqlite::params![&now, &now, id],
            )?;
        } else {
            conn.execute(
                "UPDATE nodes SET is_completed = 0, completed_at = NULL, updated_at = ?1, version = version + 1 WHERE id = ?2",
                rusqlite::params![&now, id],
            )?;
        }

        let node = conn
            .query_row(
                "SELECT * FROM nodes WHERE id = ?1",
                rusqlite::params![id],
                |row| Ok(super::repository::map_node(row)),
            )
            .map_err(|_| crate::error::Error::NotFound(format!("Node not found: {}", id)))?;

        drop(conn);
        self.activity_repo.log(
            id,
            "completed",
            &serde_json::json!({"isCompleted": node.is_completed}),
        )?;
        self.undo_stack.push_update(id.to_string(), before);
        Ok(node)
    }

    pub fn incomplete_task_descendant_count(&self, id: &str) -> Result<u32> {
        self.repo.incomplete_task_descendant_count(id)
    }

    pub fn set_completion(&mut self, id: &str, completed: bool, cascade: bool) -> Result<Node> {
        let before = self.repo.get(id)?;
        if before.node_type != NodeType::Task {
            return Err(Error::Validation(
                "Only tasks and subtasks can be completed".into(),
            ));
        }
        let node = self.repo.set_completion(id, completed, cascade)?;
        self.activity_repo.log(
            id,
            "completed",
            &serde_json::json!({
                "isCompleted": node.is_completed,
                "cascade": cascade
            }),
        )?;
        self.undo_stack.push_update(id.to_string(), before);
        Ok(node)
    }

    pub fn search(&self, query: &str, filters: SearchFilters) -> Result<Vec<SearchResult>> {
        self.repo.search(query, &filters)
    }

    pub fn quick_find(&self, query: &str) -> Result<Vec<Node>> {
        self.repo.quick_find(query)
    }

    pub fn get_today_tasks(&self) -> Result<Vec<Node>> {
        self.repo.get_today_tasks()
    }

    pub fn get_overdue_tasks(&self) -> Result<Vec<Node>> {
        self.repo.get_overdue_tasks()
    }

    pub fn get_node_count(&self) -> Result<u32> {
        self.repo.get_node_count()
    }

    pub fn undo(&mut self) -> Result<String> {
        let action = self.undo_stack.pop_undo().ok_or(Error::NothingToUndo)?;

        let description = action.description.clone();

        match &action.op {
            UndoOp::Create(node) => {
                // Undo create = remove node
                let _ = self.repo.hard_delete(&node.id);
                self.undo_stack.push_redo(UndoAction {
                    op: UndoOp::Create(node.clone()),
                    description: format!("Redo create \"{}\"", node.title),
                });
            }
            UndoOp::Update(id, prev_node) => {
                let current = self.repo.get(id)?;
                let changes = UpdateNodeInput {
                    title: Some(prev_node.title.clone()),
                    body: Some(prev_node.body.clone()),
                    node_type: Some(prev_node.node_type.clone()),
                    is_collapsed: Some(prev_node.is_collapsed),
                    properties: Some(prev_node.properties.clone()),
                };
                self.repo.update(id, &changes)?;
                // restore completion state
                if current.is_completed != prev_node.is_completed {
                    let _ = self.repo.toggle_complete_to(id, prev_node.is_completed);
                }
                self.undo_stack.push_redo(UndoAction {
                    op: UndoOp::Update(id.clone(), current),
                    description: format!("Redo update \"{}\"", prev_node.title),
                });
            }
            UndoOp::Delete(id, node) => {
                self.repo.restore_node(node)?;
                self.undo_stack.push_redo(UndoAction {
                    op: UndoOp::Delete(id.clone(), node.clone()),
                    description: format!("Redo delete \"{}\"", node.title),
                });
            }
            UndoOp::Move(id, prev_parent, prev_pos) => {
                let before = self.repo.get(id)?;
                self.repo.move_node(id, prev_parent.as_deref(), *prev_pos)?;
                self.undo_stack.push_redo(UndoAction {
                    op: UndoOp::Move(id.clone(), before.parent_id.clone(), before.position),
                    description: format!("Redo move \"{}\"", before.title),
                });
            }
        }

        Ok(description)
    }

    pub fn redo(&mut self) -> Result<String> {
        let action = self.undo_stack.pop_redo().ok_or(Error::NothingToRedo)?;

        let description = action.description.clone();

        match &action.op {
            UndoOp::Create(node) => {
                // Redo create = re-insert snapshot
                self.repo.restore_node(node)?;
                self.undo_stack.undo_stack.push(UndoAction {
                    op: UndoOp::Create(node.clone()),
                    description: format!("Undo create \"{}\"", node.title),
                });
            }
            UndoOp::Update(id, prev_node) => {
                let current = self.repo.get(id)?;
                let changes = UpdateNodeInput {
                    title: Some(prev_node.title.clone()),
                    body: Some(prev_node.body.clone()),
                    node_type: Some(prev_node.node_type.clone()),
                    is_collapsed: Some(prev_node.is_collapsed),
                    properties: Some(prev_node.properties.clone()),
                };
                self.repo.update(id, &changes)?;
                if current.is_completed != prev_node.is_completed {
                    let _ = self.repo.toggle_complete_to(id, prev_node.is_completed);
                }
                self.undo_stack.undo_stack.push(UndoAction {
                    op: UndoOp::Update(id.clone(), current),
                    description: format!("Undo update \"{}\"", prev_node.title),
                });
            }
            UndoOp::Delete(id, node) => {
                self.repo.soft_delete(id, true)?;
                self.undo_stack.undo_stack.push(UndoAction {
                    op: UndoOp::Delete(id.clone(), node.clone()),
                    description: format!("Undo delete \"{}\"", node.title),
                });
            }
            UndoOp::Move(id, prev_parent, prev_pos) => {
                let before = self.repo.get(id)?;
                self.repo.move_node(id, prev_parent.as_deref(), *prev_pos)?;
                self.undo_stack.undo_stack.push(UndoAction {
                    op: UndoOp::Move(id.clone(), before.parent_id.clone(), before.position),
                    description: format!("Undo move \"{}\"", before.title),
                });
            }
        }

        Ok(description)
    }

    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }
}
