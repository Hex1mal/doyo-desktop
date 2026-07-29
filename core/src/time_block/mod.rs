use crate::db::Database;
use crate::error::{Error, Result};
use crate::node::model::NodeType;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeBlock {
    pub id: String,
    pub task_id: Option<String>,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTimeBlockInput {
    pub task_id: Option<String>,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTimeBlockInput {
    pub task_id: Option<Option<String>>,
    pub title: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub all_day: Option<bool>,
    pub notes: Option<String>,
}

pub struct TimeBlockService {
    db: Arc<Database>,
}

impl TimeBlockService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<TimeBlock>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM time_blocks
             WHERE datetime(end_time) > datetime(?1)
               AND datetime(start_time) < datetime(?2)
             ORDER BY start_time, title",
        )?;
        let blocks = stmt
            .query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok(map_time_block(row))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(blocks)
    }

    pub fn create(&self, input: CreateTimeBlockInput) -> Result<TimeBlock> {
        self.validate_range(input.start_time, input.end_time)?;
        self.validate_task(input.task_id.as_deref())?;
        let id = Uuid::now_v7().to_string();
        {
            let conn = self.db.conn.lock().unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO time_blocks
                 (id, task_id, title, start_time, end_time, all_day, notes, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                params![
                    &id,
                    &input.task_id.as_deref(),
                    &input.title,
                    &input.start_time.to_rfc3339(),
                    &input.end_time.to_rfc3339(),
                    input.all_day as i32,
                    &input.notes,
                    &now,
                ],
            )?;
        }
        self.get(&id)
    }

    pub fn update(&self, id: &str, input: UpdateTimeBlockInput) -> Result<TimeBlock> {
        let current = self.get(id)?;
        let next_task_id = input.task_id.clone().unwrap_or(current.task_id.clone());
        let next_start = input.start_time.unwrap_or(current.start_time);
        let next_end = input.end_time.unwrap_or(current.end_time);
        self.validate_range(next_start, next_end)?;
        self.validate_task(next_task_id.as_deref())?;

        {
            let conn = self.db.conn.lock().unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE time_blocks
                 SET task_id = ?1,
                     title = ?2,
                     start_time = ?3,
                     end_time = ?4,
                     all_day = ?5,
                     notes = ?6,
                     updated_at = ?7
                 WHERE id = ?8",
                params![
                    &next_task_id.as_deref(),
                    input.title.unwrap_or(current.title),
                    next_start.to_rfc3339(),
                    next_end.to_rfc3339(),
                    input.all_day.unwrap_or(current.all_day) as i32,
                    input.notes.unwrap_or(current.notes),
                    &now,
                    id,
                ],
            )?;
        }
        self.get(id)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM time_blocks WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(Error::NotFound(format!("Time block not found: {}", id)));
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<TimeBlock> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row("SELECT * FROM time_blocks WHERE id = ?1", params![id], |row| {
            Ok(map_time_block(row))
        })
        .map_err(|_| Error::NotFound(format!("Time block not found: {}", id)))
    }

    fn validate_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<()> {
        if end <= start {
            return Err(Error::Validation("Time block end must be after start".into()));
        }
        Ok(())
    }

    fn validate_task(&self, task_id: Option<&str>) -> Result<()> {
        let Some(task_id) = task_id else {
            return Ok(());
        };
        let conn = self.db.conn.lock().unwrap();
        let row: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT type, deleted_at FROM nodes WHERE id = ?1",
                params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((node_type, deleted_at)) = row else {
            return Err(Error::Validation("Linked task does not exist".into()));
        };
        if NodeType::from_str(&node_type) != Some(NodeType::Task) || deleted_at.is_some() {
            return Err(Error::Validation("Time block can only link to an active task".into()));
        }
        Ok(())
    }
}

fn map_time_block(row: &rusqlite::Row) -> TimeBlock {
    TimeBlock {
        id: row.get(row.as_ref().column_index("id").unwrap()).unwrap(),
        task_id: row.get(row.as_ref().column_index("task_id").unwrap()).unwrap(),
        title: row.get(row.as_ref().column_index("title").unwrap()).unwrap(),
        start_time: row
            .get::<_, String>(row.as_ref().column_index("start_time").unwrap())
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap_or_default(),
        end_time: row
            .get::<_, String>(row.as_ref().column_index("end_time").unwrap())
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap_or_default(),
        all_day: row
            .get::<_, i32>(row.as_ref().column_index("all_day").unwrap())
            .unwrap()
            != 0,
        notes: row.get(row.as_ref().column_index("notes").unwrap()).unwrap(),
        created_at: row
            .get::<_, String>(row.as_ref().column_index("created_at").unwrap())
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap_or_default(),
        updated_at: row
            .get::<_, String>(row.as_ref().column_index("updated_at").unwrap())
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap_or_default(),
    }
}
