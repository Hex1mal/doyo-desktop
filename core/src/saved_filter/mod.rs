use crate::db::Database;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedFilter {
    pub id: String,
    pub name: String,
    pub definition: serde_json::Value,
    pub position: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSavedFilterInput {
    pub name: String,
    pub definition: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSavedFilterInput {
    pub name: Option<String>,
    pub definition: Option<serde_json::Value>,
    pub position: Option<f64>,
}

pub struct SavedFilterService {
    db: Arc<Database>,
}

impl SavedFilterService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<SavedFilter>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT * FROM saved_filters ORDER BY position, updated_at DESC")?;
        let rows = stmt
            .query_map([], map_saved_filter)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create(&self, input: CreateSavedFilterInput) -> Result<SavedFilter> {
        let name = clean_name(&input.name)?;
        validate_definition(&input.definition)?;
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let position = self.next_position()?;
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO saved_filters (id, name, definition, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![&id, &name, input.definition.to_string(), position, &now],
            )?;
        }
        self.get(&id)
    }

    pub fn update(&self, id: &str, input: UpdateSavedFilterInput) -> Result<SavedFilter> {
        let current = self.get(id)?;
        let name = match input.name {
            Some(value) => clean_name(&value)?,
            None => current.name,
        };
        let definition = input.definition.unwrap_or(current.definition);
        validate_definition(&definition)?;
        let position = input.position.unwrap_or(current.position);
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "UPDATE saved_filters
                 SET name = ?1, definition = ?2, position = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![&name, definition.to_string(), position, &now, id],
            )?;
        }
        self.get(id)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM saved_filters WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(Error::NotFound(format!("Saved filter not found: {}", id)));
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<SavedFilter> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row(
            "SELECT * FROM saved_filters WHERE id = ?1",
            params![id],
            map_saved_filter,
        )
        .map_err(|_| Error::NotFound(format!("Saved filter not found: {}", id)))
    }

    fn next_position(&self) -> Result<f64> {
        let conn = self.db.conn.lock().unwrap();
        let max: Option<f64> = conn
            .query_row("SELECT MAX(position) FROM saved_filters", [], |row| {
                row.get(0)
            })
            .optional()?
            .flatten();
        Ok(max.unwrap_or(0.0) + 1000.0)
    }
}

fn clean_name(value: &str) -> Result<String> {
    let clean = value.trim();
    if clean.is_empty() {
        return Err(Error::Validation("Saved filter name is required".into()));
    }
    Ok(clean.to_string())
}

fn validate_definition(value: &serde_json::Value) -> Result<()> {
    if !value.is_object() {
        return Err(Error::Validation(
            "Saved filter definition must be an object".into(),
        ));
    }
    Ok(())
}

fn parse_date(raw: String) -> DateTime<Utc> {
    raw.parse::<DateTime<Utc>>().unwrap_or_default()
}

fn map_saved_filter(row: &rusqlite::Row) -> rusqlite::Result<SavedFilter> {
    let raw_definition: String = row.get(row.as_ref().column_index("definition").unwrap())?;
    Ok(SavedFilter {
        id: row.get(row.as_ref().column_index("id").unwrap())?,
        name: row.get(row.as_ref().column_index("name").unwrap())?,
        definition: serde_json::from_str(&raw_definition).unwrap_or_else(|_| serde_json::json!({})),
        position: row.get(row.as_ref().column_index("position").unwrap())?,
        created_at: parse_date(row.get(row.as_ref().column_index("created_at").unwrap())?),
        updated_at: parse_date(row.get(row.as_ref().column_index("updated_at").unwrap())?),
    })
}
