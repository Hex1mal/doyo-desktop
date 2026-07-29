use crate::db::Database;
use crate::error::Result;
use std::path::Path;
use std::sync::Arc;

fn map_export_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "id": row.get::<_, String>("id")?,
        "parentId": row.get::<_, Option<String>>("parent_id")?,
        "position": row.get::<_, f64>("position")?,
        "type": row.get::<_, String>("type")?,
        "title": row.get::<_, String>("title")?,
        "body": row.get::<_, String>("body")?,
        "properties": row.get::<_, String>("properties")?,
        "isCollapsed": row.get::<_, i32>("is_collapsed")? != 0,
        "isCompleted": row.get::<_, i32>("is_completed")? != 0,
        "completedAt": row.get::<_, Option<String>>("completed_at")?,
        "createdAt": row.get::<_, String>("created_at")?,
        "updatedAt": row.get::<_, String>("updated_at")?,
    }))
}

pub struct ExportService {
    db: Arc<Database>,
}

impl ExportService {
    pub fn new(db: Arc<Database>) -> Self { Self { db } }

    pub fn export_json(&self, root_id: Option<&str>) -> Result<String> {
        let conn = self.db.conn.lock().unwrap();
        let nodes: Vec<serde_json::Value> = match root_id {
            Some(id) => {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE subtree AS (
                        SELECT *, 0 AS depth FROM nodes WHERE id = ?1 AND deleted_at IS NULL
                        UNION ALL
                        SELECT n.*, s.depth + 1 FROM nodes n JOIN subtree s ON n.parent_id = s.id
                        WHERE n.deleted_at IS NULL
                    )
                    SELECT * FROM subtree ORDER BY depth, position"
                )?;
                let results = stmt.query_map(rusqlite::params![id], map_export_row)?.filter_map(|r| r.ok()).collect();
                results
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT * FROM nodes WHERE deleted_at IS NULL ORDER BY type, position"
                )?;
                let results = stmt.query_map([], map_export_row)?.filter_map(|r| r.ok()).collect();
                results
            }
        };

        Ok(serde_json::to_string_pretty(&nodes)?)
    }

    pub fn export_markdown(&self, root_id: Option<&str>, output_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;
        let conn = self.db.conn.lock().unwrap();

        let nodes: Vec<(String, Option<String>, String, String, String)> = match root_id {
            Some(id) => {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE subtree AS (
                        SELECT id, parent_id, title, body, type, 0 AS depth FROM nodes WHERE id = ?1 AND deleted_at IS NULL
                        UNION ALL
                        SELECT n.id, n.parent_id, n.title, n.body, n.type, s.depth + 1
                        FROM nodes n JOIN subtree s ON n.parent_id = s.id WHERE n.deleted_at IS NULL
                    )
                    SELECT id, parent_id, title, body, type FROM subtree ORDER BY depth, position"
                )?;
                let results = stmt.query_map(rusqlite::params![id], |row| {
                    Ok((
                        row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
                    ))
                })?.filter_map(|r| r.ok()).collect();
                results
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, parent_id, title, body, type FROM nodes WHERE deleted_at IS NULL ORDER BY position"
                )?;
                let results = stmt.query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                })?.filter_map(|r| r.ok()).collect();
                results
            }
        };

        for (_id, _parent_id, title, body, _node_type) in &nodes {
            let safe_title = title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            let filename = format!("{}.md", safe_title);
            let filepath = output_dir.join(&filename);
            let content = format!("# {}\n\n{}", title, body);
            std::fs::write(filepath, content)?;
        }

        Ok(())
    }
}
