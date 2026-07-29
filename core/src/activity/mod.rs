use crate::db::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: String,
    pub node_id: String,
    pub action: String,
    pub changes: serde_json::Value,
    pub timestamp: String,
}

pub struct ActivityRepository {
    db: Arc<Database>,
}

impl ActivityRepository {
    pub fn new(db: Arc<Database>) -> Self { Self { db } }

    pub fn log(&self, node_id: &str, action: &str, changes: &serde_json::Value) -> Result<ActivityEntry> {
        let conn = self.db.conn.lock().unwrap();
        let id = Uuid::now_v7().to_string();
        let changes_str = serde_json::to_string(changes)?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO activity_log (id, node_id, action, changes, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&id, node_id, action, &changes_str, &now],
        )?;
        Ok(ActivityEntry {
            id,
            node_id: node_id.to_string(),
            action: action.to_string(),
            changes: changes.clone(),
            timestamp: now,
        })
    }

    pub fn get_for_node(&self, node_id: &str, limit: u32) -> Result<Vec<ActivityEntry>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, node_id, action, changes, timestamp FROM activity_log WHERE node_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
        )?;
        let entries = stmt.query_map(rusqlite::params![node_id, limit], |row| {
            let changes_str: String = row.get(3)?;
            Ok(ActivityEntry {
                id: row.get(0)?,
                node_id: row.get(1)?,
                action: row.get(2)?,
                changes: serde_json::from_str(&changes_str).unwrap_or_default(),
                timestamp: row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(entries)
    }
}
