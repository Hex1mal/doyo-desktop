use crate::db::Database;
use crate::error::{Error, Result};
use crate::node::model::*;
use rusqlite::{params, OptionalExtension, Transaction};
use uuid::Uuid;

pub struct NodeRepository {
    pub db: std::sync::Arc<Database>,
}

fn normalize_sibling_positions(
    tx: &Transaction<'_>,
    parent_id: Option<&str>,
    exclude_id: Option<&str>,
    insert: Option<(&str, usize)>,
    now: &str,
) -> Result<()> {
    let mut ids: Vec<String> = match parent_id {
        Some(parent_id) => {
            let mut stmt = tx.prepare(
                "SELECT id FROM nodes WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY position, created_at",
            )?;
            let rows = stmt
                .query_map(params![parent_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        }
        None => {
            let mut stmt = tx.prepare(
                "SELECT id FROM nodes WHERE parent_id IS NULL AND deleted_at IS NULL ORDER BY position, created_at",
            )?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        }
    };

    if let Some(exclude_id) = exclude_id {
        ids.retain(|id| id != exclude_id);
    }

    if let Some((insert_id, target_index)) = insert {
        ids.retain(|id| id != insert_id);
        let index = target_index.min(ids.len());
        ids.insert(index, insert_id.to_string());
    }

    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE nodes SET position = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![index as f64 * 1000.0, now, id],
        )?;
    }

    Ok(())
}

impl NodeRepository {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    pub fn create(&self, input: &CreateNodeInput) -> Result<Node> {
        let conn = self.db.conn.lock().unwrap();
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let props = serde_json::to_string(&input.properties)?;
        let position = input.position.unwrap_or_else(|| {
            // Calculate next position after last sibling
            let max_pos: Option<f64> = conn
                .query_row(
                    "SELECT MAX(position) FROM nodes WHERE parent_id IS ? AND deleted_at IS NULL",
                    params![&input.parent_id.as_deref()],
                    |row| row.get(0),
                )
                .optional()
                .unwrap_or(None);
            max_pos.map_or(0.0, |v| v + 1000.0)
        });

        conn.execute(
            "INSERT INTO nodes (id, parent_id, position, type, title, body, properties, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                &id,
                &input.parent_id.as_deref(),
                position,
                input.node_type.as_str(),
                &input.title,
                &input.body,
                &props,
                &now,
            ],
        )?;

        conn.execute(
            "INSERT INTO nodes_fts (node_id, title, body, tags) VALUES (?1, ?2, ?3, '')",
            params![&id, &input.title, &input.body],
        )?;

        let node = conn
            .query_row("SELECT * FROM nodes WHERE id = ?1", params![&id], |row| {
                Ok(map_node(row))
            })
            .map_err(|_| Error::NotFound("Failed to retrieve created node".into()))?;

