use crate::db::Database;
use crate::error::{Error, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HabitFrequency {
    Daily,
    Weekly,
}

impl HabitFrequency {
    fn as_str(&self) -> &'static str {
        match self {
            HabitFrequency::Daily => "daily",
            HabitFrequency::Weekly => "weekly",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "weekly" => HabitFrequency::Weekly,
            _ => HabitFrequency::Daily,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HabitLogStatus {
    Completed,
    Skipped,
    Partial,
}

impl HabitLogStatus {
    fn as_str(&self) -> &'static str {
        match self {
            HabitLogStatus::Completed => "completed",
            HabitLogStatus::Skipped => "skipped",
            HabitLogStatus::Partial => "partial",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "skipped" => HabitLogStatus::Skipped,
            "partial" => HabitLogStatus::Partial,
            _ => HabitLogStatus::Completed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Habit {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub color: Option<String>,
    pub frequency: HabitFrequency,
    pub days: Vec<u32>,
    pub goal: f64,
    pub goal_unit: String,
    pub start_date: NaiveDate,
    pub reminder_time: Option<String>,
    pub archived: bool,
    pub position: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitLog {
    pub id: String,
    pub habit_id: String,
    pub log_date: NaiveDate,
    pub status: HabitLogStatus,
    pub value: f64,
    pub note: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HabitSummary {
    pub active_count: usize,
    pub completed_today: usize,
    pub completion_rate: f64,
    pub best_streak: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHabitInput {
    pub title: String,
    #[serde(default)]
    pub icon: String,
    pub color: Option<String>,
    pub frequency: HabitFrequency,
    #[serde(default)]
    pub days: Vec<u32>,
    #[serde(default = "default_goal")]
    pub goal: f64,
    #[serde(default = "default_goal_unit")]
    pub goal_unit: String,
    pub start_date: NaiveDate,
    pub reminder_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateHabitInput {
    pub title: Option<String>,
    pub icon: Option<String>,
    pub color: Option<Option<String>>,
    pub frequency: Option<HabitFrequency>,
    pub days: Option<Vec<u32>>,
    pub goal: Option<f64>,
    pub goal_unit: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub reminder_time: Option<Option<String>>,
    pub archived: Option<bool>,
    pub position: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertHabitLogInput {
    pub habit_id: String,
    pub log_date: NaiveDate,
    pub status: HabitLogStatus,
    #[serde(default = "default_goal")]
    pub value: f64,
    #[serde(default)]
    pub note: String,
}

fn default_goal() -> f64 {
    1.0
}

fn default_goal_unit() -> String {
    "count".into()
}

pub struct HabitService {
    db: Arc<Database>,
}

impl HabitService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list(&self, include_archived: bool) -> Result<Vec<Habit>> {
        let conn = self.db.conn.lock().unwrap();
        let sql = if include_archived {
            "SELECT * FROM habits ORDER BY archived, position, title"
        } else {
            "SELECT * FROM habits WHERE archived = 0 ORDER BY position, title"
        };
        let mut stmt = conn.prepare(sql)?;
        let habits = stmt
            .query_map([], map_habit)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(habits)
    }

    pub fn create(&self, input: CreateHabitInput) -> Result<Habit> {
        let title = clean_title(&input.title)?;
        validate_goal(input.goal)?;
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let position = self.next_position()?;
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO habits
                 (id, title, icon, color, frequency, days, goal, goal_unit, start_date, reminder_time,
                  archived, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12, ?12)",
                params![
                    &id,
                    title,
                    input.icon,
                    input.color,
                    input.frequency.as_str(),
                    serde_json::to_string(&input.days).unwrap_or_else(|_| "[]".into()),
                    input.goal,
                    input.goal_unit.trim(),
                    input.start_date.to_string(),
                    input.reminder_time,
                    position,
                    &now,
                ],
            )?;
        }
        self.get(&id)
    }

    pub fn update(&self, id: &str, input: UpdateHabitInput) -> Result<Habit> {
        let current = self.get(id)?;
        let title = match input.title {
            Some(value) => clean_title(&value)?,
            None => current.title,
        };
        let goal = input.goal.unwrap_or(current.goal);
        validate_goal(goal)?;
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "UPDATE habits
                 SET title = ?1, icon = ?2, color = ?3, frequency = ?4, days = ?5, goal = ?6, goal_unit = ?7,
                     start_date = ?8, reminder_time = ?9, archived = ?10, position = ?11,
                     updated_at = ?12
                 WHERE id = ?13",
                params![
                    title,
                    input.icon.unwrap_or(current.icon),
                    input.color.unwrap_or(current.color),
                    input.frequency.unwrap_or(current.frequency).as_str(),
                    input.days.map(|d| serde_json::to_string(&d).unwrap_or_else(|_| "[]".into())).unwrap_or_else(|| serde_json::to_string(&current.days).unwrap_or_else(|_| "[]".into())),
                    goal,
                    input.goal_unit.unwrap_or(current.goal_unit),
                    input.start_date.unwrap_or(current.start_date).to_string(),
                    input.reminder_time.unwrap_or(current.reminder_time),
                    input.archived.unwrap_or(current.archived) as i32,
                    input.position.unwrap_or(current.position),
                    now,
                    id,
                ],
            )?;
        }
        self.get(id)
    }

    pub fn archive(&self, id: &str, archived: bool) -> Result<Habit> {
        self.update(
            id,
            UpdateHabitInput {
                archived: Some(archived),
                ..Default::default()
            },
        )
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM habits WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(Error::NotFound(format!("Habit not found: {}", id)));
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Habit> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row("SELECT * FROM habits WHERE id = ?1", params![id], map_habit)
            .map_err(|_| Error::NotFound(format!("Habit not found: {}", id)))
    }

    pub fn upsert_log(&self, input: UpsertHabitLogInput) -> Result<HabitLog> {
        self.get(&input.habit_id)?;
        if input.value < 0.0 {
            return Err(Error::Validation(
                "Habit log value cannot be negative".into(),
            ));
        }
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO habit_logs
                 (id, habit_id, log_date, status, value, note, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                 ON CONFLICT(habit_id, log_date) DO UPDATE SET
                     status = excluded.status,
                     value = excluded.value,
                     note = excluded.note,
                     updated_at = excluded.updated_at",
                params![
                    &id,
                    &input.habit_id,
                    input.log_date.to_string(),
                    input.status.as_str(),
                    input.value,
                    &input.note,
                    &now,
                ],
            )?;
        }
        self.get_log(&input.habit_id, input.log_date)
    }

    pub fn delete_log(&self, habit_id: &str, log_date: NaiveDate) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM habit_logs WHERE habit_id = ?1 AND log_date = ?2",
            params![habit_id, log_date.to_string()],
        )?;
        Ok(())
    }

    pub fn list_logs(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<HabitLog>> {
        if to < from {
            return Err(Error::Validation(
                "Habit log end date must be on or after start date".into(),
            ));
        }
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM habit_logs
             WHERE date(log_date) >= date(?1) AND date(log_date) <= date(?2)
             ORDER BY log_date DESC, updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![from.to_string(), to.to_string()], map_habit_log)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn summary(&self, from: NaiveDate, to: NaiveDate) -> Result<HabitSummary> {
        let habits = self.list(false)?;
        let logs = self.list_logs(from, to)?;
        let today = Utc::now().date_naive();
        let by_habit: HashMap<String, Vec<HabitLog>> =
            logs.into_iter().fold(HashMap::new(), |mut map, log| {
                map.entry(log.habit_id.clone()).or_default().push(log);
                map
            });
        let completed_count: usize = by_habit
            .values()
            .flatten()
            .filter(|log| log.status == HabitLogStatus::Completed)
            .count();
        let total_expected = habits
            .len()
            .saturating_mul((to - from).num_days().max(0) as usize + 1);
        let completed_today = by_habit
            .values()
            .flatten()
            .filter(|log| log.log_date == today && log.status == HabitLogStatus::Completed)
            .count();
        let best_streak = habits
            .iter()
            .map(|habit| {
                let mut dates: Vec<NaiveDate> = by_habit
                    .get(&habit.id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|log| log.status == HabitLogStatus::Completed)
                    .map(|log| log.log_date)
                    .collect();
                dates.sort();
                longest_daily_streak(&dates)
            })
            .max()
            .unwrap_or(0);
        Ok(HabitSummary {
            active_count: habits.len(),
            completed_today,
            completion_rate: if total_expected == 0 {
                0.0
            } else {
                completed_count as f64 / total_expected as f64
            },
            best_streak,
        })
    }

    fn get_log(&self, habit_id: &str, log_date: NaiveDate) -> Result<HabitLog> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row(
            "SELECT * FROM habit_logs WHERE habit_id = ?1 AND log_date = ?2",
            params![habit_id, log_date.to_string()],
            map_habit_log,
        )
        .map_err(|_| Error::NotFound("Habit log not found".into()))
    }

    fn next_position(&self) -> Result<f64> {
        let conn = self.db.conn.lock().unwrap();
        let max: Option<f64> = conn
            .query_row("SELECT MAX(position) FROM habits", [], |row| row.get(0))
            .optional()?
            .flatten();
        Ok(max.unwrap_or(0.0) + 1000.0)
    }
}

fn clean_title(value: &str) -> Result<String> {
    let clean = value.trim();
    if clean.is_empty() {
        return Err(Error::Validation("Habit title is required".into()));
    }
    Ok(clean.to_string())
}

fn validate_goal(goal: f64) -> Result<()> {
    if !goal.is_finite() || goal <= 0.0 {
        return Err(Error::Validation(
            "Habit goal must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn parse_date(raw: String) -> DateTime<Utc> {
    raw.parse::<DateTime<Utc>>().unwrap_or_default()
}

fn parse_naive_date(raw: String) -> NaiveDate {
    NaiveDate::parse_from_str(&raw, "%Y-%m-%d").unwrap_or_else(|_| Utc::now().date_naive())
}

fn longest_daily_streak(dates: &[NaiveDate]) -> i64 {
    let mut best = 0;
    let mut current = 0;
    let mut previous: Option<NaiveDate> = None;
    for date in dates {
        if previous
            .map(|prev| *date == prev + chrono::Duration::days(1))
            .unwrap_or(false)
        {
            current += 1;
        } else {
            current = 1;
        }
        best = best.max(current);
        previous = Some(*date);
    }
    best
}

fn map_habit(row: &rusqlite::Row) -> rusqlite::Result<Habit> {
    let days_raw: String = row.get(row.as_ref().column_index("days").unwrap())?;
    Ok(Habit {
        id: row.get(row.as_ref().column_index("id").unwrap())?,
        title: row.get(row.as_ref().column_index("title").unwrap())?,
        icon: row.get(row.as_ref().column_index("icon").unwrap())?,
        color: row.get(row.as_ref().column_index("color").unwrap())?,
        frequency: HabitFrequency::from_str(
            &row.get::<_, String>(row.as_ref().column_index("frequency").unwrap())?,
        ),
        days: serde_json::from_str(&days_raw).unwrap_or_default(),
        goal: row.get(row.as_ref().column_index("goal").unwrap())?,
        goal_unit: row.get(row.as_ref().column_index("goal_unit").unwrap())?,
        start_date: parse_naive_date(row.get(row.as_ref().column_index("start_date").unwrap())?),
        reminder_time: row.get(row.as_ref().column_index("reminder_time").unwrap())?,
        archived: row.get::<_, i32>(row.as_ref().column_index("archived").unwrap())? != 0,
        position: row.get(row.as_ref().column_index("position").unwrap())?,
        created_at: parse_date(row.get(row.as_ref().column_index("created_at").unwrap())?),
        updated_at: parse_date(row.get(row.as_ref().column_index("updated_at").unwrap())?),
    })
}

fn map_habit_log(row: &rusqlite::Row) -> rusqlite::Result<HabitLog> {
    Ok(HabitLog {
        id: row.get(row.as_ref().column_index("id").unwrap())?,
        habit_id: row.get(row.as_ref().column_index("habit_id").unwrap())?,
        log_date: parse_naive_date(row.get(row.as_ref().column_index("log_date").unwrap())?),
        status: HabitLogStatus::from_str(
            &row.get::<_, String>(row.as_ref().column_index("status").unwrap())?,
        ),
        value: row.get(row.as_ref().column_index("value").unwrap())?,
        note: row.get(row.as_ref().column_index("note").unwrap())?,
        created_at: parse_date(row.get(row.as_ref().column_index("created_at").unwrap())?),
        updated_at: parse_date(row.get(row.as_ref().column_index("updated_at").unwrap())?),
    })
}
