use crate::db::Database;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CountdownMode {
    Countdown,
    Countup,
}

impl CountdownMode {
    fn as_str(&self) -> &'static str {
        match self {
            CountdownMode::Countdown => "countdown",
            CountdownMode::Countup => "countup",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "countup" => CountdownMode::Countup,
            _ => CountdownMode::Countdown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Countdown {
    pub id: String,
    pub title: String,
    pub target_date: DateTime<Utc>,
    pub mode: CountdownMode,
    pub icon: String,
    pub color: Option<String>,
    pub recurrence: Option<String>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub archived: bool,
    pub position: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCountdownInput {
    pub title: String,
    pub target_date: DateTime<Utc>,
    pub mode: CountdownMode,
    #[serde(default)]
    pub icon: String,
    pub color: Option<String>,
    pub recurrence: Option<String>,
    pub reminder_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCountdownInput {
    pub title: Option<String>,
    pub target_date: Option<DateTime<Utc>>,
    pub mode: Option<CountdownMode>,
    pub icon: Option<String>,
    pub color: Option<Option<String>>,
    pub recurrence: Option<Option<String>>,
    pub reminder_at: Option<Option<DateTime<Utc>>>,
    pub archived: Option<bool>,
    pub position: Option<f64>,
}

pub struct CountdownService {
    db: Arc<Database>,
}

impl CountdownService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list(&self, include_archived: bool) -> Result<Vec<Countdown>> {
        let conn = self.db.conn.lock().unwrap();
        let sql = if include_archived {
            "SELECT * FROM countdowns ORDER BY archived, position, target_date"
        } else {
            "SELECT * FROM countdowns WHERE archived = 0 ORDER BY position, target_date"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map([], map_countdown)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create(&self, input: CreateCountdownInput) -> Result<Countdown> {
        let title = clean_title(&input.title)?;
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let position = self.next_position()?;
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO countdowns
                 (id, title, target_date, mode, icon, color, recurrence, reminder_at,
                  archived, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10, ?10)",
                params![
                    &id,
                    title,
                    input.target_date.to_rfc3339(),
                    input.mode.as_str(),
                    input.icon,
                    input.color,
                    input.recurrence,
                    input.reminder_at.map(|date| date.to_rfc3339()),
                    position,
                    &now,
                ],
            )?;
        }
        self.get(&id)
    }

    pub fn update(&self, id: &str, input: UpdateCountdownInput) -> Result<Countdown> {
        let current = self.get(id)?;
        let title = match input.title {
            Some(value) => clean_title(&value)?,
            None => current.title,
        };
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "UPDATE countdowns
                 SET title = ?1, target_date = ?2, mode = ?3, icon = ?4, color = ?5,
                     recurrence = ?6, reminder_at = ?7, archived = ?8, position = ?9,
                     updated_at = ?10
                 WHERE id = ?11",
                params![
                    title,
                    input.target_date.unwrap_or(current.target_date).to_rfc3339(),
                    input.mode.unwrap_or(current.mode).as_str(),
                    input.icon.unwrap_or(current.icon),
                    input.color.unwrap_or(current.color),
                    input.recurrence.unwrap_or(current.recurrence),
                    input
                        .reminder_at
                        .unwrap_or(current.reminder_at)
                        .map(|date| date.to_rfc3339()),
                    input.archived.unwrap_or(current.archived) as i32,
                    input.position.unwrap_or(current.position),
                    now,
                    id,
                ],
            )?;
        }
        self.get(id)
    }

    pub fn archive(&self, id: &str, archived: bool) -> Result<Countdown> {
        self.update(
            id,
            UpdateCountdownInput {
                archived: Some(archived),
                ..Default::default()
            },
        )
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM countdowns WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(Error::NotFound(format!("Countdown not found: {}", id)));
        }
        Ok(())
    }

    pub fn reorder(&self, ids: &[String]) -> Result<Vec<Countdown>> {
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.db.conn.lock().unwrap();
            for (index, id) in ids.iter().enumerate() {
                conn.execute(
                    "UPDATE countdowns SET position = ?1, updated_at = ?2 WHERE id = ?3",
                    params![(index as f64 + 1.0) * 1000.0, &now, id],
                )?;
            }
        }
        self.list(true)
    }

    pub fn get(&self, id: &str) -> Result<Countdown> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row("SELECT * FROM countdowns WHERE id = ?1", params![id], map_countdown)
            .map_err(|_| Error::NotFound(format!("Countdown not found: {}", id)))
    }

    fn next_position(&self) -> Result<f64> {
        let conn = self.db.conn.lock().unwrap();
        let max: Option<f64> = conn
            .query_row("SELECT MAX(position) FROM countdowns", [], |row| row.get(0))
            .optional()?
            .flatten();
        Ok(max.unwrap_or(0.0) + 1000.0)
    }
}

fn clean_title(value: &str) -> Result<String> {
    let clean = value.trim();
    if clean.is_empty() {
        return Err(Error::Validation("Countdown title is required".into()));
    }
    Ok(clean.to_string())
}

fn parse_date(raw: String) -> DateTime<Utc> {
    raw.parse::<DateTime<Utc>>().unwrap_or_default()
}

fn map_countdown(row: &rusqlite::Row) -> rusqlite::Result<Countdown> {
    let reminder: Option<String> = row.get(row.as_ref().column_index("reminder_at").unwrap())?;
    Ok(Countdown {
        id: row.get(row.as_ref().column_index("id").unwrap())?,
        title: row.get(row.as_ref().column_index("title").unwrap())?,
        target_date: parse_date(row.get(row.as_ref().column_index("target_date").unwrap())?),
        mode: CountdownMode::from_str(&row.get::<_, String>(row.as_ref().column_index("mode").unwrap())?),
        icon: row.get(row.as_ref().column_index("icon").unwrap())?,
        color: row.get(row.as_ref().column_index("color").unwrap())?,
        recurrence: row.get(row.as_ref().column_index("recurrence").unwrap())?,
        reminder_at: reminder.and_then(|raw| raw.parse::<DateTime<Utc>>().ok()),
        archived: row.get::<_, i32>(row.as_ref().column_index("archived").unwrap())? != 0,
        position: row.get(row.as_ref().column_index("position").unwrap())?,
        created_at: parse_date(row.get(row.as_ref().column_index("created_at").unwrap())?),
        updated_at: parse_date(row.get(row.as_ref().column_index("updated_at").unwrap())?),
    })
}