        Ok(node)
    }

    pub fn get(&self, id: &str) -> Result<Node> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row(
            "SELECT * FROM nodes WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| Ok(map_node(row)),
        )
        .map_err(|_| Error::NotFound(format!("Node not found: {}", id)))
    }

    pub fn get_any(&self, id: &str) -> Result<Node> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row("SELECT * FROM nodes WHERE id = ?1", params![id], |row| {
            Ok(map_node(row))
        })
        .map_err(|_| Error::NotFound(format!("Node not found: {}", id)))
    }

    pub fn update(&self, id: &str, changes: &UpdateNodeInput) -> Result<Node> {
        let conn = self.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let version: i32 = conn.query_row(
            "SELECT version + 1 FROM nodes WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        if let Some(ref title) = changes.title {
            conn.execute(
                "UPDATE nodes SET title = ?1, updated_at = ?2, version = ?3 WHERE id = ?4",
                params![title, &now, version, id],
            )?;
        }
        if let Some(ref body) = changes.body {
            conn.execute(
                "UPDATE nodes SET body = ?1, updated_at = ?2, version = ?3 WHERE id = ?4",
                params![body, &now, version, id],
            )?;
        }
        if let Some(ref node_type) = changes.node_type {
            conn.execute(
                "UPDATE nodes SET type = ?1, updated_at = ?2, version = ?3 WHERE id = ?4",
                params![node_type.as_str(), &now, version, id],
            )?;
        }
        if let Some(is_collapsed) = changes.is_collapsed {
            conn.execute(
                "UPDATE nodes SET is_collapsed = ?1, updated_at = ?2, version = ?3 WHERE id = ?4",
                params![is_collapsed as i32, &now, version, id],
            )?;
        }
        if let Some(ref props) = changes.properties {
            // Merge at the JSON layer, not through NodeProperties. Round-tripping
            // through the struct silently drops every key it does not declare, so
            // an unrelated single-field edit could erase metadata written by
            // another view or a newer build. Unset fields skip serialization, so
            // the patch contains exactly the keys the caller intended to change.
            let patch = serde_json::to_value(props)?;
            let props_str = merged_properties_json(&conn, id, &patch)?;
            conn.execute(
                "UPDATE nodes SET properties = ?1, updated_at = ?2, version = ?3 WHERE id = ?4",
                params![&props_str, &now, version, id],
            )?;
        }

        if changes.title.is_some() || changes.body.is_some() {
            conn.execute(
                "UPDATE nodes_fts SET title = (SELECT title FROM nodes WHERE id = ?1), body = (SELECT body FROM nodes WHERE id = ?1) WHERE node_id = ?1",
                params![id],
            )?;
        }

        let node = conn
            .query_row("SELECT * FROM nodes WHERE id = ?1", params![id], |row| {
                Ok(map_node(row))
            })
            .map_err(|_| Error::NotFound(format!("Node not found: {}", id)))?;

        Ok(node)
    }

    /// Atomically change specific property keys, leaving every other key intact.
    ///
    /// This is the safe primitive for single-field intents such as "set the due
    /// date": the caller states only what changed, so no read-modify-write of the
    /// whole blob can drop metadata it did not know about. A `null` value clears
    /// the key.
    pub fn patch_properties(&self, id: &str, patch: &serde_json::Value) -> Result<Node> {
        let conn = self.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let props_str = merged_properties_json(&conn, id, patch)?;
        conn.execute(
            "UPDATE nodes SET properties = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![&props_str, &now, id],
        )?;
        conn.query_row("SELECT * FROM nodes WHERE id = ?1", params![id], |row| {
            Ok(map_node(row))
        })
        .map_err(|_| Error::NotFound(format!("Node not found: {}", id)))
    }

    /// Replace the properties this build models, leaving any others intact.
    ///
    /// Callers use this to clear fields, which a merge cannot express: a key
    /// missing from `properties` means "remove it". That authority stops at the
    /// keys `NodeProperties` can describe. The frontend builds its replacement
    /// from a normalized node that only ever contains the modelled keys, so a
    /// plain whole-blob write let an ordinary edit — picking a node colour, say —
    /// silently delete metadata written by a newer build.
    pub fn replace_properties(&self, id: &str, properties: &NodeProperties) -> Result<Node> {
        let conn = self.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        let mut merged = read_properties_object(&conn, id)?;
        // Clear the caller's surface first so omitted keys are genuinely removed.
        for key in KNOWN_PROPERTY_KEYS {
            merged.remove(key);
        }
        if let serde_json::Value::Object(replacement) = serde_json::to_value(properties)? {
            for (key, value) in replacement {
                merged.insert(key, value);
            }
        }
        let props_str = serde_json::to_string(&serde_json::Value::Object(merged))?;

        conn.execute(
            "UPDATE nodes SET properties = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![&props_str, &now, id],
        )?;
        conn.query_row("SELECT * FROM nodes WHERE id = ?1", params![id], |row| {
            Ok(map_node(row))
        })
        .map_err(|_| Error::NotFound(format!("Node not found: {}", id)))
    }

    pub fn soft_delete(&self, id: &str, cascade: bool) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        if cascade {
            // Get all descendants
            let descendants: Vec<String> = {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE subtree AS (
                        SELECT id FROM nodes WHERE id = ?1
                        UNION ALL
                        SELECT n.id FROM nodes n JOIN subtree s ON n.parent_id = s.id
                    )
                    SELECT id FROM subtree WHERE id != ?1",
                )?;
                let ids = stmt
                    .query_map(params![id], |row| row.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                ids
            };

            for descendant_id in &descendants {
                conn.execute(
                    "UPDATE nodes SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3",
                    params![&now, &now, descendant_id],
                )?;
            }
        }

        conn.execute(
            "UPDATE nodes SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![&now, &now, id],
        )?;

        Ok(())
    }

    pub fn hard_delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        // Clean FTS for node + descendants before cascade
        conn.execute(
            "DELETE FROM nodes_fts WHERE node_id IN (
                WITH RECURSIVE subtree AS (
                    SELECT id FROM nodes WHERE id = ?1
                    UNION ALL
                    SELECT n.id FROM nodes n JOIN subtree s ON n.parent_id = s.id
                )
                SELECT id FROM subtree
            )",
            params![id],
        )?;
        conn.execute("DELETE FROM nodes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_deleted_nodes(&self) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC, updated_at DESC",
        )?;
        let nodes = stmt
            .query_map([], |row| Ok(map_node(row)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn restore_subtree(&self, id: &str, new_parent_id: Option<&str>) -> Result<Node> {
        let mut conn = self.db.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();

        if let Some(parent_id) = new_parent_id {
            let cycle_count: i32 = tx.query_row(
                "WITH RECURSIVE subtree AS (
                    SELECT id FROM nodes WHERE id = ?1
                    UNION ALL
                    SELECT n.id FROM nodes n JOIN subtree s ON n.parent_id = s.id
                )
                SELECT COUNT(*) FROM subtree WHERE id = ?2",
                params![id, parent_id],
                |row| row.get(0),
            )?;
            if cycle_count > 0 {
                return Err(Error::CycleDetected);
            }
            tx.execute(
                "UPDATE nodes SET parent_id = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
                params![parent_id, &now, id],
            )?;
        }

        tx.execute(
            "WITH RECURSIVE subtree AS (
                SELECT id FROM nodes WHERE id = ?1
                UNION ALL
                SELECT n.id FROM nodes n JOIN subtree s ON n.parent_id = s.id
            )
            UPDATE nodes
            SET deleted_at = NULL, updated_at = ?2, version = version + 1
            WHERE id IN (SELECT id FROM subtree)",
            params![id, &now],
        )?;

        let node = tx.query_row("SELECT * FROM nodes WHERE id = ?1", params![id], |row| {
            Ok(map_node(row))
        })?;
        tx.commit()?;
        Ok(node)
    }

    pub fn empty_trash(&self) -> Result<u32> {
        let conn = self.db.conn.lock().unwrap();
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE deleted_at IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "DELETE FROM nodes_fts WHERE node_id IN (SELECT id FROM nodes WHERE deleted_at IS NOT NULL)",
            [],
        )?;
        conn.execute("DELETE FROM nodes WHERE deleted_at IS NOT NULL", [])?;
        Ok(count)
    }

    pub fn restore_node(&self, node: &Node) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        // Soft-deleted path: undelete
        let updated = conn.execute(
            "UPDATE nodes SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
            params![&now, &node.id],
        )?;
        if updated == 0 {
            conn.execute(
                "INSERT INTO nodes (id, parent_id, position, type, title, body, properties, is_collapsed, is_completed, completed_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    &node.id,
                    &node.parent_id.as_deref(),
                    node.position,
                    node.node_type.as_str(),
                    &node.title,
                    &node.body,
                    &serde_json::to_string(&node.properties)?,
                    node.is_collapsed as i32,
                    node.is_completed as i32,
                    &node.completed_at.map(|d| d.to_rfc3339()),
                    &node.created_at.to_rfc3339(),
                    &node.updated_at.to_rfc3339(),
                ],
            )?;
        }
        conn.execute(
            "DELETE FROM nodes_fts WHERE node_id = ?1",
            params![&node.id],
        )?;
        conn.execute(
            "INSERT INTO nodes_fts (node_id, title, body, tags) VALUES (?1, ?2, ?3, '')",
            params![&node.id, &node.title, &node.body],
        )?;
        Ok(())
    }

    pub fn toggle_complete_to(&self, id: &str, completed: bool) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        if completed {
            conn.execute(
                "UPDATE nodes SET is_completed = 1, completed_at = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
                params![&now, &now, id],
            )?;
        } else {
            conn.execute(
                "UPDATE nodes SET is_completed = 0, completed_at = NULL, updated_at = ?1, version = version + 1 WHERE id = ?2",
                params![&now, id],
            )?;
        }
        Ok(())
    }

    pub fn incomplete_task_descendant_count(&self, id: &str) -> Result<u32> {
        let conn = self.db.conn.lock().unwrap();
        let count: u32 = conn.query_row(
            "WITH RECURSIVE descendants AS (
                SELECT id FROM nodes WHERE id = ?1 AND deleted_at IS NULL
                UNION ALL
                SELECT n.id FROM nodes n JOIN descendants d ON n.parent_id = d.id
                WHERE n.deleted_at IS NULL
            )
            SELECT COUNT(*) FROM nodes
            WHERE id IN (SELECT id FROM descendants WHERE id != ?1)
              AND type = 'Task'
              AND is_completed = 0
              AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn set_completion(&self, id: &str, completed: bool, cascade: bool) -> Result<Node> {
        let mut conn = self.db.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();

        let node_type: String = tx
            .query_row(
                "SELECT type FROM nodes WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| Error::NotFound(format!("Node not found: {}", id)))?;

        if node_type != "Task" {
            return Err(Error::Validation(
                "Only tasks and subtasks can be completed".into(),
            ));
        }

        if completed {
            if cascade {
                tx.execute(
                    "WITH RECURSIVE descendants AS (
                        SELECT id FROM nodes WHERE id = ?1 AND deleted_at IS NULL
                        UNION ALL
                        SELECT n.id FROM nodes n JOIN descendants d ON n.parent_id = d.id
                        WHERE n.deleted_at IS NULL
                    )
                    UPDATE nodes
                    SET is_completed = 1,
                        completed_at = COALESCE(completed_at, ?2),
                        updated_at = ?2,
                        version = version + 1
                    WHERE id IN (SELECT id FROM descendants)
                      AND type = 'Task'
                      AND deleted_at IS NULL",
                    params![id, &now],
                )?;
            } else {
                tx.execute(
                    "UPDATE nodes
                     SET is_completed = 1,
                         completed_at = COALESCE(completed_at, ?1),
                         updated_at = ?1,
                         version = version + 1
                     WHERE id = ?2 AND type = 'Task' AND deleted_at IS NULL",
                    params![&now, id],
                )?;
            }
        } else {
            tx.execute(
                "UPDATE nodes
                 SET is_completed = 0,
                     completed_at = NULL,
                     updated_at = ?1,
                     version = version + 1
                 WHERE id = ?2 AND type = 'Task' AND deleted_at IS NULL",
                params![&now, id],
            )?;
        }

        let node = tx
            .query_row("SELECT * FROM nodes WHERE id = ?1", params![id], |row| {
                Ok(map_node(row))
            })
            .map_err(|_| Error::NotFound(format!("Node not found: {}", id)))?;
        tx.commit()?;
        Ok(node)
    }

    pub fn move_node(&self, id: &str, new_parent_id: Option<&str>, position: f64) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        // Cycle detection: new_parent must not be in the subtree of id
        if let Some(ref parent_id) = new_parent_id {
            let cycle_count: i32 = conn.query_row(
                "WITH RECURSIVE descendants AS (
                    SELECT id FROM nodes WHERE id = ?1
                    UNION ALL
                    SELECT n.id FROM nodes n JOIN descendants d ON n.parent_id = d.id
                )
                SELECT COUNT(*) FROM descendants WHERE id = ?2",
                params![id, parent_id],
                |row| row.get(0),
            )?;
            if cycle_count > 0 {
                return Err(Error::CycleDetected);
            }
        }

        conn.execute(
            "UPDATE nodes SET parent_id = ?1, position = ?2, updated_at = ?3, version = version + 1 WHERE id = ?4",
            params![&new_parent_id, position, &now, id],
        )?;

        Ok(())
    }

    pub fn move_node_ordered(
        &self,
        id: &str,
        new_parent_id: Option<&str>,
        target_index: usize,
    ) -> Result<()> {
        let mut conn = self.db.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();

        let old_parent_id: Option<String> = tx.query_row(
            "SELECT parent_id FROM nodes WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;

        if let Some(ref parent_id) = new_parent_id {
            let cycle_count: i32 = tx.query_row(
                "WITH RECURSIVE descendants AS (
                    SELECT id FROM nodes WHERE id = ?1
                    UNION ALL
                    SELECT n.id FROM nodes n JOIN descendants d ON n.parent_id = d.id
                )
                SELECT COUNT(*) FROM descendants WHERE id = ?2",
                params![id, parent_id],
                |row| row.get(0),
            )?;
            if cycle_count > 0 {
                return Err(Error::CycleDetected);
            }
        }

        tx.execute(
            "UPDATE nodes SET parent_id = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![&new_parent_id, &now, id],
        )?;

        normalize_sibling_positions(&tx, old_parent_id.as_deref(), Some(id), None, &now)?;
        normalize_sibling_positions(&tx, new_parent_id, None, Some((id, target_index)), &now)?;

        tx.commit()?;
        Ok(())
    }

    pub fn reorder_children(&self, parent_id: &str, child_ids: &[String]) -> Result<()> {
        let mut conn = self.db.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();

        for (i, child_id) in child_ids.iter().enumerate() {
            tx.execute(
                "UPDATE nodes SET position = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3 AND parent_id = ?4",
                params![i as f64 * 1000.0, &now, child_id, parent_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn reorder_root_children(&self, child_ids: &[String]) -> Result<()> {
        let mut conn = self.db.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();

        for (i, child_id) in child_ids.iter().enumerate() {
            tx.execute(
                "UPDATE nodes SET position = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3 AND parent_id IS NULL",
                params![i as f64 * 1000.0, &now, child_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_children(&self, parent_id: Option<&str>) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        match parent_id {
            Some(pid) => {
                let mut stmt = conn.prepare(
                    "SELECT * FROM nodes WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY position",
                )?;
                let nodes = stmt
                    .query_map(params![pid], |row| Ok(map_node(row)))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(nodes)
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT * FROM nodes WHERE parent_id IS NULL AND deleted_at IS NULL ORDER BY position",
                )?;
                let nodes = stmt
                    .query_map([], |row| Ok(map_node(row)))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(nodes)
            }
        }
    }

    pub fn get_descendants(&self, id: &str, max_depth: Option<u32>) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        let sql = match max_depth {
            Some(d) => format!(
                "WITH RECURSIVE subtree AS (
                    SELECT *, 0 AS depth FROM nodes WHERE id = ?1 AND deleted_at IS NULL
                    UNION ALL
                    SELECT n.*, s.depth + 1
                    FROM nodes n JOIN subtree s ON n.parent_id = s.id
                    WHERE n.deleted_at IS NULL AND s.depth < {}
                )
                SELECT * FROM subtree WHERE id != ?1 ORDER BY depth, position",
                d
            ),
            None => "WITH RECURSIVE subtree AS (
                SELECT *, 0 AS depth FROM nodes WHERE id = ?1 AND deleted_at IS NULL
                UNION ALL
                SELECT n.*, s.depth + 1
                FROM nodes n JOIN subtree s ON n.parent_id = s.id
                WHERE n.deleted_at IS NULL
            )
            SELECT * FROM subtree WHERE id != ?1 ORDER BY depth, position"
                .to_string(),
        };

        let mut stmt = conn.prepare(&sql)?;
        let nodes = stmt
            .query_map(params![id], |row| Ok(map_node(row)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn get_ancestors(&self, id: &str) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "WITH RECURSIVE ancestors AS (
                SELECT *, 0 AS level FROM nodes WHERE id = ?1
                UNION ALL
                SELECT n.*, a.level + 1
                FROM nodes n JOIN ancestors a ON n.id = a.parent_id
            )
            SELECT * FROM ancestors WHERE id != ?1 ORDER BY level DESC",
        )?;

        let nodes = stmt
            .query_map(params![id], |row| Ok(map_node(row)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn get_full_tree(&self, root_id: Option<&str>) -> Result<Vec<Node>> {
        match root_id {
            Some(id) => self.get_descendants(id, None),
            None => {
                // Every live node, flat. This used to walk down from the roots
                // with a recursive CTE, but for the whole tree that walk is pure
                // cost: soft delete cascades to descendants and hard delete
                // cascades by foreign key, so no live node can have a dead
                // ancestor and "reachable from a root" is exactly "not deleted".
                //
                // The cost was not small. SQLite planned the recursive step as a
                // search on `idx_nodes_deleted_at` — which matches nearly every
                // row — followed by a full scan of the working table, making the
                // walk quadratic: 100 ms at 1k nodes, 11 s at 10k, 5 minutes at
                // 50k, on the query the app loads its entire tree from. Flat, the
                // same rows come back in 20 ms at 10k.
                //
                // Callers sort children by position themselves, so the ordering
                // here only needs to be stable.
                let conn = self.db.conn.lock().unwrap();
                let mut stmt = conn.prepare(
                    "SELECT * FROM nodes WHERE deleted_at IS NULL ORDER BY position, created_at",
                )?;
                let nodes = stmt
                    .query_map([], |row| Ok(map_node(row)))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(nodes)
            }
        }
    }

    pub fn search(&self, query: &str, filters: &SearchFilters) -> Result<Vec<SearchResult>> {
        let conn = self.db.conn.lock().unwrap();
        let fts_query = format!("\"{}\"", query.replace('"', "\"\""));
        let mut sql = String::from(
            "SELECT n.*, snippet(nodes_fts, 1, '<mark>', '</mark>', '...', 40) AS snippet,
             rank FROM nodes_fts
             JOIN nodes n ON nodes_fts.node_id = n.id
             WHERE nodes_fts MATCH ?1 AND n.deleted_at IS NULL",
        );

        if filters.node_types.is_some()
            || filters.priority.is_some()
            || filters.is_completed.is_some()
        {
            if let Some(ref types) = filters.node_types {
                let type_list: Vec<String> =
                    types.iter().map(|t| format!("'{}'", t.as_str())).collect();
                sql.push_str(&format!(" AND n.type IN ({})", type_list.join(",")));
            }
            if let Some(priority) = filters.priority {
                sql.push_str(&format!(
                    " AND json_extract(n.properties, '$.priority') = {}",
                    priority
                ));
            }
            if let Some(completed) = filters.is_completed {
                if completed {
                    sql.push_str(" AND n.is_completed = 1");
                } else {
                    sql.push_str(" AND n.is_completed = 0");
                }
            }
        }

        sql.push_str(" ORDER BY rank LIMIT 50");

        let mut stmt = conn.prepare(&sql)?;
        let results = stmt
            .query_map(params![fts_query], |row| {
                let snippet: String = row.get(row.as_ref().column_index("snippet").unwrap())?;
                let rank: f64 = row.get(row.as_ref().column_index("rank").unwrap())?;
                let node = map_node(row);
                Ok(SearchResult {
                    node,
                    snippet,
                    breadcrumb: vec![],
                    rank,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Enrich with breadcrumbs (inline query to avoid deadlock)
        let mut enriched = Vec::new();
        for mut result in results {
            let node_id = result.node.id.clone();
            let ancestors = get_ancestors_inline(&conn, &node_id);
            if let Ok(ancestors) = ancestors {
                result.breadcrumb = ancestors.into_iter().map(|a| a.title).collect();
            }
            enriched.push(result);
        }

        Ok(enriched)
    }

    pub fn quick_find(&self, query: &str) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE title LIKE ?1 AND deleted_at IS NULL ORDER BY updated_at DESC LIMIT 20"
        )?;
        let nodes = stmt
            .query_map(params![pattern], |row| Ok(map_node(row)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn get_today_tasks(&self) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE type = 'Task'
             AND json_extract(properties, '$.due_date') IS NOT NULL
             AND date(json_extract(properties, '$.due_date')) = date('now')
             AND deleted_at IS NULL AND is_completed = 0
             ORDER BY json_extract(properties, '$.priority'), json_extract(properties, '$.due_date')"
        )?;
        let nodes = stmt
            .query_map([], |row| Ok(map_node(row)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn get_overdue_tasks(&self) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE type = 'Task'
             AND json_extract(properties, '$.due_date') IS NOT NULL
             AND datetime(json_extract(properties, '$.due_date')) < datetime('now')
             AND deleted_at IS NULL AND is_completed = 0
             ORDER BY json_extract(properties, '$.due_date') ASC",
        )?;
        let nodes = stmt
            .query_map([], |row| Ok(map_node(row)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn get_node_count(&self) -> Result<u32> {
        let conn = self.db.conn.lock().unwrap();
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn child_count(&self, id: &str) -> Result<u32> {
        let conn = self.db.conn.lock().unwrap();
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE parent_id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn duplicate(&self, id: &str) -> Result<Node> {
        let original = self.get(id)?;
        let new_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.db.conn.lock().unwrap();

        let mut title = original.title.clone();
        if !title.ends_with("(Copy)") {
            title = format!("{} (Copy)", title);
        }

        let position = original.position + 500.0;

        conn.execute(
            "INSERT INTO nodes (id, parent_id, position, type, title, body, properties, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                &new_id,
                &original.parent_id.as_deref(),
                position,
                original.node_type.as_str(),
                &title,
                &original.body,
                &serde_json::to_string(&original.properties)?,
                &now,
            ],
        )?;

        conn.execute(
            "INSERT INTO nodes_fts (node_id, title, body, tags) VALUES (?1, ?2, ?3, '')",
            params![&new_id, &title, &original.body],
        )?;

        // Duplicate children recursively (pass conn reference)
        duplicate_children_conn(&conn, &original.id, &new_id)?;

        let node = conn
            .query_row(
                "SELECT * FROM nodes WHERE id = ?1",
                params![&new_id],
                |row| Ok(map_node(row)),
            )
            .map_err(|_| Error::NotFound("Failed to get duplicated node".into()))?;

        Ok(node)
    }
}

/// Read a node's stored properties as a raw JSON object.
///
/// Parsed as `Value`, never as `NodeProperties`: keys this build does not know
/// about must survive a read/write cycle instead of being silently discarded.
pub(crate) fn read_properties_object(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let raw: String = conn.query_row(
        "SELECT properties FROM nodes WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    Ok(match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    })
}

/// Apply `patch` over a node's stored properties and return the merged JSON.
///
/// A `null` in the patch removes that key, which is how a property is cleared;
/// every other existing key is left untouched.
pub(crate) fn merged_properties_json(
    conn: &rusqlite::Connection,
    id: &str,
    patch: &serde_json::Value,
) -> Result<String> {
    let mut existing = read_properties_object(conn, id)?;
    if let serde_json::Value::Object(patch) = patch {
        for (key, value) in patch {
            if value.is_null() {
                existing.remove(key);
                continue;
            }
            // `custom` is the extension bag every productivity view writes into -
            // GTD state, Eisenhower quadrant, Kanban status, Pareto scores. Those
            // views each own a few keys and know nothing about the others, so a
            // patch touching one key must not carry away the rest. Merging by
            // sub-key against the stored value means a caller never has to send
            // (and therefore never has to hold a fresh copy of) keys it does not
            // care about.
            if key == "custom" {
                if let (Some(serde_json::Value::Object(current)), serde_json::Value::Object(next)) =
                    (existing.get(key), value)
                {
                    let mut merged = current.clone();
                    for (sub_key, sub_value) in next {
                        if sub_value.is_null() {
                            merged.remove(sub_key);
                        } else {
                            merged.insert(sub_key.clone(), sub_value.clone());
                        }
                    }
                    existing.insert(key.clone(), serde_json::Value::Object(merged));
                    continue;
                }
            }
            existing.insert(key.clone(), value.clone());
        }
    }
    Ok(serde_json::to_string(&serde_json::Value::Object(existing))?)
}

pub(crate) fn map_node(row: &rusqlite::Row) -> Node {
    Node {
        id: row.get(row.as_ref().column_index("id").unwrap()).unwrap(),
        parent_id: row
            .get(row.as_ref().column_index("parent_id").unwrap())
            .unwrap(),
        position: row
            .get(row.as_ref().column_index("position").unwrap())
            .unwrap(),
        node_type: NodeType::parse(
            &row.get::<_, String>(row.as_ref().column_index("type").unwrap())
                .unwrap(),
        )
        .unwrap_or(NodeType::Task),
        title: row
            .get(row.as_ref().column_index("title").unwrap())
            .unwrap(),
        body: row.get(row.as_ref().column_index("body").unwrap()).unwrap(),
        properties: serde_json::from_str(
            &row.get::<_, String>(row.as_ref().column_index("properties").unwrap())
                .unwrap(),
        )
        .unwrap_or_default(),
        is_collapsed: row
            .get::<_, i32>(row.as_ref().column_index("is_collapsed").unwrap())
            .unwrap()
            != 0,
        is_completed: row
            .get::<_, i32>(row.as_ref().column_index("is_completed").unwrap())
            .unwrap()
            != 0,
        completed_at: row
            .get::<_, Option<String>>(row.as_ref().column_index("completed_at").unwrap())
            .unwrap()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::Utc))
            }),
        deleted_at: row
            .get::<_, Option<String>>(row.as_ref().column_index("deleted_at").unwrap())
            .unwrap()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::Utc))
            }),
        version: row
            .get(row.as_ref().column_index("version").unwrap())
            .unwrap(),
        clock: row
            .get(row.as_ref().column_index("clock").unwrap())
            .unwrap(),
        created_at: row
            .get::<_, String>(row.as_ref().column_index("created_at").unwrap())
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap_or_default(),
        updated_at: row
            .get::<_, String>(row.as_ref().column_index("updated_at").unwrap())
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap_or_default(),
    }
}

fn get_ancestors_inline(conn: &rusqlite::Connection, id: &str) -> Result<Vec<Node>> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE ancestors AS (
            SELECT *, 0 AS level FROM nodes WHERE id = ?1
            UNION ALL
            SELECT n.*, a.level + 1
            FROM nodes n JOIN ancestors a ON n.id = a.parent_id
        )
        SELECT * FROM ancestors WHERE id != ?1 ORDER BY level DESC",
    )?;
    let nodes = stmt
        .query_map(params![id], |row| Ok(map_node(row)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(nodes)
}

fn duplicate_children_conn(
    conn: &rusqlite::Connection,
    old_parent_id: &str,
    new_parent_id: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT * FROM nodes WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY position",
    )?;
    let children: Vec<Node> = stmt
        .query_map(params![old_parent_id], |row| Ok(map_node(row)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    for child in children {
        let new_child_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO nodes (id, parent_id, position, type, title, body, properties, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                &new_child_id,
                &Some(new_parent_id.to_string()).as_deref(),
                child.position,
                child.node_type.as_str(),
                &child.title,
                &child.body,
                &serde_json::to_string(&child.properties)?,
                &now,
            ],
        )?;
        conn.execute(
            "INSERT INTO nodes_fts (node_id, title, body, tags) VALUES (?1, ?2, ?3, '')",
            params![&new_child_id, &child.title, &child.body],
        )?;

        duplicate_children_conn(conn, &child.id, &new_child_id)?;
    }
    Ok(())
}
